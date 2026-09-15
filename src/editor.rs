//! A compact retained code editor built on QuickGUI's native text-input engine.
//!
//! The editor reuses the same selection, IME, clipboard, undo, accessibility, shaping, and scroll
//! state as [`crate::text_area`]. Its extra work is paint-only: syntax spans, a current-line wash,
//! and a gutter whose visible line numbers are drawn directly by the renderer.

#[cfg(quickgui_component_extension)]
use crate::InputDecorations;
use std::sync::Arc;

use crate::{
    Color, Element, ElementId, ElementStateStyle, FontFamily, StyledText, SyntaxLanguage,
    SyntaxTheme, styled_text_area,
    syntax::{SyntaxSpan, syntax_spans},
};

/// Maximum UTF-8 source retained by one editor model.
pub const MAX_EDITOR_SOURCE_BYTES: usize = 4 * 1024 * 1024;

use crate::{TextInputGutter, TextInputIndentation};

/// Appearance and lexical coloring for one editor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditorStyle {
    pub background: Color,
    pub foreground: Option<Color>,
    pub border: Color,
    pub focus_border: Color,
    pub border_width: f32,
    pub radius: f32,
    pub font_size: f32,
    pub line_height: f32,
    pub word_wrap: bool,
    pub syntax: SyntaxTheme,
    pub presentation: TextInputGutter,
}

impl Default for EditorStyle {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
            foreground: None,
            border: Color::TRANSPARENT,
            focus_border: Color::TRANSPARENT,
            border_width: 0.0,
            radius: 0.0,
            font_size: 13.0,
            line_height: 20.0,
            word_wrap: false,
            syntax: SyntaxTheme::default(),
            presentation: TextInputGutter::default(),
        }
    }
}

impl EditorStyle {
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

    pub const fn focus_border(mut self, color: Color) -> Self {
        self.focus_border = color;
        self.border_width = 1.0;
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

    pub const fn syntax_theme(mut self, syntax: SyntaxTheme) -> Self {
        self.syntax = syntax;
        self
    }

    pub const fn presentation(mut self, presentation: TextInputGutter) -> Self {
        self.presentation = presentation;
        self
    }

    fn sanitized(mut self) -> Self {
        self.font_size = finite_metric(self.font_size, 13.0);
        self.line_height = finite_metric(self.line_height, 20.0).max(self.font_size);
        self.border_width = finite_inset(self.border_width);
        self.radius = finite_inset(self.radius);
        self.presentation = self.presentation.sanitized();
        self
    }
}

/// Result of replacing an editor model's controlled source.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EditorUpdate {
    pub changed: bool,
    pub truncated: bool,
}

/// Caller-owned retained editor model.
///
/// Keep one instance per open document. An input listener calls [`Self::set_text`] and invalidates
/// its view; unchanged renders reuse the same shared source and syntax table.
pub struct Editor {
    source: Arc<str>,
    styled: StyledText,
    syntax_spans: Arc<[SyntaxSpan]>,
    syntax_generation: u64,
    language: SyntaxLanguage,
    style: EditorStyle,
    behavior: TextInputIndentation,
    truncated: bool,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub fn new() -> Self {
        Self::with_text("")
    }

    pub fn with_text(source: &str) -> Self {
        let (source, truncated) = bounded_source(source);
        let source: Arc<str> = Arc::from(source);
        let language = SyntaxLanguage::PlainText;
        let syntax_generation = crate::syntax::syntax_language_generation();
        let syntax_spans: Arc<[SyntaxSpan]> = syntax_spans(&source, language).into();
        Self {
            styled: styled_syntax(source.clone(), &syntax_spans, SyntaxTheme::default()),
            syntax_spans,
            syntax_generation,
            source,
            language,
            style: EditorStyle::default(),
            behavior: TextInputIndentation::default(),
            truncated,
        }
    }

