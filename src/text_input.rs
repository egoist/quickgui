use std::{
    borrow::Cow, cell::RefCell, collections::VecDeque, mem::size_of, ops::Range, rc::Rc, sync::Arc,
};
use web_time::{Duration, Instant};

use unicode_segmentation::UnicodeSegmentation;

#[cfg(feature = "text-input-decorations")]
use crate::TextInputIndentation;
use crate::{
    FontFallbacks, FontFamily, FontFeatures, HighlightStyle, PopoverMenuItem, TextHighlight,
    element::InputConstraints,
    spell::{
        Autocorrection, MAX_MISSPELLED_RANGES, MAX_SPELL_GUESSES, MAX_SPELL_WORD_BYTES,
        MAX_SPELLCHECK_BYTES, Misspelling, MisspellingKind, SPELL_CHECK_SETTLE_DELAY,
        SpellDocument, SpellDocumentTag, SpellingMenuLabels, TextCheckingPolicy, completes_word,
        default_text_checking, grammar_highlight_style, misspelling_highlight_style,
        smart_substitution, spell_check_provider, spelling_menu_items, word_range_at,
    },
    styled_text::MAX_TEXT_HIGHLIGHTS,
};

#[derive(Clone, Debug)]
pub(crate) struct TextInputState {
    text: Arc<str>,
    highlights: Arc<[TextHighlight]>,
    multiline: bool,
    constraints: InputConstraints,
    anchor: usize,
    caret: usize,
    marked: Option<Range<usize>>,
    composition_backup: Option<CompositionBackup>,
    preferred_x: Option<f32>,
    undo: VecDeque<EditSnapshot>,
    redo: VecDeque<EditSnapshot>,
    undo_bytes: usize,
    redo_bytes: usize,
    edit_group: Option<EditGroup>,
    /// Flagged ranges from the last settled check, sorted and non-overlapping.
    misspellings: Vec<Misspelling>,
    /// Bumped whenever `misspellings` changes so the projection cache can be validated cheaply.
    misspelling_revision: u64,
    /// The single one-shot deadline armed by the last accepted edit.
    settle_deadline: Option<Instant>,
    /// An accepted edit that has not yet been timestamped by the runtime's next pump.
    settle_pending: bool,
    last_autocorrection: Option<Autocorrection>,
    /// One provider checking session, released when this input unmounts.
    document: Option<Rc<SpellDocument>>,
    highlight_cache: RefCell<Option<HighlightCache>>,
    #[cfg(feature = "text-input-decorations")]
    line_starts_cache: RefCell<Option<LineStartsCache>>,
    /// The caret blink QuickGUI paints while this input is focused.
    blink: Option<CaretBlink>,
}

/// The phase of one focused input's blinking caret.
///
/// The caret is solid for one half period after every edit or caret move and then alternates, as
/// AppKit's does. Nothing here runs on its own: the paint that draws the caret derives the phase
/// from its own time, and the runtime wakes exactly once per toggle while an input is focused.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CaretBlink {
    /// When the caret last became solid: focus, an edit, or a caret move.
    epoch: Instant,
    /// The caret and text length the epoch was armed for, so a change restarts the phase.
    caret: usize,
    len: usize,
    /// The phase the last paint drew, so a due toggle is reported exactly once.
    painted_phase: u64,
}

/// Half of the caret blink period: solid for this long, then hidden for this long.
pub const CARET_BLINK_HALF_PERIOD: Duration = Duration::from_millis(530);

impl CaretBlink {
    fn phase(&self, now: Instant) -> u64 {
        let elapsed = now.saturating_duration_since(self.epoch);
        (elapsed.as_micros() / CARET_BLINK_HALF_PERIOD.as_micros()) as u64
    }

    /// The next instant the caret toggles.
    fn next_toggle(&self, now: Instant) -> Instant {
        self.epoch + CARET_BLINK_HALF_PERIOD * (self.phase(now) as u32 + 1)
    }
}

/// One memoized merge of controlled runs and projected spelling runs.
#[derive(Clone, Debug)]
struct HighlightCache {
    caret: usize,
    revision: u64,
    base: Arc<[TextHighlight]>,
    merged: Arc<[TextHighlight]>,
}

#[cfg(feature = "text-input-decorations")]
#[derive(Clone, Debug)]
struct LineStartsCache {
    text: Arc<str>,
    starts: Arc<[usize]>,
}

#[derive(Clone, Debug)]
struct CompositionBackup {
    text: Arc<str>,
    highlights: Arc<[TextHighlight]>,
    anchor: usize,
    caret: usize,
}

#[derive(Clone, Debug)]
struct EditSnapshot {
    text: Arc<str>,
    highlights: Arc<[TextHighlight]>,
    anchor: usize,
    caret: usize,
}

