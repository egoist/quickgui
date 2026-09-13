use super::*;

/// Paint an element's raster background above its fill and behind its children.
///
/// Tiles are generated only for the visible intersection of the element and its clip, and the
/// total is capped by [`MAX_BACKGROUND_IMAGE_TILES`]. Exceeding the cap deliberately paints one
/// anchored tile instead of emitting an unbounded number of image instances.
pub(super) fn push_background_image(
    scene: &mut Scene,
    layer: PaintLayerKey,
    bounds: Rect,
    corners: Corners,
    clip: Rect,
    color_matrix: ColorMatrix,
    background: &BackgroundImage,
) {
    if bounds.is_empty() {
        return;
    }
    let Some(visible) = clip.intersection(bounds) else {
        return;
    };
    let Some(tile) = background.tile_size(bounds) else {
        return;
    };
    let anchor_x = bounds.x + (bounds.width - tile.width) * background.position.x;
    let anchor_y = bounds.y + (bounds.height - tile.height) * background.position.y;
    let radius = corners.maximum();

    let axis_range = |repeats: bool, anchor: f32, extent: f32, start: f32, end: f32| {
        if !repeats {
            return (0_i64, 0_i64);
        }
        let first = ((start - anchor) / extent).floor();
        let last = ((end - anchor) / extent).ceil() - 1.0;
        if !first.is_finite() || !last.is_finite() {
            return (0, 0);
        }
        let first = first.clamp(-1.0e6, 1.0e6) as i64;
        let last = last.clamp(-1.0e6, 1.0e6) as i64;
        (first, last.max(first))
    };
    let (first_x, last_x) = axis_range(
        background.repeat.repeats_x(),
        anchor_x,
        tile.width,
        visible.x,
        visible.right(),
    );
    let (first_y, last_y) = axis_range(
        background.repeat.repeats_y(),
        anchor_y,
        tile.height,
        visible.y,
        visible.bottom(),
    );
    let columns = (last_x - first_x + 1).max(1);
    let rows = (last_y - first_y + 1).max(1);
    let (first_x, last_x, first_y, last_y) =
        if columns.saturating_mul(rows) > MAX_BACKGROUND_IMAGE_TILES as i64 {
            (0, 0, 0, 0)
        } else {
            (first_x, last_x, first_y, last_y)
        };

    for row in first_y..=last_y {
        for column in first_x..=last_x {
            let destination = Rect::new(
                anchor_x + column as f32 * tile.width,
                anchor_y + row as f32 * tile.height,
                tile.width,
                tile.height,
            );
            scene.push_image_in(
                layer,
                ImagePrimitive::new(background.image.clone(), destination)
                    .mask(bounds)
                    .radius(radius)
                    .color_matrix(color_matrix)
                    .clip(visible),
            );
        }
    }
}

/// The transform an element declares for this frame, and the fraction of its box it acts around.
///
/// Interaction, focus, and validation states override the element's own transform with the same
/// precedence the paint styles use. This is resolved before the element's box is recorded so a
/// whole subtree translation can be folded into that box instead of allocating a group texture.
///
/// `styled_focus` is the element whose focus styles paint this frame — see
/// `UiTree::styled_focus` — which is the focused element only while focus is visible.
/// `focus_within` is whether the element or a descendant owns focus, and `group_style` is the
/// overlay of the element's group styles whose groups are in their state — see
/// `GroupScope::resolve`.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolved_transform(
    element: &Element,
    hovered: &HashSet<ElementId>,
    pressed: Option<ElementId>,
    dragging: Option<ElementId>,
    drag_over: Option<ElementId>,
    styled_focus: Option<ElementId>,
    focus_within: bool,
    group_style: Option<&ElementStateStyle>,
) -> (Transform2D, Point) {
    let empty = ElementStateStyle::default();
    let interaction = if drag_over == Some(element.runtime_id) {
        &element.drag_over
    } else if dragging == Some(element.runtime_id) {
        &element.dragging
    } else if pressed == Some(element.runtime_id) {
        &element.active
    } else if hovered.contains(&element.runtime_id) {
        &element.hover
    } else {
        &empty
    };
    let focus = if styled_focus == Some(element.runtime_id) {
        &element.focus
    } else {
        &empty
    };
    let disabled = if element.accessibility.disabled {
        &element.disabled_style
    } else {
        &empty
    };
    let invalid = if element.accessibility.invalid {
        &element.invalid_style
    } else {
        &empty
    };
    let selected = if element.accessibility.selected {
        &element.selected_style
    } else {
        &empty
    };
    let within = if focus_within {
        &element.focus_within
    } else {
        &empty
    };
    let group = group_style.unwrap_or(&empty);
    let transform = disabled
        .transform
        .or(selected.transform)
        .or(interaction.transform)
        .or(invalid.transform)
        .or(focus.transform)
        .or(within.transform)
        .or(group.transform)
        .unwrap_or(element.visual.transform);
    let origin = disabled
        .transform_origin
        .or(selected.transform_origin)
        .or(interaction.transform_origin)
        .or(invalid.transform_origin)
        .or(focus.transform_origin)
        .or(within.transform_origin)
        .or(group.transform_origin)
        .unwrap_or(element.visual.transform_origin);
    (transform, origin)
}

/// One enclosing group while the tree is walked, linked to the group outside it.
///
/// A member resolves its group styles against this chain: the nearest scope when it names no
/// group, or the nearest scope carrying its name. The chain lives on the recursion's own stack
/// frames, so groups allocate nothing per element.
#[derive(Clone, Copy)]
pub(super) struct GroupScope<'a> {
    parent: Option<&'a GroupScope<'a>>,
    name: Option<&'a str>,
    hovered: bool,
    /// Whether the held press, if any, is on this group or inside it.
    pressed: bool,
}

impl<'a> GroupScope<'a> {
    /// The scope an element's children resolve against when the element is a group.
    pub(super) fn for_children(
        element: &'a Element,
        hovered: &HashSet<ElementId>,
        pressed_path: &[ElementId],
        parent: Option<&'a GroupScope<'a>>,
    ) -> Option<Self> {
        element.group.then(|| GroupScope {
            parent,
            name: element.group_name.as_deref(),
            hovered: hovered.contains(&element.runtime_id),
            pressed: pressed_path.contains(&element.runtime_id),
        })
    }

    /// Whether the group an entry follows — the nearest, or the nearest carrying its name — is in
    /// the entry's state this frame.
    fn in_state(scope: Option<&GroupScope<'_>>, target: Option<&str>, state: GroupState) -> bool {
        let mut current = scope;
        while let Some(scope) = current {
            if target.is_none_or(|name| scope.name == Some(name)) {
                return match state {
                    GroupState::Hover => scope.hovered,
                    GroupState::Active => scope.pressed,
                };
            }
            current = scope.parent;
        }
        false
    }

    /// Every group style whose group is in its state, laid over one another in declaration order,
    /// or `None` when none applies this frame.
    pub(super) fn resolve(
        scope: Option<&GroupScope<'_>>,
        element: &Element,
    ) -> Option<ElementStateStyle> {
        let mut merged: Option<ElementStateStyle> = None;
        for entry in &element.group_styles {
            if Self::in_state(scope, entry.target.as_deref(), entry.state) {
                merged
                    .get_or_insert_with(ElementStateStyle::default)
                    .overlay(&entry.style);
            }
        }
        merged
    }
}

/// The window-space point a transform origin fraction names inside an element's border box.
pub(super) fn transform_origin_point(bounds: Rect, origin: Point) -> Point {
    Point::new(
        bounds.x + bounds.width * origin.x,
        bounds.y + bounds.height * origin.y,
    )
}

/// The layer effects an element declares, in window coordinates.
///
/// `transform` is the element's own transform expressed around its window-space origin; ancestors
/// contribute their own when their groups composite.
pub(super) fn resolved_layer_effects(
    element: &Element,
    bounds: Rect,
    corners: Corners,
    transform: Transform2D,
) -> LayerEffects {
    let filters = element.visual.filters;
    let mut effects = LayerEffects {
        transform,
        blur: filters.blur(),
        drop_shadow: filters.drop_shadow(),
        color_matrix: ColorMatrix::IDENTITY,
        backdrop_blur: element.visual.backdrop.blur(),
        backdrop_matrix: element.visual.backdrop.color_matrix(),
        backdrop_corners: corners,
        blend: element.visual.blend,
    };
    // A colour chain stays a cheap per-primitive matrix until something else already forces an
    // offscreen group; then it applies to the whole subtree, as CSS specifies.
    if effects.needs_group() {
        effects.color_matrix = filters.color_matrix();
    }
    let _ = bounds;
    effects
}

