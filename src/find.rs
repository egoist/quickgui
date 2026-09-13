//! Bounded find and replace over one controlled text value, plus an unstyled `FindBar`.
//!
//! [`FindState`] is a pure, application-owned value: it never retains the document, never spawns a
//! task, and never schedules a frame. Searching happens exactly when the query, options, or text
//! change. [`FindBar`] adds roles, relationships, and keyboard behavior to caller-owned parts and
//! contributes no appearance.

use std::{ops::Range, sync::Arc};

use crate::{
    AccessibilityRole, Color, Element, ElementId, HighlightStyle, KeyBinding, TextHighlight,
};

/// Maximum matches retained for one query.
///
/// Additional matches are not searched for; [`FindState::is_truncated`] reports the bound so an
/// application can tell the user the count is a lower bound.
pub const MAX_FIND_MATCHES: usize = 4_096;

/// Maximum UTF-8 bytes accepted for one find query.
pub const MAX_FIND_QUERY_BYTES: usize = 1_024;

/// Maximum UTF-8 bytes accepted for one replacement string.
pub const MAX_FIND_REPLACEMENT_BYTES: usize = 4 * 1_024;

/// Key context declared by [`FindBar::root_with`].
pub const FIND_BAR_KEY_CONTEXT: &str = "FindBar";

/// Move to the next match, wrapping at the end.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FindNext;

/// Move to the previous match, wrapping at the start.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FindPrevious;

/// Replace the current match and advance.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FindReplace;

/// Replace every match as one undoable edit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FindReplaceAll;

/// Dismiss the find bar.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FindClose;

/// Contextual keyboard behavior for a mounted find bar.
///
/// Return advances, Shift+Return retreats, and Escape closes. Bind them once with
/// [`crate::Application::bind_keys`]-style application configuration.
pub fn find_bar_key_bindings() -> [KeyBinding; 3] {
    [
        KeyBinding::new("enter", FindNext, Some(FIND_BAR_KEY_CONTEXT)),
        KeyBinding::new("shift-enter", FindPrevious, Some(FIND_BAR_KEY_CONTEXT)),
        KeyBinding::new("escape", FindClose, Some(FIND_BAR_KEY_CONTEXT)),
    ]
}

/// Literal matching options.
///
/// Regular expressions are intentionally absent: an unbounded engine cannot honor the framework's
/// bounded, non-blocking guarantees.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct FindOptions {
    /// Compare characters exactly instead of by Unicode simple lowercase.
    pub case_sensitive: bool,
    /// Require a non-word character (or a document edge) on both sides of a match.
    pub whole_word: bool,
}

/// Bounded literal find and replace over one controlled string.
#[derive(Clone, Debug)]
pub struct FindState {
    query: Arc<str>,
    replacement: Arc<str>,
    options: FindOptions,
    matches: Vec<Range<usize>>,
    truncated: bool,
    current: Option<usize>,
    match_style: HighlightStyle,
    current_match_style: HighlightStyle,
}

impl Default for FindState {
    fn default() -> Self {
        Self::new()
    }
}

impl FindState {
    /// Create an empty, unstyled find session.
    pub fn new() -> Self {
        Self {
            query: Arc::from(""),
            replacement: Arc::from(""),
            options: FindOptions::default(),
            matches: Vec::new(),
            truncated: false,
            current: None,
            match_style: HighlightStyle::default().background(Color::rgba8(250, 204, 21, 72)),
            current_match_style: HighlightStyle::default()
                .background(Color::rgba8(249, 115, 22, 160)),
        }
    }

    /// Replace the run style used for every match.
    #[must_use]
    pub fn match_style(mut self, style: HighlightStyle) -> Self {
        self.match_style = style;
        self
    }

    /// Replace the run style used for the current match.
    #[must_use]
    pub fn current_match_style(mut self, style: HighlightStyle) -> Self {
        self.current_match_style = style;
        self
    }

    /// The current query.
    pub fn query(&self) -> &Arc<str> {
        &self.query
    }

    /// The current replacement string.
    pub fn replacement(&self) -> &Arc<str> {
        &self.replacement
    }