impl EditSnapshot {
    fn retained_bytes(&self) -> usize {
        self.text
            .len()
            .saturating_add(
                self.highlights
                    .len()
                    .saturating_mul(size_of::<TextHighlight>()),
            )
            .saturating_add(self.highlights.iter().fold(0_usize, |bytes, highlight| {
                let family_bytes = match highlight.style.family.as_ref() {
                    Some(FontFamily::Named(name)) => name.len(),
                    Some(FontFamily::SansSerif | FontFamily::Serif | FontFamily::Monospace)
                    | None => 0,
                };
                let feature_bytes = highlight
                    .style
                    .features
                    .as_ref()
                    .map_or(0, FontFeatures::retained_bytes);
                let fallback_bytes = highlight
                    .style
                    .fallbacks
                    .as_ref()
                    .and_then(Option::as_ref)
                    .map_or(0, FontFallbacks::retained_bytes);
                bytes
                    .saturating_add(family_bytes)
                    .saturating_add(feature_bytes)
                    .saturating_add(fallback_bytes)
            }))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EditKind {
    Typing,
    Backspace,
    Delete,
}

#[derive(Clone, Copy, Debug)]
struct EditGroup {
    kind: EditKind,
    caret_after: usize,
    last_edit_at: Instant,
}

const MAX_HISTORY_ENTRIES: usize = 100;
const MAX_HISTORY_BYTES_PER_STACK: usize = 512 * 1024;
const EDIT_COALESCE_WINDOW: Duration = Duration::from_secs(1);

impl TextInputState {
    #[cfg(test)]
    pub fn new(value: &str) -> Self {
        Self::with_mode(value, false)
    }

    #[cfg(test)]
    pub fn with_mode(value: &str, multiline: bool) -> Self {
        Self::with_constraints(value, multiline, InputConstraints::default())
    }

    #[cfg(test)]
    pub(crate) fn with_constraints(
        value: &str,
        multiline: bool,
        constraints: InputConstraints,
    ) -> Self {
        Self::with_styling(value, multiline, constraints, Arc::from([]))
    }

    pub(crate) fn with_styling(
        value: &str,
        multiline: bool,
        constraints: InputConstraints,
        highlights: Arc<[TextHighlight]>,
    ) -> Self {
        let normalized = normalize_text(value, multiline);
        let highlights = if matches!(&normalized, Cow::Borrowed(_)) {
            highlights
        } else {
            normalize_highlights(value, &highlights)
        };
        Self {
            text: Arc::from(normalized.as_ref()),
            highlights,
            multiline,
            constraints,
            anchor: normalized.len(),
            caret: normalized.len(),
            marked: None,
            composition_backup: None,
            preferred_x: None,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            undo_bytes: 0,
            redo_bytes: 0,
            edit_group: None,
            misspellings: Vec::new(),
            misspelling_revision: 0,
            settle_deadline: None,
            settle_pending: false,
            last_autocorrection: None,
            document: None,
            highlight_cache: RefCell::new(None),
            #[cfg(feature = "text-input-decorations")]
            line_starts_cache: RefCell::new(None),
            blink: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_multiline(&self) -> bool {
        self.multiline
    }

    #[cfg(feature = "text-input-decorations")]
    pub fn editor_behavior(&self) -> Option<TextInputIndentation> {
        self.constraints.editor
    }

    /// UTF-8 starts of every logical line, cached for the editor gutter.
    #[cfg(feature = "text-input-decorations")]
    pub fn logical_line_starts(&self) -> Arc<[usize]> {
        let mut cache = self.line_starts_cache.borrow_mut();
        if let Some(entry) = cache.as_ref()
            && (Arc::ptr_eq(&entry.text, &self.text) || entry.text == self.text)
        {
            return entry.starts.clone();
        }
        let mut starts =
            Vec::with_capacity(self.text.bytes().filter(|byte| *byte == b'\n').count() + 1);
        starts.push(0);
        starts.extend(
            self.text
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
        );
        let starts: Arc<[usize]> = starts.into();
        *cache = Some(LineStartsCache {
            text: self.text.clone(),
            starts: starts.clone(),
        });
        starts
    }

    pub fn shared_text(&self) -> Arc<str> {
        self.text.clone()
    }

    /// The controlled run table with settled spelling runs merged in.
    ///
    /// Merging happens here rather than in the retained table so an accepted edit never rewrites
    /// controlled styles and undo snapshots stay free of framework decorations. The word the caret
    /// is inside is never underlined, matching native "do not flag the word being typed" behavior.
    pub fn shared_highlights(&self) -> Arc<[TextHighlight]> {
        if !self.has_visible_misspellings() {
            return self.highlights.clone();
        }
        let mut cache = self.highlight_cache.borrow_mut();
        if let Some(entry) = cache.as_ref()
            && entry.caret == self.caret
            && entry.revision == self.misspelling_revision
            && Arc::ptr_eq(&entry.base, &self.highlights)
        {
            return entry.merged.clone();
        }
        let merged = merge_spelling_highlights(&self.highlights, &self.misspellings, self.caret);
        *cache = Some(HighlightCache {
            caret: self.caret,
            revision: self.misspelling_revision,
            base: self.highlights.clone(),
            merged: merged.clone(),
        });
        merged
    }

    fn has_visible_misspellings(&self) -> bool {
        self.misspellings
            .iter()
            .any(|flagged| !contains_caret(&flagged.range(), self.caret))
    }

    /// The application-visible value, excluding an uncommitted IME preedit.
    pub fn committed_shared_text(&self) -> Arc<str> {
        self.composition_backup
            .as_ref()
            .map(|backup| backup.text.clone())
            .unwrap_or_else(|| self.text.clone())
    }

    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Whether the next focused paint must bring the caret back into the viewport.
    ///
    /// Once that paint arms the blink for the current caret and content revision, ordinary wheel
    /// scrolling is allowed to move away from the caret until another edit, caret move, or focus
    /// transition changes this state.
    pub(crate) fn caret_reveal_required(&self) -> bool {
        self.blink
            .is_none_or(|blink| blink.caret != self.caret || blink.len != self.text.len())
    }

    /// Whether the caret is drawn at `now`, arming or restarting the blink as needed.
    ///
    /// Called by the paint of a focused input: a fresh focus, an edit, or a caret move restarts
    /// the solid phase, and the phase drawn is remembered so the runtime's toggle wake-up fires
    /// once per toggle rather than every frame.
    pub(crate) fn caret_visible_at(&mut self, now: Instant) -> bool {
        let len = self.text.len();
        let restart = self
            .blink
            .is_none_or(|blink| blink.caret != self.caret || blink.len != len);
        if restart {
            self.blink = Some(CaretBlink {
                epoch: now,
                caret: self.caret,
                len,
                painted_phase: 0,
            });
            return true;
        }
        let blink = self.blink.as_mut().expect("the blink was armed above");
        let phase = blink.phase(now);
        blink.painted_phase = phase;
        phase.is_multiple_of(2)
    }

    /// Stop blinking: the input lost focus, so the next focus starts solid again.
    pub(crate) fn clear_caret_blink(&mut self) {
        self.blink = None;
    }

    /// When the blinking caret next toggles, if it is blinking at all.
    pub(crate) fn next_caret_toggle(&self, now: Instant) -> Option<Instant> {
        self.blink.map(|blink| blink.next_toggle(now))
    }

    /// Whether a toggle became due since the last paint, so the caret needs a repaint.
    pub(crate) fn caret_toggle_due(&self, now: Instant) -> bool {
        self.blink
            .is_some_and(|blink| blink.phase(now) != blink.painted_phase)
    }

    pub fn anchor(&self) -> usize {
        self.anchor
    }

    pub fn marked(&self) -> Option<Range<usize>> {
        self.marked.clone()
    }

    pub fn preferred_x(&self) -> Option<f32> {
        self.preferred_x
    }

    pub fn set_preferred_x(&mut self, value: Option<f32>) {
        self.preferred_x = value.filter(|value| value.is_finite());
    }

    #[cfg(test)]
    pub fn sync_external(&mut self, value: &str, multiline: bool, constraints: &InputConstraints) {
        self.sync_external_styled(value, multiline, constraints, &Arc::from([]));
    }

    pub fn sync_external_styled(
        &mut self,
        value: &str,
        multiline: bool,
        constraints: &InputConstraints,
        highlights: &Arc<[TextHighlight]>,
    ) {
        self.constraints = constraints.clone();
        let normalized = normalize_text(value, multiline);
        let highlights = if matches!(&normalized, Cow::Borrowed(_)) {
            highlights.clone()
        } else {
            normalize_highlights(value, highlights)
        };
        if self.multiline == multiline && self.text.as_ref() == normalized {
            self.highlights = highlights;
            self.invalidate_highlight_cache();
            return;
        }
        if self.multiline == multiline
            && self
                .composition_backup
                .as_ref()
                .is_some_and(|backup| backup.text.as_ref() == normalized)
        {
            let visible_highlights = self
                .composition_backup
                .as_ref()
                .map(|backup| {
                    let replaced = backup.anchor.min(backup.caret)..backup.anchor.max(backup.caret);
                    let inserted_len = self.marked.as_ref().map_or(0, Range::len);
                    replace_highlights(&highlights, replaced, inserted_len, self.text.len())
                })
                .unwrap_or_else(|| highlights.clone());
            self.highlights = visible_highlights;
            if let Some(backup) = &mut self.composition_backup {
                backup.highlights = highlights;
            }
            self.invalidate_highlight_cache();
            return;
        }
        self.text = Arc::from(normalized.as_ref());
        self.highlights = highlights;
        self.multiline = multiline;
        self.anchor = normalized.len();
        self.caret = normalized.len();
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        self.clear_history();
        self.discard_misspellings();
        self.arm_settle_deadline();
    }

    pub fn move_left(&mut self, extend: bool) -> bool {
        let selection = self.selection();
        let next = if !extend && !selection.is_empty() {
            selection.start
        } else {
            previous_boundary(&self.text, self.caret)
        };
        self.move_to(next, extend)
    }

    pub fn move_right(&mut self, extend: bool) -> bool {
        let selection = self.selection();
        let next = if !extend && !selection.is_empty() {
            selection.end
        } else {
            next_boundary(&self.text, self.caret)
        };
        self.move_to(next, extend)
    }

    pub fn move_word_left(&mut self, extend: bool) -> bool {
        let selection = self.selection();
        let next = if !extend && !selection.is_empty() {
            selection.start
        } else {
            previous_word_boundary(&self.text, self.caret)
        };
        self.move_to(next, extend)
    }

    pub fn move_word_right(&mut self, extend: bool) -> bool {
        let selection = self.selection();
        let next = if !extend && !selection.is_empty() {
            selection.end
        } else {
            next_word_boundary(&self.text, self.caret)
        };
        self.move_to(next, extend)
    }

    pub fn move_home(&mut self, extend: bool) -> bool {
        self.move_to(0, extend)
    }

    pub fn move_end(&mut self, extend: bool) -> bool {
        self.move_to(self.text.len(), extend)
    }

    pub fn move_line_start(&mut self, extend: bool) -> bool {
        self.move_to(line_start(&self.text, self.caret), extend)
    }

    pub fn move_line_end(&mut self, extend: bool) -> bool {
        self.move_to(line_end(&self.text, self.caret), extend)
    }

    pub fn move_to(&mut self, index: usize, extend: bool) -> bool {
        self.edit_group = None;
        let index = boundary_at_or_before(&self.text, index.min(self.text.len()));
        let changed = self.caret != index || (!extend && self.anchor != index);
        self.caret = index;
        if !extend {
            self.anchor = index;
        }
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        changed
    }

    pub fn move_to_with_preferred_x(
        &mut self,
        index: usize,
        extend: bool,
        preferred_x: f32,
    ) -> bool {
        let changed = self.move_to(index, extend);
        self.set_preferred_x(Some(preferred_x));
        changed
    }

    pub fn select_all(&mut self) -> bool {
        self.edit_group = None;
        let changed = self.anchor != 0 || self.caret != self.text.len();
        self.anchor = 0;
        self.caret = self.text.len();
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        changed
    }

    pub fn selected_text(&self) -> Option<&str> {
        let selection = self.selection();
        (!selection.is_empty()).then(|| &self.text[selection])
    }

    pub fn set_selection(&mut self, anchor: usize, caret: usize) -> bool {
        self.edit_group = None;
        let anchor = boundary_at_or_before(&self.text, anchor.min(self.text.len()));
        let caret = boundary_at_or_before(&self.text, caret.min(self.text.len()));
        let changed = self.anchor != anchor || self.caret != caret || self.marked.is_some();
        self.anchor = anchor;
        self.caret = caret;
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        changed
    }

    pub fn set_value(&mut self, value: &str) -> bool {
        let value = normalize_text(value, self.multiline);
        self.replace_range(0..self.text.len(), &value, None)
    }

    pub fn accessibility_character_lengths(&self) -> Vec<u8> {
        selectable_character_lengths(&self.text)
    }

    pub fn accessibility_character_index(&self, byte_index: usize) -> usize {
        accessibility_character_index(&self.text, byte_index)
    }

    pub fn accessibility_byte_index(&self, character_index: usize) -> usize {
        accessibility_byte_index(&self.text, character_index)
    }

    pub fn backspace(&mut self) -> bool {
        let selection = self.selection();
        if selection.is_empty() {
            let start = previous_boundary(&self.text, self.caret);
            if start == self.caret {
                return false;
            }
            self.replace_range(start..self.caret, "", Some(EditKind::Backspace))
        } else {
            self.replace_range(selection, "", None)
        }
    }

    pub fn delete(&mut self) -> bool {
        let selection = self.selection();
        if selection.is_empty() {
            let end = next_boundary(&self.text, self.caret);
            if end == self.caret {
                return false;
            }
            self.replace_range(self.caret..end, "", Some(EditKind::Delete))
        } else {
            self.replace_range(selection, "", None)
        }
    }

    pub fn delete_word_backward(&mut self) -> bool {
        let selection = self.selection();
        let range = if selection.is_empty() {
            previous_word_boundary(&self.text, self.caret)..self.caret
        } else {
            selection
        };
        if range.is_empty() {
            return false;
        }
        self.replace_range(range, "", None)
    }

    pub fn delete_word_forward(&mut self) -> bool {
        let selection = self.selection();
        let range = if selection.is_empty() {
            self.caret..next_word_boundary(&self.text, self.caret)
        } else {
            selection
        };
        if range.is_empty() {
            return false;
        }
        self.replace_range(range, "", None)
    }

    pub fn delete_to_line_start(&mut self) -> bool {
        let selection = self.selection();
        let range = if selection.is_empty() {
            let mut start = line_start(&self.text, self.caret);
            if start == self.caret {
                start = previous_boundary(&self.text, self.caret);
            }
            start..self.caret
        } else {
            selection
        };
        if range.is_empty() {
            return false;
        }
        self.replace_range(range, "", None)
    }

    pub fn delete_to_line_end(&mut self) -> bool {
        let selection = self.selection();
        let range = if selection.is_empty() {
            let mut end = line_end(&self.text, self.caret);
            if end == self.caret {
                end = next_boundary(&self.text, self.caret);
            }
            self.caret..end
        } else {
            selection
        };
        if range.is_empty() {
            return false;
        }
        self.replace_range(range, "", None)
    }

    pub fn replace_selection(&mut self, value: &str) -> bool {
        let value = normalize_text(value, self.multiline);
        if value.is_empty() && self.selection().is_empty() && self.marked.is_none() {
            return false;
        }
        let mut range = self.marked.clone().unwrap_or_else(|| self.selection());
        let kind = (self.marked.is_none() && value.graphemes(true).count() == 1)
            .then_some(EditKind::Typing);
        if self.marked.is_some() {
            return self.replace_range(range, &value, kind);
        }

        let policy = self.text_checking_policy();
        let mut inserted: Cow<'_, str> = value;
        if policy.substitutes_on_insert()
            && let Some(substitution) =
                smart_substitution(&self.text[..range.start], &inserted, policy)
        {
            range.start = range.start.saturating_sub(substitution.remove_before);
            inserted = Cow::Owned(substitution.replacement);
        }

        if (policy.autocorrect || policy.text_replacement)
            && inserted.chars().count() == 1
            && inserted.chars().next().is_some_and(completes_word)
            && let Some((word_range, original, replacement)) =
                self.pending_word_replacement(range.start, policy)
        {
            let mut combined = replacement.clone();
            combined.push_str(&inserted);
            let corrected_range = word_range.start..word_range.start + replacement.len();
            if self.replace_range(word_range.start..range.end, &combined, None) {
                self.last_autocorrection =
                    Some(Autocorrection::new(corrected_range, original, replacement));
                return true;
            }
        }

        self.replace_range(range, &inserted, kind)
    }

    pub fn insert_newline(&mut self) -> bool {
        self.multiline && self.replace_selection("\n")
    }

    #[cfg(feature = "text-input-decorations")]
    pub fn insert_editor_newline(&mut self, behavior: TextInputIndentation) -> bool {
        if !self.multiline || !behavior.auto_indent {
            return self.insert_newline();
        }
        let selection = self.selection();
        let start = line_start(&self.text, selection.start);
        let before = &self.text[start..selection.start];
        let indentation_end = before
            .char_indices()
            .find_map(|(index, character)| (!matches!(character, ' ' | '\t')).then_some(index))
            .unwrap_or(before.len());
        let mut insertion = String::from("\n");
        insertion.push_str(&before[..indentation_end]);
        if before
            .trim_end()
            .chars()
            .next_back()
            .is_some_and(|character| matches!(character, '{' | '[' | '(' | ':'))
        {
            if behavior.insert_spaces {
                insertion.extend(std::iter::repeat_n(' ', usize::from(behavior.tab_size)));
            } else {
                insertion.push('\t');
            }
        }
        self.replace_selection(&insertion)
    }

    /// Insert or remove one indentation level. A multi-line selection changes as one undo step.
    #[cfg(feature = "text-input-decorations")]
    pub fn editor_tab(&mut self, outdent: bool, behavior: TextInputIndentation) -> bool {
        if !self.multiline {
            return false;
        }
        let behavior = behavior.sanitized();
        let selection = self.selection();
        if selection.is_empty() && !outdent {
            let start = line_start(&self.text, self.caret);
            let column = self.text[start..self.caret].chars().count();
            let count = usize::from(behavior.tab_size) - column % usize::from(behavior.tab_size);
            let insertion = if behavior.insert_spaces {
                " ".repeat(count)
            } else {
                "\t".to_owned()
            };
            return self.replace_selection(&insertion);
        }

        let first_line = line_start(&self.text, selection.start);
        let effective_end = if selection.end > selection.start
            && selection.end > 0
            && self.text.as_bytes().get(selection.end - 1) == Some(&b'\n')
        {
            selection.end - 1
        } else {
            selection.end
        };
        let last_line_end = line_end(&self.text, effective_end);
        let mut starts = vec![first_line];
        starts.extend(
            self.text[first_line..last_line_end]
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(first_line + index + 1)),
        );
        let indent = if behavior.insert_spaces {
            " ".repeat(usize::from(behavior.tab_size))
        } else {
            "\t".to_owned()
        };
        let mut edits: Vec<(usize, usize, String)> = Vec::with_capacity(starts.len());
        for start in starts {
            if outdent {
                let tail = &self.text[start..];
                let removed = if tail.starts_with('\t') {
                    1
                } else {
                    tail.bytes()
                        .take(usize::from(behavior.tab_size))
                        .take_while(|byte| *byte == b' ')
                        .count()
                };
                if removed > 0 {
                    edits.push((start, removed, String::new()));
                }
            } else {
                edits.push((start, 0, indent.clone()));
            }
        }
        if edits.is_empty() {
            return false;
        }
        let replace_end = last_line_end;
        let mut replacement = self.text[first_line..replace_end].to_owned();
        for (start, removed, inserted) in edits.iter().rev() {
            let local = start - first_line;
            replacement.replace_range(local..local + removed, inserted);
        }
        let anchor = shifted_editor_offset(self.anchor, &edits);
        let caret = shifted_editor_offset(self.caret, &edits);
        if !self.replace_range(first_line..replace_end, &replacement, None) {
            return false;
        }
        self.anchor = boundary_at_or_before(&self.text, anchor.min(self.text.len()));
        self.caret = boundary_at_or_before(&self.text, caret.min(self.text.len()));
        self.preferred_x = None;
        true
    }

    pub fn set_preedit(&mut self, value: &str, cursor: Option<(usize, usize)>) -> bool {
        #[cfg(feature = "text-input-decorations")]
        if !value.is_empty()
            && self
                .editor_behavior()
                .is_some_and(|behavior| behavior.read_only)
        {
            return false;
        }
        self.edit_group = None;
        let value = normalize_text(value, self.multiline);
        let previous_text = self.text.clone();
        let previous_anchor = self.anchor;
        let previous_caret = self.caret;
        let previous_marked = self.marked.clone();
        let previous_backup = self.composition_backup.clone();

        if value.is_empty() {
            if let Some(backup) = self.composition_backup.take() {
                self.text = backup.text;
                self.highlights = backup.highlights;
                self.anchor = backup.anchor;
                self.caret = backup.caret;
            }
            self.marked = None;
            return self.text != previous_text
                || self.anchor != previous_anchor
                || self.caret != previous_caret
                || self.marked != previous_marked;
        }

        let backup = self
            .composition_backup
            .take()
            .unwrap_or_else(|| CompositionBackup {
                text: self.text.clone(),
                highlights: self.highlights.clone(),
                anchor: self.anchor,
                caret: self.caret,
            });
        let range = self.marked.take().unwrap_or_else(|| self.selection());
        let start = range.start;
        self.replace_range_unchecked(range, &value);
        self.composition_backup = Some(backup);

        let end = start + value.len();
        self.marked = Some(start..end);
        if let Some((cursor_start, cursor_end)) = cursor {
            let local_start = boundary_at_or_before(&value, cursor_start.min(value.len()));
            let local_end = boundary_at_or_before(&value, cursor_end.min(value.len()));
            self.anchor = start + local_start;
            self.caret = start + local_end;
        } else {
            self.anchor = end;
            self.caret = end;
        }
        self.text != previous_text
            || self.anchor != previous_anchor
            || self.caret != previous_caret
            || self.marked != previous_marked
            || self.composition_backup.is_some() != previous_backup.is_some()
    }

    pub fn can_undo(&self) -> bool {
        #[cfg(feature = "text-input-decorations")]
        if self
            .editor_behavior()
            .is_some_and(|behavior| behavior.read_only)
        {
            return false;
        }
        self.undo
            .back()
            .is_some_and(|snapshot| self.accepts_existing(&snapshot.text))
    }

    pub fn can_redo(&self) -> bool {
        #[cfg(feature = "text-input-decorations")]
        if self
            .editor_behavior()
            .is_some_and(|behavior| behavior.read_only)
        {
            return false;
        }
        self.redo
            .back()
            .is_some_and(|snapshot| self.accepts_existing(&snapshot.text))
    }

    pub fn undo(&mut self) -> bool {
        #[cfg(feature = "text-input-decorations")]
        if self
            .editor_behavior()
            .is_some_and(|behavior| behavior.read_only)
        {
            return false;
        }
        self.edit_group = None;
        if !self.can_undo() {
            return false;
        }
        let Some(snapshot) = self.undo.pop_back() else {
            return false;
        };
        self.undo_bytes = self.undo_bytes.saturating_sub(snapshot.retained_bytes());
        let current = self.snapshot();
        push_bounded_history(&mut self.redo, &mut self.redo_bytes, current);
        self.restore(snapshot);
        true
    }

    pub fn redo(&mut self) -> bool {
        #[cfg(feature = "text-input-decorations")]
        if self
            .editor_behavior()
            .is_some_and(|behavior| behavior.read_only)
        {
            return false;
        }
        self.edit_group = None;
        if !self.can_redo() {
            return false;
        }
        let Some(snapshot) = self.redo.pop_back() else {
            return false;
        };
        self.redo_bytes = self.redo_bytes.saturating_sub(snapshot.retained_bytes());
        let current = self.snapshot();
        push_bounded_history(&mut self.undo, &mut self.undo_bytes, current);
        self.restore(snapshot);
        true
    }

    fn snapshot(&self) -> EditSnapshot {
        self.composition_backup
            .as_ref()
            .map(|backup| EditSnapshot {
                text: backup.text.clone(),
                highlights: backup.highlights.clone(),
                anchor: backup.anchor,
                caret: backup.caret,
            })
            .unwrap_or_else(|| EditSnapshot {
                text: self.text.clone(),
                highlights: self.highlights.clone(),
                anchor: self.anchor,
                caret: self.caret,
            })
    }

    fn record_edit(&mut self, kind: Option<EditKind>, caret_after: usize) {
        let now = Instant::now();
        let coalesced = kind.is_some_and(|kind| {
            self.edit_group.is_some_and(|group| {
                group.kind == kind
                    && group.caret_after == self.caret
                    && now.saturating_duration_since(group.last_edit_at) <= EDIT_COALESCE_WINDOW
            })
        });
        if !coalesced {
            let snapshot = self.snapshot();
            push_bounded_history(&mut self.undo, &mut self.undo_bytes, snapshot);
            self.redo.clear();
            self.redo_bytes = 0;
        }
        self.edit_group = kind.map(|kind| EditGroup {
            kind,
            caret_after,
            last_edit_at: now,
        });
    }

    fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.undo_bytes = 0;
        self.redo_bytes = 0;
        self.edit_group = None;
    }

    fn restore(&mut self, snapshot: EditSnapshot) {
        self.text = snapshot.text;
        self.highlights = snapshot.highlights;
        self.anchor = snapshot.anchor;
        self.caret = snapshot.caret;
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        self.edit_group = None;
        self.last_autocorrection = None;
        self.discard_misspellings();
        self.arm_settle_deadline();
    }

    fn replace_range(
        &mut self,
        range: Range<usize>,
        value: &str,
        edit_kind: Option<EditKind>,
    ) -> bool {
        #[cfg(feature = "text-input-decorations")]
        if self
            .editor_behavior()
            .is_some_and(|behavior| behavior.read_only)
        {
            return self.cancel_composition();
        }
        let Some((text, end)) = self.constrained_replacement(range.clone(), value) else {
            self.edit_group = None;
            return self.cancel_composition();
        };
        let committed_before = self.snapshot().text;
        let changed = self.text.as_ref() != text
            || self.anchor != end
            || self.caret != end
            || self.marked.is_some()
            || self.composition_backup.is_some();
        if !changed {
            return false;
        }
        if committed_before.as_ref() != text {
            self.record_edit(edit_kind, end);
        }
        self.highlights = replace_highlights(
            &self.highlights,
            range.clone(),
            end.saturating_sub(range.start),
            text.len(),
        );
        self.text = Arc::from(text);
        self.anchor = end;
        self.caret = end;
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        self.last_autocorrection = None;
        self.shift_misspellings(range.start..range.end, end.saturating_sub(range.start));
        self.arm_settle_deadline();
        self.invalidate_highlight_cache();
        true
    }

    fn constrained_replacement(&self, range: Range<usize>, value: &str) -> Option<(String, usize)> {
        debug_assert!(range.start <= range.end && range.end <= self.text.len());
        debug_assert!(self.text.is_char_boundary(range.start));
        debug_assert!(self.text.is_char_boundary(range.end));

        let value = if let Some(max_length) = self.constraints.max_length {
            let retained = self.text[..range.start].graphemes(true).count()
                + self.text[range.end..].graphemes(true).count();
            truncate_graphemes(value, max_length.saturating_sub(retained))
        } else {
            value
        };
        let mut text = self.text.to_string();
        text.replace_range(range.clone(), value);
        if self
            .constraints
            .filter
            .as_ref()
            .is_some_and(|filter| !filter(&text))
        {
            return None;
        }
        let end = range.start + value.len();
        Some((text, end))
    }

    fn replace_range_unchecked(&mut self, range: Range<usize>, value: &str) {
        let mut text = self.text.to_string();
        text.replace_range(range.clone(), value);
        self.highlights =
            replace_highlights(&self.highlights, range.clone(), value.len(), text.len());
        self.text = Arc::from(text);
        let end = range.start + value.len();
        self.anchor = end;
        self.caret = end;
        self.marked = None;
        self.composition_backup = None;
        self.preferred_x = None;
        self.shift_misspellings(range.start..range.end, value.len());
        self.invalidate_highlight_cache();
    }

    fn cancel_composition(&mut self) -> bool {
        let Some(backup) = self.composition_backup.take() else {
            return false;
        };
        let changed = self.text != backup.text
            || self.anchor != backup.anchor
            || self.caret != backup.caret
            || self.marked.is_some();
        self.text = backup.text;
        self.highlights = backup.highlights;
        self.anchor = backup.anchor;
        self.caret = backup.caret;
        self.marked = None;
        self.preferred_x = None;
        changed
    }

    /// The resolved text checking behavior for this input.
    pub fn text_checking_policy(&self) -> TextCheckingPolicy {
        self.constraints
            .text_checking
            .resolve(default_text_checking())
    }

    /// The single one-shot deadline at which the settled check should run.
    ///
    /// A settled or unchecked input reports `None`, so a clean window contributes no wakeup.
    pub fn spell_check_deadline(&self) -> Option<Instant> {
        self.settle_deadline
    }

    /// Run the settled check when its exact deadline has arrived.
    ///
    /// Returns whether flagged ranges changed and a repaint is required.
    pub fn advance_spell_check(&mut self, now: Instant) -> bool {
        if self.settle_pending {
            self.settle_pending = false;
            self.settle_deadline = Some(now + SPELL_CHECK_SETTLE_DELAY);
            return false;
        }
        let Some(deadline) = self.settle_deadline else {
            return false;
        };
        if now < deadline {
            return false;
        }
        self.settle_deadline = None;
        self.run_spell_check()
    }

    /// Run the settled check immediately, cancelling any armed deadline.
    pub fn run_spell_check(&mut self) -> bool {
        self.settle_deadline = None;
        self.settle_pending = false;
        let policy = self.text_checking_policy();
        if !policy.checks_after_settle() || self.marked.is_some() {
            return self.discard_misspellings();
        }
        let range = self.check_window();
        if range.is_empty() {
            return self.discard_misspellings();
        }
        let provider = spell_check_provider();
        let flagged = provider.check(&self.text, range, policy);
        let flagged = self.normalize_misspellings(flagged, policy);
        if flagged == self.misspellings {
            return false;
        }
        self.misspellings = flagged;
        self.misspelling_revision = self.misspelling_revision.wrapping_add(1);
        self.invalidate_highlight_cache();
        true
    }

    /// Flagged ranges from the last settled check.
    pub fn misspelled_ranges(&self) -> &[Misspelling] {
        &self.misspellings
    }

    /// The flagged range covering `offset`, otherwise the word at `offset`.
    pub fn spelling_range_at(&self, offset: usize) -> Option<Range<usize>> {
        self.misspellings
            .iter()
            .find(|flagged| {
                let range = flagged.range();
                range.start <= offset && offset <= range.end
            })
            .map(Misspelling::range)
            .or_else(|| word_range_at(&self.text, offset))
    }

    /// Bounded replacement guesses for the flagged word at `offset`.
    pub fn spelling_suggestions_at(&self, offset: usize) -> Vec<Arc<str>> {
        let Some(range) = self.spelling_range_at(offset) else {
            return Vec::new();
        };
        let Some(word) = self.text.get(range) else {
            return Vec::new();
        };
        if word.is_empty() || word.len() > MAX_SPELL_WORD_BYTES {
            return Vec::new();
        }
        spell_check_provider()
            .guesses(word)
            .into_iter()
            .filter(|guess| !guess.is_empty() && guess.len() <= MAX_SPELL_WORD_BYTES)
            .take(MAX_SPELL_GUESSES)
            .map(Arc::from)
            .collect()
    }

    /// Build the standard suggestion, learn, and ignore entries for the word at `offset`.
    pub fn spelling_menu_items(
        &self,
        offset: usize,
        labels: SpellingMenuLabels,
    ) -> Vec<PopoverMenuItem> {
        let Some(range) = self.spelling_range_at(offset) else {
            return Vec::new();
        };
        let Some(word) = self.text.get(range.clone()) else {
            return Vec::new();
        };
        let guesses = self.spelling_suggestions_at(offset);
        spelling_menu_items(range, word, &guesses, labels)
    }

    /// Replace one flagged range with a suggestion as a single undoable edit.
    pub fn replace_word(&mut self, range: Range<usize>, replacement: &str) -> bool {
        if range.start > range.end
            || range.end > self.text.len()
            || !self.text.is_char_boundary(range.start)
            || !self.text.is_char_boundary(range.end)
        {
            return false;
        }
        let replacement = normalize_text(replacement, self.multiline);
        self.edit_group = None;
        self.replace_range(range, &replacement, None)
    }

    /// Add one word to the provider's user dictionary and recheck immediately.
    pub fn learn_word(&mut self, word: &str) -> bool {
        if word.is_empty() || word.len() > MAX_SPELL_WORD_BYTES {
            return false;
        }
        spell_check_provider().learn(word);
        self.run_spell_check()
    }

    /// Ignore one word for this input's checking session and recheck immediately.
    pub fn ignore_word(&mut self, word: &str) -> bool {
        if word.is_empty() || word.len() > MAX_SPELL_WORD_BYTES {
            return false;
        }
        let tag = self.document_tag();
        spell_check_provider().ignore(word, tag);
        self.run_spell_check()
    }

    /// The most recent applied autocorrection, retained so an application can offer "Change back".
    pub fn last_autocorrection(&self) -> Option<&Autocorrection> {
        self.last_autocorrection.as_ref()
    }

    /// Restore the word an autocorrection replaced as a single undoable edit.
    pub fn revert_autocorrection(&mut self) -> bool {
        let Some(correction) = self.last_autocorrection.clone() else {
            return false;
        };
        let range = correction.range();
        if range.end > self.text.len()
            || self.text.get(range.clone()) != Some(correction.replacement().as_ref())
        {
            self.last_autocorrection = None;
            return false;
        }
        self.edit_group = None;
        self.replace_range(range, correction.original(), None)
    }

    /// The bounded text a dictionary lookup should define, and where it starts.
    ///
    /// The selection wins when it is non-empty; otherwise the word at the caret is used.
    pub fn definition_target(&self) -> Option<(Arc<str>, usize)> {
        let selection = self.selection();
        let range = if selection.is_empty() {
            word_range_at(&self.text, self.caret)?
        } else {
            selection
        };
        let value = self.text.get(range.clone())?.trim();
        if value.is_empty() || value.len() > crate::spell::MAX_DEFINITION_LOOKUP_BYTES {
            return None;
        }
        Some((Arc::from(value), range.start))
    }

    /// Whether a Force Touch force-click over this input should show the dictionary popover.
    pub fn looks_up_on_force_click(&self) -> bool {
        self.text_checking_policy().lookup_on_force_click
    }

    fn document_tag(&mut self) -> SpellDocumentTag {
        self.document.get_or_insert_with(SpellDocument::open).tag()
    }

    fn arm_settle_deadline(&mut self) {
        // The deadline is timestamped by the next `advance_spell_check` pump rather than by the
        // wall clock here, so the runtime's own clock (injected in deterministic tests) owns it.
        self.settle_deadline = None;
        self.settle_pending =
            self.text_checking_policy().checks_after_settle() && self.marked.is_none();
    }

    fn discard_misspellings(&mut self) -> bool {
        if self.misspellings.is_empty() {
            return false;
        }
        self.misspellings.clear();
        self.misspelling_revision = self.misspelling_revision.wrapping_add(1);
        self.invalidate_highlight_cache();
        true
    }

    /// Shift flagged ranges across one accepted replacement and drop the ranges it touched.
    fn shift_misspellings(&mut self, replaced: Range<usize>, inserted_len: usize) {
        if self.misspellings.is_empty() {
            return;
        }
        let removed_len = replaced.end.saturating_sub(replaced.start);
        let shift = inserted_len as isize - removed_len as isize;
        self.misspellings.retain_mut(|flagged| {
            if flagged.range().end <= replaced.start {
                return true;
            }
            if flagged.range().start >= replaced.end {
                let shifted = shifted_offset(flagged.range().start, shift)
                    ..shifted_offset(flagged.range().end, shift);
                *flagged = Misspelling::new(shifted, flagged.kind());
                return true;
            }
            false
        });
        self.misspelling_revision = self.misspelling_revision.wrapping_add(1);
    }

    fn invalidate_highlight_cache(&self) {
        *self.highlight_cache.borrow_mut() = None;
    }

    fn normalize_misspellings(
        &self,
        flagged: Vec<Misspelling>,
        policy: TextCheckingPolicy,
    ) -> Vec<Misspelling> {
        let mut normalized: Vec<Misspelling> = Vec::with_capacity(flagged.len());
        let mut sorted = flagged;
        sorted.sort_by_key(|flagged| (flagged.range().start, flagged.range().end));
        for entry in sorted {
            if entry.kind() == MisspellingKind::Grammar && !policy.grammar_check {
                continue;
            }
            if entry.kind() == MisspellingKind::Spelling && !policy.spellcheck {
                continue;
            }
            let range = entry.range();
            if range.start >= range.end
                || range.end > self.text.len()
                || !self.text.is_char_boundary(range.start)
                || !self.text.is_char_boundary(range.end)
            {
                continue;
            }
            if normalized
                .last()
                .is_some_and(|previous| previous.range().end > range.start)
            {
                continue;
            }
            if normalized.len() == MAX_MISSPELLED_RANGES {
                break;
            }
            normalized.push(entry);
        }
        normalized
    }

    /// The bounded UTF-8 window handed to the checker, centered on the caret.
    fn check_window(&self) -> Range<usize> {
        if self.text.len() <= MAX_SPELLCHECK_BYTES {
            return 0..self.text.len();
        }
        let half = MAX_SPELLCHECK_BYTES / 2;
        let start = boundary_at_or_before(&self.text, self.caret.saturating_sub(half));
        let end = boundary_at_or_before(
            &self.text,
            (start + MAX_SPELLCHECK_BYTES).min(self.text.len()),
        );
        start..end
    }

    fn pending_word_replacement(
        &self,
        caret: usize,
        policy: TextCheckingPolicy,
    ) -> Option<(Range<usize>, String, String)> {
        let range = word_range_at(&self.text, caret)?;
        if range.end != caret {
            return None;
        }
        let word = self.text.get(range.clone())?;
        if word.is_empty() || word.len() > MAX_SPELL_WORD_BYTES {
            return None;
        }
        let provider = spell_check_provider();
        let mut replacement = policy
            .text_replacement
            .then(|| provider.check_text_substitutions(&self.text, range.clone(), policy))
            .flatten()
            .map(|substitution| substitution.replacement().to_string());
        if replacement.is_none() && policy.autocorrect {
            replacement = provider.correction(&self.text, range.clone());
        }
        let replacement = replacement?;
        if replacement.is_empty()
            || replacement == word
            || replacement.len() > MAX_SPELL_WORD_BYTES
            || (!self.multiline && replacement.contains(['\n', '\r']))
        {
            return None;
        }
        Some((range, word.to_string(), replacement))
    }

    fn accepts_existing(&self, value: &str) -> bool {
        self.constraints
            .max_length
            .is_none_or(|max_length| value.graphemes(true).count() <= max_length)
            && self
                .constraints
                .filter
                .as_ref()
                .is_none_or(|filter| filter(value))
    }
}

fn truncate_graphemes(value: &str, max_length: usize) -> &str {
    value
        .grapheme_indices(true)
        .nth(max_length)
        .map_or(value, |(end, _)| &value[..end])
}

fn push_bounded_history(
    history: &mut VecDeque<EditSnapshot>,
    retained_bytes: &mut usize,
    snapshot: EditSnapshot,
) {
    let bytes = snapshot.retained_bytes();
    if bytes > MAX_HISTORY_BYTES_PER_STACK {
        history.clear();
        *retained_bytes = 0;
        return;
    }
    while history.len() >= MAX_HISTORY_ENTRIES
        || retained_bytes.saturating_add(bytes) > MAX_HISTORY_BYTES_PER_STACK
    {
        let Some(evicted) = history.pop_front() else {
            break;
        };
        *retained_bytes = retained_bytes.saturating_sub(evicted.retained_bytes());
    }
    *retained_bytes += bytes;
    history.push_back(snapshot);
}

/// Shift, split, and merge attributed ranges after one accepted UTF-8 replacement.
///
/// Inserted text inherits a style only when it is inserted inside a run, or replaces text whose
/// first byte was styled. At run boundaries it remains unstyled until the controlled highlighter
/// supplies the next table. Runtime splits are truncated to the same hard cap as public input.
fn replace_highlights(
    highlights: &Arc<[TextHighlight]>,
    replaced: Range<usize>,
    inserted_len: usize,
    new_text_len: usize,
) -> Arc<[TextHighlight]> {
    if highlights.is_empty() {
        return highlights.clone();
    }

    let removed_len = replaced.end.saturating_sub(replaced.start);
    let shift = inserted_len as isize - removed_len as isize;
    let inherited = highlights
        .iter()
        .find(|highlight| {
            highlight.range.start <= replaced.start
                && replaced.start < highlight.range.end
                && (!replaced.is_empty() || highlight.range.start < replaced.start)
        })
        .map(|highlight| highlight.style.clone());
    let mut transformed = Vec::with_capacity(highlights.len().saturating_add(1));

    for highlight in highlights.iter() {
        if highlight.range.end <= replaced.start {
            transformed.push(highlight.clone());
            continue;
        }
        if highlight.range.start >= replaced.end {
            transformed.push(TextHighlight {
                range: shifted_offset(highlight.range.start, shift)
                    ..shifted_offset(highlight.range.end, shift),
                style: highlight.style.clone(),
            });
            continue;
        }
        if highlight.range.start < replaced.start {
            transformed.push(TextHighlight {
                range: highlight.range.start..replaced.start,
                style: highlight.style.clone(),
            });
        }
        if highlight.range.end > replaced.end {
            transformed.push(TextHighlight {
                range: replaced.start + inserted_len..shifted_offset(highlight.range.end, shift),
                style: highlight.style.clone(),
            });
        }
    }

    if inserted_len > 0
        && let Some(style) = inherited
    {
        transformed.push(TextHighlight {
            range: replaced.start..replaced.start + inserted_len,
            style,
        });
    }
    transformed.sort_by_key(|highlight| (highlight.range.start, highlight.range.end));

    let mut normalized: Vec<TextHighlight> = Vec::with_capacity(transformed.len());
    for mut highlight in transformed {
        highlight.range.end = highlight.range.end.min(new_text_len);
        if highlight.range.start >= highlight.range.end {
            continue;
        }
        if let Some(previous) = normalized.last_mut()
            && previous.range.end == highlight.range.start
            && previous.style == highlight.style
        {
            previous.range.end = highlight.range.end;
            continue;
        }
        if normalized.len() == MAX_TEXT_HIGHLIGHTS {
            break;
        }
        normalized.push(highlight);
    }
    normalized.into()
}

fn contains_caret(range: &Range<usize>, caret: usize) -> bool {
    range.start <= caret && caret <= range.end
}

/// Overlay bounded spelling decorations onto the controlled run table.
///
/// Controlled runs and flagged ranges are each sorted and non-overlapping, so one linear sweep
/// over their combined boundaries produces a sorted, non-overlapping merged table. Only underline
/// attributes are overridden; controlled color, font, background, and strikethrough survive.
fn merge_spelling_highlights(
    base: &[TextHighlight],
    misspellings: &[Misspelling],
    caret: usize,
) -> Arc<[TextHighlight]> {
    let spelling_style = misspelling_highlight_style();
    let grammar_style = grammar_highlight_style();
    let spans: Vec<(Range<usize>, &HighlightStyle)> = misspellings
        .iter()
        .filter(|flagged| !contains_caret(&flagged.range(), caret))
        .map(|flagged| {
            let style = match flagged.kind() {
                MisspellingKind::Spelling => &spelling_style,
                MisspellingKind::Grammar => &grammar_style,
            };
            (flagged.range(), style)
        })
        .collect();

    let mut boundaries = Vec::with_capacity((base.len() + spans.len()) * 2);
    for highlight in base {
        boundaries.push(highlight.range.start);
        boundaries.push(highlight.range.end);
    }
    for (range, _) in &spans {
        boundaries.push(range.start);
        boundaries.push(range.end);
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut merged: Vec<TextHighlight> = Vec::with_capacity(boundaries.len());
    let mut base_index = 0_usize;
    let mut span_index = 0_usize;
    for window in boundaries.windows(2) {
        let (start, end) = (window[0], window[1]);
        while base_index < base.len() && base[base_index].range.end <= start {
            base_index += 1;
        }
        while span_index < spans.len() && spans[span_index].0.end <= start {
            span_index += 1;
        }
        let base_style = base
            .get(base_index)
            .filter(|highlight| highlight.range.start <= start)
            .map(|highlight| highlight.style.clone());
        let spelling = spans
            .get(span_index)
            .filter(|(range, _)| range.start <= start)
            .map(|(_, style)| *style);
        let style = match (base_style, spelling) {
            (None, None) => continue,
            (Some(style), None) => style,
            (base_style, Some(spelling)) => {
                let mut style = base_style.unwrap_or_default();
                style.underline = spelling.underline;
                style.underline_color = spelling.underline_color;
                style.underline_wavy = spelling.underline_wavy;
                style.underline_thickness = spelling.underline_thickness;
                style
            }
        };
        if let Some(previous) = merged.last_mut()
            && previous.range.end == start
            && previous.style == style
        {
            previous.range.end = end;
            continue;
        }
        if merged.len() == MAX_TEXT_HIGHLIGHTS {
            break;
        }
        merged.push(TextHighlight {
            range: start..end,
            style,
        });
    }
    merged.into()
}

fn shifted_offset(offset: usize, shift: isize) -> usize {
    if shift >= 0 {
        offset.saturating_add(shift as usize)
    } else {
        offset.saturating_sub(shift.unsigned_abs())
    }
}

#[cfg(feature = "text-input-decorations")]
fn shifted_editor_offset(offset: usize, edits: &[(usize, usize, String)]) -> usize {
    let mut shifted = offset as isize;
    for (start, removed, inserted) in edits {
        if offset < *start {
            break;
        }
        let removed_end = start.saturating_add(*removed);
        if offset <= removed_end {
            shifted = (*start + inserted.len()) as isize;
            continue;
        }
        shifted += inserted.len() as isize - *removed as isize;
    }
    shifted.max(0) as usize
}

fn previous_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .rev()
        .find_map(|(index, _)| (index < offset).then_some(index))
        .unwrap_or(0)
}

fn next_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .find_map(|(index, _)| (index > offset).then_some(index))
        .unwrap_or(text.len())
}

pub(crate) fn boundary_at_or_before(text: &str, offset: usize) -> usize {
    if offset >= text.len() {
        return text.len();
    }
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .take_while(|index| *index <= offset)
        .last()
        .unwrap_or(0)
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())]
        .rfind('\n')
        .map_or(0, |index| index + 1)
}

