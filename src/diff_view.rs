//! Virtualized file diffs with split and unified layouts.
//!
//! The data model is detached from Git and the filesystem. Build it from two in-memory files or a
//! captured unified patch, then retain one [`DiffView`] in the owning view. Parsing and lexical
//! coloring happen only when inputs change; scrolling mounts a bounded visible row window.

use std::{cell::OnceCell, ops::Range, rc::Rc, sync::Arc};

use imara_diff::{Algorithm, Diff, InternedInput, sources::lines};

use crate::{
    AccessibilityRole, Color, Element, ElementId, FontFamily, HighlightStyle, IntoElement,
    ListState, MAX_LIST_ITEMS, StyledText, SyntaxLanguage, SyntaxTheme, div, syntax::SyntaxSpan,
    syntax::syntax_spans, text,
};

/// Maximum combined UTF-8 bytes accepted by one diff document.
pub const MAX_DIFF_SOURCE_BYTES: usize = 8 * 1024 * 1024;
/// Maximum files retained from one patch.
pub const MAX_DIFF_FILES: usize = 4_096;
/// Maximum source lines retained by one diff document.
pub const MAX_DIFF_LINES: usize = 200_000;
const DEFAULT_CONTEXT_LINES: usize = 3;
const DIFF_ROW_ID_TAG: u64 = 0x6469_6666_726f_7700;
const DIFF_BODY_ID_TAG: u64 = 0x6469_6666_626f_6479;
const DIFF_HEADER_ID_TAG: u64 = 0x6469_6666_6865_6164;
const DIFF_LEFT_PANE_ID_TAG: u64 = 0x6469_6666_6c65_6674;
const DIFF_RIGHT_PANE_ID_TAG: u64 = 0x6469_6666_7269_6768;
const DIFF_LEFT_ROW_ID_TAG: u64 = 0x6469_6666_6c72_6f77;
const DIFF_RIGHT_ROW_ID_TAG: u64 = 0x6469_6666_7272_6f77;
const DIFF_GUTTER_ID_TAG: u64 = 0x6469_6666_6775_7472;
const DIFF_INDICATOR_ID_TAG: u64 = 0x6469_6666_696e_6469;
const DIFF_BAR_WIDTH: f32 = 4.0;

/// Placement of the old and new sides.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum DiffLayout {
    Unified,
    #[default]
    Split,
}

/// Changed-line marker treatment, following the three primary Pierre Diffs styles.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum DiffIndicators {
    Classic,
    #[default]
    Bars,
    None,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub text: Arc<str>,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub no_newline: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum DiffFileKind {
    #[default]
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Binary,
}

#[derive(Clone, Debug, PartialEq)]
enum DiffSegment {
    Context(Vec<(DiffLine, DiffLine)>),
    Change {
        deletions: Vec<DiffLine>,
        additions: Vec<DiffLine>,
    },
}

/// One changed region plus the unchanged lines retained around it.
#[derive(Clone, Debug, PartialEq)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub collapsed_before: usize,
    pub header: Arc<str>,
    segments: Vec<DiffSegment>,
}

impl DiffHunk {
    pub fn lines(&self) -> impl Iterator<Item = &DiffLine> {
        self.segments.iter().flat_map(|segment| match segment {
            DiffSegment::Context(lines) => lines.iter().map(|(_, line)| line).collect::<Vec<_>>(),
            DiffSegment::Change {
                deletions,
                additions,
            } => deletions.iter().chain(additions).collect(),
        })
    }
}

/// One file represented by a diff document.
#[derive(Clone, Debug, PartialEq)]
pub struct DiffFile {
    pub old_path: Arc<str>,
    pub new_path: Arc<str>,
    pub kind: DiffFileKind,
    pub hunks: Vec<DiffHunk>,
    pub additions: usize,
    pub deletions: usize,
    pub trailing_context: usize,
    pub truncated: bool,
}

impl DiffFile {
    pub fn path(&self) -> &str {
        if self.new_path.is_empty() {
            &self.old_path
        } else {
            &self.new_path
        }
    }

    pub fn is_binary(&self) -> bool {
        self.kind == DiffFileKind::Binary
    }
}

/// Immutable, render-ready diff input.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DiffDocument {
    files: Arc<[DiffFile]>,
    truncated: bool,
}

impl DiffDocument {
    /// Diff two complete in-memory files with Zed's histogram algorithm and Git-like hunk sliding.
    pub fn from_texts(
        old_path: impl Into<Arc<str>>,
        old_text: &str,
        new_path: impl Into<Arc<str>>,
        new_text: &str,
    ) -> Self {
        let old_path = old_path.into();
        let new_path = new_path.into();
        let (old_text, old_truncated) = bounded_diff_source(old_text, MAX_DIFF_SOURCE_BYTES / 2);
        let remaining = MAX_DIFF_SOURCE_BYTES.saturating_sub(old_text.len());
        let (new_text, new_truncated) = bounded_diff_source(new_text, remaining);
        let old_lines = split_source_lines(old_text);
        let new_lines = split_source_lines(new_text);
        let line_truncated = old_lines.truncated || new_lines.truncated;
        let changes = compute_changes(
            &old_text[..old_lines.retained_bytes],
            &new_text[..new_lines.retained_bytes],
        );
        let kind = if old_text.is_empty() && !new_text.is_empty() {
            DiffFileKind::Added
        } else if !old_text.is_empty() && new_text.is_empty() {
            DiffFileKind::Deleted
        } else if old_path != new_path {
            DiffFileKind::Renamed
        } else {
            DiffFileKind::Modified
        };
        let file = build_complete_file(
            old_path,
            new_path,
            kind,
            old_lines.lines,
            new_lines.lines,
            &changes,
            DEFAULT_CONTEXT_LINES,
            line_truncated,
        );
        Self {
            files: Arc::from([file]),
            truncated: old_truncated || new_truncated || line_truncated,
        }
    }

    /// Parse a captured Git or ordinary unified patch. Malformed lines are skipped safely.
    pub fn from_patch(patch: &str) -> Self {
        parse_patch(patch)
    }

    pub fn files(&self) -> &[DiffFile] {
        &self.files
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn additions(&self) -> usize {
        self.files.iter().map(|file| file.additions).sum()
    }

    pub fn deletions(&self) -> usize {
        self.files.iter().map(|file| file.deletions).sum()
    }

    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }
}

/// Optional diff colors. Text inherits from the application; fills are absent by default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiffTheme {
    pub background: Color,
    pub foreground: Option<Color>,
    pub muted: Option<Color>,
    pub border: Color,
    pub header_background: Color,
    pub gutter_background: Color,
    pub line_number: Option<Color>,
    pub hunk_background: Color,
    pub hunk_foreground: Option<Color>,
    pub added_background: Color,
    pub added_gutter_background: Color,
    pub added_foreground: Option<Color>,
    pub removed_background: Color,
    pub removed_gutter_background: Color,
    pub removed_foreground: Option<Color>,
    pub inline_added_background: Color,
    pub inline_removed_background: Color,
}

impl Default for DiffTheme {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
            foreground: None,
            muted: None,
            border: Color::TRANSPARENT,
            header_background: Color::TRANSPARENT,
            gutter_background: Color::TRANSPARENT,
            line_number: None,
            hunk_background: Color::TRANSPARENT,
            hunk_foreground: None,
            added_background: Color::TRANSPARENT,
            added_gutter_background: Color::TRANSPARENT,
            added_foreground: None,
            removed_background: Color::TRANSPARENT,
            removed_gutter_background: Color::TRANSPARENT,
            removed_foreground: None,
            inline_added_background: Color::TRANSPARENT,
            inline_removed_background: Color::TRANSPARENT,
        }
    }
}

/// Layout, typography, and change styling for a diff view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiffViewStyle {
    pub layout: DiffLayout,
    pub indicators: DiffIndicators,
    pub backgrounds: bool,
    pub line_numbers: bool,
    pub word_wrap: bool,
    pub file_header: bool,
    pub font_size: f32,
    pub line_height: f32,
    pub language: Option<SyntaxLanguage>,
    pub syntax: SyntaxTheme,
    pub theme: DiffTheme,
    pub border_width: f32,
    pub radius: f32,
    pub header_height: f32,
    pub header_padding: f32,
    pub gutter_padding: f32,
}

impl Default for DiffViewStyle {
    fn default() -> Self {
        Self {
            layout: DiffLayout::Split,
            indicators: DiffIndicators::Classic,
            backgrounds: true,
            line_numbers: true,
            word_wrap: false,
            file_header: true,
            font_size: 12.5,
            line_height: 20.0,
            language: None,
            syntax: SyntaxTheme::default(),
            theme: DiffTheme::default(),
            border_width: 0.0,
            radius: 0.0,
            header_height: 20.0,
            header_padding: 0.0,
            gutter_padding: 0.0,
        }
    }
}

impl DiffViewStyle {
    pub const fn layout(mut self, layout: DiffLayout) -> Self {
        self.layout = layout;
        self
    }