    /// The current matching options.
    pub const fn options(&self) -> FindOptions {
        self.options
    }

    /// Every retained match, in document order.
    pub fn matches(&self) -> &[Range<usize>] {
        &self.matches
    }

    /// The number of retained matches.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Whether the match list hit [`MAX_FIND_MATCHES`].
    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }

    /// The zero-based index of the current match.
    pub const fn current_index(&self) -> Option<usize> {
        self.current
    }

    /// The current match range.
    pub fn current_match(&self) -> Option<Range<usize>> {
        self.matches.get(self.current?).cloned()
    }

    /// Replace the query and research `text`, keeping the caret near the previous match.
    ///
    /// Queries longer than [`MAX_FIND_QUERY_BYTES`] are rejected and clear the session.
    pub fn set_query(&mut self, query: impl Into<Arc<str>>, text: &str) -> bool {
        let query = query.into();
        let query = if query.len() > MAX_FIND_QUERY_BYTES {
            Arc::from("")
        } else {
            query
        };
        if self.query == query {
            return false;
        }
        self.query = query;
        self.refresh(text);
        true
    }

    /// Replace the replacement string. Values over [`MAX_FIND_REPLACEMENT_BYTES`] are rejected.
    pub fn set_replacement(&mut self, replacement: impl Into<Arc<str>>) -> bool {
        let replacement = replacement.into();
        if replacement.len() > MAX_FIND_REPLACEMENT_BYTES || self.replacement == replacement {
            return false;
        }
        self.replacement = replacement;
        true
    }

    /// Replace the matching options and research `text`.
    pub fn set_options(&mut self, options: FindOptions, text: &str) -> bool {
        if self.options == options {
            return false;
        }
        self.options = options;
        self.refresh(text);
        true
    }

    /// Recompute matches after the searched text changed.
    ///
    /// The current match is preserved by document position when possible, so replacing text does
    /// not jump the user back to the first match.
    pub fn refresh(&mut self, text: &str) -> bool {
        let anchor = self.current_match().map(|range| range.start);
        let previous = std::mem::take(&mut self.matches);
        self.truncated = false;
        if !self.query.is_empty() {
            let mut cursor = 0_usize;
            while let Some(found) = next_match(text, &self.query, self.options, cursor) {
                cursor = found.end.max(found.start + 1);
                self.matches.push(found);
                if self.matches.len() == MAX_FIND_MATCHES {
                    self.truncated = true;
                    break;
                }
            }
        }
        self.current = anchor.and_then(|anchor| {
            self.matches
                .iter()
                .position(|range| range.start >= anchor)
                .or(if self.matches.is_empty() {
                    None
                } else {
                    Some(0)
                })
        });
        if self.current.is_none() && !self.matches.is_empty() {
            self.current = Some(0);
        }
        self.matches != previous
    }

    /// Select the next match, wrapping at the end of the document.
    pub fn find_next(&mut self, text: &str) -> Option<Range<usize>> {
        self.refresh(text);
        if self.matches.is_empty() {
            self.current = None;
            return None;
        }
        self.current = Some(match self.current {
            Some(index) if index + 1 < self.matches.len() => index + 1,
            Some(_) => 0,
            None => 0,
        });
        self.current_match()
    }

    /// Select the previous match, wrapping at the start of the document.
    pub fn find_previous(&mut self, text: &str) -> Option<Range<usize>> {
        self.refresh(text);
        if self.matches.is_empty() {
            self.current = None;
            return None;
        }
        self.current = Some(match self.current {
            Some(0) | None => self.matches.len() - 1,
            Some(index) => index - 1,
        });
        self.current_match()
    }

    /// Produce the text that results from replacing the current match.
    ///
    /// Returns the new value and the range the replacement now occupies. The caller applies it as
    /// one controlled edit, so it participates in the input's own undo history.
    pub fn replace_current(&mut self, text: &str) -> Option<(String, Range<usize>)> {
        let range = self.current_match()?;
        if range.end > text.len() {
            return None;
        }
        let mut replaced = String::with_capacity(text.len() + self.replacement.len());
        replaced.push_str(&text[..range.start]);
        replaced.push_str(&self.replacement);
        replaced.push_str(&text[range.end..]);
        let inserted = range.start..range.start + self.replacement.len();
        self.refresh(&replaced);
        Some((replaced, inserted))
    }

    /// Produce the text that results from replacing every match.
    ///
    /// The caller applies the returned value as one controlled edit, which the input records as a
    /// single undo entry.
    pub fn replace_all(&self, text: &str) -> Option<String> {
        if self.matches.is_empty() {
            return None;
        }
        let mut replaced = String::with_capacity(text.len());
        let mut cursor = 0_usize;
        for range in &self.matches {
            if range.start < cursor || range.end > text.len() {
                continue;
            }
            replaced.push_str(&text[cursor..range.start]);
            replaced.push_str(&self.replacement);
            cursor = range.end;
        }
        replaced.push_str(&text[cursor..]);
        Some(replaced)
    }

    /// Project match highlights for the searched text input.
    ///
    /// Ranges are sorted and non-overlapping, so the result can be merged into a controlled run
    /// table or handed to [`crate::styled_text`] directly.
    pub fn highlights(&self) -> Vec<TextHighlight> {
        let current = self.current_match();
        self.matches
            .iter()
            .map(|range| {
                let style = if Some(range) == current.as_ref() {
                    self.current_match_style.clone()
                } else {
                    self.match_style.clone()
                };
                TextHighlight {
                    range: range.clone(),
                    style,
                }
            })
            .collect()
    }

    /// The conventional `2 of 17` label for a count part.
    pub fn count_label(&self) -> String {
        if self.query.is_empty() {
            return String::new();
        }
        if self.matches.is_empty() {
            return "No results".to_owned();
        }
        let position = self.current.map_or(0, |index| index + 1);
        let total = self.matches.len();
        if self.truncated {
            format!("{position} of {total}+")
        } else {
            format!("{position} of {total}")
        }
    }
}