fn line_end(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    text[offset..]
        .find('\n')
        .map_or(text.len(), |index| offset + index)
}

fn previous_word_boundary(text: &str, offset: usize) -> usize {
    let offset = boundary_at_or_before(text, offset.min(text.len()));
    let mut previous = 0;
    let mut segments = text.split_word_bound_indices().peekable();
    while let Some((start, segment)) = segments.next() {
        if start >= offset {
            break;
        }
        if !segment_is_word(segment) {
            continue;
        }
        let mut end = start + segment.len();
        while let Some((next_start, next_segment)) = segments.peek() {
            if *next_start != end || !segment_is_word(next_segment) {
                break;
            }
            end = *next_start + next_segment.len();
            segments.next();
        }
        if offset <= end {
            return start;
        }
        previous = start;
    }
    previous
}

fn next_word_boundary(text: &str, offset: usize) -> usize {
    let offset = boundary_at_or_before(text, offset.min(text.len()));
    let mut segments = text.split_word_bound_indices().peekable();
    while let Some((start, segment)) = segments.next() {
        if !segment_is_word(segment) {
            continue;
        }
        let mut end = start + segment.len();
        while let Some((next_start, next_segment)) = segments.peek() {
            if *next_start != end || !segment_is_word(next_segment) {
                break;
            }
            end = *next_start + next_segment.len();
            segments.next();
        }
        if end > offset {
            return end;
        }
    }
    text.len()
}