fn has_visible_border(widths: Insets) -> bool {
    widths.top > 0.0 || widths.right > 0.0 || widths.bottom > 0.0 || widths.left > 0.0
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_selectable_text(
    element: &Element,
    document_index: Option<usize>,
    content: &Arc<str>,
    style: &TextStyle,
    highlights: Option<&Arc<[TextHighlight]>>,
    bounds: Rect,
    parent_clip: Rect,
    layer: PaintLayerKey,
    order: PaintOrder,
    scale_factor: f32,
    selection: Option<StaticTextSelection>,
    indices: &HashMap<ElementId, usize>,
    regions: &mut Vec<SelectableTextRegion>,
    scene: &mut Scene,
    renderer: &mut impl TextLayoutEngine,
    rebuild_geometry: bool,
) {
    let Some(document_index) = document_index else {
        return;
    };
    let Some(clip) = parent_clip.intersection(bounds) else {
        return;
    };
    if rebuild_geometry {
        let mut links = Vec::new();
        for highlight in highlights.map(|h| h.as_ref()).unwrap_or_default() {
            let Some(url) = &highlight.style.link else {
                continue;
            };
            let visible_y =
                (clip.y - bounds.y).max(0.0)..(clip.bottom() - bounds.y).min(bounds.height);
            for rect in renderer
                .text_selection_rects_with_highlights(
                    TextId::new(element.runtime_id.value()),
                    content,
                    style,
                    highlights,
                    bounds.width,
                    scale_factor,
                    visible_y,
                    highlight.range.start,
                    highlight.range.end,
                )
                .into_iter()
                .take(4096usize.saturating_sub(links.len()))
            {
                links.push((rect, url.clone()));
            }
            if links.len() == 4096 {
                break;
            }
        }
        regions.push(SelectableTextRegion {
            links,
            document_index,
            bounds,
            clip,
            style: style.clone(),
            highlights: highlights.cloned(),
            order,
        });
    }
    let Some(range) =
        static_selection_range_for_entry(selection, indices, document_index, content.len())
    else {
        return;
    };
    let visible_y = (clip.y - bounds.y).max(0.0)..(clip.bottom() - bounds.y).min(bounds.height);
    for rect in renderer.text_selection_rects_with_highlights(
        TextId::new(element.runtime_id.value()),
        content,
        style,
        highlights,
        bounds.width,
        scale_factor,
        visible_y,
        range.start,
        range.end,
    ) {
        scene.push_quad_in(
            layer,
            Quad::new(
                Rect::new(
                    bounds.x + rect.x,
                    bounds.y + rect.y,
                    rect.width,
                    rect.height,
                ),
                static_selection_color(),
            )
            .clip(clip),
        );
    }
}

fn element_paint_layer(element: &Element, parent: PaintLayerKey) -> PaintLayerKey {
    let plane = element.plane.unwrap_or(parent.plane);
    // A pinned header and its children must cover later in-flow siblings. Keep
    // explicitly declared stacking orders, and use this same layer for hit testing.
    let own_z_index = element
        .z_index
        .unwrap_or(if element.sticky.is_some() { 1 } else { 0 });
    PaintLayerKey {
        plane,
        z_index: if plane == parent.plane {
            parent.z_index.saturating_add(own_z_index)
        } else {
            own_z_index
        },
        group: parent.group,
    }
}

#[cfg(feature = "inspector")]
#[allow(clippy::too_many_arguments)]
pub(super) fn collect_inspector_nodes(
    element: &Element,
    parent: Option<ElementId>,
    depth: usize,
    parent_layer: PaintLayerKey,
    parent_clip: Rect,
    viewport: Rect,
    element_bounds: &HashMap<ElementId, Rect>,
    hit_lookup: &HashMap<ElementId, HitRegion>,
    focused: Option<ElementId>,
    source_order: &mut usize,
    snapshot: &mut crate::inspector::InspectorSnapshot,
) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    let Some(bounds) = element_bounds.get(&element.runtime_id).copied() else {
        return;
    };
    if snapshot.nodes.len() == crate::inspector::MAX_INSPECTOR_NODES {
        snapshot.nodes_truncated = true;
        return;
    }

    let layer = element_paint_layer(element, parent_layer);
    let source = *source_order;
    *source_order = source.saturating_add(1);
    let clip = if element.portal {
        viewport
    } else {
        parent_clip
    };
    let accessibility = crate::inspector::inspector_accessibility(element);
    let on_focus_path = snapshot.focus_path.contains(&element.runtime_id);
    snapshot.nodes.push(crate::inspector::InspectorNode {
        id: element.runtime_id,
        parent,
        depth,
        explicit_id: element.explicit_id.is_some(),
        kind: crate::inspector::InspectorElementKind::from_element(&element.kind),
        bounds,
        clip,
        plane: layer.plane,
        z_index: layer.z_index,
        source_order: source,
        portal: element.portal,
        focused: focused == Some(element.runtime_id),
        on_focus_path,
        hit_region: hit_lookup
            .get(&element.runtime_id)
            .copied()
            .map(crate::inspector::InspectorHitRegion::from_region),
        accessibility,
    });

    let Some(child_clip) = crate::inspector::child_clip(element, clip, bounds) else {
        return;
    };
    for child in &element.children {
        collect_inspector_nodes(
            child,
            Some(element.runtime_id),
            depth.saturating_add(1),
            layer,
            child_clip,
            viewport,
            element_bounds,
            hit_lookup,
            focused,
            source_order,
            snapshot,
        );
        if snapshot.nodes_truncated {
            return;
        }
    }
}