    pub const fn indicators(mut self, indicators: DiffIndicators) -> Self {
        self.indicators = indicators;
        self
    }

    pub const fn backgrounds(mut self, backgrounds: bool) -> Self {
        self.backgrounds = backgrounds;
        self
    }

    pub const fn line_numbers(mut self, line_numbers: bool) -> Self {
        self.line_numbers = line_numbers;
        self
    }

    pub const fn word_wrap(mut self, word_wrap: bool) -> Self {
        self.word_wrap = word_wrap;
        self
    }

    pub const fn file_header(mut self, file_header: bool) -> Self {
        self.file_header = file_header;
        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = finite_metric(font_size, 12.5);
        self
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = finite_metric(line_height, 20.0).max(self.font_size);
        self
    }

    pub const fn language(mut self, language: Option<SyntaxLanguage>) -> Self {
        self.language = language;
        self
    }

    pub const fn syntax_theme(mut self, syntax: SyntaxTheme) -> Self {
        self.syntax = syntax;
        self
    }

    pub const fn theme(mut self, theme: DiffTheme) -> Self {
        self.theme = theme;
        self
    }

    fn sanitized(mut self) -> Self {
        self.border_width = finite_inset(self.border_width);
        self.radius = finite_inset(self.radius);
        self.header_height = finite_inset(self.header_height);
        self.header_padding = finite_inset(self.header_padding);
        self.gutter_padding = finite_inset(self.gutter_padding);
        self.font_size = finite_metric(self.font_size, 12.5);
        self.line_height = finite_metric(self.line_height, 20.0).max(self.font_size);
        self
    }
}

#[derive(Clone, Debug)]
struct RenderSide {
    line: Option<u32>,
    kind: DiffLineKind,
    source: Arc<str>,
    syntax: SyntaxTheme,
    syntax_spans: Arc<[SyntaxSpan]>,
    inline: Option<Range<usize>>,
    inline_color: Color,
    styled: Rc<OnceCell<StyledText>>,
    no_newline: bool,
}

impl RenderSide {
    fn styled_text(&self) -> StyledText {
        self.styled
            .get_or_init(|| {
                styled_diff_line(
                    self.source.clone(),
                    self.syntax,
                    &self.syntax_spans,
                    self.inline.clone(),
                    self.inline_color,
                )
            })
            .clone()
    }
}

#[derive(Clone, Debug)]
enum RenderRow {
    FileHeader {
        path: Arc<str>,
        additions: usize,
        deletions: usize,
    },
    Gap(Arc<str>),
    Pair {
        old: Option<Box<RenderSide>>,
        new: Option<Box<RenderSide>>,
    },
    Unified {
        old_line: Option<u32>,
        new_line: Option<u32>,
        side: Box<RenderSide>,
    },
    Notice(Arc<str>),
}

#[derive(Clone, Copy, Debug, Default)]
struct DiffContentWidths {
    old: f32,
    new: f32,
    unified: f32,
}

/// Retained virtualized diff component.
pub struct DiffView {
    document: DiffDocument,
    style: DiffViewStyle,
    rows: Arc<[RenderRow]>,
    list: ListState,
    content_widths: DiffContentWidths,
    syntax_generation: u64,
}

impl DiffView {
    pub fn new(document: DiffDocument) -> Self {
        let style = DiffViewStyle::default();
        let syntax_generation = crate::syntax::syntax_language_generation();
        let rows = build_render_rows(&document, style);
        let list = ListState::new(rows.len(), style.line_height).with_overscan(6);
        let content_widths = diff_content_widths(&document, style);
        Self {
            document,
            style,
            rows: rows.into(),
            list,
            content_widths,
            syntax_generation,
        }
    }

    pub fn from_texts(
        old_path: impl Into<Arc<str>>,
        old_text: &str,
        new_path: impl Into<Arc<str>>,
        new_text: &str,
    ) -> Self {
        Self::new(DiffDocument::from_texts(
            old_path, old_text, new_path, new_text,
        ))
    }

    pub fn from_patch(patch: &str) -> Self {
        Self::new(DiffDocument::from_patch(patch))
    }

    pub fn with_style(mut self, style: DiffViewStyle) -> Self {
        self.set_style(style);
        self
    }

    pub const fn document(&self) -> &DiffDocument {
        &self.document
    }

