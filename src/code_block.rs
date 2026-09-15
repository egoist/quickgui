//! Read-only, selectable, virtualized source-code presentation.
//!
//! A [`CodeBlock`] parses its bounded source with the shared Tree-sitter highlighter when the
//! source, language, or syntax theme changes. Scrolling only mounts logical rows and materializes
//! their already-computed paint spans; it never parses or creates an editor caret.

use std::{cell::OnceCell, ops::Range, rc::Rc, sync::Arc};

use crate::{
    AccessibilityRole, Color, Element, ElementId, FontFamily, IntoElement, ListState, StyledText,
    SyntaxLanguage, SyntaxTheme, div,
    syntax::{SyntaxSpan, syntax_spans},
    text,
};

/// Maximum UTF-8 source retained by one code block.
pub const MAX_CODE_BLOCK_SOURCE_BYTES: usize = 4 * 1024 * 1024;
/// Maximum logical lines retained by one code block.
pub const MAX_CODE_BLOCK_LINES: usize = 200_000;
const CODE_BLOCK_BODY_ID_TAG: u64 = 0x636f_6465_626f_6479;
const CODE_BLOCK_ROW_ID_TAG: u64 = 0x636f_6465_726f_7700;
const CODE_BLOCK_GUTTER_ID_TAG: u64 = 0x636f_6465_6775_7472;

/// Appearance and behavior of a read-only code block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CodeBlockStyle {
    pub background: Color,
    pub foreground: Option<Color>,
    pub border: Color,
    pub border_width: f32,
    pub radius: f32,
    pub gutter_background: Color,
    pub gutter_foreground: Option<Color>,
    pub gutter_border: Color,
    pub font_size: f32,
    pub line_height: f32,
    pub word_wrap: bool,
    pub line_numbers: bool,
    pub minimum_line_number_digits: u8,
    pub content_padding_left: f32,
    pub content_padding_right: f32,
    pub content_padding_y: f32,
    pub gutter_padding_left: f32,
    pub gutter_padding_right: f32,
    pub syntax: SyntaxTheme,
}

impl Default for CodeBlockStyle {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
            foreground: None,
            border: Color::TRANSPARENT,
            border_width: 0.0,
            radius: 0.0,
            gutter_background: Color::TRANSPARENT,
            gutter_foreground: None,
            gutter_border: Color::TRANSPARENT,
            font_size: 13.0,
            line_height: 20.0,
            word_wrap: false,
            line_numbers: false,
            minimum_line_number_digits: 2,
            content_padding_left: 0.0,
            content_padding_right: 0.0,
            content_padding_y: 0.0,
            gutter_padding_left: 0.0,
            gutter_padding_right: 0.0,
            syntax: SyntaxTheme::default(),
        }
    }
}

impl CodeBlockStyle {
    fn inner_radius(self) -> f32 {
        (self.radius - self.border_width).max(0.0)
    }

    pub const fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    pub const fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    pub const fn border(mut self, color: Color) -> Self {
        self.border = color;
        self.border_width = 1.0;
        self
    }

    pub const fn gutter_background(mut self, color: Color) -> Self {
        self.gutter_background = color;
        self
    }

    pub const fn gutter_foreground(mut self, color: Color) -> Self {
        self.gutter_foreground = Some(color);
        self
    }

    pub const fn gutter_border(mut self, color: Color) -> Self {
        self.gutter_border = color;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = finite_metric(size, 13.0);
        self
    }

    pub fn line_height(mut self, height: f32) -> Self {
        self.line_height = finite_metric(height, 20.0).max(self.font_size);
        self
    }

    pub const fn word_wrap(mut self, word_wrap: bool) -> Self {
        self.word_wrap = word_wrap;
        self
    }

    pub const fn line_numbers(mut self, line_numbers: bool) -> Self {
        self.line_numbers = line_numbers;
        self
    }

    pub fn minimum_line_number_digits(mut self, digits: usize) -> Self {
        self.minimum_line_number_digits = digits.clamp(1, 12) as u8;
        self
    }