    pub fn with_language(mut self, language: SyntaxLanguage) -> Self {
        self.set_language(language);
        self
    }

    pub fn with_style(mut self, style: EditorStyle) -> Self {
        self.set_style(style);
        self
    }

    pub fn with_behavior(mut self, behavior: TextInputIndentation) -> Self {
        self.set_behavior(behavior);
        self
    }

    pub fn text(&self) -> &str {
        &self.source
    }

    pub const fn language(&self) -> SyntaxLanguage {
        self.language
    }

    pub const fn style(&self) -> EditorStyle {
        self.style
    }

    pub const fn behavior(&self) -> TextInputIndentation {
        self.behavior
    }

    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub fn set_text(&mut self, source: &str) -> EditorUpdate {
        let (source, truncated) = bounded_source(source);
        if self.source.as_ref() == source {
            self.truncated = truncated;
            return EditorUpdate {
                changed: false,
                truncated,
            };
        }
        self.source = Arc::from(source);
        self.truncated = truncated;
        self.refresh_syntax();
        EditorUpdate {
            changed: true,
            truncated,
        }
    }

    pub fn set_language(&mut self, language: SyntaxLanguage) -> bool {
        if self.language == language {
            return false;
        }
        self.language = language;
        self.refresh_syntax();
        true
    }

    pub fn set_style(&mut self, style: EditorStyle) -> bool {
        let style = style.sanitized();
        if self.style == style {
            return false;
        }
        let syntax_changed = self.style.syntax != style.syntax;
        self.style = style;
        if syntax_changed {
            self.refresh_styled_text();
        }
        true
    }

    pub fn set_behavior(&mut self, behavior: TextInputIndentation) -> bool {
        let behavior = behavior.sanitized();
        if self.behavior == behavior {
            return false;
        }
        self.behavior = behavior;
        true
    }

    pub fn set_read_only(&mut self, read_only: bool) -> bool {
        self.set_behavior(self.behavior.read_only(read_only))
    }

    /// Build the editor leaf. Attach a stable [`crate::InputListener`] with
    /// [`Element::on_input`](crate::Element::on_input).
    pub fn element(&self, id: impl Into<ElementId>) -> Element {
        let style = self.style;
        let mut element = styled_text_area(self.styled.clone())
            .id(id)
            .w_full()
            .h_full()
            .min_w(0.0)
            .min_h(0.0)
            .bg(style.background)
            .when(style.foreground.is_some(), |element| {
                element.text_color(style.foreground.unwrap())
            })
            .font_family(FontFamily::Monospace)
            .text_size(style.font_size)
            .line_height(style.line_height)
            .border(style.border_width, style.border)
            .rounded(style.radius)
            .focus(move |_state: ElementStateStyle| {
                let state = ElementStateStyle::default();
                if style.focus_border.a > 0.0 {
                    state.border(style.border_width, style.focus_border)
                } else {
                    state
                }
            })
            .spellcheck(false)
            .grammar_check(false)
            .autocorrect(false)
            .smart_quotes(false)
            .smart_dashes(false)
            .text_replacement(false);
        element = if style.word_wrap {
            element.wrap()
        } else {
            element.no_wrap()
        };
        #[cfg(not(quickgui_component_extension))]
        let crate::element::ElementKind::TextInput(input) = &mut element.kind else {
            unreachable!("styled_text_area always constructs a text input")
        };
        #[cfg(not(quickgui_component_extension))]
        {
            input.editor = Some(style.presentation.sanitized());
            input.constraints.editor = Some(self.behavior.sanitized());
        }
        #[cfg(quickgui_component_extension)]
        element.set_input_decorations(style.presentation, self.behavior);
        element.accessibility_read_only(self.behavior.read_only)
    }

    /// Refresh captures after registering a grammar used by this document or an injection.
    /// Returns false without parsing when the registry is unchanged; preserves editing state.
    pub fn refresh_syntax_languages(&mut self) -> bool {
        if self.syntax_generation == crate::syntax::syntax_language_generation() {
            return false;
        }
        self.refresh_syntax();
        true
    }