    pub const fn style(&self) -> DiffViewStyle {
        self.style
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn visible_rows(&self) -> Range<usize> {
        self.list.visible_rows().range
    }

    pub fn list_state(&self) -> &ListState {
        &self.list
    }

    pub fn set_document(&mut self, document: DiffDocument) -> bool {
        if self.document == document {
            return false;
        }
        self.document = document;
        self.rebuild(true);
        true
    }

    pub fn set_style(&mut self, style: DiffViewStyle) -> bool {
        let style = style.sanitized();
        if self.style == style {
            return false;
        }
        self.style = style;
        self.rebuild(false);
        true
    }

    /// Render a fixed header and a variable-height virtualized body.
    pub fn element(&self, id: impl Into<ElementId>) -> Element {
        let id = id.into();
        let style = self.style;
        let theme = style.theme;
        let visible = self.visible_rows();
        let rows = self.rows.clone();
        let body = if style.layout == DiffLayout::Split && !style.word_wrap {
            render_split_body(id, &self.list, visible, rows, style, self.content_widths)
        } else {
            let rendered_rows = self
                .list
                .render_rows(visible, move |index| {
                    render_row(
                        derived_diff_id(id, DIFF_ROW_ID_TAG, index as u64),
                        &rows[index],
                        style,
                    )
                    .accessibility_row_index(index)
                })
                .min_w(self.content_widths.unified);
            let mut body = div()
                .id(derived_diff_id(id, DIFF_BODY_ID_TAG, 0))
                .w_full()
                .flex_1()
                .min_w(0.0)
                .min_h(0.0)
                .accessibility_role(AccessibilityRole::Grid)
                .accessibility_row_count(self.rows.len())
                .horizontal_scrollbar_left_inset(diff_gutter_inset(style, true))
                .child(rendered_rows);
            if !style.word_wrap {
                body = body.overflow_x_scroll();
            }
            body.variable_virtual_scroll(&self.list)
        };
        let mut root = div()
            .id(id)
            .w_full()
            .h_full()
            .min_w(0.0)
            .min_h(0.0)
            .flex_col()
            .overflow_hidden()
            .bg(theme.background)
            .when(theme.foreground.is_some(), |element| {
                element.text_color(theme.foreground.unwrap())
            })
            .font_family(FontFamily::Monospace)
            .text_size(style.font_size)
            .line_height(style.line_height)
            .border(style.border_width, theme.border)
            .rounded(style.radius)
            .accessibility_role(AccessibilityRole::Region);
        if style.file_header {
            root = root.child(render_main_header(
                derived_diff_id(id, DIFF_HEADER_ID_TAG, 0),
                &self.document,
                style,
            ));
        }
        root.child(body)
    }

    /// Refresh registered language inference and injections while retaining scroll anchors.
    pub fn refresh_syntax_languages(&mut self) -> bool {
        if self.syntax_generation == crate::syntax::syntax_language_generation() {
            return false;
        }
        self.rebuild(false);
        true
    }

    fn rebuild(&mut self, reset: bool) {
        self.syntax_generation = crate::syntax::syntax_language_generation();
        let rows = build_render_rows(&self.document, self.style);
        self.rows = rows.into();
        self.content_widths = diff_content_widths(&self.document, self.style);
        if reset {
            self.list.reset(self.rows.len());
        } else {
            self.list.set_item_count(self.rows.len());
            self.list.remeasure();
        }
    }
}

fn render_main_header(id: ElementId, document: &DiffDocument, style: DiffViewStyle) -> Element {
    let theme = style.theme;
    let label: Arc<str> = match document.files() {
        [] => Arc::from("Diff"),
        [file] if file.old_path != file.new_path && !file.old_path.is_empty() => {
            Arc::from(format!("{}  →  {}", file.old_path, file.new_path))
        }
        [file] => Arc::from(file.path()),
        files => Arc::from(format!("{} files", files.len())),
    };
    let additions = document.additions();
    let deletions = document.deletions();
    div()
        .id(id)
        .w_full()
        .h(style.header_height.max(style.line_height))
        .flex_none()
        .flex_row()
        .items_center()
        .gap(style.header_padding)
        .padding(0.0, style.header_padding, 0.0, style.header_padding)
        .bg(theme.header_background)
        .rounded_t((style.radius - style.border_width).max(0.0))
        .border_bottom(style.border_width, theme.border)
        .accessibility_role(AccessibilityRole::Group)
        .child(text(label).flex_1().min_w(0.0).no_wrap().text_ellipsis())
        .when(additions > 0, |element| {
            element.child(
                text(format!("+{additions}")).when(theme.added_foreground.is_some(), |element| {
                    element.text_color(theme.added_foreground.unwrap())
                }),
            )
        })
        .when(deletions > 0, |element| {
            element.child(
                text(format!("−{deletions}")).when(theme.removed_foreground.is_some(), |element| {
                    element.text_color(theme.removed_foreground.unwrap())
                }),
            )
        })
}

fn render_split_body(
    id: ElementId,
    list: &ListState,
    visible: Range<usize>,
    rows: Arc<[RenderRow]>,
    style: DiffViewStyle,
    widths: DiffContentWidths,
) -> Element {
    let gutter_inset = diff_gutter_inset(style, false);
    let left_rows = rows.clone();
    let rendered_left = list
        .render_rows(visible, move |index| {
            render_split_column_row(
                derived_diff_id(id, DIFF_LEFT_ROW_ID_TAG, index as u64),
                &left_rows[index],
                false,
                style,
            )
            .accessibility_row_index(index)
        })
        .min_w(widths.old);
    let rendered_right = list
        .render_mirrored_rows(DIFF_RIGHT_ROW_ID_TAG, move |index| {
            render_split_column_row(
                derived_diff_id(id, DIFF_RIGHT_ROW_ID_TAG, index as u64),
                &rows[index],
                true,
                style,
            )
            .accessibility_row_index(index)
        })
        .min_w(widths.new);

    let left = div()
        .id(derived_diff_id(id, DIFF_LEFT_PANE_ID_TAG, 0))
        .w_fraction(0.5)
        .h_full()
        .min_w(0.0)
        .min_h(0.0)
        .overflow_x_scroll()
        .horizontal_scrollbar_left_inset(gutter_inset)
        .hide_vertical_scrollbar()
        .variable_virtual_scroll(list)
        .accessibility_role(AccessibilityRole::Grid)
        .accessibility_row_count(list.item_count())
        .child(rendered_left);
    let right = div()
        .id(derived_diff_id(id, DIFF_RIGHT_PANE_ID_TAG, 0))
        .w_fraction(0.5)
        .h_full()
        .min_w(0.0)
        .min_h(0.0)
        .overflow_x_scroll()
        .horizontal_scrollbar_left_inset(gutter_inset)
        .variable_virtual_scroll_mirror(list)
        .border_left(style.border_width, style.theme.border)
        .accessibility_role(AccessibilityRole::Grid)
        .accessibility_row_count(list.item_count())
        .child(rendered_right);

    div()
        .id(derived_diff_id(id, DIFF_BODY_ID_TAG, 0))
        .w_full()
        .flex_1()
        .min_w(0.0)
        .min_h(0.0)
        .flex_row()
        .items_stretch()
        .overflow_hidden()
        .child(left)
        .child(right)
}

fn render_split_column_row(
    id: ElementId,
    row: &RenderRow,
    new_side: bool,
    style: DiffViewStyle,
) -> Element {
    match row {
        RenderRow::FileHeader {
            path,
            additions,
            deletions,
        } => div()
            .id(id)
            .w_full()
            .h(style.header_height.max(style.line_height))
            .flex_row()
            .items_center()
            .gap(style.header_padding)
            .padding(0.0, style.header_padding, 0.0, style.header_padding)
            .bg(style.theme.header_background)
            .border_bottom(style.border_width, style.theme.border)
            .accessibility_role(AccessibilityRole::Row)
            .child(text(path.clone()).flex_1().min_w(0.0).no_wrap())
            .child(
                text(format!("+{additions}"))
                    .when(style.theme.added_foreground.is_some(), |element| {
                        element.text_color(style.theme.added_foreground.unwrap())
                    }),
            )
            .child(
                text(format!("−{deletions}"))
                    .when(style.theme.removed_foreground.is_some(), |element| {
                        element.text_color(style.theme.removed_foreground.unwrap())
                    }),
            ),
        RenderRow::Gap(label) => div()
            .id(id)
            .w_full()
            .h(style.line_height)
            .flex_row()
            .items_center()
            .padding(0.0, style.header_padding, 0.0, style.header_padding)
            .bg(style.theme.hunk_background)
            .when(style.theme.hunk_foreground.is_some(), |element| {
                element.text_color(style.theme.hunk_foreground.unwrap())
            })
            .accessibility_role(AccessibilityRole::Separator)
            .child(text(label.clone()).text_size((style.font_size - 1.0).max(9.0))),
        RenderRow::Notice(label) => div()
            .id(id)
            .w_full()
            .h(style.line_height)
            .flex_row()
            .items_center()
            .padding(0.0, style.header_padding, 0.0, style.header_padding)
            .when(style.theme.muted.is_some(), |element| {
                element.text_color(style.theme.muted.unwrap())
            })
            .accessibility_role(AccessibilityRole::Row)
            .child(text(label.clone())),
        RenderRow::Pair { old, new } => render_side(
            id,
            if new_side {
                new.as_deref()
            } else {
                old.as_deref()
            },
            new_side,
            style,
        )
        .w_full()
        .h(style.line_height),
        RenderRow::Unified { side, .. } => render_side(id, Some(side.as_ref()), new_side, style)
            .w_full()
            .h(style.line_height),
    }
}

fn render_row(id: ElementId, row: &RenderRow, style: DiffViewStyle) -> Element {
    match row {
        RenderRow::FileHeader {
            path,
            additions,
            deletions,
        } => div()
            .id(id)
            .w_full()
            .h(style.header_height.max(style.line_height))
            .flex_row()
            .items_center()
            .gap(style.header_padding)
            .padding(0.0, style.header_padding, 0.0, style.header_padding)
            .bg(style.theme.header_background)
            .border_bottom(style.border_width, style.theme.border)
            .accessibility_role(AccessibilityRole::Row)
            .child(text(path.clone()).flex_1().min_w(0.0).no_wrap())
            .child(
                text(format!("+{additions}"))
                    .when(style.theme.added_foreground.is_some(), |element| {
                        element.text_color(style.theme.added_foreground.unwrap())
                    }),
            )
            .child(
                text(format!("−{deletions}"))
                    .when(style.theme.removed_foreground.is_some(), |element| {
                        element.text_color(style.theme.removed_foreground.unwrap())
                    }),
            ),
        RenderRow::Gap(label) => div()
            .id(id)
            .w_full()
            .h(style.line_height)
            .flex_row()
            .items_center()
            .padding(0.0, style.header_padding, 0.0, style.header_padding)
            .bg(style.theme.hunk_background)
            .when(style.theme.hunk_foreground.is_some(), |element| {
                element.text_color(style.theme.hunk_foreground.unwrap())
            })
            .accessibility_role(AccessibilityRole::Separator)
            .child(text(label.clone()).text_size((style.font_size - 1.0).max(9.0))),
        RenderRow::Notice(label) => div()
            .id(id)
            .w_full()
            .h(style.line_height)
            .flex_row()
            .items_center()
            .padding(0.0, style.header_padding, 0.0, style.header_padding)
            .when(style.theme.muted.is_some(), |element| {
                element.text_color(style.theme.muted.unwrap())
            })
            .accessibility_role(AccessibilityRole::Row)
            .child(text(label.clone())),
        RenderRow::Pair { old, new } => div()
            .id(id)
            .w_full()
            .min_h(style.line_height)
            .flex_row()
            .items_stretch()
            .accessibility_role(AccessibilityRole::Row)
            .child(
                render_side(
                    derived_diff_id(id, DIFF_LEFT_ROW_ID_TAG, 0),
                    old.as_deref(),
                    false,
                    style,
                )
                .w_fraction(0.5),
            )
            .child(
                render_side(
                    derived_diff_id(id, DIFF_RIGHT_ROW_ID_TAG, 0),
                    new.as_deref(),
                    true,
                    style,
                )
                .w_fraction(0.5)
                .border_left(style.border_width, style.theme.border),
            ),
        RenderRow::Unified {
            old_line,
            new_line,
            side,
        } => render_unified(id, *old_line, *new_line, side, style),
    }
}

fn render_side(
    id: ElementId,
    side: Option<&RenderSide>,
    new_side: bool,
    style: DiffViewStyle,
) -> Element {
    let kind = side.map_or(DiffLineKind::Context, |side| side.kind);
    let background = side_background(kind, style);
    let viewport_corners = diff_viewport_corners(style, Some(new_side));
    let mut row = div()
        .id(id)
        .min_w(0.0)
        .min_h(style.line_height)
        .flex_row()
        .items_stretch()
        .bg(background)
        .scroll_background_corners(viewport_corners)
        .accessibility_role(AccessibilityRole::GridCell);
    if let Some(gutter) = render_gutter(
        id,
        side.and_then(|side| side.line),
        kind,
        style,
        viewport_corners,
    ) {
        row = row.child(gutter);
    }
    let content = side.map_or_else(|| StyledText::new(""), RenderSide::styled_text);
    let mut content = content
        .into_element()
        .scroll_clip(
            [0.0, 0.0, 0.0, diff_gutter_inset(style, false)],
            diff_viewport_corners(style, Some(new_side)),
        )
        .flex_1()
        .min_w(0.0)
        .min_h(style.line_height)
        .text_size(style.font_size)
        .line_height(style.line_height)
        .user_select_text();
    content = if style.word_wrap {
        content.wrap()
    } else {
        content.no_wrap()
    };
    row = row.child(content);
    if side.is_some_and(|side| side.no_newline) {
        row = row.child(
            text("⏎̸")
                .padding(0.0, style.gutter_padding, 0.0, 0.0)
                .text_size((style.font_size - 2.0).max(8.0))
                .when(style.theme.muted.is_some(), |element| {
                    element.text_color(style.theme.muted.unwrap())
                })
                .accessibility_label(if new_side {
                    "New file has no newline at end"
                } else {
                    "Old file has no newline at end"
                }),
        );
    }
    row
}

fn render_unified(
    id: ElementId,
    old_line: Option<u32>,
    new_line: Option<u32>,
    side: &RenderSide,
    style: DiffViewStyle,
) -> Element {
    let kind = side.kind;
    let viewport_corners = diff_viewport_corners(style, None);
    let mut row = div()
        .id(id)
        .w_full()
        .min_h(style.line_height)
        .flex_row()
        .items_stretch()
        .bg(side_background(kind, style))
        .scroll_background_corners(viewport_corners)
        .accessibility_role(AccessibilityRole::Row);
    let number = if kind == DiffLineKind::Deletion {
        old_line
    } else {
        new_line.or(old_line)
    };
    if let Some(gutter) = render_gutter(id, number, kind, style, viewport_corners) {
        row = row.child(gutter);
    }
    let mut content = side
        .styled_text()
        .into_element()
        .scroll_clip(
            [0.0, 0.0, 0.0, diff_gutter_inset(style, true)],
            diff_viewport_corners(style, None),
        )
        .flex_1()
        .min_w(0.0)
        .min_h(style.line_height)
        .text_size(style.font_size)
        .line_height(style.line_height)
        .user_select_text();
    content = if style.word_wrap {
        content.wrap()
    } else {
        content.no_wrap()
    };
    row = row.child(content);
    if side.no_newline {
        row = row.child(
            text("⏎̸")
                .padding(0.0, style.gutter_padding, 0.0, 0.0)
                .text_size((style.font_size - 2.0).max(8.0))
                .when(style.theme.muted.is_some(), |element| {
                    element.text_color(style.theme.muted.unwrap())
                })
                .accessibility_label("No newline at end of file"),
        );
    }
    row
}

fn diff_viewport_corners(style: DiffViewStyle, new_side: Option<bool>) -> [f32; 4] {
    let radius = (style.radius - style.border_width).max(0.0);
    let top = if style.file_header { 0.0 } else { radius };
    if style.layout == DiffLayout::Split && !style.word_wrap {
        match new_side {
            Some(false) => [top, 0.0, 0.0, radius],
            Some(true) => [0.0, top, radius, 0.0],
            None => [top, top, radius, radius],
        }
    } else {
        [top, top, radius, radius]
    }
}

fn render_gutter(
    id: ElementId,
    number: Option<u32>,
    kind: DiffLineKind,
    style: DiffViewStyle,
    viewport_corners: [f32; 4],
) -> Option<Element> {
    let width = diff_gutter_inset(style, false);
    if width == 0.0 {
        return None;
    }
    let color = match kind {
        DiffLineKind::Addition => style.theme.added_foreground,
        DiffLineKind::Deletion => style.theme.removed_foreground,
        DiffLineKind::Context => style.theme.line_number,
    };
    let mut gutter = div()
        .id(derived_diff_id(id, DIFF_GUTTER_ID_TAG, 0))
        .w(width)
        .min_h(style.line_height)
        .flex_none()
        .flex_row()
        .items_stretch()
        .sticky_left(0.0)
        .z_index(2)
        .user_select_none()
        .cursor_default()
        .bg(side_gutter_background(kind, style))
        .scroll_background_corners(viewport_corners);
    if style.indicators == DiffIndicators::Bars {
        gutter = gutter.child(
            div()
                .id(derived_diff_id(id, DIFF_INDICATOR_ID_TAG, 0))
                .w(DIFF_BAR_WIDTH)
                .flex_none()
                .bg(indicator_color(kind, style.theme))
                .scroll_background_corners(viewport_corners),
        );
    } else if style.indicators == DiffIndicators::Classic {
        gutter = gutter.child(
            div()
                .id(derived_diff_id(id, DIFF_INDICATOR_ID_TAG, 0))
                .w(18.0)
                .flex_none()
                .items_center()
                .justify_center()
                .when(color.is_some(), |element| {
                    element.text_color(color.unwrap())
                })
                .child(text(marker(kind, style.indicators))),
        );
    }
    if style.line_numbers {
        gutter = gutter.child(
            div()
                .w(48.0)
                .flex_none()
                .items_center()
                .justify_end()
                .padding(0.0, style.gutter_padding, 0.0, 0.0)
                .when(color.is_some(), |element| {
                    element.text_color(color.unwrap())
                })
                .child(number.map_or_else(|| text(""), |value| text(value.to_string()))),
        );
    }
    Some(gutter)
}

fn side_background(kind: DiffLineKind, style: DiffViewStyle) -> Color {
    if !style.backgrounds {
        return style.theme.background;
    }
    match kind {
        DiffLineKind::Context => style.theme.background,
        DiffLineKind::Addition => style.theme.added_background,
        DiffLineKind::Deletion => style.theme.removed_background,
    }
}

fn side_gutter_background(kind: DiffLineKind, style: DiffViewStyle) -> Color {
    if !style.backgrounds {
        return style.theme.gutter_background;
    }
    // The row already paints beneath its fixed gutter. Transparent defaults share
    // that exact background; only an explicit gutter override adds another fill.
    match kind {
        DiffLineKind::Context => style.theme.gutter_background,
        DiffLineKind::Addition => style.theme.added_gutter_background,
        DiffLineKind::Deletion => style.theme.removed_gutter_background,
    }
}

fn indicator_color(kind: DiffLineKind, theme: DiffTheme) -> Color {
    match kind {
        DiffLineKind::Addition => theme.added_foreground.unwrap_or(Color::TRANSPARENT),
        DiffLineKind::Deletion => theme.removed_foreground.unwrap_or(Color::TRANSPARENT),
        DiffLineKind::Context => Color::TRANSPARENT,
    }
}

fn marker(kind: DiffLineKind, indicators: DiffIndicators) -> &'static str {
    if indicators != DiffIndicators::Classic {
        return "";
    }
    match kind {
        DiffLineKind::Addition => "+",
        DiffLineKind::Deletion => "−",
        DiffLineKind::Context => " ",
    }
}