fn next_match(text: &str, query: &str, options: FindOptions, from: usize) -> Option<Range<usize>> {
    if query.is_empty() || from > text.len() {
        return None;
    }
    let mut index = from;
    while index <= text.len() {
        if !text.is_char_boundary(index) {
            index += 1;
            continue;
        }
        if let Some(end) = match_at(text, index, query, options.case_sensitive)
            && (!options.whole_word || is_whole_word(text, index..end))
        {
            return Some(index..end);
        }
        if index == text.len() {
            break;
        }
        index += 1;
    }
    None
}

fn match_at(text: &str, index: usize, query: &str, case_sensitive: bool) -> Option<usize> {
    let mut haystack = text.get(index..)?.chars();
    let mut end = index;
    for expected in query.chars() {
        let found = haystack.next()?;
        if !characters_match(found, expected, case_sensitive) {
            return None;
        }
        end += found.len_utf8();
    }
    Some(end)
}

fn characters_match(found: char, expected: char, case_sensitive: bool) -> bool {
    if case_sensitive {
        found == expected
    } else {
        found == expected || found.to_lowercase().eq(expected.to_lowercase())
    }
}

fn is_whole_word(text: &str, range: Range<usize>) -> bool {
    let before = text[..range.start].chars().next_back();
    let after = text[range.end..].chars().next();
    !before.is_some_and(is_word_character) && !after.is_some_and(is_word_character)
}

fn is_word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

const FIND_BAR_QUERY_TAG: u64 = 0x6669_6e64_5f71_7565;
const FIND_BAR_REPLACE_TAG: u64 = 0x6669_6e64_5f72_706c;
const FIND_BAR_COUNT_TAG: u64 = 0x6669_6e64_5f63_6e74;
const FIND_BAR_NEXT_TAG: u64 = 0x6669_6e64_5f6e_7874;
const FIND_BAR_PREVIOUS_TAG: u64 = 0x6669_6e64_5f70_7276;
const FIND_BAR_REPLACE_BUTTON_TAG: u64 = 0x6669_6e64_5f72_6231;
const FIND_BAR_REPLACE_ALL_TAG: u64 = 0x6669_6e64_5f72_6261;
const FIND_BAR_CLOSE_TAG: u64 = 0x6669_6e64_5f63_6c73;