    pub const fn syntax_theme(mut self, syntax: SyntaxTheme) -> Self {
        self.syntax = syntax;
        self
    }

    fn sanitized(mut self) -> Self {
        self.border_width = finite_inset(self.border_width, 0.0);
        self.radius = finite_inset(self.radius, 0.0);
        self.font_size = finite_metric(self.font_size, 13.0);
        self.line_height = finite_metric(self.line_height, 20.0).max(self.font_size);
        self.minimum_line_number_digits = self.minimum_line_number_digits.clamp(1, 12);
        self.content_padding_left = finite_inset(self.content_padding_left, 0.0);
        self.content_padding_right = finite_inset(self.content_padding_right, 0.0);
        self.content_padding_y = finite_inset(self.content_padding_y, 0.0);
        self.gutter_padding_left = finite_inset(self.gutter_padding_left, 0.0);
        self.gutter_padding_right = finite_inset(self.gutter_padding_right, 0.0);
        self
    }
}

/// Result of replacing a code block's controlled source.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CodeBlockUpdate {
    pub changed: bool,
    pub truncated: bool,
}

#[derive(Clone, Debug)]
struct CodeBlockLine {
    source: Arc<str>,
    syntax: Arc<[SyntaxSpan]>,
    styled: Rc<OnceCell<StyledText>>,
    columns: usize,
}

impl CodeBlockLine {
    fn styled_text(&self, theme: SyntaxTheme) -> StyledText {
        self.styled
            .get_or_init(|| {
                StyledText::new(self.source.clone()).with_highlights(
                    self.syntax
                        .iter()
                        .map(|span| (span.range.clone(), theme.style(span.kind))),
                )
            })
            .clone()
    }
}

/// Caller-owned retained code block.
///
/// Keep one instance for each displayed source. Source and language updates rebuild the bounded
/// Tree-sitter capture table; viewport movement reuses it and mounts only visible rows.
pub struct CodeBlock {
    source: Arc<str>,
    language: SyntaxLanguage,
    style: CodeBlockStyle,
    lines: Arc<[CodeBlockLine]>,
    list: ListState,
    content_width: f32,
    syntax_generation: u64,
    truncated: bool,
}

impl Default for CodeBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeBlock {
    pub fn new() -> Self {
        Self::with_text("")
    }

    pub fn with_text(source: &str) -> Self {
        let (source, truncated) = bounded_source(source);
        let source: Arc<str> = Arc::from(source);
        let language = SyntaxLanguage::PlainText;
        let style = CodeBlockStyle::default();
        let syntax_generation = crate::syntax::syntax_language_generation();
        let lines: Arc<[CodeBlockLine]> = build_lines(&source, language).into();
        let list = ListState::new(lines.len(), style.line_height).with_overscan(6);
        let content_width = code_content_width(&lines, style);
        Self {
            source,
            language,
            style,
            lines,
            list,
            content_width,
            syntax_generation,
            truncated,
        }
    }

    pub fn with_language(mut self, language: SyntaxLanguage) -> Self {
        self.set_language(language);
        self
    }

    pub fn with_style(mut self, style: CodeBlockStyle) -> Self {
        self.set_style(style);
        self
    }

    pub fn text(&self) -> &str {
        &self.source
    }

    pub const fn language(&self) -> SyntaxLanguage {
        self.language
    }

    pub const fn style(&self) -> CodeBlockStyle {
        self.style
    }

    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn visible_rows(&self) -> Range<usize> {
        self.list.visible_rows().range
    }

    pub const fn list_state(&self) -> &ListState {
        &self.list
    }

    pub fn set_text(&mut self, source: &str) -> CodeBlockUpdate {
        let (source, truncated) = bounded_source(source);
        if self.source.as_ref() == source && self.truncated == truncated {
            return CodeBlockUpdate {
                changed: false,
                truncated,
            };
        }
        self.source = Arc::from(source);
        self.truncated = truncated;
        self.rebuild(true);
        CodeBlockUpdate {
            changed: true,
            truncated,
        }
    }

