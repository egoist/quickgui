use super::{NativeListState, NativeNode, property};
use quickgui::{Element, Vector};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    revision: u64,
    index: Option<usize>,
    x: f32,
    y: f32,
}
pub(super) fn validate(raw: &str) -> bool {
    raw.len() <= 512
        && serde_json::from_str::<Request>(raw).is_ok_and(|r| r.x.is_finite() && r.y.is_finite())
}
fn request(node: &NativeNode) -> Option<Request> {
    node.string(property::SCROLL_REQUEST)
        .and_then(|raw| serde_json::from_str(raw).ok())
}
pub(super) fn apply(element: Element, node: &NativeNode) -> Element {
    let Some(r) = request(node) else {
        return element;
    };
    match r.index {
        Some(index) => element.scroll_to_child(index, Vector::new(r.x, r.y), r.revision),
        None => element.scroll_to(Vector::new(r.x, r.y), r.revision),
    }
}
pub(super) fn apply_list(state: &mut NativeListState, node: &NativeNode) {
    let Some(r) = request(node) else {
        state.scroll_revision = None;
        return;
    };
    if state.scroll_revision == Some(r.revision) || state.list.viewport_size().height <= 0.0 {
        return;
    }
    let origin = r.index.map_or(0.0, |index| {
        state
            .list
            .item_rect(index)
            .map_or(state.list.content_height(), |rect| {
                rect.y + state.list.scroll_offset()
            })
    });
    state.list.scroll_to_pixels(origin + r.y);
    state.scroll_revision = Some(r.revision);
}