/// Unstyled find-and-replace composition over caller-owned parts.
///
/// The application owns every visual decision, the two controlled inputs, and the edit that a
/// replacement performs. `FindBar` supplies stable part identities, roles, relationships, the
/// find-bar key context, and an accessible match count. It retains no task, timer, or registry.
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use = "a FindBar descriptor has no effect until its parts are mounted"]
pub struct FindBar {
    id: ElementId,
    label: Arc<str>,
    count: Arc<str>,
    has_matches: bool,
    can_replace: bool,
}

impl FindBar {
    /// Create a descriptor rooted at one stable ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: Arc::from("Find"),
            count: Arc::from(""),
            has_matches: false,
            can_replace: false,
        }
    }

    /// Project one [`FindState`] into the descriptor's accessible count and enabled state.
    pub fn state(mut self, state: &FindState) -> Self {
        self.count = Arc::from(state.count_label());
        self.has_matches = state.match_count() > 0;
        self.can_replace = self.has_matches && !state.query().is_empty();
        self
    }

    /// Replace the accessible name of the find bar group.
    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = label.into();
        self
    }

    /// The root element ID.
    pub const fn id(&self) -> ElementId {
        self.id
    }

    /// The query input ID, usable as a focus target.
    pub fn query_input_id(&self) -> ElementId {
        derived_find_id(self.id, FIND_BAR_QUERY_TAG)
    }

    /// The replacement input ID.
    pub fn replace_input_id(&self) -> ElementId {
        derived_find_id(self.id, FIND_BAR_REPLACE_TAG)
    }

    /// The match-count element ID.
    pub fn count_id(&self) -> ElementId {
        derived_find_id(self.id, FIND_BAR_COUNT_TAG)
    }

    /// The rendered match count, for example `2 of 17`.
    pub fn count_text(&self) -> &Arc<str> {
        &self.count
    }

    /// Decorate the application-owned root and declare the find-bar key context.
    pub fn root_with(&self, root: Element) -> Element {
        root.id(self.id)
            .key_context(FIND_BAR_KEY_CONTEXT)
            .accessibility_role(AccessibilityRole::Group)
            .accessibility_label(self.label.clone())
            .accessibility_described_by(self.count_id())
    }
    /// Create the unstyled root part. Use [`Self::root_with`] to supply an existing element.
    pub fn root(&self) -> Element {
        self.root_with(crate::div())
    }

    /// Decorate the caller-owned query text input.
    pub fn query_input_with(&self, input: Element) -> Element {
        input
            .id(self.query_input_id())
            .accessibility_label(self.label.clone())
            .accessibility_described_by(self.count_id())
    }
    /// Create the unstyled query input part. Use [`Self::query_input_with`] to supply an existing element.
    pub fn query_input(&self) -> Element {
        self.query_input_with(crate::text_input(""))
    }

    /// Decorate the caller-owned replacement text input.
    pub fn replace_input_with(&self, input: Element) -> Element {
        input
            .id(self.replace_input_id())
            .accessibility_label("Replace with")
    }
    /// Create the unstyled replace input part. Use [`Self::replace_input_with`] to supply an existing element.
    pub fn replace_input(&self) -> Element {
        self.replace_input_with(crate::text_input(""))
    }

    /// Decorate the visible match count and expose it as this bar's description.
    pub fn count_with(&self, count: Element) -> Element {
        count
            .id(self.count_id())
            .accessibility_role(AccessibilityRole::Label)
            .accessibility_value(self.count.clone())
    }
    /// Create the unstyled count part. Use [`Self::count_with`] to supply an existing element.
    pub fn count(&self) -> Element {
        self.count_with(crate::div())
    }

    /// Decorate the "next match" control.
    pub fn next_with(&self, button: Element) -> Element {
        self.command_part(button, FIND_BAR_NEXT_TAG, "Next match", !self.has_matches)
    }
    /// Create the unstyled next part. Use [`Self::next_with`] to supply an existing element.
    pub fn next(&self) -> Element {
        self.next_with(crate::button())
    }

    /// Decorate the "previous match" control.
    pub fn previous_with(&self, button: Element) -> Element {
        self.command_part(
            button,
            FIND_BAR_PREVIOUS_TAG,
            "Previous match",
            !self.has_matches,
        )
    }
    /// Create the unstyled previous part. Use [`Self::previous_with`] to supply an existing element.
    pub fn previous(&self) -> Element {
        self.previous_with(crate::button())
    }

    /// Decorate the "replace current match" control.
    pub fn replace_with(&self, button: Element) -> Element {
        self.command_part(
            button,
            FIND_BAR_REPLACE_BUTTON_TAG,
            "Replace",
            !self.can_replace,
        )
    }
    /// Create the unstyled replace part. Use [`Self::replace_with`] to supply an existing element.
    pub fn replace(&self) -> Element {
        self.replace_with(crate::button())
    }

    /// Decorate the "replace every match" control.
    pub fn replace_all_with(&self, button: Element) -> Element {
        self.command_part(
            button,
            FIND_BAR_REPLACE_ALL_TAG,
            "Replace all",
            !self.can_replace,
        )
    }
    /// Create the unstyled replace all part. Use [`Self::replace_all_with`] to supply an existing element.
    pub fn replace_all(&self) -> Element {
        self.replace_all_with(crate::button())
    }

    /// Decorate the dismissal control.
    pub fn close_with(&self, button: Element) -> Element {
        self.command_part(button, FIND_BAR_CLOSE_TAG, "Close find bar", false)
    }
    /// Create the unstyled close part. Use [`Self::close_with`] to supply an existing element.
    pub fn close(&self) -> Element {
        self.close_with(crate::button())
    }

    fn command_part(&self, button: Element, tag: u64, label: &str, disabled: bool) -> Element {
        button
            .id(derived_find_id(self.id, tag))
            .accessibility_role(AccessibilityRole::Button)
            .accessibility_label(label)
            .disabled(disabled)
    }
}

