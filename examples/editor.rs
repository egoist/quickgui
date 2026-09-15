use quickgui::{
    Application, CodeBlock, CodeBlockStyle, Color, DiffIndicators, DiffLayout, DiffTheme, DiffView,
    DiffViewStyle, Editor, EditorStyle, IntoElement, SyntaxLanguage, SyntaxTheme, TextInputGutter,
    TitleBarStyle, View, ViewContext, WindowOptions, button, div, text,
};

const BEFORE: &str = r#"pub fn greeting(name: &str) -> String {
    format!("Hello, {name}!")
}

fn main() {
    println!("{}", greeting("world"));
}
"#;

const AFTER: &str = r#"pub fn greeting(name: &str) -> String {
    format!("Hello from QuickGUI, {name}!")
}

fn main() {
    let audience = "editor";
    println!("{}", greeting(audience));
}
"#;

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions::new("QuickGUI — Editor and diff")
                .size(1180.0, 720.0)
                .title_bar_style(TitleBarStyle::HiddenInset)
                .traffic_light_position(16.0, 19.0),
            EditorDemo::new(),
        );
    })
}

struct EditorDemo {
    editor: Editor,
    code: CodeBlock,
    diff: DiffView,
    layout: DiffLayout,
}

impl EditorDemo {
    fn new() -> Self {
        let rust = SyntaxLanguage::from_name("rust").unwrap_or_else(|| {
            let mut definition = quickgui::SyntaxLanguageDefinition::new(
                "rust",
                tree_sitter_rust::LANGUAGE.into(),
                tree_sitter_rust::HIGHLIGHTS_QUERY,
            );
            definition.extensions = vec!["rs".into()];
            quickgui::register_syntax_language(definition).expect("valid example Rust grammar")
        });
        let (before, after) = scrollable_diff_sources();
        let code_style = CodeBlockStyle {
            background: Color::rgb8(30, 31, 36),
            foreground: Some(Color::rgb8(220, 223, 230)),
            border: Color::rgb8(57, 60, 68),
            border_width: 1.0,
            radius: 6.0,
            gutter_foreground: Some(Color::rgb8(104, 109, 120)),
            content_padding_y: 8.0,
            gutter_padding_left: 8.0,
            gutter_padding_right: 10.0,
            line_numbers: true,
            syntax: SyntaxTheme {
                keyword: Some(Color::rgb8(198, 120, 221)),
                literal: Some(Color::rgb8(209, 154, 102)),
                string: Some(Color::rgb8(152, 195, 121)),
                comment: Some(Color::rgb8(92, 99, 112)),
                number: Some(Color::rgb8(209, 154, 102)),
                r#type: Some(Color::rgb8(229, 192, 123)),
                function: Some(Color::rgb8(97, 175, 239)),
                metadata: Some(Color::rgb8(86, 182, 194)),
            },
            ..Default::default()
        };
        let editor_style = EditorStyle {
            background: code_style.background,
            foreground: code_style.foreground,
            border: code_style.border,
            focus_border: code_style.border,
            border_width: code_style.border_width,
            radius: code_style.radius,
            font_size: code_style.font_size,
            line_height: code_style.line_height,
            word_wrap: code_style.word_wrap,
            syntax: code_style.syntax,
            presentation: TextInputGutter {
                line_numbers: code_style.line_numbers,
                minimum_line_number_digits: code_style.minimum_line_number_digits,
                gutter_background: code_style.gutter_background,
                gutter_foreground: code_style.gutter_foreground,
                gutter_active_foreground: Some(Color::rgb8(214, 217, 224)),
                gutter_border: code_style.gutter_border,
                active_line_background: Color::rgba8(255, 255, 255, 8),
                content_padding_y: code_style.content_padding_y,
                gutter_padding_left: code_style.gutter_padding_left,
                gutter_padding_right: code_style.gutter_padding_right,
                ..TextInputGutter::default()
            },
            ..EditorStyle::default()
        };
        Self {
            editor: Editor::with_text(AFTER)
                .with_language(rust)
                .with_style(editor_style),
            code: CodeBlock::with_text(&after)
                .with_language(rust)
                .with_style(code_style),
            diff: DiffView::from_texts("before.rs", &before, "after.rs", &after).with_style(
                DiffViewStyle {
                    layout: DiffLayout::Split,
                    language: Some(rust),
                    indicators: DiffIndicators::Bars,
                    syntax: code_style.syntax,
                    border_width: 1.0,
                    radius: 7.0,
                    header_height: 38.0,
                    header_padding: 12.0,
                    gutter_padding: 8.0,
                    theme: DiffTheme {
                        background: Color::rgb8(24, 25, 29),
                        foreground: Some(Color::rgb8(218, 221, 228)),
                        muted: Some(Color::rgb8(123, 128, 139)),
                        border: Color::rgb8(48, 51, 58),
                        header_background: Color::rgb8(20, 21, 24),
                        line_number: Some(Color::rgb8(113, 118, 128)),
                        hunk_background: Color::rgb8(45, 47, 52),
                        hunk_foreground: Some(Color::rgb8(168, 173, 184)),
                        added_background: Color::rgba8(0, 194, 129, 16),
                        added_foreground: Some(Color::rgb8(0, 194, 129)),
                        removed_background: Color::rgba8(255, 53, 63, 16),
                        removed_foreground: Some(Color::rgb8(255, 53, 63)),
                        inline_added_background: Color::rgba8(0, 194, 129, 55),
                        inline_removed_background: Color::rgba8(255, 53, 63, 55),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            layout: DiffLayout::Split,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editable_surface_shares_the_frame_and_document_spacing_with_read_only() {
        let demo = EditorDemo::new();
        let editor = demo.editor.style();
        let read_only = demo.code.style();
        assert_eq!(editor.border, read_only.border);
        assert_eq!(editor.background, read_only.background);
        assert_eq!(
            editor.presentation.content_padding_left,
            read_only.content_padding_left
        );
        assert_eq!(
            editor.presentation.content_padding_right,
            read_only.content_padding_right
        );
        assert_eq!(
            editor.presentation.content_padding_y,
            read_only.content_padding_y
        );
        assert_eq!(
            read_only.content_padding_left,
            CodeBlockStyle::default().content_padding_left
        );
    }
}

fn scrollable_diff_sources() -> (String, String) {
    let mut before = BEFORE.to_owned();
    let mut after = AFTER.to_owned();
    before.push_str("\nfn generated_rows() {\n");
    after.push_str("\nfn generated_rows() {\n");
    for index in 0..48 {
        before.push_str(&format!(
            "    let row_{index} = \"before value {index}: {}\";\n",
            "old".repeat(24)
        ));
        after.push_str(&format!(
            "    let row_{index} = \"after value {index}: {}\";\n",
            "new".repeat(28)
        ));
    }
    before.push_str("}\n");
    after.push_str("}\n");
    (before, after)
}

impl View for EditorDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let edit = cx.input_listener("demo-editor", |view, value, cx| {
            if view.editor.set_text(value).changed {
                view.code.set_text(value);
                cx.invalidate();
            }
        });
        let toggle = cx.listener("toggle-layout", |view, cx| {
            view.layout = match view.layout {
                DiffLayout::Split => DiffLayout::Unified,
                DiffLayout::Unified => DiffLayout::Split,
            };
            view.diff.set_style(view.diff.style().layout(view.layout));
            cx.invalidate();
        });
        div()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(15, 16, 19))
            .text_color(Color::rgb8(226, 228, 234))
            .child(
                div()
                    .h(52.0)
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .padding(0.0, 16.0, 0.0, 82.0)
                    .app_region_drag()
                    .border_bottom(1.0, Color::rgb8(45, 48, 55))
                    .child(
                        text("Editor + CodeBlock + Diff View")
                            .font_semibold()
                            .flex_1(),
                    )
                    .child(
                        button()
                            .id("toggle-layout")
                            .on_click(toggle)
                            .app_region_no_drag()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(Color::rgb8(42, 45, 52))
                            .hover(|style| style.bg(Color::rgb8(55, 59, 68)))
                            .child(match self.layout {
                                DiffLayout::Split => "Use unified diff",
                                DiffLayout::Unified => "Use split diff",
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .flex_row()
                    .gap_3()
                    .p_3()
                    .child(
                        div()
                            .w_fraction(0.44)
                            .h_full()
                            .min_w(0.0)
                            .min_h(0.0)
                            .flex_col()
                            .gap_3()
                            .child(
                                self.editor
                                    .element("demo-editor")
                                    .on_input(edit)
                                    .flex_1()
                                    .min_h(0.0),
                            )
                            .child(self.code.element("demo-code-block").h(190.0).flex_none()),
                    )
                    .child(self.diff.element("demo-diff").flex_1().min_w(0.0)),
            )
    }
}