fn diff_content_widths(document: &DiffDocument, style: DiffViewStyle) -> DiffContentWidths {
    if style.word_wrap {
        return DiffContentWidths::default();
    }
    let mut old_columns = 0_usize;
    let mut new_columns = 0_usize;
    for file in document.files() {
        for hunk in &file.hunks {
            for segment in &hunk.segments {
                match segment {
                    DiffSegment::Context(lines) => {
                        for (old, new) in lines {
                            old_columns = old_columns.max(display_columns(&old.text));
                            new_columns = new_columns.max(display_columns(&new.text));
                        }
                    }
                    DiffSegment::Change {
                        deletions,
                        additions,
                    } => {
                        for line in deletions {
                            old_columns = old_columns.max(display_columns(&line.text));
                        }
                        for line in additions {
                            new_columns = new_columns.max(display_columns(&line.text));
                        }
                    }
                }
            }
        }
    }
    let text_width = |columns: usize| columns.min(32_768) as f32 * style.font_size * 0.62 + 24.0;
    let indicator_width = match style.indicators {
        DiffIndicators::Classic => 18.0,
        DiffIndicators::Bars => DIFF_BAR_WIDTH,
        DiffIndicators::None => 0.0,
    };
    let split_gutter = if style.line_numbers { 48.0 } else { 0.0 };
    let unified_gutters = split_gutter;
    let pane_width =
        |columns| (split_gutter + indicator_width + text_width(columns)).min(1_000_000.0);
    DiffContentWidths {
        old: pane_width(old_columns),
        new: pane_width(new_columns),
        unified: (unified_gutters + indicator_width + text_width(old_columns.max(new_columns)))
            .min(1_000_000.0),
    }
}

fn diff_gutter_inset(style: DiffViewStyle, _unified: bool) -> f32 {
    let line_numbers = if style.line_numbers { 48.0 } else { 0.0 };
    line_numbers
        + match style.indicators {
            DiffIndicators::Classic => 18.0,
            DiffIndicators::Bars => DIFF_BAR_WIDTH,
            DiffIndicators::None => 0.0,
        }
}

fn display_columns(value: &str) -> usize {
    value.chars().fold(0_usize, |columns, character| {
        columns.saturating_add(if character == '\t' { 4 } else { 1 })
    })
}