    pub fn set_language(&mut self, language: SyntaxLanguage) -> bool {
        if self.language == language {
            return false;
        }
        self.language = language;
        self.rebuild(false);
        true
    }

    pub fn set_style(&mut self, style: CodeBlockStyle) -> bool {
        let style = style.sanitized();
        if self.style == style {
            return false;
        }
        let previous = self.style;
        let syntax_changed = previous.syntax != style.syntax;
        let geometry_changed = previous.font_size != style.font_size
            || previous.line_height != style.line_height
            || previous.word_wrap != style.word_wrap
            || previous.line_numbers != style.line_numbers
            || previous.minimum_line_number_digits != style.minimum_line_number_digits
            || previous.content_padding_left != style.content_padding_left
            || previous.content_padding_right != style.content_padding_right
            || previous.content_padding_y != style.content_padding_y
            || previous.gutter_padding_left != style.gutter_padding_left
            || previous.gutter_padding_right != style.gutter_padding_right;
        self.style = style;
        if syntax_changed {
            self.lines = self
                .lines
                .iter()
                .map(|line| CodeBlockLine {
                    source: line.source.clone(),
                    syntax: line.syntax.clone(),
                    styled: Rc::new(OnceCell::new()),
                    columns: line.columns,
                })
                .collect::<Vec<_>>()
                .into();
        }
        if geometry_changed {
            self.content_width = code_content_width(&self.lines, self.style);
            self.list.remeasure();
        }
        true
    }

    /// Build a selectable, caret-free code surface with a virtualized logical-line window.
    pub fn element(&self, id: impl Into<ElementId>) -> Element {
        let id = id.into();
        let style = self.style;
        let gutter_width = gutter_width(self.lines.len(), style);
        let lines = self.lines.clone();
        let line_count = lines.len();
        let rows = self
            .list
            .render_rows(self.visible_rows(), move |index| {
                render_line(
                    derived_code_block_id(id, CODE_BLOCK_ROW_ID_TAG, index as u64),
                    index,
                    line_count,
                    &lines[index],
                    style,
                    gutter_width,
                )
                .accessibility_row_index(index)
            })
            .min_w(if style.word_wrap {
                0.0
            } else {
                self.content_width
            });
        let mut body = div()
            .id(derived_code_block_id(id, CODE_BLOCK_BODY_ID_TAG, 0))
            .relative()
            .w_full()
            .h_full()
            .min_w(0.0)
            .min_h(0.0)
            .accessibility_role(AccessibilityRole::Grid)
            .accessibility_row_count(line_count)
            .horizontal_scrollbar_left_inset(gutter_width)
            .child(rows);
        if !style.word_wrap {
            body = body.overflow_x_scroll();
        }
        body = body.variable_virtual_scroll(&self.list);
        div()
            .id(id)
            .w_full()
            .h_full()
            .min_w(0.0)
            .min_h(0.0)
            .overflow_hidden()
            .bg(style.background)
            .when(style.foreground.is_some(), |element| {
                element.text_color(style.foreground.unwrap())
            })
            .font_family(FontFamily::Monospace)
            .text_size(style.font_size)
            .line_height(style.line_height)
            .border(style.border_width, style.border)
            .rounded(style.radius)
            .accessibility_role(AccessibilityRole::Region)
            .child(body)
    }

    /// Refresh newly registered injection grammars without resetting scroll state.
    pub fn refresh_syntax_languages(&mut self) -> bool {
        if self.syntax_generation == crate::syntax::syntax_language_generation() {
            return false;
        }
        self.rebuild(false);
        true
    }

    fn rebuild(&mut self, reset_scroll: bool) {
        self.syntax_generation = crate::syntax::syntax_language_generation();
        self.lines = build_lines(&self.source, self.language).into();
        self.content_width = code_content_width(&self.lines, self.style);
        if reset_scroll {
            self.list.reset(self.lines.len());
        } else {
            self.list.set_item_count(self.lines.len());
            self.list.remeasure();
        }
    }
}