pub(super) fn element_hit_region(
    element: &Element,
    bounds: Rect,
    clip: Rect,
    order: PaintOrder,
    selectable_text: bool,
    transform: Option<Transform2D>,
) -> Option<HitRegion> {
    if !(element.clickable
        || element.pointer_listener
        || (element.mouse_listeners.is_some() && !element.accessibility.disabled)
        || element.scroll_wheel_listener
        || element.touch_listener
        || element.context_menu_listener
        || element.mouse_pressure_listener
        || element.pinch_listener
        || element.rotation_listener
        || element.smart_magnify_listener
        || element.tooltip.is_some()
        || element.drag_source
        || element.drop_target
        || element.cursor_style.is_some()
        || selectable_text
        || element.focusable
        || element.blocks_pointer
        || element.app_region.is_some()
        || element.group
        || element.has_stateful_paint()
        || element.has_stateful_cursor())
    {
        return None;
    }

    Some(HitRegion {
        transform,
        id: element.runtime_id,
        bounds: expand_hit_bounds(bounds, element.hit_slop),
        clip,
        clickable: element.clickable && !element.accessibility.disabled,
        pointer_listener: element.pointer_listener && !element.accessibility.disabled,
        drag_source: element.drag_source && !element.accessibility.disabled,
        drop_target: element.drop_target && !element.accessibility.disabled,
        focusable: element.focusable && element.focus_on_pointer && !element.accessibility.disabled,
        cursor_style: effective_cursor_style(element, selectable_text),
        cursor_states: CursorStateStyles {
            hover: element.hover.cursor_style,
            active: element.active.cursor_style,
            focus: element.focus.cursor_style,
            invalid: element
                .accessibility
                .invalid
                .then_some(element.invalid_style.cursor_style)
                .flatten(),
            selected: element
                .accessibility
                .selected
                .then_some(element.selected_style.cursor_style)
                .flatten(),
            dragging: element.dragging.cursor_style,
            drag_over: element.drag_over.cursor_style,
        },
        // A group tracks hover for its descendants' group styles even when it paints nothing
        // stateful itself.
        stateful: element.has_stateful_paint() || element.group,
        blocks_pointer: element.blocks_pointer,
        app_region: element.app_region,
        order,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn collect_layout_hit_regions(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
    natural_bounds: &HashMap<ElementId, Rect>,
    style_transitions: &HashMap<ElementId, StyleTransitionPlayback>,
    resolved_bounds: &mut HashMap<ElementId, Rect>,
    scroll_offsets: &mut HashMap<ElementId, Vector>,
    selectable_text_indices: &HashMap<ElementId, usize>,
    hit_regions: &mut Vec<HitRegion>,
    hovered: &HashSet<ElementId>,
    pressed: Option<ElementId>,
    dragging: Option<ElementId>,
    drag_over: Option<ElementId>,
    styled_focus: Option<ElementId>,
    pressed_path: &[ElementId],
    focused_path: &[ElementId],
    groups: Option<&GroupScope<'_>>,
    parent_origin: LayoutFrame,
    parent_clip: Rect,
    viewport: Rect,
    parent_layer: PaintLayerKey,
    source_order: &mut usize,
) -> Result<(), UiError> {
    if element.is_display_none() || element.is_visibility_hidden() {
        return Ok(());
    }
    let node = element
        .taffy_node
        .expect("layout nodes are assigned before hit testing");
    let layout = taffy.layout(node)?;
    let natural = positioned_rect(element, parent_origin, layout);
    let bounds = if let Some(anchor) = element.anchor {
        let anchor_bounds = match anchor.target {
            AnchorTarget::Element(target) => resolved_bounds
                .get(&target)
                .or_else(|| natural_bounds.get(&target))
                .copied()
                .ok_or(UiError::MissingAnchor {
                    element: element.runtime_id,
                    anchor: target,
                })?,
            AnchorTarget::Point(point) => Rect::new(point.x, point.y, 0.0, 0.0),
        };
        resolve_anchored(
            anchor_bounds,
            Size::new(layout.size.width, layout.size.height),
            viewport,
            AnchorGeometry::of(&anchor),
        )
        .bounds
    } else {
        natural
    };
    let focus_within = focused_path.contains(&element.runtime_id);
    let group_style = GroupScope::resolve(groups, element);
    // Mirror the paint path: a pure translation moves the painted box, anything else transforms
    // it through a compositing group whose accumulated matrix the hit region carries.
    let (declared_transform, transform_origin) = resolved_transform(
        element,
        hovered,
        pressed,
        dragging,
        drag_over,
        styled_focus,
        focus_within,
        group_style.as_ref(),
    );
    let declared_transform = if element.transition.as_ref().is_some_and(|transition| {
        transition
            .properties
            .contains(TransitionProperties::TRANSFORM)
    }) {
        style_transitions
            .get(&element.runtime_id)
            .map_or(declared_transform, |playback| playback.current.transform)
    } else {
        declared_transform
    };
    let window_transform =
        declared_transform.around(transform_origin_point(bounds, transform_origin));
    let translated = window_transform.is_translation();
    let bounds = if translated {
        bounds.translate(Vector::new(window_transform.tx, window_transform.ty))
    } else {
        bounds
    };
    resolved_bounds.insert(element.runtime_id, bounds);
    let opens_group = !translated
        && resolved_layer_effects(
            element,
            bounds,
            element
                .visual
                .corners(element.visual.radius)
                .resolve(bounds.width, bounds.height),
            window_transform,
        )
        .needs_group();
    let group_transform = if opens_group {
        parent_origin.transform.compose(window_transform)
    } else {
        parent_origin.transform
    };
    let hit_transform = (!group_transform.is_identity()).then_some(group_transform);
    let hit_bounds = expand_hit_bounds(bounds, element.hit_slop);
    let effective_parent_clip = if element.portal {
        viewport
    } else {
        parent_clip
    };
    if element.children.is_empty()
        && effective_parent_clip.intersection(bounds).is_none()
        && effective_parent_clip.intersection(hit_bounds).is_none()
        && !element_has_outset_shadow(element)
    {
        return Ok(());
    }

    let layer = element_paint_layer(element, parent_layer);
    let order = PaintOrder {
        layer,
        source: *source_order,
    };
    *source_order = (*source_order).saturating_add(1);
    let parent_clip = effective_parent_clip;
    if let Some(region) = element_hit_region(
        element,
        bounds,
        parent_clip,
        order,
        selectable_text_indices.contains_key(&element.runtime_id),
        hit_transform,
    ) {
        hit_regions.push(region);
    }

    let clips_children = element.layout.overflow.x != Overflow::Visible
        || element.layout.overflow.y != Overflow::Visible;
    let child_clip = if clips_children {
        let Some(clip) = parent_clip.intersection(bounds) else {
            return Ok(());
        };
        clip
    } else {
        parent_clip
    };

    let is_scrollable = !matches!(&element.kind, ElementKind::TextInput(_))
        && (element.layout.overflow.x == Overflow::Scroll
            || element.layout.overflow.y == Overflow::Scroll);
    let mut scroll = Vector::ZERO;
    if is_scrollable {
        let max_offset = Vector::new(
            (layout.content_size.width - layout.size.width).max(0.0),
            (layout.content_size.height - layout.size.height).max(0.0),
        );
        let offset = scroll_offsets.entry(element.runtime_id).or_default();
        offset.x = offset.x.clamp(0.0, max_offset.x);
        offset.y = offset.y.clamp(0.0, max_offset.y);
        scroll = *offset;
    } else if let Some(virtual_scroll) = &element.virtual_scroll {
        let max_offset_y = virtual_scroll
            .handle
            .max_offset(virtual_scroll.max_offset_y)
            .max(0.0);
        let offset = scroll_offsets.entry(element.runtime_id).or_default();
        offset.x = 0.0;
        offset.y = virtual_scroll.handle.offset().clamp(0.0, max_offset_y);
        scroll.y =
            virtual_scroll.handle.presented_offset(offset.y) - virtual_scroll.mount.layout_offset_y;
    }

    let mut child_origin = child_frame(
        element,
        layout,
        bounds,
        scroll,
        is_scrollable,
        parent_origin,
    );
    child_origin.transform = group_transform;
    // A group opens the scope its descendants resolve against, chained to the groups outside it
    // so a member can follow a named one past the nearest.
    let own_scope = GroupScope::for_children(element, hovered, pressed_path, groups);
    let child_groups = own_scope.as_ref().or(groups);
    for child in &element.children {
        collect_layout_hit_regions(
            child,
            taffy,
            natural_bounds,
            style_transitions,
            resolved_bounds,
            scroll_offsets,
            selectable_text_indices,
            hit_regions,
            hovered,
            pressed,
            dragging,
            drag_over,
            styled_focus,
            pressed_path,
            focused_path,
            child_groups,
            child_origin,
            child_clip,
            viewport,
            layer,
            source_order,
        )?;
    }
    Ok(())
}

/// Paint one element and its subtree.
///
/// `styled_focus` is the element whose focus styles paint — see `UiTree::styled_focus`. It is the
/// focused element only while focus is visible, except that a focused text input is always styled,
/// which is also why it can stand in for the real focus when this function paints a caret.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_element(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
    natural_bounds: &HashMap<ElementId, Rect>,
    element_bounds: &mut HashMap<ElementId, Rect>,
    scroll_offsets: &mut HashMap<ElementId, Vector>,
    hovered: &HashSet<ElementId>,
    pressed: Option<ElementId>,
    dragging: Option<ElementId>,
    drag_over: Option<ElementId>,
    styled_focus: Option<ElementId>,
    pressed_path: &[ElementId],
    focused_path: &[ElementId],
    groups: Option<&GroupScope<'_>>,
    scale_factor: f32,
    scene: &mut Scene,
    renderer: &mut impl TextLayoutEngine,
    text_inputs: &mut HashMap<ElementId, TextInputState>,
    animations: &mut HashMap<ElementId, AnimationPlayback>,
    animations_enabled: bool,
    paint_time: Instant,
    transition_context: &mut StyleTransitionPaintContext<'_>,
    hit_regions: &mut Vec<HitRegion>,
    scroll_regions: &mut Vec<ScrollRegion>,
    scrollbar_states: &mut HashMap<ElementId, ScrollbarState>,
    dismiss_regions: &mut Vec<DismissRegion>,
    #[cfg(target_os = "macos")] native_views: &mut Vec<NativeViewPlacement>,
    text_input_regions: &mut Vec<TextInputRegion>,
    selectable_text_indices: &HashMap<ElementId, usize>,
    selectable_text_regions: &mut Vec<SelectableTextRegion>,
    static_text_selection: Option<StaticTextSelection>,
    parent_origin: LayoutFrame,
    parent_clip: Rect,
    viewport: Rect,
    parent_layer: PaintLayerKey,
    source_order: &mut usize,
    inherited_state_text_color: Option<Color>,
    work: &mut crate::PipelineMetrics,
    rebuild_geometry: bool,
    cache: &mut super::paint_cache::PaintCache,
    dirty_parent: bool,
) -> Result<(), UiError> {
    let dirty_subtree = dirty_parent || cache.dirty_subtree(element.runtime_id);
    let eligible = cache.eligible(element.runtime_id, parent_layer);
    let key = super::paint_cache::PaintCacheKey {
        frame: parent_origin,
        clip: parent_clip,
        layer: parent_layer,
        opacity: scene.current_opacity(),
        text_color: inherited_state_text_color,
    };
    if eligible
        && !rebuild_geometry
        && !dirty_subtree
        && let Some(count) = cache.replay(element.runtime_id, key, scene)
    {
        *source_order = source_order.saturating_add(count);
        work.reused_subtrees += 1;
        return Ok(());
    }
    let capture = eligible.then(|| scene.begin_fragment());
    let groups_before = scene.groups().len();
    let source_before = *source_order;
    let result = paint_element_contents(
        element,
        taffy,
        natural_bounds,
        element_bounds,
        scroll_offsets,
        hovered,
        pressed,
        dragging,
        drag_over,
        styled_focus,
        pressed_path,
        focused_path,
        groups,
        scale_factor,
        scene,
        renderer,
        text_inputs,
        animations,
        animations_enabled,
        paint_time,
        transition_context,
        hit_regions,
        scroll_regions,
        scrollbar_states,
        dismiss_regions,
        #[cfg(target_os = "macos")]
        native_views,
        text_input_regions,
        selectable_text_indices,
        selectable_text_regions,
        static_text_selection,
        parent_origin,
        parent_clip,
        viewport,
        parent_layer,
        source_order,
        inherited_state_text_color,
        work,
        rebuild_geometry,
        cache,
        dirty_subtree,
    );
    if let Some(start) = capture {
        let commands = scene.finish_fragment(start);
        if result.is_ok() && scene.groups().len() == groups_before {
            cache.record(
                element.runtime_id,
                key,
                commands,
                *source_order - source_before,
            );
        }
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn paint_element_contents(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
    natural_bounds: &HashMap<ElementId, Rect>,
    element_bounds: &mut HashMap<ElementId, Rect>,
    scroll_offsets: &mut HashMap<ElementId, Vector>,
    hovered: &HashSet<ElementId>,
    pressed: Option<ElementId>,
    dragging: Option<ElementId>,
    drag_over: Option<ElementId>,
    styled_focus: Option<ElementId>,
    pressed_path: &[ElementId],
    focused_path: &[ElementId],
    groups: Option<&GroupScope<'_>>,
    scale_factor: f32,
    scene: &mut Scene,
    renderer: &mut impl TextLayoutEngine,
    text_inputs: &mut HashMap<ElementId, TextInputState>,
    animations: &mut HashMap<ElementId, AnimationPlayback>,
    animations_enabled: bool,
    paint_time: Instant,
    transition_context: &mut StyleTransitionPaintContext<'_>,
    hit_regions: &mut Vec<HitRegion>,
    scroll_regions: &mut Vec<ScrollRegion>,
    scrollbar_states: &mut HashMap<ElementId, ScrollbarState>,
    dismiss_regions: &mut Vec<DismissRegion>,
    #[cfg(target_os = "macos")] native_views: &mut Vec<NativeViewPlacement>,
    text_input_regions: &mut Vec<TextInputRegion>,
    selectable_text_indices: &HashMap<ElementId, usize>,
    selectable_text_regions: &mut Vec<SelectableTextRegion>,
    static_text_selection: Option<StaticTextSelection>,
    parent_origin: LayoutFrame,
    parent_clip: Rect,
    viewport: Rect,
    parent_layer: PaintLayerKey,
    source_order: &mut usize,
    inherited_state_text_color: Option<Color>,
    work: &mut crate::PipelineMetrics,
    rebuild_geometry: bool,
    cache: &mut super::paint_cache::PaintCache,
    dirty_parent: bool,
) -> Result<(), UiError> {
    work.painted_nodes += 1;
    if element.is_display_none() || element.is_visibility_hidden() {
        return Ok(());
    }
    let node = element
        .taffy_node
        .expect("layout nodes are assigned before paint");
    let layout = taffy.layout(node)?;
    let natural = positioned_rect(element, parent_origin, layout);
    let bounds = if let Some(anchor) = element.anchor {
        let anchor_bounds = match anchor.target {
            AnchorTarget::Element(target) => element_bounds
                .get(&target)
                .or_else(|| natural_bounds.get(&target))
                .copied()
                .ok_or(UiError::MissingAnchor {
                    element: element.runtime_id,
                    anchor: target,
                })?,
            AnchorTarget::Point(point) => Rect::new(point.x, point.y, 0.0, 0.0),
        };
        let resolved = resolve_anchored(
            anchor_bounds,
            Size::new(layout.size.width, layout.size.height),
            viewport,
            AnchorGeometry::of(&anchor),
        );
        // The declared placement is a preference. Publishing the placement this frame actually
        // used is what lets an application draw a flip-aware arrow or size a popup to the room it
        // was given without re-deriving the collision decision QuickGUI just made.
        if let Some(handle) = &element.anchor_placement {
            handle.report(resolved);
        }
        resolved.bounds
    } else {
        natural
    };
    let focus_within = focused_path.contains(&element.runtime_id);
    let group_style = GroupScope::resolve(groups, element);
    // A subtree transform never moves layout. A pure translation is folded into the painted box
    // here — children, clips, and hit bounds follow it for free — while anything else opens a
    // compositing group below.
    let (target_transform, transform_origin) = resolved_transform(
        element,
        hovered,
        pressed,
        dragging,
        drag_over,
        styled_focus,
        focus_within,
        group_style.as_ref(),
    );
    // Keep the cheap offscreen path for static geometry. A transform transition must be sampled
    // first because its current position may still be visible, or move into view on a later frame.
    if !element.transition.as_ref().is_some_and(|transition| {
        transition
            .properties
            .contains(TransitionProperties::TRANSFORM)
    }) {
        let transform = target_transform.around(transform_origin_point(bounds, transform_origin));
        let bounds = if transform.is_translation() {
            bounds.translate(Vector::new(transform.tx, transform.ty))
        } else {
            bounds
        };
        let clip = if element.portal {
            viewport
        } else {
            parent_clip
        };
        if element.children.is_empty()
            && clip.intersection(bounds).is_none()
            && clip
                .intersection(expand_hit_bounds(bounds, element.hit_slop))
                .is_none()
            && !element_has_outset_shadow(element)
        {
            if rebuild_geometry {
                element_bounds.insert(element.runtime_id, bounds);
            }
            if let Some(handle) = &element.layout_bounds {
                handle.report(bounds);
            }
            return Ok(());
        }
    }
    let empty_state = ElementStateStyle::default();
    let interaction_state = if drag_over == Some(element.runtime_id) {
        &element.drag_over
    } else if dragging == Some(element.runtime_id) {
        &element.dragging
    } else if pressed == Some(element.runtime_id) {
        &element.active
    } else if hovered.contains(&element.runtime_id) {
        &element.hover
    } else {
        &empty_state
    };
    let focus_state = if styled_focus == Some(element.runtime_id) {
        &element.focus
    } else {
        &empty_state
    };
    let disabled_state = if element.accessibility.disabled {
        &element.disabled_style
    } else {
        &empty_state
    };
    let invalid_state = if element.accessibility.invalid {
        &element.invalid_style
    } else {
        &empty_state
    };
    // A selected row keeps its selected paint while hovered or pressed, as a native list does, so
    // the selected state sits above every pointer state and beneath `disabled`.
    let selected_state = if element.accessibility.selected {
        &element.selected_style
    } else {
        &empty_state
    };
    let focus_within_state = if focus_within {
        &element.focus_within
    } else {
        &empty_state
    };
    let group_state = group_style.as_ref().unwrap_or(&empty_state);
    let target_fill = disabled_state
        .background
        .or(selected_state.background)
        .or(interaction_state.background)
        .or(invalid_state.background)
        .or(focus_state.background)
        .or(focus_within_state.background)
        .or(group_state.background)
        .or(element.visual.background)
        .unwrap_or(Color::TRANSPARENT);
    let target_border = disabled_state
        .border_color
        .or(selected_state.border_color)
        .or(interaction_state.border_color)
        .or(invalid_state.border_color)
        .or(focus_state.border_color)
        .or(focus_within_state.border_color)
        .or(group_state.border_color)
        .or(element.visual.border_color)
        .unwrap_or(Color::TRANSPARENT);
    let target_border_widths = disabled_state
        .border_width
        .or(selected_state.border_width)
        .or(interaction_state.border_width)
        .or(invalid_state.border_width)
        .or(focus_state.border_width)
        .or(focus_within_state.border_width)
        .or(group_state.border_width)
        .map(Insets::all)
        .unwrap_or(element.visual.border_widths);
    let target_radius = disabled_state
        .radius
        .or(selected_state.radius)
        .or(interaction_state.radius)
        .or(invalid_state.radius)
        .or(focus_state.radius)
        .or(focus_within_state.radius)
        .or(group_state.radius)
        .unwrap_or(element.visual.radius);
    let target_gradient = disabled_state
        .background_gradient
        .or(selected_state.background_gradient)
        .or(interaction_state.background_gradient)
        .or(invalid_state.background_gradient)
        .or(focus_state.background_gradient)
        .or(focus_within_state.background_gradient)
        .or(group_state.background_gradient)
        .or(element.visual.background_gradient);
    let target_outline = disabled_state
        .outline
        .or(selected_state.outline)
        .or(interaction_state.outline)
        .or(invalid_state.outline)
        .or(focus_state.outline)
        .or(focus_within_state.outline)
        .or(group_state.outline)
        .or(element.visual.outline);
    let target_shadows = disabled_state
        .shadows
        .as_deref()
        .or(selected_state.shadows.as_deref())
        .or(interaction_state.shadows.as_deref())
        .or(invalid_state.shadows.as_deref())
        .or(focus_state.shadows.as_deref())
        .or(focus_within_state.shadows.as_deref())
        .or(group_state.shadows.as_deref())
        .or(element.visual.shadows.as_deref())
        .unwrap_or_default();
    let target_opacity = disabled_state
        .opacity
        .or(selected_state.opacity)
        .or(interaction_state.opacity)
        .or(invalid_state.opacity)
        .or(focus_state.opacity)
        .or(focus_within_state.opacity)
        .or(group_state.opacity)
        .unwrap_or(element.visual.opacity)
        .clamp(0.0, 1.0);
    let target_state_text_color = disabled_state
        .text_color
        .or(selected_state.text_color)
        .or(interaction_state.text_color)
        .or(invalid_state.text_color)
        .or(focus_state.text_color)
        .or(focus_within_state.text_color)
        .or(group_state.text_color)
        .or(inherited_state_text_color);
    let sampled_transition = element.transition.as_ref().map(|config| {
        let text_fallback = sane_transition_color(element.resolved_typography.color, Color::BLACK);
        transition_context.sample(
            element.runtime_id,
            TransitionPaintStyle {
                background: sane_transition_color(target_fill, Color::TRANSPARENT),
                border_color: sane_transition_color(target_border, Color::TRANSPARENT),
                border_widths: target_border_widths,
                radius: target_radius.max(0.0),
                opacity: target_opacity,
                transform: target_transform,
                text_color: target_state_text_color
                    .map(|color| sane_transition_color(color, text_fallback)),
                text_fallback,
                shadows: TransitionShadowList::from_slice(target_shadows),
            },
            config,
        )
    });
    let fill = sampled_transition
        .as_ref()
        .map_or(target_fill, |style| style.background);
    let border = sampled_transition
        .as_ref()
        .map_or(target_border, |style| style.border_color);
    let border_widths = sampled_transition
        .as_ref()
        .map_or(target_border_widths, |style| style.border_widths);
    let radius = sampled_transition
        .as_ref()
        .map_or(target_radius, |style| style.radius);
    let shadows = sampled_transition
        .as_ref()
        .map_or(target_shadows, |style| style.shadows.as_slice());
    let opacity = sampled_transition
        .as_ref()
        .map_or(target_opacity, |style| style.opacity);
    let state_text_color = sampled_transition
        .as_ref()
        .map_or(target_state_text_color, |style| style.text_color);
    let declared_transform = sampled_transition
        .as_ref()
        .map_or(target_transform, |style| style.transform);
    let window_transform =
        declared_transform.around(transform_origin_point(bounds, transform_origin));
    let translated = window_transform.is_translation();
    let bounds = if translated {
        bounds.translate(Vector::new(window_transform.tx, window_transform.ty))
    } else {
        bounds
    };
    if rebuild_geometry {
        element_bounds.insert(element.runtime_id, bounds);
    }
    if let Some(handle) = &element.layout_bounds {
        handle.report(bounds);
    }
    let hit_bounds = expand_hit_bounds(bounds, element.hit_slop);

    // A clipped leaf cannot contribute pixels or interaction regions. Avoid emitting offscreen
    // text/image primitives for long documents while retaining its measured and accessibility
    // bounds above. Outset shadows are the one leaf effect allowed to cross its own bounds.
    let effective_parent_clip = if element.portal {
        viewport
    } else {
        parent_clip
    };
    if element.children.is_empty()
        && effective_parent_clip.intersection(bounds).is_none()
        && effective_parent_clip.intersection(hit_bounds).is_none()
        && !element_has_outset_shadow(element)
    {
        return Ok(());
    }

    let layer = element_paint_layer(element, parent_layer);
    let order = PaintOrder {
        layer,
        source: *source_order,
    };
    *source_order = (*source_order).saturating_add(1);
    let parent_clip = if element.portal {
        viewport
    } else {
        parent_clip
    };

    let previous_opacity = scene.multiply_opacity(opacity);
    // Explicit per-corner radii replace the single transitionable radius.
    let corners = element
        .visual
        .corner_radii
        .unwrap_or(Corners::all(radius))
        .resolve(bounds.width, bounds.height);
    // Everything this element declares that cannot be one more instanced primitive opens a
    // compositing group. Its subtree — including Glyphon text — renders into a bounded offscreen
    // texture and is composited back through the transform, filters, and blend mode below. The
    // scene refuses the group when a bound is already reached, in which case the subtree paints
    // directly and without the effect.
    let effects = resolved_layer_effects(
        element,
        bounds,
        corners,
        if translated {
            Transform2D::IDENTITY
        } else {
            window_transform
        },
    );
    let group: Option<GroupHandle> = scene.begin_group(layer, bounds, parent_clip, effects);
    let layer = group.map_or(layer, |handle| handle.content_key());
    let order = PaintOrder {
        layer,
        source: order.source,
    };
    let group_transform = match &group {
        Some(_) => parent_origin.transform.compose(window_transform),
        None => parent_origin.transform,
    };
    let hit_transform = (!group_transform.is_identity()).then_some(group_transform);
    // A group applies the element's colour chain to the whole composited subtree; without one the
    // chain stays the cheap per-primitive matrix it has always been.
    let raster_color_matrix = if group.is_some() {
        ColorMatrix::IDENTITY
    } else {
        element.visual.filters.color_matrix()
    };
    push_element_shadows(scene, layer, bounds, corners, parent_clip, shadows, false);
    if fill.a > 0.0 || target_gradient.is_some() {
        scene.push_edge_quad_in(
            layer,
            EdgeQuad::new(bounds, fill)
                .corner_radii(corners)
                .background(target_gradient)
                .clip(parent_clip),
        );
    }
    push_element_shadows(scene, layer, bounds, corners, parent_clip, shadows, true);
    if let Some(background_image) = element.visual.background_image.as_deref() {
        push_background_image(
            scene,
            layer,
            bounds,
            corners,
            parent_clip,
            raster_color_matrix,
            background_image,
        );
    }
    // The outline ring lives outside the border box and never participates in layout.
    if let Some(outline) = target_outline
        && let Some(ring) = outline.ring(bounds)
    {
        scene.push_edge_quad_in(
            layer,
            EdgeQuad::new(ring, Color::TRANSPARENT)
                .corner_radii(
                    corners
                        .expanded(outline.offset + outline.width)
                        .resolve(ring.width, ring.height),
                )
                .border(Insets::all(outline.width), outline.color)
                .border_style(outline.style)
                .clip(parent_clip),
        );
    }

    let selectable_document_index = selectable_text_indices.get(&element.runtime_id).copied();
    if rebuild_geometry
        && let Some(region) = element_hit_region(
            element,
            bounds,
            parent_clip,
            order,
            selectable_document_index.is_some(),
            hit_transform,
        )
    {
        hit_regions.push(region);
    }
    if rebuild_geometry && !element.dismiss_policy.is_empty() {
        dismiss_regions.push(DismissRegion {
            id: element.runtime_id,
            bounds,
            clip: parent_clip,
            policy: element.dismiss_policy,
            restore_focus: element.restore_focus.map(|handle| handle.id()),
            order,
        });
    }

    let mut text_input_scroll = None;
    match &element.kind {
        ElementKind::Text(content) => {
            let mut style = element.resolved_typography.clone();
            if let Some(color) = state_text_color {
                style.color = color;
            }
            let text_bounds = text_content_bounds(bounds, layout);
            let text_clip = own_text_clip(element, bounds, parent_clip);
            let decorations = if style.has_decorations() && !text_bounds.is_empty() {
                text_clip.intersection(text_bounds).map(|clip| {
                    let visible_y = (clip.y - text_bounds.y).max(0.0)
                        ..(clip.bottom() - text_bounds.y).min(text_bounds.height);
                    renderer
                        .text_geometry(
                            TextId::new(element.runtime_id.value()),
                            content,
                            &style,
                            None,
                            text_bounds.width,
                            scale_factor,
                            visible_y,
                        )
                        .decorations
                })
            } else {
                None
            };
            paint_selectable_text(
                element,
                selectable_document_index,
                content,
                &style,
                None,
                text_bounds,
                text_clip,
                layer,
                order,
                scale_factor,
                static_text_selection,
                selectable_text_indices,
                selectable_text_regions,
                scene,
                renderer,
                rebuild_geometry,
            );
            scene.push_text_in(
                layer,
                TextRun::new(
                    TextId::new(element.runtime_id.value()),
                    content.clone(),
                    text_bounds,
                    style,
                )
                .clip(text_clip),
            );
            if let Some(decorations) = decorations {
                for decoration in decorations {
                    push_text_paint_rect(
                        scene,
                        layer,
                        decoration,
                        Point::new(text_bounds.x, text_bounds.y),
                        text_clip,
                    );
                }
            }
        }
        ElementKind::StyledText(styled) => {
            let mut style = element.resolved_typography.clone();
            if let Some(color) = state_text_color {
                style.color = color;
            }
            let text_id = TextId::new(element.runtime_id.value());
            let highlights = styled.shared_highlights().clone();
            let text_bounds = text_content_bounds(bounds, layout);
            let text_clip = own_text_clip(element, bounds, parent_clip);
            if !text_bounds.is_empty()
                && let Some(clip) = text_clip.intersection(text_bounds)
            {
                let visible_y = (clip.y - text_bounds.y).max(0.0)
                    ..(clip.bottom() - text_bounds.y).min(text_bounds.height);
                let geometry = renderer.text_geometry(
                    text_id,
                    styled.content(),
                    &style,
                    Some(&highlights),
                    text_bounds.width,
                    scale_factor,
                    visible_y,
                );
                for background in geometry.backgrounds {
                    scene.push_quad_in(
                        layer,
                        Quad::new(
                            Rect::new(
                                text_bounds.x + background.rect.x,
                                text_bounds.y + background.rect.y,
                                background.rect.width,
                                background.rect.height,
                            ),
                            background.color,
                        )
                        .radius(if let TextPaintKind::Rounded(radius) = background.kind {
                            radius
                        } else {
                            0.0
                        })
                        .clip(text_clip),
                    );
                }
                paint_selectable_text(
                    element,
                    selectable_document_index,
                    styled.content(),
                    &style,
                    Some(&highlights),
                    text_bounds,
                    text_clip,
                    layer,
                    order,
                    scale_factor,
                    static_text_selection,
                    selectable_text_indices,
                    selectable_text_regions,
                    scene,
                    renderer,
                    rebuild_geometry,
                );
                for decoration in geometry
                    .decorations
                    .iter()
                    .filter(|decoration| decoration.kind == TextPaintKind::SolidUnderlay)
                {
                    push_text_paint_rect(
                        scene,
                        layer,
                        *decoration,
                        Point::new(text_bounds.x, text_bounds.y),
                        clip,
                    );
                }
                scene.push_text_in(
                    layer,
                    TextRun::new(text_id, styled.content().clone(), text_bounds, style)
                        .with_highlights(highlights)
                        .clip(text_clip),
                );
                for decoration in geometry
                    .decorations
                    .into_iter()
                    .filter(|decoration| decoration.kind != TextPaintKind::SolidUnderlay)
                {
                    push_text_paint_rect(
                        scene,
                        layer,
                        decoration,
                        Point::new(text_bounds.x, text_bounds.y),
                        clip,
                    );
                }
            }
        }
        ElementKind::Image(image) => {
            if !bounds.is_empty()
                && let Some(clip) = parent_clip.intersection(bounds)
                && let Some(source) = match &image.resolved {
                    ImageResolution::Ready(source) => Some(source),
                    ImageResolution::Animated(animation) => {
                        let frame_index = animations
                            .get_mut(&element.runtime_id)
                            .map(|playback| {
                                playback.activate(paint_time, animations_enabled);
                                playback.frame_index
                            })
                            .unwrap_or(0);
                        animation.frame(frame_index).map(|frame| frame.image())
                    }
                    ImageResolution::Loading | ImageResolution::Failed => None,
                }
            {
                let fitted = fit_image(bounds, source.size(), image.object_fit);
                scene.push_image_in(
                    layer,
                    ImagePrimitive::new(source.clone(), fitted.destination)
                        .source_uv(fitted.source_uv)
                        .mask(bounds)
                        .radius(element.visual.corners(element.visual.radius).maximum())
                        .color_matrix(raster_color_matrix)
                        .clip(clip),
                );
            }
        }
        ElementKind::Svg(svg) => {
            if !bounds.is_empty()
                && let Some(clip) = parent_clip.intersection(bounds)
            {
                let fitted = fit_image(bounds, svg.svg.size(), svg.object_fit);
                let color = state_text_color.unwrap_or(element.resolved_typography.color);
                scene.push_svg_in(
                    layer,
                    SvgPrimitive::new(svg.svg.clone(), fitted.destination, color)
                        .source_uv(fitted.source_uv)
                        .mask(bounds)
                        .radius(element.visual.corners(element.visual.radius).maximum())
                        .transform(svg.transform)
                        .clip(clip),
                );
            }
        }
        ElementKind::Path(path) => {
            if let Some(clip) = parent_clip.intersection(bounds)
                && let Some((scale, translation)) = fit_path(bounds, &path.path, path.object_fit)
            {
                let background = path.background.unwrap_or_else(|| {
                    state_text_color
                        .unwrap_or(element.resolved_typography.color)
                        .into()
                });
                scene.push_path_in(
                    layer,
                    PathPrimitive::new(&path.path, background)
                        .scale_xy(scale[0], scale[1])
                        .translate(translation.x, translation.y)
                        .clip(clip),
                );
            }
        }
        ElementKind::Canvas(canvas) => {
            if !bounds.is_empty()
                && let Some(clip) = parent_clip.intersection(bounds)
            {
                let local_bounds = Rect::new(0.0, 0.0, bounds.width, bounds.height);
                let mut context = Canvas::new(
                    scene,
                    layer,
                    Point::new(bounds.x, bounds.y),
                    Size::new(bounds.width, bounds.height),
                    clip,
                );
                (canvas.painter.as_ref())(local_bounds, &mut context);
            }
        }
        ElementKind::CustomShader(shader) => {
            if !bounds.is_empty()
                && let Some(clip) = parent_clip.intersection(bounds)
            {
                scene.push_custom_shader_in(
                    layer,
                    CustomShaderPrimitive::new(shader.shader.clone(), bounds)
                        .parameters(shader.parameters)
                        .clip(clip),
                );
            }
        }
        ElementKind::TextInput(input) => {
            let input_state = text_inputs.entry(element.runtime_id).or_insert_with(|| {
                TextInputState::with_styling(
                    &input.value,
                    input.multiline,
                    input.constraints.clone(),
                    input.highlights.clone(),
                )
            });
            let mut style = element.resolved_typography.clone();
            if let Some(color) = state_text_color {
                style.color = color;
            }
            let vertical_inset = if input.multiline {
                10.0_f32.min(bounds.height * 0.5)
            } else {
                ((bounds.height - style.line_height) * 0.5).max(0.0)
            };
            let text_viewport = bounds.inset(input.presentation.content_insets.unwrap_or(Insets {
                top: vertical_inset,
                right: 12.0,
                bottom: vertical_inset,
                left: 12.0,
            }));
            if text_viewport.width > 0.0
                && text_viewport.height > 0.0
                && let Some(text_clip) = parent_clip.intersection(text_viewport)
            {
                let source_content = input_state.shared_text();
                let password = input
                    .password
                    .then(|| PasswordDisplay::new(&source_content));
                let content = password
                    .as_ref()
                    .map_or_else(|| source_content.clone(), |display| display.content.clone());
                let display_index = |source_index| {
                    password
                        .as_ref()
                        .map_or(source_index, |display| display.display_index(source_index))
                };
                let highlights = (!input.password && !content.is_empty())
                    .then(|| input_state.shared_highlights())
                    .filter(|highlights| !highlights.is_empty());
                let text_id = TextId::new(element.runtime_id.value());
                // A focused text input is always the styled focus, so this is the real focus too.
                let is_focused = styled_focus == Some(element.runtime_id);
                let content_size = if content.is_empty() {
                    Size::new(0.0, style.line_height)
                } else if let Some(highlights) = &highlights {
                    renderer.measure_styled_text(
                        text_id,
                        &content,
                        &style,
                        highlights,
                        Some(text_viewport.width),
                        scale_factor,
                    )
                } else {
                    renderer.measure_text(
                        text_id,
                        &content,
                        &style,
                        Some(text_viewport.width),
                        scale_factor,
                    )
                };
                let caret = if content.is_empty() {
                    Point::ZERO
                } else {
                    renderer.text_caret_position_with_highlights(
                        text_id,
                        &content,
                        &style,
                        highlights.as_ref(),
                        text_viewport.width,
                        scale_factor,
                        display_index(input_state.caret()),
                    )
                };
                let max_scroll = text_input_max_scroll(
                    content_size,
                    Size::new(text_viewport.width, text_viewport.height),
                    style.line_height,
                    input.multiline,
                );
                let mut scroll = scroll_offsets
                    .get(&element.runtime_id)
                    .copied()
                    .unwrap_or_default();
                scroll.x = scroll.x.clamp(0.0, max_scroll.x);
                scroll.y = scroll.y.clamp(0.0, max_scroll.y);
                let scroll_before_caret = scroll;
                if is_focused && !content.is_empty() {
                    scroll = scroll_to_reveal_caret(
                        Size::new(text_viewport.width, text_viewport.height),
                        caret,
                        style.line_height,
                        scroll,
                        max_scroll,
                    );
                }
                scroll_offsets.insert(element.runtime_id, scroll);
                if scroll != scroll_before_caret {
                    let scrollbar_state = scrollbar_states.entry(element.runtime_id).or_default();
                    if !scrollbar_state.hovered && !scrollbar_state.dragging {
                        scrollbar_state.visible_until =
                            paint_time.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
                    }
                }
                let caret_inset = input
                    .presentation
                    .caret_height_em
                    .map_or(if input.multiline { 1.0 } else { 2.0 }, |em| {
                        (style.line_height - (style.font_size * em).min(style.line_height)) * 0.5
                    });
                let caret_bounds = Rect::new(
                    text_viewport.x + caret.x - scroll.x,
                    text_viewport.y + caret.y - scroll.y + caret_inset,
                    input.presentation.caret_width.unwrap_or(1.5),
                    (style.line_height - caret_inset * 2.0).max(1.0),
                );

                let mut decorations = Vec::new();
                if !content.is_empty() && (highlights.is_some() || style.has_decorations()) {
                    let geometry = renderer.text_geometry(
                        text_id,
                        &content,
                        &style,
                        highlights.as_ref(),
                        text_viewport.width,
                        scale_factor,
                        scroll.y..scroll.y + text_viewport.height,
                    );
                    for background in geometry.backgrounds {
                        scene.push_quad_in(
                            layer,
                            Quad::new(
                                Rect::new(
                                    text_viewport.x + background.rect.x - scroll.x,
                                    text_viewport.y + background.rect.y - scroll.y,
                                    background.rect.width,
                                    background.rect.height,
                                ),
                                background.color,
                            )
                            .radius(if let TextPaintKind::Rounded(radius) = background.kind {
                                radius
                            } else {
                                0.0
                            })
                            .clip(text_clip),
                        );
                    }
                    decorations = geometry.decorations;
                }

                let selection = input_state.selection();
                if is_focused && !selection.is_empty() && !content.is_empty() {
                    for rect in renderer.text_selection_rects_with_highlights(
                        text_id,
                        &content,
                        &style,
                        highlights.as_ref(),
                        text_viewport.width,
                        scale_factor,
                        scroll.y..scroll.y + text_viewport.height,
                        display_index(selection.start),
                        display_index(selection.end),
                    ) {
                        scene.push_quad_in(
                            layer,
                            Quad::new(
                                Rect::new(
                                    text_viewport.x + rect.x - scroll.x,
                                    text_viewport.y + rect.y - scroll.y,
                                    rect.width,
                                    rect.height,
                                ),
                                Color::rgba8(48, 120, 196, 105),
                            )
                            .clip(text_clip),
                        );
                    }
                }
                // The caret is the text's own colour, as a native field's is, and it blinks on
                // AppKit's cadence: solid after every edit or caret move, then alternating.
                if is_focused && selection.is_empty() {
                    if input_state.caret_visible_at(paint_time) {
                        scene.push_quad_in(
                            layer,
                            Quad::new(
                                caret_bounds,
                                input.presentation.caret_color.unwrap_or(style.color),
                            )
                            .clip(text_clip),
                        );
                    }
                } else {
                    input_state.clear_caret_blink();
                }
                if let Some(marked) = input_state.marked()
                    && !marked.is_empty()
                    && !content.is_empty()
                {
                    for rect in renderer.text_selection_rects_with_highlights(
                        text_id,
                        &content,
                        &style,
                        highlights.as_ref(),
                        text_viewport.width,
                        scale_factor,
                        scroll.y..scroll.y + text_viewport.height,
                        display_index(marked.start),
                        display_index(marked.end),
                    ) {
                        scene.push_quad_in(
                            layer,
                            Quad::new(
                                Rect::new(
                                    text_viewport.x + rect.x - scroll.x,
                                    text_viewport.y + rect.bottom() - scroll.y - 1.5,
                                    rect.width,
                                    1.0,
                                ),
                                style.color,
                            )
                            .clip(text_clip),
                        );
                    }
                }

                let (display_text, display_style) =
                    if content.is_empty() && !input.placeholder.is_empty() {
                        let mut placeholder_style = style.clone();
                        placeholder_style.color = input
                            .presentation
                            .placeholder_color
                            .unwrap_or(Color::rgb8(132, 137, 148));
                        (input.placeholder.clone(), placeholder_style)
                    } else {
                        (content.clone(), style.clone())
                    };
                let text_bounds = Rect::new(
                    text_viewport.x - scroll.x,
                    text_viewport.y - scroll.y,
                    text_viewport.width,
                    content_size.height.max(text_viewport.height),
                );
                let mut text_run =
                    TextRun::new(text_id, display_text, text_bounds, display_style).clip(text_clip);
                if let Some(highlights) = &highlights {
                    text_run = text_run.with_highlights(highlights.clone());
                }
                scene.push_text_in(layer, text_run);
                for decoration in decorations {
                    push_text_paint_rect(
                        scene,
                        layer,
                        decoration,
                        Point::new(text_viewport.x - scroll.x, text_viewport.y - scroll.y),
                        text_clip,
                    );
                }
                text_input_regions.push(TextInputRegion {
                    id: element.runtime_id,
                    bounds: text_viewport,
                    clip: text_clip,
                    content,
                    password,
                    highlights,
                    style,
                    scroll,
                    max_scroll,
                    caret_bounds,
                });
                if input.multiline && max_scroll.y > 0.0 {
                    let scrollbar_bounds = Rect::new(
                        bounds.x,
                        text_viewport.y,
                        bounds.width,
                        text_viewport.height,
                    );
                    text_input_scroll = Some((
                        bounds,
                        scrollbar_bounds,
                        parent_clip.intersection(bounds).unwrap_or(text_clip),
                        scroll,
                        max_scroll,
                    ));
                }
            }
        }
        #[cfg(target_os = "macos")]
        ElementKind::NativeView(view) => {
            if layer.plane != ScenePlane::Base {
                return Err(UiError::NativeViewInOverlay(element.runtime_id));
            }
            // The AppKit frame may carry an outset for effects drawn past the control; layout
            // and hit regions keep the element's own box.
            let frame = view.frame(bounds);
            if let Some(clip) = parent_clip.intersection(frame) {
                native_views.push(NativeViewPlacement {
                    id: element.runtime_id,
                    view: view.clone(),
                    bounds: frame,
                    clip,
                    corner_radius: if view.outset() > 0.0 {
                        0.0
                    } else {
                        element.visual.corners(element.visual.radius).maximum()
                    },
                    opacity: scene.current_opacity(),
                    z_index: layer.z_index,
                    source_order: order.source,
                });
            }
        }
        ElementKind::Container | ElementKind::ContainerQuery(_) => {}
    }

    let clips_children = element.layout.overflow.x != Overflow::Visible
        || element.layout.overflow.y != Overflow::Visible;
    let child_clip = if clips_children {
        match parent_clip.intersection(bounds) {
            Some(clip) => clip,
            None => {
                if let Some(group) = group {
                    scene.end_group(group);
                }
                scene.restore_opacity(previous_opacity);
                return Ok(());
            }
        }
    } else {
        parent_clip
    };

    let is_scrollable = !matches!(&element.kind, ElementKind::TextInput(_))
        && (element.layout.overflow.x == Overflow::Scroll
            || element.layout.overflow.y == Overflow::Scroll);
    let mut scroll = Vector::ZERO;
    let mut scroll_max_offset = None;
    if is_scrollable {
        let max_offset = Vector::new(
            (layout.content_size.width - layout.size.width).max(0.0),
            (layout.content_size.height - layout.size.height).max(0.0),
        );
        let offset = scroll_offsets.entry(element.runtime_id).or_default();
        offset.x = offset.x.clamp(0.0, max_offset.x);
        offset.y = offset.y.clamp(0.0, max_offset.y);
        scroll = *offset;
        scroll_max_offset = Some(max_offset);
    } else if let Some(virtual_scroll) = &element.virtual_scroll {
        let max_offset_y = virtual_scroll
            .handle
            .max_offset(virtual_scroll.max_offset_y)
            .max(0.0);
        let offset = scroll_offsets.entry(element.runtime_id).or_default();
        offset.x = 0.0;
        offset.y = virtual_scroll.handle.offset().clamp(0.0, max_offset_y);
        scroll.y =
            virtual_scroll.handle.presented_offset(offset.y) - virtual_scroll.mount.layout_offset_y;
    }

    let mut child_origin = child_frame(
        element,
        layout,
        bounds,
        scroll,
        scroll_max_offset.is_some(),
        parent_origin,
    );
    child_origin.transform = group_transform;
    // A group opens the scope its descendants resolve against, chained to the groups outside it
    // so a member can follow a named one past the nearest.
    let own_scope = GroupScope::for_children(element, hovered, pressed_path, groups);
    let child_groups = own_scope.as_ref().or(groups);
    for child in &element.children {
        paint_element(
            child,
            taffy,
            natural_bounds,
            element_bounds,
            scroll_offsets,
            hovered,
            pressed,
            dragging,
            drag_over,
            styled_focus,
            pressed_path,
            focused_path,
            child_groups,
            scale_factor,
            scene,
            renderer,
            text_inputs,
            animations,
            animations_enabled,
            paint_time,
            transition_context,
            hit_regions,
            scroll_regions,
            scrollbar_states,
            dismiss_regions,
            #[cfg(target_os = "macos")]
            native_views,
            text_input_regions,
            selectable_text_indices,
            selectable_text_regions,
            static_text_selection,
            child_origin,
            child_clip,
            viewport,
            layer,
            source_order,
            state_text_color,
            work,
            rebuild_geometry,
            cache,
            dirty_parent,
        )?;
    }
    if border.a > 0.0 && has_visible_border(border_widths) {
        scene.push_edge_quad_in(
            layer,
            EdgeQuad::new(bounds, border.with_alpha(0.0))
                .corner_radii(corners)
                .border(border_widths, border)
                .border_style(element.visual.border_style)
                .clip(parent_clip),
        );
    }

    if let Some((text_bounds, scrollbar_bounds, text_clip, text_scroll, max_offset)) =
        text_input_scroll
    {
        let scrollbar_order = PaintOrder {
            layer,
            source: *source_order,
        };
        *source_order = (*source_order).saturating_add(1);
        let region = ScrollRegion {
            id: element.runtime_id,
            rtl: false,
            bounds: text_bounds,
            scrollbar_bounds,
            clip: text_clip,
            max_offset,
            virtual_scroll: false,
            order,
            scrollbar_order,
        };
        if rebuild_geometry {
            scroll_regions.push(region);
        }
        paint_vertical_scrollbar(
            scene,
            layer,
            region,
            text_scroll.y,
            scrollbar_states
                .get(&region.id)
                .copied()
                .unwrap_or_default(),
            paint_time,
        );
    }

    let vertical_scroll = if let Some(max_offset) = scroll_max_offset {
        Some((max_offset, scroll.y, false))
    } else if let Some(virtual_scroll) = &element.virtual_scroll {
        let max_offset = Vector::new(
            0.0,
            virtual_scroll
                .handle
                .max_offset(virtual_scroll.max_offset_y)
                .max(0.0),
        );
        let offset = scroll_offsets.entry(element.runtime_id).or_default();
        offset.x = 0.0;
        offset.y = offset.y.clamp(0.0, max_offset.y);
        Some((max_offset, offset.y, true))
    } else {
        None
    };

    // Ordinary overflow containers and retained virtual lists intentionally converge here. This
    // keeps their hit track, hover expansion, captured drag, paint style, and autohide behavior
    // identical; virtual lists differ only in how an offset change is propagated back to the view.
    if let Some((max_offset, scroll_offset_y, virtual_scroll)) = vertical_scroll {
        let scrollbar_order = PaintOrder {
            layer,
            source: *source_order,
        };
        *source_order = (*source_order).saturating_add(1);
        let region = ScrollRegion {
            id: element.runtime_id,
            rtl: element.resolved_direction.is_rtl(),
            bounds,
            scrollbar_bounds: bounds,
            clip: child_clip,
            max_offset,
            virtual_scroll,
            order,
            scrollbar_order,
        };
        if rebuild_geometry {
            scroll_regions.push(region);
        }
        paint_vertical_scrollbar(
            scene,
            layer,
            region,
            scroll_offset_y,
            scrollbar_states
                .get(&region.id)
                .copied()
                .unwrap_or_default(),
            paint_time,
        );
    }
    if let Some(group) = group {
        scene.end_group(group);
    }
    scene.restore_opacity(previous_opacity);
    Ok(())
}

pub(super) fn push_text_paint_rect(
    scene: &mut Scene,
    layer: PaintLayerKey,
    paint: TextPaintRect,
    origin: Point,
    clip: Rect,
) {
    let rect = Rect::new(
        origin.x + paint.rect.x,
        origin.y + paint.rect.y,
        paint.rect.width,
        paint.rect.height,
    );
    match paint.kind {
        TextPaintKind::Rounded(radius) => {
            scene.push_quad_in(
                layer,
                Quad::new(rect, paint.color).radius(radius).clip(clip),
            );
        }
        TextPaintKind::Solid | TextPaintKind::SolidUnderlay => {
            scene.push_quad_in(layer, Quad::new(rect, paint.color).clip(clip));
        }
        TextPaintKind::WavyUnderline {
            baseline,
            amplitude,
            thickness,
            wavelength,
        } => {
            scene.push_wavy_underline_in(
                layer,
                WavyUnderline::new(
                    rect,
                    origin.y + baseline,
                    amplitude,
                    thickness,
                    wavelength,
                    paint.color,
                )
                .clip(clip),
            );
        }
    }
}

pub(super) fn effective_cursor_style(
    element: &Element,
    selectable_text: bool,
) -> Option<CursorStyle> {
    if element.accessibility.disabled {
        return element.disabled_style.cursor_style.or_else(|| {
            element
                .cursor_style_explicit
                .then_some(element.cursor_style)
                .flatten()
        });
    }
    if element.cursor_style_explicit {
        return element.cursor_style;
    }
    if element.drag_source {
        return Some(CursorStyle::OpenHand);
    }
    element
        .cursor_style
        .or_else(|| selectable_text.then_some(CursorStyle::IBeam))
}

pub(super) fn paint_vertical_scrollbar(
    scene: &mut Scene,
    layer: PaintLayerKey,
    region: ScrollRegion,
    scroll_offset_y: f32,
    state: ScrollbarState,
    now: Instant,
) {
    if !state.visible(now) {
        return;
    }
    let Some(scrollbar) = vertical_scrollbar_geometry(region, scroll_offset_y) else {
        return;
    };
    let expanded = state.hovered || state.dragging;
    let width = if expanded { 8.0 } else { 4.0 };
    let alpha = if expanded { 210 } else { 150 };
    scene.push_quad_in(
        layer,
        Quad::new(
            Rect::new(
                region.scrollbar_bounds.right() - 2.0 - width,
                scrollbar.thumb.y + 2.0,
                width,
                (scrollbar.thumb.height - 4.0).max(4.0),
            ),
            Color::rgba8(142, 147, 160, alpha),
        )
        .radius(width * 0.5)
        .clip(region.clip),
    );
}

pub(super) fn vertical_scrollbar_geometry(
    region: ScrollRegion,
    scroll_offset_y: f32,
) -> Option<VerticalScrollbarGeometry> {
    let bounds = region.scrollbar_bounds;
    let viewport_height = bounds.height;
    if region.max_offset.y <= 0.0 || viewport_height <= 0.0 {
        return None;
    }
    let content_height = viewport_height + region.max_offset.y;
    let thumb_height = (viewport_height * viewport_height / content_height)
        .max(24.0)
        .min(viewport_height);
    let travel = viewport_height - thumb_height;
    let thumb_y =
        bounds.y + travel * (scroll_offset_y.clamp(0.0, region.max_offset.y) / region.max_offset.y);
    Some(VerticalScrollbarGeometry {
        track: Rect::new(
            (bounds.right() - 12.0).max(bounds.x),
            bounds.y,
            bounds.width.min(12.0),
            viewport_height,
        ),
        thumb: Rect::new(
            (bounds.right() - 12.0).max(bounds.x),
            thumb_y,
            bounds.width.min(12.0),
            thumb_height,
        ),
        travel,
    })
}
