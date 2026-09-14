use crate::*;

fn number(value: &Value, key: &str, default: f32) -> f32 {
    value
        .get(key)
        .and_then(Value::as_f64)
        .map(|v| v as f32)
        .filter(|v| v.is_finite())
        .unwrap_or(default)
}
fn boolean(value: &Value, key: &str, default: bool) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(default)
}
fn paint(value: &Value, key: &str, default: Color) -> Color {
    value.get(key).and_then(parse_color).unwrap_or(default)
}
fn parse_color(value: &Value) -> Option<Color> {
    if let Some(value) = value.as_u64() {
        return Some(Color::rgba8(
            value as u8,
            (value >> 8) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
        ));
    }
    let text = value.as_str()?;
    if text == "transparent" {
        return Some(Color::TRANSPARENT);
    }
    let hex = text.strip_prefix('#')?;
    let hex = match hex.len() {
        3 | 4 => hex.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 | 8 => hex.to_owned(),
        _ => return None,
    };
    let packed = u32::from_str_radix(&hex, 16).ok()?;
    Some(if hex.len() == 6 {
        Color::rgb8((packed >> 16) as u8, (packed >> 8) as u8, packed as u8)
    } else {
        Color::rgba8(
            (packed >> 24) as u8,
            (packed >> 16) as u8,
            (packed >> 8) as u8,
            packed as u8,
        )
    })
}
fn optional_paint(value: &Value, key: &str, default: Option<Color>) -> Option<Color> {
    value.get(key).and_then(parse_color).or(default)
}
fn syntax(value: &Value) -> SyntaxTheme {
    let v = &value["syntaxTheme"];
    let d = SyntaxTheme::default();
    SyntaxTheme {
        keyword: optional_paint(v, "keyword", d.keyword),
        literal: optional_paint(v, "literal", d.literal),
        string: optional_paint(v, "string", d.string),
        comment: optional_paint(v, "comment", d.comment),
        number: optional_paint(v, "number", d.number),
        r#type: optional_paint(v, "type", d.r#type),
        function: optional_paint(v, "function", d.function),
        metadata: optional_paint(v, "metadata", d.metadata),
    }
}
pub fn editor_style(props: &Value) -> editor::EditorStyle {
    let d = editor::EditorStyle::default();
    let s = &props["style"];
    editor::EditorStyle {
        background: paint(s, "backgroundColor", d.background),
        foreground: optional_paint(s, "color", d.foreground),
        border: paint(s, "borderColor", d.border),
        focus_border: paint(&s["focus"], "borderColor", d.focus_border),
        border_width: number(s, "borderWidth", 0.0),
        radius: number(s, "borderRadius", 0.0),
        font_size: number(s, "fontSize", d.font_size),
        line_height: number(s, "lineHeight", d.line_height),
        word_wrap: boolean(s, "wrap", false),
        syntax: syntax(props),
        presentation: TextInputGutter {
            line_numbers: boolean(props, "lineNumbers", true),
            gutter_background: paint(props, "gutterBackground", d.presentation.gutter_background),
            gutter_foreground: optional_paint(
                props,
                "gutterColor",
                d.presentation.gutter_foreground,
            ),
            gutter_active_foreground: optional_paint(
                props,
                "activeLineNumberColor",
                d.presentation.gutter_active_foreground,
            ),
            active_line_background: paint(
                props,
                "activeLineBackground",
                d.presentation.active_line_background,
            ),
            content_padding_left: number(&props["contentPadding"], "left", 0.0),
            content_padding_right: number(&props["contentPadding"], "right", 0.0),
            content_padding_y: number(&props["contentPadding"], "vertical", 0.0),
            gutter_padding_left: number(&props["gutterPadding"], "left", 0.0),
            gutter_padding_right: number(&props["gutterPadding"], "right", 0.0),
            ..d.presentation
        },
    }
}
pub fn behavior(props: &Value) -> TextInputIndentation {
    TextInputIndentation::default()
        .tab_size(number(props, "tabSize", 4.0) as usize)
        .insert_spaces(boolean(props, "insertSpaces", true))
        .auto_indent(boolean(props, "autoIndent", true))
        .read_only(boolean(props, "readOnly", false))
}
pub fn code_style(props: &Value) -> code_block::CodeBlockStyle {
    let d = code_block::CodeBlockStyle::default();
    let s = &props["style"];
    code_block::CodeBlockStyle {
        background: paint(s, "backgroundColor", d.background),
        foreground: optional_paint(s, "color", d.foreground),
        border: paint(s, "borderColor", d.border),
        border_width: number(s, "borderWidth", 0.0),
        radius: number(s, "borderRadius", 0.0),
        content_padding_left: number(&props["contentPadding"], "left", 0.0),
        content_padding_right: number(&props["contentPadding"], "right", 0.0),
        content_padding_y: number(&props["contentPadding"], "vertical", 0.0),
        gutter_padding_left: number(&props["gutterPadding"], "left", 0.0),
        gutter_padding_right: number(&props["gutterPadding"], "right", 0.0),
        gutter_background: paint(props, "gutterBackground", d.gutter_background),
        gutter_foreground: optional_paint(props, "gutterColor", d.gutter_foreground),
        font_size: number(s, "fontSize", d.font_size),
        line_height: number(s, "lineHeight", d.line_height),
        word_wrap: boolean(props, "wrap", d.word_wrap),
        line_numbers: boolean(props, "lineNumbers", d.line_numbers),
        syntax: syntax(props),
        ..d
    }
}
pub fn diff_style(props: &Value) -> diff_view::DiffViewStyle {
    let d = diff_view::DiffViewStyle::default();
    let o = &props["options"];
    let s = &props["style"];
    let t = &props["theme"];
    let mut theme = d.theme;
    theme.background = paint(s, "backgroundColor", theme.background);
    theme.foreground = optional_paint(s, "color", theme.foreground);
    theme.border = paint(s, "borderColor", theme.border);
    macro_rules! colors {($($field:ident=>$key:literal),*)=>{$(theme.$field=paint(t,$key,theme.$field);)*};}
    colors!(header_background=>"headerBackground",hunk_background=>"hunkBackground",added_background=>"addedBackground",removed_background=>"removedBackground",added_gutter_background=>"addedGutterBackground",removed_gutter_background=>"removedGutterBackground",inline_added_background=>"inlineAddedBackground",inline_removed_background=>"inlineRemovedBackground");
    theme.line_number = optional_paint(t, "lineNumberColor", theme.line_number);
    theme.hunk_foreground = optional_paint(t, "hunkColor", theme.hunk_foreground);
    theme.added_foreground = optional_paint(t, "addedColor", theme.added_foreground);
    theme.removed_foreground = optional_paint(t, "removedColor", theme.removed_foreground);
    theme.muted = optional_paint(t, "mutedColor", theme.muted);
    diff_view::DiffViewStyle {
        layout: if o["layout"] == "unified" {
            diff_view::DiffLayout::Unified
        } else {
            diff_view::DiffLayout::Split
        },
        indicators: match o["indicators"].as_str() {
            Some("classic") => diff_view::DiffIndicators::Classic,
            Some("none") => diff_view::DiffIndicators::None,
            Some("bars") => diff_view::DiffIndicators::Bars,
            _ => diff_view::DiffIndicators::Classic,
        },
        backgrounds: boolean(o, "backgrounds", true),
        line_numbers: boolean(o, "lineNumbers", true),
        word_wrap: boolean(o, "wrap", false),
        file_header: boolean(o, "fileHeader", true),
        language: o["language"].as_str().and_then(SyntaxLanguage::from_name),
        font_size: number(s, "fontSize", d.font_size),
        line_height: number(s, "lineHeight", d.line_height),
        syntax: syntax(props),
        theme,
        border_width: number(s, "borderWidth", 0.0),
        radius: number(s, "borderRadius", 0.0),
        header_height: number(&props["presentation"], "headerHeight", d.header_height),
        header_padding: number(&props["presentation"], "headerPadding", 0.0),
        gutter_padding: number(&props["presentation"], "gutterPadding", 0.0),
    }
}