fn build_render_rows(document: &DiffDocument, style: DiffViewStyle) -> Vec<RenderRow> {
    let mut rows = Vec::new();
    for file in document.files() {
        let file_row_start = rows.len();
        if document.file_count() > 1 {
            rows.push(RenderRow::FileHeader {
                path: Arc::from(file.path()),
                additions: file.additions,
                deletions: file.deletions,
            });
        }
        if file.is_binary() {
            rows.push(RenderRow::Notice(Arc::from("Binary file changed")));
            continue;
        }
        let language = style
            .language
            .or_else(|| SyntaxLanguage::from_path(file.path()))
            .unwrap_or(SyntaxLanguage::PlainText);
        for hunk in &file.hunks {
            if hunk.collapsed_before > 0 {
                rows.push(RenderRow::Gap(Arc::from(format!(
                    "{} unmodified {}",
                    hunk.collapsed_before,
                    if hunk.collapsed_before == 1 {
                        "line"
                    } else {
                        "lines"
                    }
                ))));
            }
            for segment in &hunk.segments {
                match (style.layout, segment) {
                    (DiffLayout::Split, DiffSegment::Context(lines)) => {
                        for (old, new) in lines {
                            rows.push(RenderRow::Pair {
                                old: Some(Box::new(render_side_data(old, language, style, None))),
                                new: Some(Box::new(render_side_data(new, language, style, None))),
                            });
                        }
                    }
                    (
                        DiffLayout::Split,
                        DiffSegment::Change {
                            deletions,
                            additions,
                        },
                    ) => {
                        let count = deletions.len().max(additions.len());
                        for index in 0..count {
                            let old = deletions.get(index);
                            let new = additions.get(index);
                            let inline = old
                                .zip(new)
                                .map(|(old, new)| inline_change_ranges(&old.text, &new.text));
                            rows.push(RenderRow::Pair {
                                old: old.map(|line| {
                                    Box::new(render_side_data(
                                        line,
                                        language,
                                        style,
                                        inline.as_ref().map(|ranges| ranges.0.clone()),
                                    ))
                                }),
                                new: new.map(|line| {
                                    Box::new(render_side_data(
                                        line,
                                        language,
                                        style,
                                        inline.as_ref().map(|ranges| ranges.1.clone()),
                                    ))
                                }),
                            });
                        }
                    }
                    (DiffLayout::Unified, DiffSegment::Context(lines)) => {
                        for (old, new) in lines {
                            rows.push(RenderRow::Unified {
                                old_line: old.old_line,
                                new_line: new.new_line,
                                side: Box::new(render_side_data(new, language, style, None)),
                            });
                        }
                    }
                    (
                        DiffLayout::Unified,
                        DiffSegment::Change {
                            deletions,
                            additions,
                        },
                    ) => {
                        let count = deletions.len().max(additions.len());
                        let inline = (0..count)
                            .map(|index| {
                                deletions
                                    .get(index)
                                    .zip(additions.get(index))
                                    .map(|(old, new)| inline_change_ranges(&old.text, &new.text))
                            })
                            .collect::<Vec<_>>();
                        for (index, line) in deletions.iter().enumerate() {
                            rows.push(RenderRow::Unified {
                                old_line: line.old_line,
                                new_line: None,
                                side: Box::new(render_side_data(
                                    line,
                                    language,
                                    style,
                                    inline[index].as_ref().map(|ranges| ranges.0.clone()),
                                )),
                            });
                        }
                        for (index, line) in additions.iter().enumerate() {
                            rows.push(RenderRow::Unified {
                                old_line: None,
                                new_line: line.new_line,
                                side: Box::new(render_side_data(
                                    line,
                                    language,
                                    style,
                                    inline[index].as_ref().map(|ranges| ranges.1.clone()),
                                )),
                            });
                        }
                    }
                }
                if rows.len() == MAX_LIST_ITEMS {
                    prepare_diff_syntax(&mut rows[file_row_start..], style.layout, language);
                    return rows;
                }
            }
        }
        if !file.hunks.is_empty() && file.trailing_context > 0 {
            rows.push(RenderRow::Gap(Arc::from(format!(
                "{} unmodified {}",
                file.trailing_context,
                if file.trailing_context == 1 {
                    "line"
                } else {
                    "lines"
                }
            ))));
        }
        if file.truncated {
            rows.push(RenderRow::Notice(Arc::from(
                "Diff truncated: the source exceeded the safety limit",
            )));
        }
        if file.hunks.is_empty() && !file.truncated {
            rows.push(RenderRow::Notice(Arc::from("No textual changes")));
        }
        prepare_diff_syntax(&mut rows[file_row_start..], style.layout, language);
    }
    if document.files().is_empty() {
        rows.push(RenderRow::Notice(Arc::from("No diff")));
    }
    rows.truncate(MAX_LIST_ITEMS);
    rows
}

fn render_side_data(
    line: &DiffLine,
    _language: SyntaxLanguage,
    style: DiffViewStyle,
    inline: Option<Range<usize>>,
) -> RenderSide {
    let line_number = match line.kind {
        DiffLineKind::Deletion => line.old_line,
        DiffLineKind::Addition => line.new_line,
        DiffLineKind::Context => line.new_line.or(line.old_line),
    };
    let inline_color = match line.kind {
        DiffLineKind::Addition => style.theme.inline_added_background,
        DiffLineKind::Deletion => style.theme.inline_removed_background,
        DiffLineKind::Context => Color::TRANSPARENT,
    };
    RenderSide {
        line: line_number,
        kind: line.kind,
        source: line.text.clone(),
        syntax: style.syntax,
        syntax_spans: Arc::from([]),
        inline,
        inline_color,
        styled: Rc::new(OnceCell::new()),
        no_newline: line.no_newline,
    }
}

#[derive(Clone, Copy)]
enum DiffSyntaxLane {
    Old,
    New,
    Unified,
}

struct DiffSyntaxTarget {
    row: usize,
    range: Range<usize>,
}

fn prepare_diff_syntax(rows: &mut [RenderRow], layout: DiffLayout, language: SyntaxLanguage) {
    if language == SyntaxLanguage::PlainText {
        return;
    }
    match layout {
        DiffLayout::Split => {
            prepare_diff_syntax_lane(rows, DiffSyntaxLane::Old, language);
            prepare_diff_syntax_lane(rows, DiffSyntaxLane::New, language);
        }
        DiffLayout::Unified => {
            prepare_diff_syntax_lane(rows, DiffSyntaxLane::Unified, language);
        }
    }
}

fn prepare_diff_syntax_lane(
    rows: &mut [RenderRow],
    lane: DiffSyntaxLane,
    language: SyntaxLanguage,
) {
    let mut source = String::new();
    let mut targets = Vec::new();
    for (row, rendered) in rows.iter().enumerate() {
        let Some(side) = syntax_lane_side(rendered, lane) else {
            continue;
        };
        let start = source.len();
        source.push_str(&side.source);
        let end = source.len();
        source.push('\n');
        targets.push(DiffSyntaxTarget {
            row,
            range: start..end,
        });
    }
    if source.is_empty() {
        return;
    }
    let spans = syntax_spans(&source, language);
    for target in targets {
        let local = spans
            .iter()
            .filter_map(|span| {
                let start = span.range.start.max(target.range.start);
                let end = span.range.end.min(target.range.end);
                (start < end).then(|| SyntaxSpan {
                    range: start - target.range.start..end - target.range.start,
                    kind: span.kind,
                })
            })
            .collect::<Vec<_>>();
        if let Some(side) = syntax_lane_side_mut(&mut rows[target.row], lane) {
            side.syntax_spans = local.into();
        }
    }
}

fn syntax_lane_side(row: &RenderRow, lane: DiffSyntaxLane) -> Option<&RenderSide> {
    match (row, lane) {
        (RenderRow::Pair { old, .. }, DiffSyntaxLane::Old) => old.as_deref(),
        (RenderRow::Pair { new, .. }, DiffSyntaxLane::New) => new.as_deref(),
        (RenderRow::Unified { side, .. }, DiffSyntaxLane::Unified) => Some(side.as_ref()),
        _ => None,
    }
}

fn syntax_lane_side_mut(row: &mut RenderRow, lane: DiffSyntaxLane) -> Option<&mut RenderSide> {
    match (row, lane) {
        (RenderRow::Pair { old, .. }, DiffSyntaxLane::Old) => old.as_deref_mut(),
        (RenderRow::Pair { new, .. }, DiffSyntaxLane::New) => new.as_deref_mut(),
        (RenderRow::Unified { side, .. }, DiffSyntaxLane::Unified) => Some(side.as_mut()),
        _ => None,
    }
}

