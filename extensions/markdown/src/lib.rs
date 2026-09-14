//! Markdown parser and component construction, independent of the host renderer.
pub use quickgui_extension_sdk::builder::*;
use quickgui_extension_sdk::{
    self as sdk, Component, ComponentFactory, ComponentRuntime, WakeHandle, schema as w,
};
use serde_json::Value;
use std::ptr;
pub const MAX_TEXT_HIGHLIGHTS: usize = 4096;
#[path = "../../../src/markdown/mod.rs"]
mod markdown;
pub struct Factory;
struct Surface {
    markdown: markdown::Markdown,
    props: Value,
    renderers: Renderers,
}
impl ComponentFactory for Factory {
    fn create(name: &str, props: Value, _: WakeHandle) -> Result<Box<dyn Component>, String> {
        if name != "markdown" {
            return Err(format!("unknown markdown component {name}"));
        }
        let mut surface = Surface {
            markdown: markdown::Markdown::new(),
            props: Value::Null,
            renderers: Renderers::default(),
        };
        surface.update(props)?;
        Ok(Box::new(surface))
    }
}
impl Component for Surface {
    fn update(&mut self, props: Value) -> Result<bool, String> {
        if self.props == props {
            return Ok(false);
        }
        self.markdown
            .set_streaming(props["streaming"].as_bool().unwrap_or(false));
        self.markdown
            .set_text(props["value"].as_str().unwrap_or(""));
        self.markdown.set_style(presentation(&props));
        self.props = props;
        Ok(true)
    }
    fn render(&mut self, request: w::RenderRequest) -> Result<w::Frame, String> {
        if request.renderer.is_some() {
            return self.renderers.rows(&request);
        }
        let reference = self
            .props
            .get("codeBlockComponent")
            .filter(|v| !v.is_null())
            .map(|v| {
                serde_json::from_value::<w::ComponentRef>(v.clone()).map_err(|e| e.to_string())
            })
            .transpose()?;
        let maximum = self.props["codeBlockMaxHeight"]
            .as_f64()
            .unwrap_or(320.0)
            .clamp(40.0, 4096.0) as f32;
        Ok(self.renderers.root(|| {
            if let Some(reference) = reference {
                self.markdown
                    .element_with_code_blocks(ElementId::new(0), |request| {
                        let mut reference = reference.clone();
                        if !reference.props.is_object() {
                            reference.props = serde_json::json!({});
                        }
                        if !reference.props["style"].is_object() {
                            reference.props["style"] = serde_json::json!({});
                        }
                        for (source, target) in [
                            ("codeBackground", "backgroundColor"),
                            ("codeColor", "color"),
                            ("codeBorderColor", "borderColor"),
                            ("codeFontSize", "fontSize"),
                        ] {
                            if !self.props["theme"][source].is_null()
                                && reference.props["style"][target].is_null()
                            {
                                reference.props["style"][target] =
                                    self.props["theme"][source].clone();
                            }
                        }
                        reference.props["value"] = Value::String(request.code.into());
                        reference.props["language"] = request
                            .language
                            .map(|v| Value::String(v.into()))
                            .unwrap_or(Value::Null);
                        let height =
                            (request.code.lines().count().max(1) as f32 * 20.0 + 18.0).min(maximum);
                        let mut element = div().id(request.element_id).w_full().h(height);
                        element.node.kind = w::Primitive::Component;
                        element.node.component = Some(reference);
                        element
                    })
            } else {
                self.markdown.element(ElementId::new(0))
            }
        }))
    }
}
fn parse_color(value: &Value) -> Option<Color> {
    if let Some(n) = value.as_u64() {
        return Some(Color::rgba8(
            n as u8,
            (n >> 8) as u8,
            (n >> 16) as u8,
            (n >> 24) as u8,
        ));
    }
    let hex = value.as_str()?.strip_prefix('#')?;
    let hex = if hex.len() == 3 || hex.len() == 4 {
        hex.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        hex.to_owned()
    };
    let n = u32::from_str_radix(&hex, 16).ok()?;
    match hex.len() {
        6 => Some(Color::rgb8((n >> 16) as u8, (n >> 8) as u8, n as u8)),
        8 => Some(Color::rgba8(
            (n >> 24) as u8,
            (n >> 16) as u8,
            (n >> 8) as u8,
            n as u8,
        )),
        _ => None,
    }
}
fn presentation(props: &Value) -> markdown::MarkdownStyle {
    let mut style = markdown::MarkdownStyle::default();
    let theme = &props["theme"];
    let base = &props["style"];
    style.text_color = parse_color(&base["color"]);
    style.font_size = base["fontSize"].as_f64().unwrap_or(style.font_size as f64) as f32;
    style.line_height = base["lineHeight"]
        .as_f64()
        .unwrap_or(style.line_height as f64) as f32;
    style.link_color = parse_color(&theme["linkColor"]);
    style.muted_color = parse_color(&theme["mutedColor"]);
    style.code_text_color = parse_color(&theme["codeColor"]);
    style.code_background = parse_color(&theme["codeBackground"]).or(style.code_background);
    style.border_color = parse_color(&theme["codeBorderColor"]).or(style.border_color);
    style.block_gap = theme["blockGap"].as_f64().unwrap_or(style.block_gap as f64) as f32;
    style.code_font_size = theme["codeFontSize"]
        .as_f64()
        .unwrap_or(style.code_font_size as f64) as f32;
    style
}
static API: sdk::abi::ComponentApi = ComponentRuntime::<Factory>::API;
static DESCRIPTOR: sdk::abi::Extension = sdk::abi::Extension {
    abi_version: sdk::abi::ABI_VERSION,
    descriptor_size: size_of::<sdk::abi::Extension>() as u32,
    kind: sdk::abi::COMPONENT_EXTENSION,
    api_size: size_of::<sdk::abi::ComponentApi>() as u32,
    name: sdk::abi::Bytes::new(b"markdown"),
    version: sdk::abi::Bytes::new(env!("CARGO_PKG_VERSION").as_bytes()),
    api: ptr::addr_of!(API).cast(),
};
#[unsafe(no_mangle)]
pub extern "C" fn quickgui_extension_v1() -> *const sdk::abi::Extension {
    ptr::addr_of!(DESCRIPTOR)
}
