use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_COMPONENT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_COMPONENT_NODES: usize = 8192;
pub const MAX_COMPONENT_DEPTH: usize = 64;
pub const MAX_COMPONENT_ROWS: usize = 1024;
pub const MAX_COMPONENT_STYLES: usize = 128;

/// Linear-light, unpremultiplied RGBA.
pub type Paint = [f32; 4];

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub foreground: Option<Paint>,
    pub background: Option<Paint>,
    pub font_family: Option<String>,
    pub underline_color: Option<Paint>,
    pub weight: Option<u16>,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", content = "value", rename_all = "kebab-case")]
pub enum Style {
    Width(f32),
    Height(f32),
    MinWidth(f32),
    MinHeight(f32),
    MaxWidth(f32),
    MaxHeight(f32),
    WidthFraction(f32),
    HeightFraction(f32),
    FlexGrow(f32),
    FlexShrink(f32),
    FlexBasis(f32),
    FlexDirection(String),
    AlignItems(String),
    AlignSelf(String),
    JustifyContent(String),
    Gap(f32),
    Padding([f32; 4]),
    Background(Paint),
    Foreground(Paint),
    Border([f32; 4], Paint),
    Radius(f32),
    CornerRadii([f32; 4]),
    ScrollBackgroundCorners([f32; 4]),
    ScrollClip([f32; 4], [f32; 4]),
    FontFamily(String),
    FontSize(f32),
    LineHeight(f32),
    FontWeight(u16),
    Italic(bool),
    Wrap(bool),
    TextAlign(String),
    TextEllipsis,
    MonospaceWidth(f32),
    BasicTextShaping,
    FontThicken(bool),
    Overflow(String),
    OverflowX(String),
    OverflowY(String),
    Position(String),
    Top(f32),
    Left(f32),
    Right(f32),
    Bottom(f32),
    StickyLeft(f32),
    ZIndex(i16),
    UserSelect(bool),
    Cursor(String),
    Role(String),
    Label(String),
    RowCount(usize),
    RowIndex(usize),
    Hidden,
    GridColumns(u16),
    HorizontalScrollbarInset(f32),
    HideVerticalScrollbar(bool),
    Focusable(bool),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputOptions {
    pub read_only: bool,
    pub multiline: bool,
    pub tab_size: Option<u8>,
    pub insert_spaces: bool,
    pub auto_indent: bool,
    pub gutter: Option<Gutter>,
    #[serde(default)]
    pub text_checking: TextChecking,
}

/// Per-input overrides. None inherits the application's text-checking policy.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TextChecking {
    pub spellcheck: Option<bool>,
    pub grammar_check: Option<bool>,
    pub autocorrect: Option<bool>,
    pub smart_quotes: Option<bool>,
    pub smart_dashes: Option<bool>,
    pub text_replacement: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gutter {
    pub line_numbers: bool,
    pub minimum_digits: u8,
    pub background: Paint,
    pub foreground: Option<Paint>,
    pub active_foreground: Option<Paint>,
    pub border: Paint,
    pub active_line_background: Paint,
    pub content_padding: [f32; 4],
    pub padding_left: f32,
    pub padding_right: f32,
}

/// List identity belongs to a component instance; mirrored columns share its metric state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct List {
    pub key: u64,
    pub renderer: u64,
    pub count: usize,
    pub estimate: f32,
    pub overscan: usize,
    pub generation: u64,
    pub measurement_generation: u64,
    pub mirror: bool,
    pub row_styles: Vec<Style>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Primitive {
    #[default]
    View,
    Text,
    Input,
    Button,
    Svg,
    Canvas,
    Component,
}

/// A declarative node. Keys are local to this instance and are namespaced by the host.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub key: u64,
    pub kind: Primitive,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spans: Vec<Span>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<Style>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub focus_styles: Vec<Style>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Node>,
    pub input: Option<InputOptions>,
    pub list: Option<List>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<String>,
    pub component: Option<ComponentRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rectangles: Vec<PaintRect>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaintRect {
    pub bounds: [f32; 4],
    pub color: Paint,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentRef {
    pub package: String,
    pub name: String,
    pub props: Value,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RenderRequest {
    pub renderer: Option<u64>,
    pub start: usize,
    pub end: usize,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    pub focused: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Frame {
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub events: Vec<OutputEvent>,
    pub repaint_after_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputEvent {
    pub target: u64,
    pub kind: String,
    pub value: Value,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventResult {
    pub changed: bool,
    pub prevent_default: bool,
    pub stop_propagation: bool,
    pub clipboard: Option<String>,
    pub read_clipboard: bool,
    pub events: Vec<OutputEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputEvent {
    pub kind: String,
    pub value: Value,
}

impl Frame {
    /// Validate every declaration before the host mutates a retained subtree.
    pub fn validate(&self) -> Result<(), &'static str> {
        fn node(value: &Node, depth: usize, count: &mut usize) -> Result<(), &'static str> {
            *count += 1;
            if depth > MAX_COMPONENT_DEPTH
                || *count > MAX_COMPONENT_NODES
                || value.styles.len() > MAX_COMPONENT_STYLES
                || value.spans.len() > 4096
                || value.events.len() > 16
                || value.text.len() > MAX_COMPONENT_BYTES
            {
                return Err("component declaration exceeds its limits");
            }
            for span in &value.spans {
                if span.start >= span.end
                    || span.end > value.text.len()
                    || !value.text.is_char_boundary(span.start)
                    || !value.text.is_char_boundary(span.end)
                {
                    return Err("invalid component text span");
                }
            }
            if let Some(list) = &value.list
                && (list.count > 1_000_000
                    || !list.estimate.is_finite()
                    || list.estimate <= 0.0
                    || list.overscan > 128)
            {
                return Err("invalid component list");
            }
            if value.kind == Primitive::Component && value.component.is_none() {
                return Err("component reference is missing");
            }
            for child in &value.children {
                node(child, depth + 1, count)?;
            }
            Ok(())
        }
        let mut count = 0;
        for root in &self.nodes {
            node(root, 0, &mut count)?;
        }
        Ok(())
    }
}