fn segment_is_word(segment: &str) -> bool {
    segment
        .chars()
        .any(|character| character.is_alphanumeric() || character == '_')
}

fn normalize_text(value: &str, multiline: bool) -> Cow<'_, str> {
    if multiline {
        if !value.contains('\r') {
            return Cow::Borrowed(value);
        }
        let mut result = String::with_capacity(value.len());
        let mut characters = value.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '\r' {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                result.push('\n');
            } else {
                result.push(character);
            }
        }
        return Cow::Owned(result);
    }

    if !value.contains('\r') && !value.contains('\n') {
        return Cow::Borrowed(value);
    }
    let mut result = String::with_capacity(value.len());
    let mut previous_was_cr = false;
    for ch in value.chars() {
        match ch {
            '\r' => {
                result.push(' ');
                previous_was_cr = true;
            }
            '\n' if previous_was_cr => {
                previous_was_cr = false;
            }
            '\n' => {
                result.push(' ');
                previous_was_cr = false;
            }
            _ => {
                result.push(ch);
                previous_was_cr = false;
            }
        }
    }
    Cow::Owned(result)
}

/// Remap validated run boundaries when CR/CRLF normalization changes UTF-8 offsets.
///
/// At most two endpoints per bounded run are retained while the source string is scanned once;
/// this avoids an offset table proportional to the complete document size.
fn normalize_highlights(value: &str, highlights: &Arc<[TextHighlight]>) -> Arc<[TextHighlight]> {
    if highlights.is_empty() {
        return highlights.clone();
    }

    let mut endpoints = Vec::with_capacity(highlights.len().saturating_mul(2));
    for highlight in highlights.iter() {
        endpoints.push(highlight.range.start);
        endpoints.push(highlight.range.end);
    }
    endpoints.sort_unstable();
    endpoints.dedup();

    let mut mapped = Vec::with_capacity(endpoints.len());
    let mut endpoint_index = 0;
    let mut source_offset = 0;
    let mut normalized_offset = 0;
    let bytes = value.as_bytes();
    let record = |source_offset: usize,
                  normalized_offset: usize,
                  endpoint_index: &mut usize,
                  mapped: &mut Vec<usize>| {
        while endpoints.get(*endpoint_index).copied() == Some(source_offset) {
            mapped.push(normalized_offset);
            *endpoint_index += 1;
        }
    };
    record(
        source_offset,
        normalized_offset,
        &mut endpoint_index,
        &mut mapped,
    );

    while source_offset < value.len() {
        if bytes[source_offset] == b'\r' {
            source_offset += 1;
            normalized_offset += 1;
            record(
                source_offset,
                normalized_offset,
                &mut endpoint_index,
                &mut mapped,
            );
            if bytes.get(source_offset) == Some(&b'\n') {
                source_offset += 1;
                record(
                    source_offset,
                    normalized_offset,
                    &mut endpoint_index,
                    &mut mapped,
                );
            }
            continue;
        }
        let length = value[source_offset..]
            .chars()
            .next()
            .map_or(1, char::len_utf8);
        source_offset += length;
        normalized_offset += length;
        record(
            source_offset,
            normalized_offset,
            &mut endpoint_index,
            &mut mapped,
        );
    }
    debug_assert_eq!(mapped.len(), endpoints.len());

    let mapped_offset = |offset: usize| {
        endpoints
            .binary_search(&offset)
            .ok()
            .and_then(|index| mapped.get(index).copied())
            .unwrap_or(normalized_offset)
    };
    let mut normalized: Vec<TextHighlight> = Vec::with_capacity(highlights.len());
    for highlight in highlights.iter() {
        let range = mapped_offset(highlight.range.start)..mapped_offset(highlight.range.end);
        if range.is_empty() {
            continue;
        }
        if let Some(previous) = normalized.last_mut()
            && previous.range.end == range.start
            && previous.style == highlight.style
        {
            previous.range.end = range.end;
        } else {
            normalized.push(TextHighlight {
                range,
                style: highlight.style.clone(),
            });
        }
    }
    normalized.into()
}

