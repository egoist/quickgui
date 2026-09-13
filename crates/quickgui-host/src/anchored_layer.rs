//! Host declaration for a floating layer whose placement remains in the native layout engine.
use super::{NativeNode, property};
use quickgui::{AnchorPlacement, Element, ElementId, Point};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Layer {
    #[serde(default)]
    round_offset: bool,
    target: Option<u32>,
    position: Option<[f32; 2]>,
    placement: String,
    gap: f32,
    align_offset: f32,
    margin: f32,
    flip: bool,
    occlude: bool,
    priority: i16,
}

pub(super) fn validate(source: &str) -> bool {
    source.len() <= 2048
        && serde_json::from_str::<Layer>(source).is_ok_and(|layer| {
            layer.target.is_some() != layer.position.is_some()
                && placement(&layer.placement).is_some()
                && layer.gap.is_finite()
                && layer.align_offset.is_finite()
                && layer.margin.is_finite()
                && layer
                    .position
                    .is_none_or(|point| point.iter().all(|value| value.is_finite()))
        })
}

fn placement(value: &str) -> Option<AnchorPlacement> {
    use AnchorPlacement::*;
    Some(match value {
        "top-start" => TopStart,
        "top" => Top,
        "top-end" => TopEnd,
        "bottom-start" => BottomStart,
        "bottom" => Bottom,
        "bottom-end" => BottomEnd,
        "left-start" => LeftStart,
        "left" => Left,
        "left-end" => LeftEnd,
        "right-start" => RightStart,
        "right" => Right,
        "right-end" => RightEnd,
        _ => return None,
    })
}

pub(super) fn apply(element: Element, node: &NativeNode, scale: f32) -> Element {
    let Some(layer) = node
        .string(property::ANCHORED_LAYER)
        .and_then(|source| serde_json::from_str::<Layer>(source).ok())
    else {
        return element;
    };
    let Some(placement) = placement(&layer.placement) else {
        return element;
    };
    let element = if let Some([x, y]) = layer.position {
        element.anchor_at(Point::new(x, y), placement)
    } else if let Some(target) = layer.target {
        element.anchor_to(ElementId::new(u64::from(target)), placement)
    } else {
        return element;
    };
    let element = if layer.round_offset {
        element.anchor_offset_rounding(scale)
    } else {
        element
    };
    element
        .anchor_gap(layer.gap)
        .anchor_align_offset(layer.align_offset)
        .viewport_margin(layer.margin)
        .anchor_flip(layer.flip)
        .pointer_blocking(layer.occlude)
        .z_index(layer.priority)
}