fn styled_diff_line(
    source: Arc<str>,
    syntax: SyntaxTheme,
    spans: &[SyntaxSpan],
    inline: Option<Range<usize>>,
    inline_color: Color,
) -> StyledText {
    let mut boundaries = Vec::with_capacity(spans.len() * 2 + 4);
    boundaries.extend([0, source.len()]);
    for span in spans {
        boundaries.extend([span.range.start, span.range.end]);
    }
    if let Some(inline) = &inline {
        boundaries.extend([inline.start.min(source.len()), inline.end.min(source.len())]);
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    StyledText::new(source).with_highlights(
        boundaries
            .windows(2)
            .filter_map(|boundary| {
                let range = boundary[0]..boundary[1];
                if range.is_empty() {
                    return None;
                }
                let token = spans
                    .iter()
                    .find(|span| span.range.start <= range.start && span.range.end >= range.end);
                let inline = inline
                    .as_ref()
                    .is_some_and(|inline| inline.start < range.end && inline.end > range.start);
                if token.is_none() && !inline {
                    return None;
                }
                let mut highlight =
                    token.map_or_else(HighlightStyle::default, |span| syntax.style(span.kind));
                if inline {
                    highlight = highlight.background(inline_color);
                }
                Some((range, highlight))
            })
            .take(crate::MAX_TEXT_HIGHLIGHTS),
    )
}

fn inline_change_ranges(old: &str, new: &str) -> (Range<usize>, Range<usize>) {
    let mut old_prefix = 0;
    let mut new_prefix = 0;
    let mut old_chars = old.char_indices();
    let mut new_chars = new.char_indices();
    loop {
        match (old_chars.next(), new_chars.next()) {
            (Some((old_index, old_char)), Some((new_index, new_char))) if old_char == new_char => {
                old_prefix = old_index + old_char.len_utf8();
                new_prefix = new_index + new_char.len_utf8();
            }
            _ => break,
        }
    }
    let old_tail = &old[old_prefix..];
    let new_tail = &new[new_prefix..];
    let mut old_suffix = 0;
    let mut new_suffix = 0;
    let mut old_chars = old_tail.char_indices().rev();
    let mut new_chars = new_tail.char_indices().rev();
    loop {
        match (old_chars.next(), new_chars.next()) {
            (Some((old_index, old_char)), Some((new_index, new_char))) if old_char == new_char => {
                old_suffix = old_tail.len() - old_index;
                new_suffix = new_tail.len() - new_index;
            }
            _ => break,
        }
    }
    (
        old_prefix..old.len().saturating_sub(old_suffix),
        new_prefix..new.len().saturating_sub(new_suffix),
    )
}

#[derive(Clone, Debug)]
struct SourceLine {
    text: Arc<str>,
    no_newline: bool,
}

struct SourceLines {
    lines: Vec<SourceLine>,
    truncated: bool,
    retained_bytes: usize,
}

fn split_source_lines(source: &str) -> SourceLines {
    let mut output = Vec::new();
    let mut truncated = false;
    let mut retained_bytes = 0;
    for line in lines(source) {
        if output.len() == MAX_DIFF_LINES {
            truncated = true;
            break;
        }
        let retained_line_bytes = line.len();
        let no_newline = !line.ends_with('\n');
        let line = line
            .strip_suffix('\n')
            .unwrap_or(line)
            .strip_suffix('\r')
            .unwrap_or_else(|| line.strip_suffix('\n').unwrap_or(line));
        output.push(SourceLine {
            text: Arc::from(line),
            no_newline,
        });
        retained_bytes += retained_line_bytes;
    }
    SourceLines {
        lines: output,
        truncated,
        retained_bytes,
    }
}

#[derive(Clone, Debug)]
struct RawChange {
    old: Range<usize>,
    new: Range<usize>,
}

fn compute_changes(old: &str, new: &str) -> Vec<RawChange> {
    let input = InternedInput::new(lines(old), lines(new));
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_lines(&input);
    diff.hunks()
        .map(|hunk| RawChange {
            old: hunk.before.start as usize..hunk.before.end as usize,
            new: hunk.after.start as usize..hunk.after.end as usize,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn build_complete_file(
    old_path: Arc<str>,
    new_path: Arc<str>,
    kind: DiffFileKind,
    old: Vec<SourceLine>,
    new: Vec<SourceLine>,
    changes: &[RawChange],
    context: usize,
    truncated: bool,
) -> DiffFile {
    let mut hunks = Vec::new();
    let mut previous_old_end = 0;
    let mut previous_new_end = 0;
    let mut additions = 0;
    let mut deletions = 0;
    let mut change_index = 0;
    while change_index < changes.len() {
        let first_index = change_index;
        let mut old_end = (changes[change_index].old.end + context).min(old.len());
        let mut new_end = (changes[change_index].new.end + context).min(new.len());
        change_index += 1;
        while change_index < changes.len()
            && changes[change_index].old.start <= old_end + context
            && changes[change_index].new.start <= new_end + context
        {
            old_end = (changes[change_index].old.end + context).min(old.len());
            new_end = (changes[change_index].new.end + context).min(new.len());
            change_index += 1;
        }
        let first = &changes[first_index];
        let old_start = first.old.start.saturating_sub(context);
        let new_start = first.new.start.saturating_sub(context);
        let collapsed_before = old_start
            .saturating_sub(previous_old_end)
            .min(new_start.saturating_sub(previous_new_end));
        let mut segments = Vec::new();
        let mut old_cursor = old_start;
        let mut new_cursor = new_start;
        for change in &changes[first_index..change_index] {
            let context_count = change
                .old
                .start
                .saturating_sub(old_cursor)
                .min(change.new.start.saturating_sub(new_cursor));
            if context_count > 0 {
                segments.push(DiffSegment::Context(
                    (0..context_count)
                        .map(|offset| {
                            context_pair(
                                &old[old_cursor + offset],
                                &new[new_cursor + offset],
                                old_cursor + offset,
                                new_cursor + offset,
                            )
                        })
                        .collect(),
                ));
            }
            let deletion_lines = old[change.old.clone()]
                .iter()
                .enumerate()
                .map(|(offset, line)| {
                    source_diff_line(
                        line,
                        DiffLineKind::Deletion,
                        Some(change.old.start + offset),
                        None,
                    )
                })
                .collect::<Vec<_>>();
            let addition_lines = new[change.new.clone()]
                .iter()
                .enumerate()
                .map(|(offset, line)| {
                    source_diff_line(
                        line,
                        DiffLineKind::Addition,
                        None,
                        Some(change.new.start + offset),
                    )
                })
                .collect::<Vec<_>>();
            deletions += deletion_lines.len();
            additions += addition_lines.len();
            segments.push(DiffSegment::Change {
                deletions: deletion_lines,
                additions: addition_lines,
            });
            old_cursor = change.old.end;
            new_cursor = change.new.end;
        }
        let trailing = old_end
            .saturating_sub(old_cursor)
            .min(new_end.saturating_sub(new_cursor));
        if trailing > 0 {
            segments.push(DiffSegment::Context(
                (0..trailing)
                    .map(|offset| {
                        context_pair(
                            &old[old_cursor + offset],
                            &new[new_cursor + offset],
                            old_cursor + offset,
                            new_cursor + offset,
                        )
                    })
                    .collect(),
            ));
        }
        let old_count = old_end.saturating_sub(old_start);
        let new_count = new_end.saturating_sub(new_start);
        hunks.push(DiffHunk {
            old_start: one_based_hunk_start(old_start, old_count),
            old_count: old_count as u32,
            new_start: one_based_hunk_start(new_start, new_count),
            new_count: new_count as u32,
            collapsed_before,
            header: Arc::from(format!(
                "@@ -{},{} +{},{} @@",
                one_based_hunk_start(old_start, old_count),
                old_count,
                one_based_hunk_start(new_start, new_count),
                new_count
            )),
            segments,
        });
        previous_old_end = old_end;
        previous_new_end = new_end;
    }
    let trailing_context = old
        .len()
        .saturating_sub(previous_old_end)
        .min(new.len().saturating_sub(previous_new_end));
    DiffFile {
        old_path,
        new_path,
        kind,
        hunks,
        additions,
        deletions,
        trailing_context,
        truncated,
    }
}

fn one_based_hunk_start(start: usize, count: usize) -> u32 {
    if count == 0 {
        start as u32
    } else {
        start.saturating_add(1) as u32
    }
}

fn context_pair(
    old: &SourceLine,
    new: &SourceLine,
    old_index: usize,
    new_index: usize,
) -> (DiffLine, DiffLine) {
    (
        source_diff_line(old, DiffLineKind::Context, Some(old_index), Some(new_index)),
        source_diff_line(new, DiffLineKind::Context, Some(old_index), Some(new_index)),
    )
}

fn source_diff_line(
    line: &SourceLine,
    kind: DiffLineKind,
    old: Option<usize>,
    new: Option<usize>,
) -> DiffLine {
    DiffLine {
        kind,
        text: line.text.clone(),
        old_line: old.map(|line| line.saturating_add(1) as u32),
        new_line: new.map(|line| line.saturating_add(1) as u32),
        no_newline: line.no_newline,
    }
}

#[derive(Default)]
struct PatchFile {
    old_path: String,
    new_path: String,
    kind: DiffFileKind,
    binary: bool,
    hunks: Vec<DiffHunk>,
    current_header: Option<ParsedHunkHeader>,
    current_lines: Vec<DiffLine>,
    additions: usize,
    deletions: usize,
    previous_old_end: usize,
    previous_new_end: usize,
    truncated: bool,
}

#[derive(Clone)]
struct ParsedHunkHeader {
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
    raw: Arc<str>,
}

fn parse_patch(patch: &str) -> DiffDocument {
    let (patch, source_truncated) = bounded_diff_source(patch, MAX_DIFF_SOURCE_BYTES);
    let mut files = Vec::new();
    let mut current: Option<PatchFile> = None;
    let mut old_line = 0_u32;
    let mut new_line = 0_u32;
    let mut retained_lines = 0;
    let mut structure_truncated = false;
    for raw in patch.lines() {
        if let Some(header) = raw.strip_prefix("diff --git ") {
            flush_patch_file(&mut current, &mut files);
            if files.len() == MAX_DIFF_FILES {
                structure_truncated = true;
                break;
            }
            let (old_path, new_path) = parse_git_header_paths(header);
            current = Some(PatchFile {
                old_path,
                new_path,
                ..PatchFile::default()
            });
            continue;
        }
        if raw.starts_with("--- ")
            && current
                .as_ref()
                .is_some_and(|file| file.current_header.is_some() || !file.hunks.is_empty())
        {
            flush_patch_file(&mut current, &mut files);
            if files.len() == MAX_DIFF_FILES {
                structure_truncated = true;
                break;
            }
            current = Some(PatchFile::default());
        }
        if current.is_none() && raw.starts_with("--- ") {
            current = Some(PatchFile::default());
        }
        let Some(file) = current.as_mut() else {
            continue;
        };
        if let Some(header) = parse_hunk_header(raw) {
            flush_patch_hunk(file);
            old_line = if header.old_start == 0 {
                1
            } else {
                header.old_start
            };
            new_line = if header.new_start == 0 {
                1
            } else {
                header.new_start
            };
            file.current_header = Some(header);
            continue;
        }
        if file.current_header.is_some() {
            if raw.starts_with('\\') {
                if let Some(line) = file.current_lines.last_mut() {
                    line.no_newline = true;
                }
                continue;
            }
            let Some(marker) = raw.as_bytes().first().copied() else {
                continue;
            };
            if !matches!(marker, b' ' | b'+' | b'-') {
                continue;
            }
            if retained_lines == MAX_DIFF_LINES {
                file.truncated = true;
                continue;
            }
            retained_lines += 1;
            let content = Arc::from(raw.get(1..).unwrap_or_default());
            let line = match marker {
                b'+' => {
                    let line = DiffLine {
                        kind: DiffLineKind::Addition,
                        text: content,
                        old_line: None,
                        new_line: Some(new_line),
                        no_newline: false,
                    };
                    new_line = new_line.saturating_add(1);
                    file.additions += 1;
                    line
                }
                b'-' => {
                    let line = DiffLine {
                        kind: DiffLineKind::Deletion,
                        text: content,
                        old_line: Some(old_line),
                        new_line: None,
                        no_newline: false,
                    };
                    old_line = old_line.saturating_add(1);
                    file.deletions += 1;
                    line
                }
                _ => {
                    let line = DiffLine {
                        kind: DiffLineKind::Context,
                        text: content,
                        old_line: Some(old_line),
                        new_line: Some(new_line),
                        no_newline: false,
                    };
                    old_line = old_line.saturating_add(1);
                    new_line = new_line.saturating_add(1);
                    line
                }
            };
            file.current_lines.push(line);
            continue;
        }
        if let Some(value) = raw.strip_prefix("new file mode ") {
            let _ = value;
            file.kind = DiffFileKind::Added;
        } else if let Some(value) = raw.strip_prefix("deleted file mode ") {
            let _ = value;
            file.kind = DiffFileKind::Deleted;
        } else if let Some(path) = raw.strip_prefix("rename from ") {
            file.kind = DiffFileKind::Renamed;
            file.old_path = path.to_owned();
        } else if let Some(path) = raw.strip_prefix("rename to ") {
            file.kind = DiffFileKind::Renamed;
            file.new_path = path.to_owned();
        } else if let Some(path) = raw.strip_prefix("copy from ") {
            file.kind = DiffFileKind::Copied;
            file.old_path = path.to_owned();
        } else if let Some(path) = raw.strip_prefix("copy to ") {
            file.kind = DiffFileKind::Copied;
            file.new_path = path.to_owned();
        } else if raw.starts_with("Binary files ") || raw == "GIT binary patch" {
            file.binary = true;
            file.kind = DiffFileKind::Binary;
        } else if let Some(path) = raw.strip_prefix("--- ") {
            if path == "/dev/null" {
                file.old_path.clear();
                file.kind = DiffFileKind::Added;
            } else {
                file.old_path = path.strip_prefix("a/").unwrap_or(path).to_owned();
            }
        } else if let Some(path) = raw.strip_prefix("+++ ") {
            if path == "/dev/null" {
                file.new_path.clear();
                file.kind = DiffFileKind::Deleted;
            } else {
                file.new_path = path.strip_prefix("b/").unwrap_or(path).to_owned();
            }
        }
    }
    if files.len() < MAX_DIFF_FILES {
        flush_patch_file(&mut current, &mut files);
    } else if current.is_some() {
        structure_truncated = true;
    }
    if (source_truncated || structure_truncated || retained_lines == MAX_DIFF_LINES)
        && let Some(last) = files.last_mut()
    {
        last.truncated = true;
    }
    DiffDocument {
        files: files.into(),
        truncated: source_truncated || retained_lines == MAX_DIFF_LINES || structure_truncated,
    }
}

fn flush_patch_file(current: &mut Option<PatchFile>, files: &mut Vec<DiffFile>) {
    let Some(mut file) = current.take() else {
        return;
    };
    flush_patch_hunk(&mut file);
    if file.old_path.is_empty() && file.new_path.is_empty() && file.hunks.is_empty() && !file.binary
    {
        return;
    }
    if files.len() == MAX_DIFF_FILES {
        return;
    }
    files.push(DiffFile {
        old_path: Arc::from(file.old_path),
        new_path: Arc::from(file.new_path),
        kind: file.kind,
        hunks: file.hunks,
        additions: file.additions,
        deletions: file.deletions,
        trailing_context: 0,
        truncated: file.truncated,
    });
}

fn flush_patch_hunk(file: &mut PatchFile) {
    let Some(header) = file.current_header.take() else {
        return;
    };
    if file.hunks.len() == MAX_DIFF_LINES {
        file.current_lines.clear();
        file.truncated = true;
        return;
    }
    let old_start_index = header
        .old_start
        .saturating_sub((header.old_count > 0) as u32) as usize;
    let new_start_index = header
        .new_start
        .saturating_sub((header.new_count > 0) as u32) as usize;
    let collapsed_before = old_start_index
        .saturating_sub(file.previous_old_end)
        .min(new_start_index.saturating_sub(file.previous_new_end));
    let lines = std::mem::take(&mut file.current_lines);
    let mut segments = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if lines[index].kind == DiffLineKind::Context {
            let start = index;
            while index < lines.len() && lines[index].kind == DiffLineKind::Context {
                index += 1;
            }
            segments.push(DiffSegment::Context(
                lines[start..index]
                    .iter()
                    .cloned()
                    .map(|line| (line.clone(), line))
                    .collect(),
            ));
        } else {
            let start = index;
            while index < lines.len() && lines[index].kind != DiffLineKind::Context {
                index += 1;
            }
            let deletions = lines[start..index]
                .iter()
                .filter(|line| line.kind == DiffLineKind::Deletion)
                .cloned()
                .collect();
            let additions = lines[start..index]
                .iter()
                .filter(|line| line.kind == DiffLineKind::Addition)
                .cloned()
                .collect();
            segments.push(DiffSegment::Change {
                deletions,
                additions,
            });
        }
    }
    file.previous_old_end = old_start_index + header.old_count as usize;
    file.previous_new_end = new_start_index + header.new_count as usize;
    file.hunks.push(DiffHunk {
        old_start: header.old_start,
        old_count: header.old_count,
        new_start: header.new_start,
        new_count: header.new_count,
        collapsed_before,
        header: header.raw,
        segments,
    });
}

fn parse_hunk_header(line: &str) -> Option<ParsedHunkHeader> {
    let rest = line.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(" +")?;
    let (new, _) = rest.split_once(" @@")?;
    let (old_start, old_count) = parse_hunk_range(old)?;
    let (new_start, new_count) = parse_hunk_range(new)?;
    Some(ParsedHunkHeader {
        old_start,
        old_count,
        new_start,
        new_count,
        raw: Arc::from(line),
    })
}

fn parse_hunk_range(value: &str) -> Option<(u32, u32)> {
    let (start, count) = value.split_once(',').unwrap_or((value, "1"));
    Some((start.parse().ok()?, count.parse().ok()?))
}

fn parse_git_header_paths(value: &str) -> (String, String) {
    if value.len() % 2 == 1 {
        let middle = value.len() / 2;
        if value.as_bytes().get(middle) == Some(&b' ')
            && value.starts_with("a/")
            && value
                .get(middle + 1..)
                .is_some_and(|right| right.starts_with("b/"))
            && value.get(2..middle) == value.get(middle + 3..)
        {
            return (value[2..middle].to_owned(), value[middle + 3..].to_owned());
        }
    }
    if let Some(index) = value.find(" b/")
        && value.starts_with("a/")
    {
        return (value[2..index].to_owned(), value[index + 3..].to_owned());
    }
    (value.to_owned(), value.to_owned())
}

fn bounded_diff_source(source: &str, maximum: usize) -> (&str, bool) {
    if source.len() <= maximum {
        return (source, false);
    }
    let mut end = maximum;
    while !source.is_char_boundary(end) {
        end -= 1;
    }
    (&source[..end], true)
}

fn finite_inset(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 256.0)
    } else {
        0.0
    }
}

fn finite_metric(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(1.0, 512.0)
    } else {
        fallback
    }
}