fn derived_find_id(parent: ElementId, tag: u64) -> ElementId {
    let mut hash = parent.as_u64() ^ tag;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == parent.as_u64() || hash == u64::MAX {
        hash ^= tag.rotate_left(17);
    }
    ElementId::new(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TextCheckingOverrides, element::InputConstraints, text_input::TextInputState};

    #[test]
    fn literal_matching_honors_case_and_word_boundaries() {
        let text = "Cat cats CAT concat";
        let mut find = FindState::new();
        assert!(find.set_query("cat", text));
        assert_eq!(find.match_count(), 4);

        assert!(find.set_options(
            FindOptions {
                case_sensitive: true,
                whole_word: false
            },
            text,
        ));
        assert_eq!(find.matches(), &[4..7, 16..19]);

        assert!(find.set_options(
            FindOptions {
                case_sensitive: false,
                whole_word: true
            },
            text,
        ));
        assert_eq!(find.matches(), &[0..3, 9..12]);
    }

    #[test]
    fn navigation_wraps_in_both_directions() {
        let text = "a b a b a";
        let mut find = FindState::new();
        assert!(find.set_query("a", text));
        assert_eq!(find.current_match(), Some(0..1));
        assert_eq!(find.find_next(text), Some(4..5));
        assert_eq!(find.find_next(text), Some(8..9));
        assert_eq!(find.find_next(text), Some(0..1));
        assert_eq!(find.find_previous(text), Some(8..9));
        assert_eq!(find.current_index(), Some(2));
        assert_eq!(find.count_label(), "3 of 3");

        let mut empty = FindState::new();
        assert!(empty.set_query("zz", text));
        assert_eq!(empty.find_next(text), None);
        assert_eq!(empty.count_label(), "No results");
    }

    #[test]
    fn matches_are_bounded_and_report_truncation() {
        let text = "x".repeat(MAX_FIND_MATCHES + 32);
        let mut find = FindState::new();
        assert!(find.set_query("x", &text));
        assert_eq!(find.match_count(), MAX_FIND_MATCHES);
        assert!(find.is_truncated());
        assert!(find.count_label().ends_with('+'));

        let mut oversized = FindState::new();
        assert!(!oversized.set_query("x".repeat(MAX_FIND_QUERY_BYTES + 1), &text));
        assert!(oversized.query().is_empty());
        assert!(!oversized.set_replacement("y".repeat(MAX_FIND_REPLACEMENT_BYTES + 1)));
    }

    #[test]
    fn replacing_the_current_match_advances_without_losing_position() {
        let text = "one two one";
        let mut find = FindState::new();
        assert!(find.set_query("one", text));
        assert!(find.set_replacement("1"));
        assert_eq!(find.find_next(text), Some(8..11));

        let (replaced, range) = find.replace_current(text).expect("a current match exists");
        assert_eq!(replaced, "one two 1");
        assert_eq!(range, 8..9);
        assert_eq!(find.match_count(), 1);
    }

    #[test]
    fn replace_all_is_one_edit_and_one_undo_entry() {
        let text = "red red red";
        let mut find = FindState::new();
        assert!(find.set_query("red", text));
        assert!(find.set_replacement("blue"));
        let replaced = find.replace_all(text).expect("matches exist");
        assert_eq!(replaced, "blue blue blue");

        let mut input = TextInputState::with_constraints(
            text,
            false,
            InputConstraints {
                read_only: false,
                max_length: None,
                filter: None,
                text_checking: TextCheckingOverrides::default(),
            },
        );
        assert!(input.set_value(&replaced));
        assert_eq!(input.text(), "blue blue blue");
        assert!(input.undo());
        assert_eq!(input.text(), "red red red");
        assert!(!input.can_undo());
    }

    #[test]
    fn highlights_distinguish_the_current_match() {
        let text = "aa aa";
        let mut find = FindState::new();
        assert!(find.set_query("aa", text));
        let highlights = find.highlights();
        assert_eq!(highlights.len(), 2);
        assert_eq!(highlights[0].range(), 0..2);
        assert_ne!(highlights[0].style(), highlights[1].style());

        find.find_next(text);
        let highlights = find.highlights();
        assert_eq!(highlights[1].style(), &find.current_match_style);
        assert_eq!(highlights[0].style(), &find.match_style);
    }

    #[test]
    fn find_bar_parts_are_stable_labelled_and_state_driven() {
        let text = "alpha alpha";
        let mut find = FindState::new();
        assert!(find.set_query("alpha", text));
        let bar = FindBar::new("editor-find").state(&find);
        assert_eq!(bar.count_text().as_ref(), "1 of 2");

        let root = bar.root_with(crate::div().w(320.0));
        assert_eq!(root.explicit_id, Some(bar.id()));
        assert!(root.key_context.is_some());
        assert_eq!(root.accessibility.role, AccessibilityRole::Group);
        assert_eq!(
            root.accessibility.relations.described_by(),
            Some(bar.count_id())
        );

        let count = bar.count_with(crate::text("1 of 2"));
        assert_eq!(count.explicit_id, Some(bar.count_id()));
        assert_eq!(count.accessibility.value.as_deref(), Some("1 of 2"));

        let next = bar.next_with(crate::button());
        assert_eq!(
            next.explicit_id,
            Some(derived_find_id(bar.id(), FIND_BAR_NEXT_TAG))
        );
        assert!(!next.accessibility.disabled);

        let empty = FindBar::new("editor-find").state(&FindState::new());
        assert!(empty.next_with(crate::button()).accessibility.disabled);
        assert!(
            empty
                .replace_all_with(crate::button())
                .accessibility
                .disabled
        );

        let ids = [
            bar.id(),
            bar.query_input_id(),
            bar.replace_input_id(),
            bar.count_id(),
        ];
        for (index, id) in ids.iter().enumerate() {
            assert!(!ids[..index].contains(id));
        }
    }

    #[test]
    fn find_bar_key_bindings_cover_return_shift_return_and_escape() {
        let bindings = find_bar_key_bindings();
        assert_eq!(bindings.len(), 3);
        assert!(
            bindings
                .iter()
                .all(|binding| binding.context_predicate().is_some())
        );
        assert!(
            bindings
                .iter()
                .any(|binding| binding.action().downcast_ref::<FindPrevious>().is_some())
        );
    }
}