    fn refresh_syntax(&mut self) {
        self.syntax_generation = crate::syntax::syntax_language_generation();
        self.syntax_spans = syntax_spans(&self.source, self.language).into();
        self.refresh_styled_text();
    }

    fn refresh_styled_text(&mut self) {
        self.styled = styled_syntax(self.source.clone(), &self.syntax_spans, self.style.syntax);
    }
}

fn styled_syntax(source: Arc<str>, spans: &[SyntaxSpan], theme: SyntaxTheme) -> StyledText {
    StyledText::new(source).with_highlights(
        spans
            .iter()
            .map(|span| (span.range.clone(), theme.style(span.kind))),
    )
}

fn bounded_source(source: &str) -> (&str, bool) {
    if source.len() <= MAX_EDITOR_SOURCE_BYTES {
        return (source, false);
    }
    let mut end = MAX_EDITOR_SOURCE_BYTES;
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

#[cfg(all(test, not(quickgui_component_extension)))]
mod tests {
    use super::*;
    use crate::element::ElementKind;
    use crate::{
        Application, ClipboardItem, IntoElement, Vector, View, ViewContext, WindowOptions,
    };

    #[test]
    fn editor_reuses_source_and_rehighlights_only_semantic_changes() {
        let mut editor = Editor::with_text("fn main() {}\n");
        assert!(editor.set_language(SyntaxLanguage::Rust));
        assert!(!editor.set_language(SyntaxLanguage::Rust));
        assert!(!editor.set_text("fn main() {}\n").changed);
        let syntax = editor.syntax_spans.clone();
        let mut theme = editor.style().syntax;
        theme.keyword = Some(Color::rgb8(255, 0, 255));
        assert!(editor.set_style(editor.style().syntax_theme(theme)));
        assert!(Arc::ptr_eq(&syntax, &editor.syntax_spans));
        assert!(editor.set_text("fn other() {}\n").changed);
        assert_eq!(editor.text(), "fn other() {}\n");
    }

    #[test]
    fn editor_element_is_a_code_configured_multiline_input() {
        let mut editor = Editor::with_text("let answer = 42;");
        editor.set_language(SyntaxLanguage::Rust);
        #[cfg(feature = "bundled-languages")]
        assert!(!editor.syntax_spans.is_empty());
        let element = editor.element("source");
        assert_eq!(element.focus, ElementStateStyle::default());
        let ElementKind::TextInput(input) = element.kind else {
            panic!("editor did not build a text input")
        };
        assert!(input.multiline);
        assert!(
            input
                .editor
                .is_some_and(|presentation| presentation.line_numbers)
        );
        assert_eq!(
            input.constraints.editor,
            Some(TextInputIndentation::default())
        );
        assert!(
            input
                .highlights
                .iter()
                .all(|highlight| highlight.style.color.is_none())
        );
    }

    #[test]
    fn editor_focus_border_is_opt_in() {
        let focus = Color::rgb8(82, 139, 255);
        let element = Editor::new()
            .with_style(EditorStyle::default().focus_border(focus))
            .element("focused-source");
        assert_eq!(element.focus.border_color, Some(focus));
        assert_eq!(element.focus.border_width, Some(1.0));
    }

    #[test]
    fn oversized_source_stops_on_a_utf8_boundary() {
        let source = format!("{}🙂", "x".repeat(MAX_EDITOR_SOURCE_BYTES - 1));
        let editor = Editor::with_text(&source);
        assert!(editor.is_truncated());
        assert!(editor.text().is_char_boundary(editor.text().len()));
        assert!(editor.text().len() <= MAX_EDITOR_SOURCE_BYTES);
    }

    struct ControlledEditor {
        editor: Editor,
    }

    impl View for ControlledEditor {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let input = cx.input_listener("paste-editor", |view, value, cx| {
                if view.editor.set_text(value).changed {
                    cx.invalidate();
                }
            });
            self.editor
                .element("paste-editor")
                .on_input(input)
                .size(300.0, 100.0)
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn active_line_fill_covers_the_code_viewport_including_text_insets() {
        let highlight = Color::rgb8(30, 90, 60);
        for line_numbers in [false, true] {
            let presentation = TextInputGutter {
                line_numbers,
                active_line_background: highlight,
                content_padding_left: 20.0,
                content_padding_right: 30.0,
                content_padding_y: 8.0,
                ..Default::default()
            };
            let (mut cx, view) = Application::new()
                .into_test_context(
                    WindowOptions::default().size(340.0, 140.0),
                    ControlledEditor {
                        editor: Editor::with_text(&"wide code ".repeat(100))
                            .with_style(EditorStyle::default().presentation(presentation)),
                    },
                )
                .unwrap();
            let window = view.window_handle();
            let left = if line_numbers { 80 } else { 8 };
            for scrolled in [false, true] {
                if scrolled {
                    assert!(
                        cx.simulate_retained_scroll(
                            window,
                            "paste-editor",
                            Vector::new(-160.0, 0.0)
                        )
                        .unwrap()
                    );
                }
                let snapshot = cx.capture_screenshot(window).unwrap();
                // Sample below the glyph ink, including the original text-inset columns.
                assert_eq!(
                    snapshot.pixel(8, 54),
                    Some([30, 90, 60, 255]),
                    "gutter and code must share the active row, gutter={line_numbers}, scrolled={scrolled}"
                );
                assert_eq!(
                    snapshot.pixel(left, 54),
                    Some([30, 90, 60, 255]),
                    "left edge, gutter={line_numbers}, scrolled={scrolled}"
                );
                assert_eq!(
                    snapshot.pixel(595, 54),
                    Some([30, 90, 60, 255]),
                    "right edge, gutter={line_numbers}, scrolled={scrolled}"
                );
            }
        }
    }

    #[test]
    fn pasted_multiline_source_scrolls_immediately_without_a_followup_edit() {
        let pasted = (0..80)
            .map(|line| format!("let row_{line} = \"{}\";\n", "wide".repeat(30)))
            .collect::<String>();
        let (mut cx, view) = Application::new()
            .into_test_context(
                WindowOptions::default().size(340.0, 140.0),
                ControlledEditor {
                    editor: Editor::with_text("").with_language(SyntaxLanguage::Rust),
                },
            )
            .unwrap();
        let window = view.window_handle();
        cx.focus(window, "paste-editor").unwrap();
        cx.update(view, |_view, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(pasted.as_str()).unwrap())
                .unwrap();
        })
        .unwrap();
        let primary = if cfg!(target_os = "macos") {
            "cmd-v"
        } else {
            "ctrl-v"
        };
        cx.simulate_keystrokes(window, primary).unwrap();
        assert_eq!(
            cx.focused_input_value(window).unwrap().as_deref(),
            Some(pasted.as_str())
        );
        // Force the paste frame that arms the caret revision before applying a user scroll.
        cx.element_bounds(window, "paste-editor").unwrap();
        assert!(
            cx.simulate_retained_scroll(
                window,
                ElementId::named("paste-editor"),
                Vector::new(-120.0, -120.0),
            )
            .unwrap()
        );
        // Force the retained-scroll paint. A focused editor must not snap back to the pasted
        // caret merely because the pointer moved the viewport.
        cx.element_bounds(window, "paste-editor").unwrap();
        let offset = cx.retained_scroll_offset(window, "paste-editor").unwrap();
        assert!(offset.x > 0.0, "horizontal scroll snapped back: {offset:?}");
        assert!(offset.y > 0.0, "vertical scroll snapped back: {offset:?}");
    }
}