fn derived_diff_id(parent: ElementId, tag: u64, index: u64) -> ElementId {
    let mut hash = parent.as_u64() ^ tag ^ index.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == parent.as_u64() || hash == u64::MAX {
        hash ^= tag.rotate_left(13);
    }
    ElementId::new(hash)
}

#[cfg(all(test, not(quickgui_component_extension)))]
mod tests {
    use super::*;
    use crate::{Application, IntoElement, Vector, View, ViewContext, WindowOptions};

    const PATCH: &str = "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,4 +1,5 @@\n fn main() {\n-    println!(\"old\");\n+    println!(\"new\");\n+    work();\n }\n ";

    #[test]
    fn computes_complete_hunks_with_collapsed_context() {
        let old = (1..=20)
            .map(|line| format!("line {line}\n"))
            .collect::<String>();
        let new = old.replace("line 10\n", "line ten\n");
        let document = DiffDocument::from_texts("old.txt", &old, "new.txt", &new);
        let file = &document.files()[0];
        assert_eq!((file.additions, file.deletions), (1, 1));
        assert_eq!(file.hunks.len(), 1);
        assert_eq!(file.hunks[0].collapsed_before, 6);
        assert_eq!(file.trailing_context, 7);
    }

    #[test]
    fn parses_patch_lines_and_line_numbers() {
        let document = DiffDocument::from_patch(PATCH);
        let file = &document.files()[0];
        assert_eq!(file.path(), "src/main.rs");
        assert_eq!((file.additions, file.deletions), (2, 1));
        let changed = file
            .hunks
            .iter()
            .flat_map(|hunk| hunk.lines())
            .filter(|line| line.kind != DiffLineKind::Context)
            .collect::<Vec<_>>();
        assert_eq!(changed.len(), 3);
        assert_eq!(changed[0].old_line, Some(2));
        assert_eq!(changed[1].new_line, Some(2));
    }