fn render_line(
    id: ElementId,
    index: usize,
    line_count: usize,
    line: &CodeBlockLine,
    style: CodeBlockStyle,
    gutter_width: f32,
) -> Element {
    let top = if index == 0 {
        style.content_padding_y
    } else {
        0.0
    };
    let bottom = if index + 1 == line_count {
        style.content_padding_y
    } else {
        0.0
    };
    let mut row = div()
        .id(id)
        .w_full()
        .min_h(style.line_height + top + bottom)
        .flex_row()
        .items_stretch()
        .accessibility_role(AccessibilityRole::Row);
    if style.line_numbers {
        row = row.child(
            div()
                .id(derived_code_block_id(id, CODE_BLOCK_GUTTER_ID_TAG, 0))
                .w(gutter_width)
                .min_h(style.line_height + top + bottom)
                .flex_none()
                .items_center()
                .justify_end()
                .padding(
                    top,
                    style.gutter_padding_right,
                    bottom,
                    style.gutter_padding_left,
                )
                .sticky_left(0.0)
                .z_index(1)
                .bg(style.gutter_background)
                .scroll_background_corners([style.inner_radius(), 0.0, 0.0, style.inner_radius()])
                .when(style.gutter_foreground.is_some(), |gutter| {
                    gutter.text_color(style.gutter_foreground.unwrap())
                })
                .when(style.gutter_border.a > 0.0, |gutter| {
                    gutter.border_right(1.0, style.gutter_border)
                })
                .user_select_none()
                .accessibility_role(AccessibilityRole::RowHeader)
                .child(text((index + 1).to_string())),
        );
    }
    let mut content = line
        .styled_text(style.syntax)
        .into_element()
        .scroll_clip([0.0, 0.0, 0.0, gutter_width], [style.inner_radius(); 4])
        .flex_1()
        .min_w(0.0)
        .min_h(style.line_height + top + bottom)
        .padding(
            top,
            style.content_padding_right,
            bottom,
            style.content_padding_left,
        )
        .text_size(style.font_size)
        .line_height(style.line_height)
        .user_select_text()
        .accessibility_role(AccessibilityRole::GridCell);
    content = if style.word_wrap {
        content.wrap()
    } else {
        content.no_wrap()
    };
    row.child(content)
}

fn build_lines(source: &str, language: SyntaxLanguage) -> Vec<CodeBlockLine> {
    let spans = syntax_spans(source, language);
    let mut lines = Vec::new();
    let mut base = 0;
    for line_with_break in source.split_inclusive('\n') {
        let line = line_with_break
            .strip_suffix('\n')
            .unwrap_or(line_with_break);
        let line = line.strip_suffix('\r').unwrap_or(line);
        push_line(&mut lines, line, base, &spans);
        base += line_with_break.len();
    }
    if source.is_empty() || source.ends_with('\n') {
        push_line(&mut lines, "", source.len(), &spans);
    }
    lines
}

fn push_line(lines: &mut Vec<CodeBlockLine>, line: &str, base: usize, spans: &[SyntaxSpan]) {
    let end = base + line.len();
    let syntax = spans
        .iter()
        .filter_map(|span| {
            let start = span.range.start.max(base);
            let end = span.range.end.min(end);
            (start < end).then(|| SyntaxSpan {
                range: start - base..end - base,
                kind: span.kind,
            })
        })
        .collect::<Vec<_>>();
    lines.push(CodeBlockLine {
        source: Arc::from(line),
        syntax: syntax.into(),
        styled: Rc::new(OnceCell::new()),
        columns: display_columns(line),
    });
}

fn code_content_width(lines: &[CodeBlockLine], style: CodeBlockStyle) -> f32 {
    if style.word_wrap {
        return 0.0;
    }
    let columns = lines.iter().map(|line| line.columns).max().unwrap_or(0);
    let text = columns.min(32_768) as f32 * style.font_size * 0.62
        + style.content_padding_left
        + style.content_padding_right;
    (text
        + if style.line_numbers {
            gutter_width(lines.len(), style)
        } else {
            0.0
        })
    .min(1_000_000.0)
}

