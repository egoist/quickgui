use super::{NativeNode, property};
use quickgui::{Color, Element, InputPresentation, Insets};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Presentation {
    insets: [f32; 4],
    caret_width: f32,
    caret_height_em: f32,
    caret_color: u32,
    placeholder_color: u32,
}
pub(super) fn validate(source: &str) -> bool {
    source.len() <= 1024
        && serde_json::from_str::<Presentation>(source).is_ok_and(|p| {
            p.insets
                .into_iter()
                .chain([p.caret_width, p.caret_height_em])
                .all(|v| v.is_finite() && v >= 0.0 && v <= 4096.0)
        })
}
fn color(value: u32) -> Color {
    Color::rgba8(
        value as u8,
        (value >> 8) as u8,
        (value >> 16) as u8,
        (value >> 24) as u8,
    )
}
pub(super) fn apply(element: Element, node: &NativeNode) -> Element {
    let Some(p) = node
        .string(property::INPUT_PRESENTATION)
        .and_then(|raw| serde_json::from_str::<Presentation>(raw).ok())
    else {
        return element;
    };
    element.input_presentation(InputPresentation {
        content_insets: Some(Insets {
            top: p.insets[0],
            right: p.insets[1],
            bottom: p.insets[2],
            left: p.insets[3],
        }),
        caret_width: Some(p.caret_width),
        caret_height_em: Some(p.caret_height_em),
        caret_color: Some(color(p.caret_color)),
        placeholder_color: Some(color(p.placeholder_color)),
    })
}