    #[test]
    fn split_and_unified_rows_have_expected_shape() {
        let document = DiffDocument::from_patch(PATCH);
        let split = DiffView::new(document.clone());
        let split_rows = split.row_count();
        let mut unified = DiffView::new(document);
        unified.set_style(DiffViewStyle::default().layout(DiffLayout::Unified));
        assert!(unified.row_count() > split_rows);
        assert!(split.visible_rows().end <= split.row_count());
    }

    #[test]
    fn inline_ranges_keep_unicode_boundaries() {
        let (old, new) = inline_change_ranges("café🙂x", "café🙂y");
        assert_eq!(&"café🙂x"[old], "x");
        assert_eq!(&"café🙂y"[new], "y");
    }

    #[test]
    fn default_gutters_share_the_row_without_a_second_background() {
        let mut style = DiffViewStyle::default();
        for kind in [
            DiffLineKind::Context,
            DiffLineKind::Addition,
            DiffLineKind::Deletion,
        ] {
            assert_eq!(side_gutter_background(kind, style), Color::TRANSPARENT);
        }
        let custom = Color::rgba8(30, 90, 60, 100);
        style.theme.added_gutter_background = custom;
        assert_eq!(
            side_gutter_background(DiffLineKind::Addition, style),
            custom
        );
    }

    #[test]
    fn source_limits_are_reported_without_splitting_utf8() {
        let patch = format!("{}🙂", "x".repeat(MAX_DIFF_SOURCE_BYTES - 1));
        let document = DiffDocument::from_patch(&patch);
        assert!(document.is_truncated());
    }

    #[test]
    fn final_newline_changes_are_not_lost() {
        let document = DiffDocument::from_texts("value.txt", "value\n", "value.txt", "value");
        assert_eq!((document.additions(), document.deletions()), (1, 1));
        let lines = document.files()[0].hunks[0].lines().collect::<Vec<_>>();
        assert!(lines.iter().any(|line| line.no_newline));
    }

    #[test]
    fn identical_files_render_one_notice_without_a_fake_context_gap() {
        let view = DiffView::from_texts("same.txt", "one\ntwo\n", "same.txt", "one\ntwo\n");
        assert_eq!(view.row_count(), 1);
        assert!(matches!(view.rows[0], RenderRow::Notice(_)));
    }

    #[test]
    fn ordinary_unified_patches_can_contain_multiple_files() {
        let patch = "--- old-a.txt\n+++ new-a.txt\n@@ -1 +1 @@\n-a\n+A\n--- old-b.txt\n+++ new-b.txt\n@@ -1 +1 @@\n-b\n+B\n";
        let document = DiffDocument::from_patch(patch);
        assert_eq!(document.file_count(), 2);
        assert_eq!((document.additions(), document.deletions()), (2, 2));
        assert_eq!(document.files()[1].path(), "new-b.txt");
    }

    struct ScrollDiff {
        diff: DiffView,
    }

    impl View for ScrollDiff {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            self.diff.element("scroll-diff").size(321.0, 120.0)
        }
    }

    #[test]
    fn split_diff_combines_horizontal_overflow_with_virtual_vertical_scroll() {
        let old = (0..180)
            .map(|line| format!("old {line} {}\n", "wide".repeat(30)))
            .collect::<String>();
        let new = (0..180)
            .map(|line| format!("new {line} {}\n", "wider".repeat(30)))
            .collect::<String>();
        let diff = DiffView::from_texts("old.rs", &old, "new.rs", &new);
        let list = diff.list_state().clone();
        let (mut cx, view) = Application::new()
            .into_test_context(
                WindowOptions::default().size(361.0, 160.0),
                ScrollDiff { diff },
            )
            .unwrap();
        let window = view.window_handle();
        let root = ElementId::named("scroll-diff");
        let left = derived_diff_id(root, DIFF_LEFT_PANE_ID_TAG, 0);
        let right = derived_diff_id(root, DIFF_RIGHT_PANE_ID_TAG, 0);
        let left_bounds = cx.element_bounds(window, left).unwrap();
        let right_bounds = cx.element_bounds(window, right).unwrap();
        let first_left_row = derived_diff_id(root, DIFF_LEFT_ROW_ID_TAG, 0);
        let first_left_gutter = derived_diff_id(first_left_row, DIFF_GUTTER_ID_TAG, 0);
        let gutter_bounds = cx.element_bounds(window, first_left_gutter).unwrap();
        assert!((left_bounds.width - right_bounds.width).abs() <= 1.0);
        assert!((left_bounds.right() - right_bounds.x).abs() <= 1.0);
        assert!(
            cx.simulate_retained_scroll(window, left, Vector::new(-180.0, 0.0))
                .unwrap()
        );
        let scrolled_gutter_bounds = cx.element_bounds(window, first_left_gutter).unwrap();
        assert_eq!(scrolled_gutter_bounds.x, gutter_bounds.x);
        assert_eq!(scrolled_gutter_bounds.width, gutter_bounds.width);
        assert!(
            cx.simulate_retained_scroll(window, left, Vector::new(0.0, -100.0))
                .unwrap()
        );
        let left_offset = cx.retained_scroll_offset(window, left).unwrap();
        let right_offset = cx.retained_scroll_offset(window, right).unwrap();
        assert!(
            left_offset.x > 0.0,
            "left horizontal offset did not move: {left_offset:?}"
        );
        assert!(
            left_offset.y > 0.0,
            "left vertical offset did not move: {left_offset:?}"
        );
        assert_eq!(right_offset.x, 0.0);
        assert_eq!(right_offset.y, left_offset.y);
        assert!(list.scroll_offset() > 0.0);
        let moved_left_bounds = cx.element_bounds(window, left).unwrap();
        let moved_right_bounds = cx.element_bounds(window, right).unwrap();
        assert_eq!(moved_left_bounds, left_bounds);
        assert_eq!(moved_right_bounds, right_bounds);
    }
}