fn gutter_width(line_count: usize, style: CodeBlockStyle) -> f32 {
    if !style.line_numbers {
        return 0.0;
    }
    let digits = decimal_digits(line_count).max(style.minimum_line_number_digits as usize);
    digits as f32 * style.font_size * 0.62
        + style.gutter_padding_left
        + style.gutter_padding_right
        + if style.gutter_border.a > 0.0 {
            1.0
        } else {
            0.0
        }
}

fn decimal_digits(mut value: usize) -> usize {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}

fn display_columns(value: &str) -> usize {
    value.chars().fold(0_usize, |columns, character| {
        columns.saturating_add(if character == '\t' { 4 } else { 1 })
    })
}

fn bounded_source(source: &str) -> (&str, bool) {
    let mut end = floor_char_boundary(source, source.len().min(MAX_CODE_BLOCK_SOURCE_BYTES));
    let mut lines = 1;
    for (index, byte) in source[..end].bytes().enumerate() {
        if byte == b'\n' {
            if lines == MAX_CODE_BLOCK_LINES {
                end = index;
                break;
            }
            lines += 1;
        }
    }
    (&source[..end], end < source.len())
}

fn floor_char_boundary(value: &str, mut index: usize) -> usize {
    while !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn finite_metric(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(1.0, 512.0)
    } else {
        fallback
    }
}

fn finite_inset(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 256.0)
    } else {
        fallback
    }
}

fn derived_code_block_id(parent: ElementId, tag: u64, index: u64) -> ElementId {
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

    #[test]
    fn code_block_rebuilds_syntax_only_for_semantic_inputs() {
        let mut block = CodeBlock::with_text("fn main() {}\n");
        assert!(block.set_language(SyntaxLanguage::Rust));
        assert!(!block.set_language(SyntaxLanguage::Rust));
        assert!(!block.set_text("fn main() {}\n").changed);
        assert!(block.set_text("fn answer() -> u8 { 42 }\n").changed);
        assert_eq!(block.line_count(), 2);
        #[cfg(feature = "bundled-languages")]
        assert!(!block.lines[0].syntax.is_empty());
    }

    #[test]
    fn only_mounted_rows_materialize_styled_text() {
        let source = (0..2_000)
            .map(|line| format!("let value_{line}: usize = {line};\n"))
            .collect::<String>();
        let block = CodeBlock::with_text(&source).with_language(SyntaxLanguage::Rust);
        block.list_state().set_viewport_size(320.0, 100.0);
        let mounted = block.visible_rows();
        assert!(mounted.len() < block.line_count());
        let _ = block.element("code");
        assert_eq!(
            block
                .lines
                .iter()
                .filter(|line| line.styled.get().is_some())
                .count(),
            mounted.len()
        );
    }

    #[test]
    fn source_limits_preserve_utf8_and_logical_line_bounds() {
        let source = format!(
            "{}🙂\n{}",
            "x".repeat(MAX_CODE_BLOCK_SOURCE_BYTES - 1),
            "tail\n".repeat(MAX_CODE_BLOCK_LINES)
        );
        let block = CodeBlock::with_text(&source);
        assert!(block.is_truncated());
        assert!(block.text().is_char_boundary(block.text().len()));
        assert!(block.text().len() <= MAX_CODE_BLOCK_SOURCE_BYTES);
        assert!(block.line_count() <= MAX_CODE_BLOCK_LINES);

        let too_many_lines = "line\n".repeat(MAX_CODE_BLOCK_LINES);
        let block = CodeBlock::with_text(&too_many_lines);
        assert!(block.is_truncated());
        assert_eq!(block.line_count(), MAX_CODE_BLOCK_LINES);
    }

    #[test]
    fn line_numbers_expand_without_changing_the_half_open_source() {
        let block = CodeBlock::with_text("one\r\ntwo\n").with_style(
            CodeBlockStyle::default()
                .line_numbers(true)
                .minimum_line_number_digits(4),
        );
        assert_eq!(block.line_count(), 3);
        assert_eq!(block.lines[0].source.as_ref(), "one");
        assert_eq!(
            gutter_width(block.line_count(), block.style()),
            4.0 * block.style().font_size * 0.62
        );
    }
}