pub(crate) fn selectable_character_lengths(value: &str) -> Vec<u8> {
    let mut lengths = Vec::with_capacity(value.graphemes(true).count());
    for grapheme in value.graphemes(true) {
        if let Ok(length) = u8::try_from(grapheme.len()) {
            lengths.push(length);
        } else {
            // Pathological combining sequences can exceed AccessKit's per-character u8 length.
            // Splitting those into scalar values preserves the required total UTF-8 byte count.
            lengths.extend(grapheme.chars().map(|ch| ch.len_utf8() as u8));
        }
    }
    lengths
}

pub(crate) fn accessibility_character_index(value: &str, byte_index: usize) -> usize {
    accessibility_character_index_from_lengths(&selectable_character_lengths(value), byte_index)
}

pub(crate) fn accessibility_character_index_from_lengths(
    character_lengths: &[u8],
    byte_index: usize,
) -> usize {
    let mut offset = 0_usize;
    character_lengths
        .iter()
        .take_while(|length| {
            let next = offset + usize::from(**length);
            if next <= byte_index {
                offset = next;
                true
            } else {
                false
            }
        })
        .count()
}

pub(crate) fn accessibility_byte_index(value: &str, character_index: usize) -> usize {
    selectable_character_lengths(value)
        .into_iter()
        .take(character_index)
        .map(usize::from)
        .sum::<usize>()
        .min(value.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_movement_uses_grapheme_boundaries() {
        let family = "a👨‍👩‍👧‍👦b";
        let mut input = TextInputState::new(family);
        assert!(input.move_left(false));
        assert_eq!(&family[input.caret()..], "b");
        assert!(input.move_left(false));
        assert_eq!(input.caret(), 1);
        assert!(input.backspace());
        assert_eq!(input.text(), "👨‍👩‍👧‍👦b");
    }

    #[test]
    fn replacement_and_selection_remain_utf8_safe() {
        let mut input = TextInputState::new("café");
        input.move_left(false);
        input.move_left(true);
        assert_eq!(input.selected_text(), Some("f"));
        assert!(input.replace_selection("🙂"));
        assert_eq!(input.text(), "ca🙂é");
    }

    #[test]
    fn preedit_replaces_previous_marked_text() {
        let mut input = TextInputState::new("hello ");
        assert!(input.set_preedit("n", Some((1, 1))));
        assert_eq!(input.text(), "hello n");
        assert_eq!(input.marked(), Some(6..7));
        assert!(input.set_preedit("你", Some((3, 3))));
        assert_eq!(input.text(), "hello 你");
        assert_eq!(input.marked(), Some(6..9));
        assert!(input.set_preedit("", None));
        assert_eq!(input.text(), "hello ");
        assert!(input.replace_selection("你"));
        assert_eq!(input.text(), "hello 你");
    }

    #[test]
    fn controlled_sync_preserves_and_cancel_restores_active_preedit() {
        let mut input = TextInputState::new("hello");
        input.move_left(true);
        assert_eq!(input.selected_text(), Some("o"));
        assert!(input.set_preedit("お", Some((3, 3))));
        assert_eq!(input.text(), "hellお");

        input.sync_external("hello", false, &InputConstraints::default());
        assert_eq!(input.text(), "hellお");

        assert!(input.set_preedit("", None));
        assert_eq!(input.text(), "hello");
        assert_eq!(input.selected_text(), Some("o"));
        assert!(input.replace_selection("お"));
        assert_eq!(input.text(), "hellお");
    }

    #[test]
    fn controlled_style_changes_remap_across_active_preedit() {
        let cool = crate::styled_text("token").with_highlights([(
            0..5,
            crate::HighlightStyle::default().color(crate::Color::rgb8(56, 189, 248)),
        )]);
        let (content, highlights) = cool.into_parts();
        let mut input =
            TextInputState::with_styling(&content, false, InputConstraints::default(), highlights);
        assert!(input.set_selection(2, 2));
        assert!(input.set_preedit("你", Some((3, 3))));

        let warm = crate::styled_text("token").with_highlights([(
            0..5,
            crate::HighlightStyle::default().color(crate::Color::rgb8(244, 114, 182)),
        )]);
        let (_, warm_highlights) = warm.into_parts();
        input.sync_external_styled(
            "token",
            false,
            &InputConstraints::default(),
            &warm_highlights,
        );

        assert_eq!(input.shared_highlights()[0].range(), 0..8);
        assert_eq!(
            input.shared_highlights()[0].style().color,
            Some(crate::Color::rgb8(244, 114, 182))
        );
        assert!(input.set_preedit("", None));
        assert_eq!(input.shared_highlights().as_ref(), warm_highlights.as_ref());
    }

    #[test]
    fn multiline_paste_is_normalized() {
        let mut input = TextInputState::new("");
        input.replace_selection("one\r\ntwo\nthree");
        assert_eq!(input.text(), "one two three");
    }

    #[test]
    fn multiline_editing_preserves_lines_and_normalizes_crlf() {
        let mut input = TextInputState::with_mode("one\r\ntwo\rthree", true);
        assert_eq!(input.text(), "one\ntwo\nthree");
        assert!(input.move_line_start(false));
        assert_eq!(input.caret(), 8);
        assert!(input.move_line_end(false));
        assert_eq!(input.caret(), input.text().len());
        assert!(input.insert_newline());
        assert_eq!(input.text(), "one\ntwo\nthree\n");
    }

    #[test]
    fn newline_normalization_remaps_attributed_byte_ranges_in_one_pass() {
        let styled = crate::styled_text("a\r\nb🙂\rc").with_highlights([
            (0..4, crate::HighlightStyle::default().font_bold()),
            (
                4..9,
                crate::HighlightStyle::default().color(crate::Color::rgb8(96, 165, 250)),
            ),
        ]);
        let (content, highlights) = styled.into_parts();
        let input =
            TextInputState::with_styling(&content, true, InputConstraints::default(), highlights);

        assert_eq!(input.text(), "a\nb🙂\nc");
        assert_eq!(input.shared_highlights()[0].range(), 0..3);
        assert_eq!(input.shared_highlights()[1].range(), 3..8);
    }

    #[test]
    fn line_deletion_joins_adjacent_lines_at_the_boundary() {
        let mut backward = TextInputState::with_mode("one\ntwo\nthree", true);
        assert!(backward.move_line_start(false));
        assert_eq!(&backward.text()[backward.caret()..], "three");
        assert!(backward.delete_to_line_start());
        assert_eq!(backward.text(), "one\ntwothree");

        let mut forward = TextInputState::with_mode("one\ntwo", true);
        assert!(forward.move_home(false));
        assert!(forward.move_line_end(false));
        assert_eq!(forward.caret(), 3);
        assert!(forward.delete_to_line_end());
        assert_eq!(forward.text(), "onetwo");
    }

    #[test]
    fn single_line_inputs_reject_newlines_but_normalize_multiline_replacement() {
        let mut single_line = TextInputState::new("value");
        assert!(!single_line.insert_newline());
        assert_eq!(single_line.text(), "value");

        let mut multiline = TextInputState::with_mode("", true);
        assert!(multiline.replace_selection("one\r\ntwo\rthree"));
        assert_eq!(multiline.text(), "one\ntwo\nthree");
    }

    #[test]
    fn word_navigation_and_deletion_respect_unicode_boundaries() {
        let mut input = TextInputState::with_mode("alpha café_beta 世界", true);
        assert!(input.move_word_left(false));
        assert_eq!(&input.text()[input.caret()..], "世界");
        assert!(input.move_word_left(false));
        assert_eq!(&input.text()[input.caret()..], "café_beta 世界");
        assert!(input.delete_word_forward());
        assert_eq!(input.text(), "alpha  世界");
        assert!(input.delete_word_backward());
        assert_eq!(input.text(), " 世界");
    }

    #[test]
    fn undo_and_redo_restore_text_and_selection() {
        let mut input = TextInputState::new("a");
        assert!(input.replace_selection("b"));
        input.move_to(input.caret(), false);
        assert!(input.replace_selection("c"));
        assert_eq!(input.text(), "abc");

        assert!(input.undo());
        assert_eq!(input.text(), "ab");
        assert_eq!(input.selection(), 2..2);
        assert!(input.undo());
        assert_eq!(input.text(), "a");
        assert!(input.redo());
        assert_eq!(input.text(), "ab");

        assert!(input.replace_selection("!"));
        assert_eq!(input.text(), "ab!");
        assert!(!input.can_redo());
    }

    #[test]
    fn adjacent_native_typing_and_deletion_coalesce_without_a_timer() {
        let mut input = TextInputState::new("replace me");
        assert!(input.select_all());
        for character in "GPU hot".chars() {
            assert!(input.replace_selection(&character.to_string()));
        }
        assert_eq!(input.text(), "GPU hot");
        assert_eq!(input.undo.len(), 1);
        assert!(input.undo());
        assert_eq!(input.text(), "replace me");
        assert!(input.redo());
        assert_eq!(input.text(), "GPU hot");

        let undo_before_backspace = input.undo.len();
        for _ in 0..3 {
            assert!(input.backspace());
        }
        assert_eq!(input.text(), "GPU ");
        assert_eq!(input.undo.len(), undo_before_backspace + 1);
        assert!(input.undo());
        assert_eq!(input.text(), "GPU hot");
    }

    #[test]
    fn typing_coalescing_expires_without_scheduling_work() {
        let mut input = TextInputState::new("");
        assert!(input.replace_selection("a"));
        input.edit_group.as_mut().unwrap().last_edit_at = Instant::now()
            .checked_sub(EDIT_COALESCE_WINDOW + Duration::from_millis(1))
            .unwrap();
        assert!(input.replace_selection("b"));

        assert_eq!(input.undo.len(), 2);
        assert!(input.undo());
        assert_eq!(input.text(), "a");
    }

    #[test]
    fn attributed_edits_shift_split_merge_and_restore_bounded_runs() {
        let styled = crate::styled_text("abcdef").with_highlights([(
            1..5,
            crate::HighlightStyle::default()
                .font_bold()
                .color(crate::Color::rgb8(94, 234, 212)),
        )]);
        let (content, highlights) = styled.into_parts();
        let mut input =
            TextInputState::with_styling(&content, false, InputConstraints::default(), highlights);

        assert!(input.set_selection(3, 3));
        assert!(input.replace_selection("X"));
        assert_eq!(input.text(), "abcXdef");
        assert_eq!(input.shared_highlights()[0].range(), 1..6);

        assert!(input.set_selection(2, 5));
        assert!(input.replace_selection("🙂"));
        assert_eq!(input.text(), "ab🙂ef");
        assert_eq!(input.shared_highlights()[0].range(), 1..7);
        assert!(input.undo());
        assert_eq!(input.text(), "abcXdef");
        assert_eq!(input.shared_highlights()[0].range(), 1..6);
    }

    #[test]
    fn attributed_insertions_at_run_boundaries_do_not_bleed_styles() {
        let styled = crate::styled_text("abcd")
            .with_highlights([(1..3, crate::HighlightStyle::default().underline())]);
        let (content, highlights) = styled.into_parts();
        let mut input =
            TextInputState::with_styling(&content, false, InputConstraints::default(), highlights);

        assert!(input.set_selection(1, 1));
        assert!(input.replace_selection("X"));
        assert_eq!(input.shared_highlights()[0].range(), 2..4);
        assert!(input.set_selection(4, 4));
        assert!(input.replace_selection("Y"));
        assert_eq!(input.shared_highlights()[0].range(), 2..4);
    }

    #[test]
    fn cancelling_preedit_restores_attributed_content_exactly() {
        let styled = crate::styled_text("token")
            .with_highlights([(0..5, crate::HighlightStyle::default().font_semibold())]);
        let (content, highlights) = styled.into_parts();
        let original = highlights.clone();
        let mut input =
            TextInputState::with_styling(&content, false, InputConstraints::default(), highlights);

        assert!(input.set_selection(2, 2));
        assert!(input.set_preedit("你", Some((3, 3))));
        assert_eq!(input.shared_highlights()[0].range(), 0..8);
        assert!(input.set_preedit("", None));
        assert_eq!(input.text(), "token");
        assert_eq!(input.shared_highlights().as_ref(), original.as_ref());
    }

    #[test]
    fn composition_commit_creates_one_undo_step() {
        let mut input = TextInputState::new("hello ");
        assert!(input.set_preedit("n", Some((1, 1))));
        assert!(input.set_preedit("ni", Some((2, 2))));
        assert!(input.replace_selection("你"));
        assert_eq!(input.text(), "hello 你");
        assert_eq!(input.undo.len(), 1);
        assert!(input.undo());
        assert_eq!(input.text(), "hello ");
    }

    #[test]
    fn max_length_truncates_at_unicode_grapheme_boundaries() {
        let constraints = InputConstraints {
            max_length: Some(3),
            filter: None,
            text_checking: crate::TextCheckingOverrides::default(),
            #[cfg(feature = "text-input-decorations")]
            editor: None,
        };
        let mut input = TextInputState::with_constraints("a", false, constraints);

        assert!(input.replace_selection("👨‍👩‍👧‍👦bc"));
        assert_eq!(input.text(), "a👨‍👩‍👧‍👦b");
        assert_eq!(input.selection(), input.text().len()..input.text().len());
        assert!(input.undo());
        assert_eq!(input.text(), "a");

        assert!(input.set_value("é🙂xy"));
        assert_eq!(input.text(), "é🙂x");
    }

    #[test]
    fn rejected_edits_do_not_mutate_text_selection_or_history() {
        let constraints = InputConstraints {
            max_length: None,
            filter: Some(Arc::new(|value| {
                value.chars().all(|character| character.is_ascii_digit())
            })),
            text_checking: crate::TextCheckingOverrides::default(),
            #[cfg(feature = "text-input-decorations")]
            editor: None,
        };
        let mut input = TextInputState::with_constraints("12", false, constraints);
        let selection = input.selection();

        assert!(!input.replace_selection("a"));
        assert_eq!(input.text(), "12");
        assert_eq!(input.selection(), selection);
        assert!(!input.can_undo());
    }

    #[test]
    fn rejected_ime_commit_restores_the_preedit_backup_without_history() {
        let constraints = InputConstraints {
            max_length: None,
            filter: Some(Arc::new(|value| value.is_ascii())),
            text_checking: crate::TextCheckingOverrides::default(),
            #[cfg(feature = "text-input-decorations")]
            editor: None,
        };
        let mut input = TextInputState::with_constraints("hello", false, constraints);

        assert!(input.set_preedit("你", Some((3, 3))));
        assert_eq!(input.text(), "hello你");
        assert_eq!(input.committed_shared_text().as_ref(), "hello");
        assert!(input.replace_selection("你"));
        assert_eq!(input.text(), "hello");
        assert_eq!(input.selection(), 5..5);
        assert!(!input.can_undo());
    }

    #[test]
    fn changed_constraints_can_disable_an_incompatible_undo_without_popping_it() {
        let mut input = TextInputState::new("ab");
        assert!(input.replace_selection("c"));
        assert!(input.can_undo());
        input.sync_external(
            "abc",
            false,
            &InputConstraints {
                max_length: Some(1),
                filter: None,
                text_checking: crate::TextCheckingOverrides::default(),
                #[cfg(feature = "text-input-decorations")]
                editor: None,
            },
        );

        assert!(!input.can_undo());
        assert!(!input.undo());
        assert_eq!(input.text(), "abc");

        input.sync_external("abc", false, &InputConstraints::default());
        assert!(input.can_undo());
        assert!(input.undo());
        assert_eq!(input.text(), "ab");
    }

    #[test]
    fn edit_history_is_bounded_by_count_and_bytes() {
        let mut input = TextInputState::new("");
        for _ in 0..(MAX_HISTORY_ENTRIES + 50) {
            assert!(input.replace_selection("x"));
            input.move_to(input.caret(), false);
        }
        assert!(input.undo.len() <= MAX_HISTORY_ENTRIES);
        assert!(input.undo_bytes <= MAX_HISTORY_BYTES_PER_STACK);

        input.set_value(&"x".repeat(MAX_HISTORY_BYTES_PER_STACK + 1));
        assert!(input.replace_selection("y"));
        assert!(!input.can_undo());
        assert_eq!(input.undo_bytes, 0);
    }

    #[test]
    fn attributed_history_counts_run_and_named_family_storage() {
        let content: Arc<str> = Arc::from("x".repeat(1_200));
        let highlights = (0..600).map(|index| {
            (
                index * 2..index * 2 + 1,
                crate::HighlightStyle::default().font_family(FontFamily::Named(Arc::from(
                    format!("family-{index}-{}", "x".repeat(900)),
                ))),
            )
        });
        let styled = crate::styled_text(content).with_highlights(highlights);
        let (content, highlights) = styled.into_parts();
        let mut input =
            TextInputState::with_styling(&content, false, InputConstraints::default(), highlights);

        assert!(input.replace_selection("!"));
        assert_eq!(input.undo_bytes, 0);
        assert!(!input.can_undo());
    }

    fn checking_input(value: &str, policy: TextCheckingPolicy) -> TextInputState {
        let constraints = InputConstraints {
            max_length: None,
            filter: None,
            text_checking: crate::TextCheckingOverrides {
                spellcheck: Some(policy.spellcheck),
                grammar_check: Some(policy.grammar_check),
                autocorrect: Some(policy.autocorrect),
                smart_quotes: Some(policy.smart_quotes),
                smart_dashes: Some(policy.smart_dashes),
                text_replacement: Some(policy.text_replacement),
                lookup_on_force_click: Some(policy.lookup_on_force_click),
            },
            #[cfg(feature = "text-input-decorations")]
            editor: None,
        };
        TextInputState::with_constraints(value, false, constraints)
    }

    fn install(provider: crate::TestSpellCheckProvider) -> Rc<crate::TestSpellCheckProvider> {
        let provider = Rc::new(provider);
        crate::set_shared_spell_check_provider(provider.clone());
        provider
    }

    #[test]
    fn settled_checks_run_once_on_one_exact_deadline() {
        let _provider = install(crate::TestSpellCheckProvider::new().misspelling("helo"));
        let mut input = checking_input(
            "",
            TextCheckingPolicy {
                spellcheck: true,
                ..TextCheckingPolicy::NONE
            },
        );
        assert!(input.spell_check_deadline().is_none());

        let start = Instant::now();
        assert!(input.replace_selection("helo world"));
        assert!(input.spell_check_deadline().is_none());
        assert!(!input.advance_spell_check(start));
        let first = input
            .spell_check_deadline()
            .expect("the first pump after an edit arms one deadline");
        assert_eq!(first, start + SPELL_CHECK_SETTLE_DELAY);
        assert!(!input.advance_spell_check(first - Duration::from_millis(1)));
        assert!(input.replace_selection("!"));
        assert!(input.spell_check_deadline().is_none());
        assert!(!input.advance_spell_check(start + Duration::from_millis(50)));
        let second = input
            .spell_check_deadline()
            .expect("edits re-arm the deadline");
        assert!(second > first);

        assert!(input.set_selection(0, 0));
        assert!(input.advance_spell_check(second));
        assert_eq!(
            input
                .misspelled_ranges()
                .iter()
                .map(Misspelling::range)
                .collect::<Vec<_>>(),
            vec![0..4]
        );
        assert!(input.spell_check_deadline().is_none());
        assert!(!input.advance_spell_check(Instant::now()));
        crate::clear_spell_check_provider();
    }

    #[test]
    fn disabled_checking_never_arms_a_deadline_or_calls_the_provider() {
        let _provider = install(crate::TestSpellCheckProvider::new().misspelling("helo"));
        let mut input = checking_input("", TextCheckingPolicy::NONE);
        assert!(input.replace_selection("helo"));
        assert!(input.spell_check_deadline().is_none());
        assert!(!input.advance_spell_check(Instant::now()));
        assert!(input.misspelled_ranges().is_empty());
        assert!(input.shared_highlights().is_empty());
        crate::clear_spell_check_provider();
    }

    #[test]
    fn misspellings_project_wavy_runs_and_skip_the_word_under_the_caret() {
        let _provider = install(crate::TestSpellCheckProvider::new().misspelling("helo"));
        let mut input = checking_input(
            "helo there",
            TextCheckingPolicy {
                spellcheck: true,
                ..TextCheckingPolicy::NONE
            },
        );
        assert!(input.set_selection(2, 2));
        assert!(input.run_spell_check());
        assert_eq!(input.misspelled_ranges().len(), 1);
        // The caret sits inside the flagged word, so nothing is underlined yet.
        assert!(input.shared_highlights().is_empty());

        assert!(input.set_selection(10, 10));
        let highlights = input.shared_highlights();
        assert_eq!(highlights.len(), 1);
        assert_eq!(highlights[0].range(), 0..4);
        assert_eq!(highlights[0].style().underline_wavy, Some(true));
        assert_eq!(
            highlights[0].style().underline_color,
            Some(crate::Color::rgb8(248, 113, 113))
        );
        crate::clear_spell_check_provider();
    }

    #[test]
    fn spelling_runs_merge_over_controlled_styles_without_replacing_them() {
        let _provider = install(crate::TestSpellCheckProvider::new().misspelling("helo"));
        let styled = crate::styled_text("helo there").with_highlights([(
            0..10,
            crate::HighlightStyle::default().color(crate::Color::rgb8(94, 234, 212)),
        )]);
        let (content, highlights) = styled.into_parts();
        let controlled = highlights.clone();
        let mut input = TextInputState::with_styling(
            &content,
            false,
            InputConstraints {
                max_length: None,
                filter: None,
                text_checking: crate::TextCheckingOverrides {
                    spellcheck: Some(true),
                    ..crate::TextCheckingOverrides::default()
                },
                #[cfg(feature = "text-input-decorations")]
                editor: None,
            },
            highlights,
        );
        assert!(input.run_spell_check());

        let merged = input.shared_highlights();
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].range(), 0..4);
        assert_eq!(merged[1].range(), 4..10);
        assert_eq!(
            merged[0].style().color,
            Some(crate::Color::rgb8(94, 234, 212))
        );
        assert_eq!(merged[0].style().underline_wavy, Some(true));
        assert_eq!(merged[1].style().underline_wavy, None);
        // The controlled table itself is untouched, so undo snapshots stay decoration-free.
        assert_eq!(input.highlights.as_ref(), controlled.as_ref());
        crate::clear_spell_check_provider();
    }

    #[test]
    fn suggestions_and_menu_items_come_from_the_provider() {
        let _provider = install(
            crate::TestSpellCheckProvider::new()
                .misspelling("helo")
                .guess("helo", &["hello", "halo"]),
        );
        let mut input = checking_input(
            "helo there",
            TextCheckingPolicy {
                spellcheck: true,
                ..TextCheckingPolicy::NONE
            },
        );
        assert!(input.run_spell_check());

        let suggestions = input.spelling_suggestions_at(2);
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].as_ref(), "hello");

        let items = input.spelling_menu_items(2, crate::SpellingMenuLabels::default());
        assert_eq!(items.len(), 5);
        assert_eq!(items[0].label().as_ref(), "hello");

        assert!(input.replace_word(0..4, "hello"));
        assert_eq!(input.text(), "hello there");
        assert!(input.undo());
        assert_eq!(input.text(), "helo there");
        crate::clear_spell_check_provider();
    }

    #[test]
    fn learning_and_ignoring_words_clears_their_flagged_ranges() {
        let provider = install(crate::TestSpellCheckProvider::new().misspelling("helo"));
        let mut input = checking_input(
            "helo helo",
            TextCheckingPolicy {
                spellcheck: true,
                ..TextCheckingPolicy::NONE
            },
        );
        assert!(input.set_selection(0, 0));
        assert!(input.run_spell_check());
        assert_eq!(input.misspelled_ranges().len(), 2);

        assert!(input.ignore_word("helo"));
        assert!(input.misspelled_ranges().is_empty());
        assert_eq!(provider.ignored().len(), 1);
        assert_eq!(provider.open_documents(), 1);

        assert!(!input.learn_word("helo"));
        assert_eq!(provider.learned().len(), 1);

        drop(input);
        assert_eq!(provider.open_documents(), 0);
        crate::clear_spell_check_provider();
    }

    #[test]
    fn autocorrect_replaces_the_completed_word_as_one_undoable_edit() {
        let _provider = install(crate::TestSpellCheckProvider::new().correction_for("teh", "the"));
        let mut input = checking_input(
            "",
            TextCheckingPolicy {
                autocorrect: true,
                ..TextCheckingPolicy::NONE
            },
        );
        for character in "teh".chars() {
            assert!(input.replace_selection(&character.to_string()));
        }
        assert_eq!(input.text(), "teh");
        assert!(input.last_autocorrection().is_none());

        assert!(input.replace_selection(" "));
        assert_eq!(input.text(), "the ");
        let correction = input
            .last_autocorrection()
            .expect("a correction is retained");
        assert_eq!(correction.original().as_ref(), "teh");
        assert_eq!(correction.replacement().as_ref(), "the");
        assert_eq!(correction.range(), 0..3);

        assert!(input.undo());
        assert_eq!(input.text(), "teh");
        crate::clear_spell_check_provider();
    }

    #[test]
    fn autocorrections_can_be_changed_back_without_retyping() {
        let _provider = install(crate::TestSpellCheckProvider::new().correction_for("teh", "the"));
        let mut input = checking_input(
            "teh",
            TextCheckingPolicy {
                autocorrect: true,
                ..TextCheckingPolicy::NONE
            },
        );
        assert!(input.replace_selection(" "));
        assert_eq!(input.text(), "the ");
        assert!(input.revert_autocorrection());
        assert_eq!(input.text(), "teh ");
        assert!(!input.revert_autocorrection());
        crate::clear_spell_check_provider();
    }

    #[test]
    fn replacement_dictionary_substitutions_apply_at_a_word_boundary() {
        let _provider =
            install(crate::TestSpellCheckProvider::new().replacement_for("omw", "On my way!"));
        let mut input = checking_input(
            "omw",
            TextCheckingPolicy {
                text_replacement: true,
                ..TextCheckingPolicy::NONE
            },
        );
        assert!(input.replace_selection("!"));
        assert_eq!(input.text(), "On my way!!");
        crate::clear_spell_check_provider();
    }

    #[test]
    fn smart_quotes_and_dashes_apply_at_insertion_time() {
        let mut input = checking_input(
            "",
            TextCheckingPolicy {
                smart_quotes: true,
                smart_dashes: true,
                ..TextCheckingPolicy::NONE
            },
        );
        for character in "\"hi\" a--b".chars() {
            input.replace_selection(&character.to_string());
        }
        assert_eq!(input.text(), "\u{201c}hi\u{201d} a\u{2014}b");

        let mut plain = checking_input("", TextCheckingPolicy::NONE);
        for character in "\"hi\" a--b".chars() {
            plain.replace_selection(&character.to_string());
        }
        assert_eq!(plain.text(), "\"hi\" a--b");
    }

    #[test]
    fn pasted_and_composed_text_is_never_substituted() {
        let mut input = checking_input(
            "",
            TextCheckingPolicy {
                smart_quotes: true,
                smart_dashes: true,
                ..TextCheckingPolicy::NONE
            },
        );
        assert!(input.replace_selection("\"quoted\" and --"));
        assert_eq!(input.text(), "\"quoted\" and --");

        assert!(input.set_preedit("\"", Some((1, 1))));
        assert_eq!(&input.text()[input.text().len() - 1..], "\"");
    }

    #[test]
    fn edits_shift_flagged_ranges_and_drop_the_ranges_they_touch() {
        let _provider = install(
            crate::TestSpellCheckProvider::new()
                .misspelling("helo")
                .misspelling("wrld"),
        );
        let mut input = checking_input(
            "helo wrld",
            TextCheckingPolicy {
                spellcheck: true,
                ..TextCheckingPolicy::NONE
            },
        );
        assert!(input.set_selection(0, 0));
        assert!(input.run_spell_check());
        assert_eq!(input.misspelled_ranges().len(), 2);

        assert!(input.set_selection(2, 3));
        assert!(input.replace_selection("XY"));
        let ranges = input
            .misspelled_ranges()
            .iter()
            .map(Misspelling::range)
            .collect::<Vec<_>>();
        assert_eq!(ranges, vec![6..10]);
        crate::clear_spell_check_provider();
    }

    #[test]
    fn checked_windows_and_result_lists_stay_bounded() {
        let long_word = "z".repeat(9);
        let mut provider = crate::TestSpellCheckProvider::new();
        provider = provider.misspelling(&long_word);
        let _provider = install(provider);

        let text = format!("{long_word} ").repeat(MAX_MISSPELLED_RANGES + 64);
        let mut input = checking_input(
            &text,
            TextCheckingPolicy {
                spellcheck: true,
                ..TextCheckingPolicy::NONE
            },
        );
        assert!(input.set_selection(0, 0));
        assert!(input.run_spell_check());
        assert_eq!(input.misspelled_ranges().len(), MAX_MISSPELLED_RANGES);

        let huge = "a".repeat(MAX_SPELLCHECK_BYTES * 3);
        let wide = checking_input(
            &huge,
            TextCheckingPolicy {
                spellcheck: true,
                ..TextCheckingPolicy::NONE
            },
        );
        assert_eq!(wide.caret(), huge.len());
        let window = wide.check_window();
        assert!(window.len() <= MAX_SPELLCHECK_BYTES);
        crate::clear_spell_check_provider();
    }

    #[test]
    fn definition_targets_prefer_the_selection_then_the_caret_word() {
        let mut input = checking_input("alpha beta", TextCheckingPolicy::NONE);
        assert!(input.set_selection(8, 8));
        assert_eq!(
            input
                .definition_target()
                .map(|(value, start)| (value.to_string(), start)),
            Some(("beta".to_owned(), 6))
        );
        assert!(input.set_selection(0, 5));
        assert_eq!(
            input
                .definition_target()
                .map(|(value, start)| (value.to_string(), start)),
            Some(("alpha".to_owned(), 0))
        );

        let empty = checking_input("   ", TextCheckingPolicy::NONE);
        assert!(empty.definition_target().is_none());
        assert!(!empty.looks_up_on_force_click());
        let forcing = checking_input(
            "word",
            TextCheckingPolicy {
                lookup_on_force_click: true,
                ..TextCheckingPolicy::NONE
            },
        );
        assert!(forcing.looks_up_on_force_click());
    }

    #[test]
    fn accessibility_offsets_use_editor_graphemes() {
        let input = TextInputState::new("a👨‍👩‍👧‍👦é");
        assert_eq!(input.accessibility_character_lengths(), vec![1, 25, 2]);
        assert_eq!(input.accessibility_character_index(1), 1);
        assert_eq!(input.accessibility_character_index(26), 2);
        assert_eq!(input.accessibility_byte_index(2), 26);
        assert_eq!(input.accessibility_byte_index(99), input.text().len());
    }

    #[cfg(feature = "text-input-decorations")]
    #[test]
    fn editor_tab_indents_and_outdents_a_multiline_selection_atomically() {
        let behavior = crate::TextInputIndentation::default().tab_size(2);
        let mut input = TextInputState::with_mode("one\ntwo\nthree", true);
        assert!(input.set_selection(0, 7));
        assert!(input.editor_tab(false, behavior));
        assert_eq!(input.text(), "  one\n  two\nthree");
        assert_eq!(input.selection(), 2..11);
        assert!(input.editor_tab(true, behavior));
        assert_eq!(input.text(), "one\ntwo\nthree");
        assert_eq!(input.selection(), 0..7);
        assert!(input.undo());
        assert_eq!(input.text(), "  one\n  two\nthree");
    }

    #[cfg(feature = "text-input-decorations")]
    #[test]
    fn editor_newline_copies_indent_and_steps_after_an_opener() {
        let behavior = crate::TextInputIndentation::default().tab_size(2);
        let mut input = TextInputState::with_mode("  if ready {", true);
        assert!(input.insert_editor_newline(behavior));
        assert_eq!(input.text(), "  if ready {\n    ");
        assert_eq!(input.selection(), input.text().len()..input.text().len());
    }

    #[cfg(feature = "text-input-decorations")]
    #[test]
    fn read_only_editor_keeps_selection_but_refuses_every_text_history_path() {
        let mut constraints = InputConstraints::default();
        constraints.editor = Some(crate::TextInputIndentation::default().read_only(true));
        let mut input = TextInputState::with_constraints("value", true, constraints);
        assert!(input.set_selection(0, 5));
        assert_eq!(input.selected_text(), Some("value"));
        assert!(!input.replace_selection("other"));
        assert!(!input.insert_editor_newline(crate::TextInputIndentation::default()));
        assert!(!input.can_undo());
        assert!(!input.undo());
        assert_eq!(input.text(), "value");
    }
}
