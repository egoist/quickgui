use super::*;
use taffy::TraversePartialTree;

/// Layout identities outlive a view declaration. Taffy's per-node cache then skips clean
/// subtrees using its constraint keys, and a changed leaf dirties only its ancestor path.
#[derive(Default)]
pub(super) struct LayoutNodeCache {
    pub(super) nodes: HashMap<ElementId, NodeId>,
    pub(super) positions: HashMap<ElementId, (ElementId, usize)>,
    child_updates: Vec<(NodeId, Vec<NodeId>)>,
    root_constraints: HashMap<NodeId, (Size, f32)>,
    pub(super) layout_passes: usize,
    pub(super) measured_nodes: usize,
    pub(super) reconciled_nodes: usize,
}

impl LayoutNodeCache {
    fn reconcile(
        &mut self,
        taffy: &mut TaffyTree<MeasureContext>,
        id: ElementId,
        style: &TaffyStyle,
        context: Option<MeasureContext>,
        children: Vec<NodeId>,
    ) -> Result<NodeId, UiError> {
        self.reconciled_nodes += 1;
        let node = if let Some(&node) = self.nodes.get(&id) {
            if taffy.style(node)? != style {
                taffy.set_style(node, style.clone())?;
            }
            match (taffy.get_node_context(node), context) {
                (None, None) => {}
                (Some(old), Some(new)) if old.same_measurement(&new) => {
                    // Keep the complete current style for any later measurement, without
                    // invalidating layout for foreground, shadow, or decoration changes.
                    *taffy.get_node_context_mut(node).unwrap() = new;
                }
                (_, context) => taffy.set_node_context(node, context)?,
            }
            node
        } else {
            let node = match context {
                Some(context) => taffy.new_leaf_with_context(style.clone(), context)?,
                None => taffy.new_leaf(style.clone())?,
            };
            self.nodes.insert(id, node);
            node
        };
        if !taffy.child_ids(node).eq(children.iter().copied()) {
            self.child_updates.push((node, children));
        }
        Ok(node)
    }

    pub(super) fn commit_children(
        &mut self,
        taffy: &mut TaffyTree<MeasureContext>,
    ) -> Result<(), UiError> {
        if !self.child_updates.is_empty() {
            // A disconnected query root can become an ordinary child (or vice versa). Its
            // last root constraints no longer describe the layout written by its new parent.
            self.root_constraints.clear();
        }
        // Detach all changed edges before attaching any. A keyed child can move to a parent
        // visited earlier, or swap ancestry with an old parent; neither may create a transient
        // cycle during Taffy's upward dirty propagation.
        for (node, _) in &self.child_updates {
            taffy.set_children(*node, &[])?;
        }
        for (node, children) in self.child_updates.drain(..) {
            if !children.is_empty() {
                taffy.set_children(node, &children)?;
            }
        }
        Ok(())
    }

    pub(super) fn retain(
        &mut self,
        taffy: &mut TaffyTree<MeasureContext>,
        mounted: &HashSet<ElementId>,
    ) -> Result<(), UiError> {
        let removed: Vec<_> = self
            .nodes
            .iter()
            .filter_map(|(id, node)| (!mounted.contains(id)).then_some((*id, *node)))
            .collect();
        for (id, node) in removed {
            // Taffy removes the layout node separately from its measurement context. Drop
            // text/image references too, so unmounting a large document releases its contents.
            taffy.set_node_context(node, None)?;
            taffy.remove(node)?;
            self.nodes.remove(&id);
            self.positions.remove(&id);
            self.root_constraints.remove(&node);
        }
        Ok(())
    }

    pub(super) fn invalidate_measurements(
        &self,
        taffy: &mut TaffyTree<MeasureContext>,
    ) -> Result<(), UiError> {
        for &node in self.nodes.values() {
            if taffy.get_node_context(node).is_some() {
                taffy.mark_dirty(node)?;
            }
        }
        Ok(())
    }

    pub(super) fn compute_layout(
        &mut self,
        taffy: &mut TaffyTree<MeasureContext>,
        root: NodeId,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
    ) -> Result<(), UiError> {
        if self.root_constraints.get(&root) == Some(&(viewport, scale_factor))
            && !taffy.dirty(root)?
        {
            // Avoid even Taffy's final whole-subtree pixel-rounding walk on paint-only
            // declarations. Its internal cache still skips clean branches of a dirty root.
            return Ok(());
        }
        self.measured_nodes += compute_layout_measured(
            taffy,
            root,
            TaffySize {
                width: AvailableSpace::Definite(viewport.width),
                height: AvailableSpace::Definite(viewport.height),
            },
            scale_factor,
            renderer,
        )?;
        self.root_constraints.insert(root, (viewport, scale_factor));
        self.layout_passes += 1;
        Ok(())
    }
}

impl MeasureContext {
    fn same_measurement(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Text {
                    id,
                    content,
                    style,
                    highlights,
                },
                Self::Text {
                    id: other_id,
                    content: other_content,
                    style: other_style,
                    highlights: other_highlights,
                },
            ) => {
                id == other_id
                    && content == other_content
                    && same_text_metrics(style, other_style)
                    && highlights == other_highlights
            }
            (Self::Image { intrinsic }, Self::Image { intrinsic: other }) => intrinsic == other,
            _ => false,
        }
    }
}

fn same_text_metrics(a: &TextStyle, b: &TextStyle) -> bool {
    a.font_size == b.font_size
        && a.line_height == b.line_height
        && a.monospace_width == b.monospace_width
        && a.family == b.family
        && a.features == b.features
        && a.fallbacks == b.fallbacks
        && a.weight == b.weight
        && a.font_style == b.font_style
        && a.align == b.align
        && a.wrap == b.wrap
        && a.text_overflow == b.text_overflow
        && a.line_clamp == b.line_clamp
        && a.shaping == b.shaping
        && a.direction == b.direction
        && a.letter_spacing == b.letter_spacing
        && a.word_spacing == b.word_spacing
        && a.transform == b.transform
        && a.word_break == b.word_break
        && a.overflow_wrap == b.overflow_wrap
        && a.hyphens == b.hyphens
}

pub(super) fn fixed_text_layout_size(
    known: TaffySize<Option<f32>>,
    available: TaffySize<AvailableSpace>,
    layout: &TaffyStyle,
    text: &TextStyle,
) -> Option<TaffySize<f32>> {
    let width = known.width.or_else(|| absolute_length(layout.size.width));
    let height = known.height.or_else(|| absolute_length(layout.size.height));
    if let (Some(width), Some(height)) = (width, height) {
        return Some(TaffySize { width, height });
    }

    // Taffy invokes leaf measurement again during PerformLayout to calculate content metadata,
    // even when flexbox has already assigned the leaf a definite main-axis size. A web-style
    // `flex: 1 1 0; min-width: 0` no-wrap label explicitly opts out of intrinsic width sizing, so
    // shaping it here cannot affect layout. Its fixed line box also makes the height definite.
    let fills_available_width = text.wrap == TextWrap::None
        && absolute_length(layout.flex_basis) == Some(0.0)
        && absolute_length(layout.min_size.width) == Some(0.0);
    if !fills_available_width {
        return None;
    }

    let width = width.unwrap_or_else(|| match available.width {
        AvailableSpace::Definite(width) => width.max(0.0),
        // This element has explicitly disabled intrinsic width participation. Taffy may still
        // request content metadata during PerformLayout; reporting zero here cannot influence
        // the final flex width, which was established from the zero basis and flex growth.
        AvailableSpace::MinContent | AvailableSpace::MaxContent => 0.0,
    });
    height.map(|height| TaffySize { width, height })
}

pub(super) fn absolute_length(dimension: Dimension) -> Option<f32> {
    (dimension.tag() == CompactLength::LENGTH_TAG).then(|| dimension.value().max(0.0))
}

pub(super) fn compute_detached_layout(
    taffy: &mut TaffyTree<MeasureContext>,
    root: NodeId,
    viewport: Size,
    scale_factor: f32,
    renderer: &mut impl TextLayoutEngine,
) -> Result<(), UiError> {
    compute_detached_layout_available(
        taffy,
        root,
        TaffySize {
            width: AvailableSpace::Definite(viewport.width),
            height: AvailableSpace::Definite(viewport.height),
        },
        scale_factor,
        renderer,
    )
}

pub(super) fn compute_detached_layout_available(
    taffy: &mut TaffyTree<MeasureContext>,
    root: NodeId,
    available: TaffySize<AvailableSpace>,
    scale_factor: f32,
    renderer: &mut impl TextLayoutEngine,
) -> Result<(), UiError> {
    compute_layout_measured(taffy, root, available, scale_factor, renderer).map(|_| ())
}

fn compute_layout_measured(
    taffy: &mut TaffyTree<MeasureContext>,
    root: NodeId,
    available: TaffySize<AvailableSpace>,
    scale_factor: f32,
    renderer: &mut impl TextLayoutEngine,
) -> Result<usize, UiError> {
    let mut measured = 0;
    taffy.compute_layout_with_measure(
        root,
        available,
        |known, available, _node, context, style| {
            let Some(context) = context else {
                return TaffySize::ZERO;
            };
            if let (Some(width), Some(height)) = (known.width, known.height) {
                return TaffySize { width, height };
            }
            measured += 1;
            match context {
                MeasureContext::Text {
                    id,
                    content,
                    style: text_style,
                    highlights,
                } => {
                    if let Some(size) = fixed_text_layout_size(known, available, style, text_style)
                    {
                        return size;
                    }
                    // For a leaf with padding or a border, Taffy passes its assigned border-box
                    // width in `known` while the definite available width has already had those
                    // insets removed. Text shaping is content-box work, so using `known.width`
                    // here makes layout count fewer lines than paint and lets the final line cross
                    // a grid-row border during resize.
                    let max_width = match available.width {
                        AvailableSpace::Definite(width) => Some(width.max(0.0)),
                        AvailableSpace::MinContent => None,
                        AvailableSpace::MaxContent => None,
                    };
                    let measured = if let Some(highlights) = highlights {
                        renderer.measure_styled_text(
                            *id,
                            content,
                            text_style,
                            highlights,
                            max_width,
                            scale_factor,
                        )
                    } else {
                        renderer.measure_text(*id, content, text_style, max_width, scale_factor)
                    };
                    TaffySize {
                        width: measured.width,
                        height: measured.height,
                    }
                }
                MeasureContext::Image { intrinsic } => {
                    measure_image(known.width, known.height, *intrinsic)
                }
            }
        },
    )?;
    Ok(measured)
}

/// Lay out every callback subtree as an independent root within its query's assigned box.
///
/// Query child nodes intentionally exist in the same Taffy arena but are not connected to the
/// query leaf. Keeping them in one arena preserves QuickGUI's compact node IDs and lets the normal
/// paint, hit-test, accessibility, and focus walks keep traversing the ordinary `Element` tree.
pub(super) fn compute_container_query_child_layouts(
    element: &Element,
    taffy: &mut TaffyTree<MeasureContext>,
    scale_factor: f32,
    renderer: &mut impl TextLayoutEngine,
    mut layout_nodes: Option<&mut LayoutNodeCache>,
) -> Result<(), UiError> {
    if element.is_display_none() {
        return Ok(());
    }
    if matches!(&element.kind, ElementKind::ContainerQuery(_)) {
        debug_assert!(element.children.len() <= 1);
        let Some(child) = element.children.first() else {
            return Ok(());
        };
        let node = element
            .taffy_node
            .expect("container query nodes are assigned before isolated layout");
        let layout = taffy.layout(node)?;
        let available = Size::new(
            finite_layout_length(layout.size.width),
            finite_layout_length(layout.size.height),
        );
        let child_node = child
            .taffy_node
            .expect("container query children are assigned before isolated layout");
        if let Some(cache) = layout_nodes.as_deref_mut() {
            cache.compute_layout(taffy, child_node, available, scale_factor, renderer)?;
        } else {
            compute_detached_layout(taffy, child_node, available, scale_factor, renderer)?;
        }
        compute_container_query_child_layouts(child, taffy, scale_factor, renderer, layout_nodes)?;
        return Ok(());
    }

    for child in &element.children {
        compute_container_query_child_layouts(
            child,
            taffy,
            scale_factor,
            renderer,
            layout_nodes.as_deref_mut(),
        )?;
    }
    Ok(())
}

#[cfg(any(test, feature = "test-support"))]
pub(super) fn mark_layout_nodes_dirty(
    element: &Element,
    taffy: &mut TaffyTree<MeasureContext>,
) -> Result<(), UiError> {
    for child in &element.children {
        mark_layout_nodes_dirty(child, taffy)?;
    }
    let node = element
        .taffy_node
        .expect("mounted elements have layout nodes before visual measurement invalidation");
    taffy.mark_dirty(node)?;
    Ok(())
}

pub(super) fn finite_layout_length(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

pub(super) fn validate_container_query_limits(root: &Element) -> Result<(), UiError> {
    pub(super) fn visit(
        element: &Element,
        query_depth: usize,
        query_count: &mut usize,
    ) -> Result<(), UiError> {
        let query_depth = if matches!(&element.kind, ElementKind::ContainerQuery(_)) {
            *query_count += 1;
            if *query_count > MAX_CONTAINER_QUERIES_PER_WINDOW {
                return Err(UiError::TooManyContainerQueries);
            }
            let depth = query_depth + 1;
            if depth > MAX_CONTAINER_QUERY_DEPTH {
                return Err(UiError::ContainerQueryDepthExceeded);
            }
            depth
        } else {
            query_depth
        };

        for child in &element.children {
            visit(child, query_depth, query_count)?;
        }
        Ok(())
    }

    visit(root, 0, &mut 0)
}

pub(super) fn container_queries_need_resolution(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
) -> Result<bool, UiError> {
    if element.is_display_none() {
        return Ok(false);
    }
    if let ElementKind::ContainerQuery(query) = &element.kind {
        let node = element
            .taffy_node
            .expect("container query nodes are assigned before size reconciliation");
        let layout = taffy.layout(node)?;
        let assigned_size = Size::new(
            finite_layout_length(layout.size.width),
            finite_layout_length(layout.size.height),
        );
        if query.resolved_size != Some(assigned_size) || element.children.len() != 1 {
            return Ok(true);
        }
    }
    for child in &element.children {
        if container_queries_need_resolution(child, taffy)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) struct ContainerQueryResolveContext<'a, F>
where
    F: FnMut(&mut Element),
{
    pub(super) taffy: &'a TaffyTree<MeasureContext>,
    pub(super) prepare: &'a mut F,
    pub(super) animations: &'a mut HashMap<ElementId, DeclarativeAnimationPlayback>,
    pub(super) motion_ids: &'a mut HashSet<ElementId>,
    pub(super) time_animation_ids: &'a mut HashSet<ElementId>,
    pub(super) springs: &'a mut HashMap<ElementId, DeclarativeSpringPlayback>,
    pub(super) spring_ids: &'a mut HashSet<ElementId>,
    pub(super) request_frame: &'a mut bool,
    pub(super) deadline: &'a mut Option<Instant>,
    pub(super) now: Instant,
    pub(super) animation_epoch: Instant,
    pub(super) enabled: bool,
    pub(super) reduce_motion: bool,
    pub(super) sanitize_detached: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ContainerQueryResolution {
    pub(super) changed: bool,
    pub(super) replaced_existing_subtree: bool,
}

impl ContainerQueryResolution {
    pub(super) fn merge(&mut self, other: Self) {
        self.changed |= other.changed;
        self.replaced_existing_subtree |= other.replaced_existing_subtree;
    }
}

impl<F> ContainerQueryResolveContext<'_, F>
where
    F: FnMut(&mut Element),
{
    pub(super) fn resolve(
        &mut self,
        element: &mut Element,
    ) -> Result<ContainerQueryResolution, UiError> {
        if element.is_display_none() {
            return Ok(ContainerQueryResolution::default());
        }
        if matches!(&element.kind, ElementKind::ContainerQuery(_)) {
            let node = element
                .taffy_node
                .expect("container query nodes are assigned before callback resolution");
            let layout = self.taffy.layout(node)?;
            let assigned_size = Size::new(
                finite_layout_length(layout.size.width),
                finite_layout_length(layout.size.height),
            );
            let needs_resolution = match &element.kind {
                ElementKind::ContainerQuery(query) => {
                    query.resolved_size != Some(assigned_size) || element.children.len() != 1
                }
                _ => unreachable!(),
            };

            if needs_resolution {
                let replaced_existing_subtree = !element.children.is_empty();
                let mut removed_motion_ids = Vec::new();
                if let ElementKind::ContainerQuery(query) = &mut element.kind {
                    removed_motion_ids.append(&mut query.resolved_motion_ids);
                }
                for child in &mut element.children {
                    drain_container_query_motion_ids(child, &mut removed_motion_ids);
                }
                for id in removed_motion_ids {
                    self.motion_ids.remove(&id);
                    self.time_animation_ids.remove(&id);
                    self.spring_ids.remove(&id);
                }

                let mut child = match &element.kind {
                    ElementKind::ContainerQuery(query) => query.render(assigned_size),
                    _ => unreachable!(),
                };
                (self.prepare)(&mut child);
                if self.sanitize_detached {
                    sanitize_detached_element(&mut child, true);
                }
                let mut resolved_motion_ids = Vec::new();
                resolve_declarative_animations(
                    &mut child,
                    self.animations,
                    self.motion_ids,
                    self.time_animation_ids,
                    self.springs,
                    self.spring_ids,
                    self.request_frame,
                    self.deadline,
                    self.now,
                    self.animation_epoch,
                    self.enabled,
                    self.reduce_motion,
                    &mut resolved_motion_ids,
                )?;
                if self.sanitize_detached {
                    sanitize_detached_element(&mut child, false);
                }
                element.children.clear();
                element.children.push(child);
                if let ElementKind::ContainerQuery(query) = &mut element.kind {
                    query.resolved_size = Some(assigned_size);
                    query.resolved_motion_ids = resolved_motion_ids;
                    query.layout_pending = true;
                }
                return Ok(ContainerQueryResolution {
                    changed: true,
                    replaced_existing_subtree,
                });
            }
        }

        let mut resolution = ContainerQueryResolution::default();
        for child in &mut element.children {
            resolution.merge(self.resolve(child)?);
        }
        Ok(resolution)
    }
}

pub(super) fn drain_container_query_motion_ids(element: &mut Element, ids: &mut Vec<ElementId>) {
    if let ElementKind::ContainerQuery(query) = &mut element.kind {
        ids.append(&mut query.resolved_motion_ids);
    }
    for child in &mut element.children {
        drain_container_query_motion_ids(child, ids);
    }
}

pub(super) fn sanitize_detached_element(element: &mut Element, preserve_motion: bool) {
    element.explicit_id = None;
    element.runtime_id = ElementId::new(0);
    element.taffy_node = None;
    element.clickable = false;
    element.pointer_listener = false;
    element.scroll_wheel_listener = false;
    element.touch_listener = false;
    element.context_menu_listener = false;
    element.mouse_pressure_listener = false;
    element.pinch_listener = false;
    element.rotation_listener = false;
    element.smart_magnify_listener = false;
    element.drag_source = false;
    element.drop_target = false;
    element.drop_predicates.clear();
    element.cursor_style = None;
    element.cursor_style_explicit = false;
    element.hover.cursor_style = None;
    element.active.cursor_style = None;
    element.focus.cursor_style = None;
    element.disabled_style.cursor_style = None;
    element.invalid_style.cursor_style = None;
    element.selected_style.cursor_style = None;
    element.dragging.cursor_style = None;
    element.drag_over.cursor_style = None;
    element.focus_within.cursor_style = None;
    for entry in &mut element.group_styles {
        entry.style.cursor_style = None;
    }
    element.user_select = UserSelect::None;
    element.resolved_user_select = false;
    element.focusable = false;
    element.focus_on_pointer = true;
    element.focus_trap = false;
    element.restore_previous_focus = false;
    element.key_context = None;
    element.auto_focus = false;
    element.activation_target = None;
    element.plane = None;
    element.anchor = None;
    element.tooltip = None;
    element.app_region = None;
    element.virtual_scroll = None;
    element.scroll_to_end_revision = None;
    element.scroll_request = None;
    element.list_item_measurement = None;
    if !preserve_motion {
        element.animation = None;
        element.spring = None;
    }
    element.transition = None;
    element.blocks_pointer = false;
    element.dismiss_policy = DismissPolicy::default();
    element.restore_focus = None;
    for child in &mut element.children {
        sanitize_detached_element(child, preserve_motion);
    }
}

impl Default for UiTree {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct PointerResult {
    pub open_url: Option<Arc<str>>,
    pub repaint: bool,
    pub clicked: Option<ElementId>,
    pub dismissed: Option<DismissRequest>,
    pub pointer_listener: Option<ElementId>,
    pub drag_source: Option<ElementId>,
}

pub(super) fn text_input_max_scroll(
    content_size: Size,
    viewport: Size,
    line_height: f32,
    multiline: bool,
) -> Vector {
    Vector::new(
        (content_size.width - viewport.width).max(0.0),
        if multiline {
            (content_size.height.max(line_height) - viewport.height).max(0.0)
        } else {
            0.0
        },
    )
}

pub(super) fn scroll_to_reveal_caret(
    viewport: Size,
    caret: Point,
    line_height: f32,
    mut scroll: Vector,
    max_scroll: Vector,
) -> Vector {
    let horizontal_margin = 16.0_f32.min(viewport.width * 0.5);
    let left_edge = horizontal_margin;
    let right_edge = (viewport.width - horizontal_margin).max(left_edge);
    if caret.x - scroll.x > right_edge {
        scroll.x = caret.x - right_edge;
    } else if caret.x - scroll.x < left_edge {
        scroll.x = (caret.x - left_edge).max(0.0);
    }

    let bottom_edge = (viewport.height - 2.0).max(0.0);
    if caret.y + line_height - scroll.y > bottom_edge {
        scroll.y = caret.y + line_height - bottom_edge;
    } else if caret.y < scroll.y {
        scroll.y = caret.y;
    }
    Vector::new(
        scroll.x.clamp(0.0, max_scroll.x),
        scroll.y.clamp(0.0, max_scroll.y),
    )
}

pub(super) fn text_input_index_at(
    region: &TextInputRegion,
    point: Point,
    scale_factor: f32,
    renderer: &mut impl TextLayoutEngine,
) -> usize {
    if region.content.is_empty() {
        return 0;
    }
    let display_index = renderer.text_index_for_point_with_highlights(
        TextId::new(region.id.value()),
        &region.content,
        &region.style,
        region.highlights.as_ref(),
        region.bounds.width,
        scale_factor,
        Point::new(
            point.x - region.bounds.x + region.scroll.x,
            point.y - region.bounds.y + region.scroll.y,
        ),
    );
    region.password.as_ref().map_or(display_index, |password| {
        password.source_index(display_index)
    })
}

pub(super) fn squared_distance_to_rect(point: Point, rect: Rect) -> f32 {
    let dx = if point.x < rect.x {
        rect.x - point.x
    } else if point.x > rect.right() {
        point.x - rect.right()
    } else {
        0.0
    };
    let dy = if point.y < rect.y {
        rect.y - point.y
    } else if point.y > rect.bottom() {
        point.y - rect.bottom()
    } else {
        0.0
    };
    dx * dx + dy * dy
}

pub(super) fn nearest_selectable_text_region(
    regions: &[SelectableTextRegion],
    point: Point,
) -> Option<&SelectableTextRegion> {
    regions
        .iter()
        .filter_map(|region| {
            region
                .bounds
                .intersection(region.clip)
                .map(|visible| (region, visible))
        })
        .min_by(|(left, left_visible), (right, right_visible)| {
            let left_distance = squared_distance_to_rect(point, *left_visible);
            let right_distance = squared_distance_to_rect(point, *right_visible);
            left_distance
                .total_cmp(&right_distance)
                .then_with(|| right.order.cmp(&left.order))
        })
        .map(|(region, _)| region)
}

pub(super) fn selection_separator(previous: Option<Rect>, next: Option<Rect>) -> &'static str {
    let (Some(previous), Some(next)) = (previous, next) else {
        return "\n";
    };
    let overlap = previous.bottom().min(next.bottom()) - previous.y.max(next.y);
    if overlap > previous.height.min(next.height) * 0.4 {
        " "
    } else {
        "\n"
    }
}

pub(super) fn push_bounded_text(output: &mut String, value: &str, limit: usize) {
    let remaining = limit.saturating_sub(output.len());
    if remaining == 0 {
        return;
    }
    let mut end = value.len().min(remaining);
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    output.push_str(&value[..end]);
}

pub(super) fn measure_image(
    width: Option<f32>,
    height: Option<f32>,
    intrinsic: Size,
) -> TaffySize<f32> {
    match (width, height) {
        (Some(width), Some(height)) => TaffySize { width, height },
        (Some(width), None) => TaffySize {
            width,
            height: width * intrinsic.height / intrinsic.width,
        },
        (None, Some(height)) => TaffySize {
            width: height * intrinsic.width / intrinsic.height,
            height,
        },
        (None, None) => TaffySize {
            width: intrinsic.width,
            height: intrinsic.height,
        },
    }
}

pub(super) fn fit_path(bounds: Rect, path: &Path, fit: ObjectFit) -> Option<([f32; 2], Vector)> {
    let intrinsic = path.size();
    if bounds.is_empty() || intrinsic.is_empty() {
        return None;
    }
    let fitted = fit_image(bounds, intrinsic, fit);
    let source_width = intrinsic.width * fitted.source_uv.width;
    let source_height = intrinsic.height * fitted.source_uv.height;
    if source_width <= 0.0 || source_height <= 0.0 {
        return None;
    }
    let scale = [
        fitted.destination.width / source_width,
        fitted.destination.height / source_height,
    ];
    let source_bounds = path.bounds();
    let translation = Vector::new(
        fitted.destination.x - (source_bounds.x + intrinsic.width * fitted.source_uv.x) * scale[0],
        fitted.destination.y - (source_bounds.y + intrinsic.height * fitted.source_uv.y) * scale[1],
    );
    Some((scale, translation))
}

pub(super) fn build_pending_container_query_subtrees(
    element: &mut Element,
    taffy: &mut TaffyTree<MeasureContext>,
    layout_nodes: &mut LayoutNodeCache,
    seen_ids: &mut HashSet<ElementId>,
) -> Result<bool, UiError> {
    if element.is_display_none() {
        return Ok(false);
    }
    let layout_pending = matches!(
        &element.kind,
        ElementKind::ContainerQuery(query) if query.layout_pending
    );
    if layout_pending {
        debug_assert_eq!(element.children.len(), 1);
        let parent_id = element.runtime_id;
        let inherited_typography = element.resolved_typography.clone();
        let inherited_user_select = element.resolved_user_select;
        let inherited_direction = element.resolved_direction;
        let child = element
            .children
            .first_mut()
            .expect("a pending query layout has callback contents");
        match collect_explicit_ids(child, seen_ids) {
            Ok(()) => {}
            Err(UiError::DuplicateId(_)) => return Ok(true),
            Err(error) => return Err(error),
        }
        build_retained_layout_node(
            taffy,
            layout_nodes,
            seen_ids,
            child,
            parent_id,
            0,
            &inherited_typography,
            inherited_user_select,
            inherited_direction,
        )?;
        if let ElementKind::ContainerQuery(query) = &mut element.kind {
            query.layout_pending = false;
        }
        return Ok(false);
    }

    for child in &mut element.children {
        if build_pending_container_query_subtrees(child, taffy, layout_nodes, seen_ids)? {
            return Ok(true);
        }
    }
    Ok(false)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_layout_node(
    taffy: &mut TaffyTree<MeasureContext>,
    seen_ids: &mut HashSet<ElementId>,
    element: &mut Element,
    parent_id: ElementId,
    child_index: usize,
    inherited_typography: &TextStyle,
    inherited_user_select: bool,
    inherited_direction: Direction,
) -> Result<NodeId, UiError> {
    let mut layout_nodes = LayoutNodeCache::default();
    let node = build_retained_layout_node(
        taffy,
        &mut layout_nodes,
        seen_ids,
        element,
        parent_id,
        child_index,
        inherited_typography,
        inherited_user_select,
        inherited_direction,
    )?;
    layout_nodes.commit_children(taffy)?;
    Ok(node)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_retained_layout_node(
    taffy: &mut TaffyTree<MeasureContext>,
    layout_nodes: &mut LayoutNodeCache,
    seen_ids: &mut HashSet<ElementId>,
    element: &mut Element,
    parent_id: ElementId,
    child_index: usize,
    inherited_typography: &TextStyle,
    inherited_user_select: bool,
    inherited_direction: Direction,
) -> Result<NodeId, UiError> {
    let id = if let Some(id) = element.explicit_id {
        id
    } else {
        let mut generated = ElementId::new(mix_id(parent_id.value(), child_index as u64));
        let mut collision_index = 0_u64;
        while !seen_ids.insert(generated) {
            collision_index = collision_index.wrapping_add(1);
            generated = ElementId::new(mix_id(generated.value(), collision_index));
        }
        generated
    };
    element.runtime_id = id;
    layout_nodes.positions.insert(id, (parent_id, child_index));
    element.resolved_direction = element.direction.unwrap_or(inherited_direction);
    apply_logical_insets(element);
    element.resolved_typography = element.typography.resolve(inherited_typography);
    // Editable controls must retain a one-to-one mapping between their controlled value and the
    // shaped buffer. Their own viewport already provides web-style clipping and caret scrolling;
    // inherited display-only truncation would otherwise hide editable bytes from IME and hit
    // testing.
    if matches!(&element.kind, ElementKind::TextInput(_)) {
        element.resolved_typography.text_overflow = None;
        element.resolved_typography.line_clamp = None;
    }
    let automatic_user_select = inherited_user_select
        && !element.clickable
        && !element.pointer_listener
        && !element.drag_source
        && !matches!(
            element.accessibility.role,
            AccessibilityRole::Button
                | AccessibilityRole::CheckBox
                | AccessibilityRole::MenuItem
                | AccessibilityRole::MenuItemCheckBox
                | AccessibilityRole::MenuItemRadio
        )
        && !matches!(&element.kind, ElementKind::TextInput(_));
    element.resolved_user_select = match element.user_select {
        UserSelect::Auto => automatic_user_select,
        UserSelect::Text => true,
        UserSelect::None => false,
    };

    let mut child_nodes = Vec::with_capacity(element.children.len());
    for (index, child) in element.children.iter_mut().enumerate() {
        child_nodes.push(build_retained_layout_node(
            taffy,
            layout_nodes,
            seen_ids,
            child,
            id,
            index,
            &element.resolved_typography,
            element.resolved_user_select,
            element.resolved_direction,
        )?);
    }

    // Children have inherited the logical alignment and shaping direction; resolve this
    // element's own copy against its direction now so nothing downstream of layout ever sees an
    // unresolved logical value in a retained shaping key.
    element.resolved_typography.align = resolve_logical_align(
        element.resolved_typography.align,
        element.resolved_direction,
    );
    if element.resolved_typography.direction == TextDirection::Auto
        && element.resolved_direction.is_rtl()
    {
        element.resolved_typography.direction = TextDirection::Rtl;
    }
    // Editable controls never case-map their content: the shaped buffer must stay byte-identical
    // to the controlled value so caret indices, IME state, and clipboard round-trips agree.
    if matches!(&element.kind, ElementKind::TextInput(_)) {
        element.resolved_typography.transform = None;
    }

    let context = match &element.kind {
        ElementKind::Container => None,
        // Query contents are deliberately disconnected from this leaf. Their layout is computed
        // later as a separate root using this node's assigned size, so callback contents cannot
        // feed intrinsic size back into the query box.
        ElementKind::ContainerQuery(_) => None,
        ElementKind::Text(content) => Some(MeasureContext::Text {
            id: TextId::new(id.value()),
            content: content.clone(),
            style: Box::new(element.resolved_typography.clone()),
            highlights: None,
        }),
        ElementKind::StyledText(styled) => Some(MeasureContext::Text {
            id: TextId::new(id.value()),
            content: styled.content().clone(),
            style: Box::new(element.resolved_typography.clone()),
            highlights: Some(styled.shared_highlights().clone()),
        }),
        ElementKind::Image(image) => match &image.resolved {
            ImageResolution::Ready(source) => Some(MeasureContext::Image {
                intrinsic: source.size(),
            }),
            ImageResolution::Animated(animation) => Some(MeasureContext::Image {
                intrinsic: animation.size(),
            }),
            ImageResolution::Loading | ImageResolution::Failed => None,
        },
        ElementKind::Svg(svg) => Some(MeasureContext::Image {
            intrinsic: svg.svg.size(),
        }),
        ElementKind::Path(path) => {
            let intrinsic = path.path.size();
            if intrinsic.is_empty() {
                None
            } else {
                Some(MeasureContext::Image { intrinsic })
            }
        }
        ElementKind::Canvas(_) | ElementKind::CustomShader(_) => None,
        ElementKind::TextInput(input) => {
            let content = if input.value.is_empty() {
                input.placeholder.clone()
            } else if input.password {
                PasswordDisplay::new(&input.value).content
            } else {
                input.value.clone()
            };
            let highlights =
                (!input.password && !input.value.is_empty() && !input.highlights.is_empty())
                    .then(|| input.highlights.clone());
            Some(MeasureContext::Text {
                id: TextId::new(id.value()),
                content,
                style: Box::new(element.resolved_typography.clone()),
                highlights,
            })
        }
        #[cfg(target_os = "macos")]
        ElementKind::NativeView(_) => None,
    };
    let attaches_children = matches!(&element.kind, ElementKind::Container)
        || matches!(&element.kind, ElementKind::Image(image)
            if matches!(&image.resolved, ImageResolution::Loading | ImageResolution::Failed));
    if !attaches_children {
        child_nodes.clear();
    }
    let node = layout_nodes.reconcile(taffy, id, &element.layout, context, child_nodes)?;
    if let ElementKind::ContainerQuery(query) = &mut element.kind {
        query.layout_pending = false;
    }
    element.taffy_node = Some(node);
    Ok(node)
}

/// Maximum scroll-snap containers indexed for one rendered window.
///
/// Reaching the bound drops later containers from snapping; ordinary scrolling is unaffected.
pub const MAX_SCROLL_SNAP_CONTAINERS_PER_WINDOW: usize = 256;

/// Maximum scroll-snap children indexed across every container in one rendered window.
pub const MAX_SCROLL_SNAP_POINTS_PER_WINDOW: usize = 4_096;

/// Maximum sticky elements retained in one window's view declaration.
pub const MAX_STICKY_ELEMENTS_PER_WINDOW: usize = 4_096;

/// Distance within which a `Proximity` container still snaps, as a fraction of its viewport.
const SCROLL_SNAP_PROXIMITY_FRACTION: f32 = 0.5;

/// Upper bound on the proximity window, in logical pixels.
const SCROLL_SNAP_PROXIMITY_LIMIT: f32 = 200.0;

/// One scroll container that snaps, as of the last completed geometry pass.
#[derive(Clone, Copy, Debug)]
pub(super) struct ScrollSnapContainer {
    pub(super) id: ElementId,
    pub(super) style: ScrollSnapStyle,
    /// Painted padding box of the container.
    pub(super) viewport: Rect,
    /// Scroll offset the painted geometry was collected at.
    pub(super) offset: Vector,
    /// Maximum scroll offset on each axis.
    pub(super) max_offset: Vector,
    pub(super) direction: Direction,
}

/// One snap position contributed by a scroll-snap child.
#[derive(Clone, Copy, Debug)]
pub(super) struct ScrollSnapPoint {
    pub(super) container: ElementId,
    pub(super) align: SnapAlign,
    pub(super) stop_always: bool,
    /// Painted bounds of the child.
    pub(super) bounds: Rect,
}

/// Bounded per-window scroll-snap geometry, rebuilt in place by each geometry pass.
///
/// Both vectors are cleared and refilled rather than reallocated, so a settled window performs no
/// allocation at all and a scrolling one stays inside the exported bounds.
#[derive(Debug, Default)]
pub(super) struct ScrollSnapGeometry {
    pub(super) containers: Vec<ScrollSnapContainer>,
    pub(super) points: Vec<ScrollSnapPoint>,
}

impl ScrollSnapGeometry {
    pub(super) fn clear(&mut self) {
        self.containers.clear();
        self.points.clear();
    }

    pub(super) fn is_empty(&self) -> bool {
        self.containers.is_empty()
    }

    pub(super) fn push_container(
        &mut self,
        id: ElementId,
        style: ScrollSnapStyle,
        viewport: Rect,
        offset: Vector,
        max_offset: Vector,
        direction: Direction,
    ) {
        if self.containers.len() >= MAX_SCROLL_SNAP_CONTAINERS_PER_WINDOW {
            return;
        }
        self.containers.push(ScrollSnapContainer {
            id,
            style,
            viewport,
            offset,
            max_offset,
            direction,
        });
    }

    pub(super) fn push_point(
        &mut self,
        container: ElementId,
        align: SnapAlign,
        stop_always: bool,
        bounds: Rect,
    ) {
        if self.points.len() >= MAX_SCROLL_SNAP_POINTS_PER_WINDOW {
            return;
        }
        self.points.push(ScrollSnapPoint {
            container,
            align,
            stop_always,
            bounds,
        });
    }

    pub(super) fn container(&self, id: ElementId) -> Option<&ScrollSnapContainer> {
        self.containers.iter().find(|container| container.id == id)
    }

    /// Resolve the offset one container should rest at.
    ///
    /// `current` is the container's live offset, which may already have moved past the offset the
    /// retained geometry was collected at; the geometry is only used to recover each child's
    /// position along the inline axis, so no repaint is required before resolving.
    /// `gesture_start` is the offset the gesture began from; a child declaring
    /// `snap_stop_always` between that offset and the settled one takes priority so a fling can
    /// never skip past it. Returns `None` when nothing should move.
    pub(super) fn resolve(
        &self,
        id: ElementId,
        gesture_start: Vector,
        current: Vector,
    ) -> Option<Vector> {
        let container = self.container(id)?;
        let resolved = Vector::new(
            self.resolve_axis(container, SnapAxis::X, gesture_start.x, current.x),
            self.resolve_axis(container, SnapAxis::Y, gesture_start.y, current.y),
        );
        (resolved != current).then_some(resolved)
    }

    fn resolve_axis(
        &self,
        container: &ScrollSnapContainer,
        axis: SnapAxis,
        start: f32,
        current: f32,
    ) -> f32 {
        let Some(strictness) = axis.strictness(container.style) else {
            return current;
        };
        let max_offset = axis.component(container.max_offset);
        if max_offset <= 0.0 {
            return current;
        }
        let viewport_len = axis.length(container.viewport);

        let mut nearest: Option<(f32, f32)> = None;
        let mut blocking: Option<(f32, f32)> = None;
        for point in self
            .points
            .iter()
            .filter(|point| point.container == container.id)
        {
            let target = axis
                .snap_target(container, point, viewport_len)
                .clamp(0.0, max_offset);
            let distance = (target - current).abs();
            if nearest.is_none_or(|(_, best)| distance < best) {
                nearest = Some((target, distance));
            }
            // A `snap_stop_always` child between the gesture's start and where it settled must
            // capture the gesture instead of being flown past.
            if point.stop_always
                && ((start < target && target < current) || (current < target && target < start))
                && blocking.is_none_or(|(_, best)| distance < best)
            {
                blocking = Some((target, distance));
            }
        }
        if let Some((target, _)) = blocking {
            return target;
        }
        let Some((target, distance)) = nearest else {
            return current;
        };
        match strictness {
            SnapStrictness::Mandatory => target,
            SnapStrictness::Proximity => {
                let window = (viewport_len * SCROLL_SNAP_PROXIMITY_FRACTION)
                    .min(SCROLL_SNAP_PROXIMITY_LIMIT);
                if distance <= window { target } else { current }
            }
        }
    }
}

/// One axis of scroll-snap resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SnapAxis {
    X,
    Y,
}

impl SnapAxis {
    fn component(self, value: Vector) -> f32 {
        match self {
            Self::X => value.x,
            Self::Y => value.y,
        }
    }

    fn length(self, rect: Rect) -> f32 {
        match self {
            Self::X => rect.width,
            Self::Y => rect.height,
        }
    }

    fn strictness(self, style: ScrollSnapStyle) -> Option<SnapStrictness> {
        match self {
            Self::X => style.x,
            Self::Y => style.y,
        }
    }

    /// The offset at which `point` would sit at its declared alignment.
    ///
    /// Painted bounds already include the container's current translation, so the child's
    /// inline-start distance is recovered by adding the offset the geometry was collected at.
    fn snap_target(
        self,
        container: &ScrollSnapContainer,
        point: &ScrollSnapPoint,
        viewport_len: f32,
    ) -> f32 {
        let viewport = container.viewport;
        let (position, length) = match self {
            Self::X if container.direction.is_rtl() => (
                viewport.right() - point.bounds.right() + container.offset.x,
                point.bounds.width,
            ),
            Self::X => (
                point.bounds.x - viewport.x + container.offset.x,
                point.bounds.width,
            ),
            Self::Y => (
                point.bounds.y - viewport.y + container.offset.y,
                point.bounds.height,
            ),
        };
        match point.align {
            SnapAlign::Start => position,
            SnapAlign::Center => position - (viewport_len - length) * 0.5,
            SnapAlign::End => position - (viewport_len - length),
        }
    }
}

/// Reject view declarations that retain more sticky elements than one window may track.
pub(super) fn validate_sticky_limits(root: &Element) -> Result<(), UiError> {
    fn visit(element: &Element, count: &mut usize) -> Result<(), UiError> {
        if element.sticky.is_some() {
            *count += 1;
            if *count > MAX_STICKY_ELEMENTS_PER_WINDOW {
                return Err(UiError::TooManyStickyElements);
            }
        }
        for child in &element.children {
            visit(child, count)?;
        }
        Ok(())
    }

    visit(root, &mut 0)
}

/// Resolve a direction-relative [`TextAlign`] into a physical edge.
pub(super) fn resolve_logical_align(align: TextAlign, direction: Direction) -> TextAlign {
    match (align, direction) {
        (TextAlign::Start, Direction::Ltr) | (TextAlign::End, Direction::Rtl) => TextAlign::Left,
        (TextAlign::Start, Direction::Rtl) | (TextAlign::End, Direction::Ltr) => TextAlign::Right,
        (align, _) => align,
    }
}

/// Fold direction-relative padding, margin, and border declarations into physical Taffy edges.
///
/// This runs once per element per layout build, before its Taffy node is created, so the layout
/// engine only ever sees resolved physical values and no per-frame work is added.
pub(super) fn apply_logical_insets(element: &mut Element) {
    let Some(logical) = element.logical_insets.as_deref().copied() else {
        return;
    };
    let rtl = element.resolved_direction.is_rtl();
    let (padding_left, padding_right) = if rtl {
        (logical.padding_end, logical.padding_start)
    } else {
        (logical.padding_start, logical.padding_end)
    };
    let (border_left, border_right) = if rtl {
        (logical.border_end, logical.border_start)
    } else {
        (logical.border_start, logical.border_end)
    };
    if let Some(value) = padding_left {
        element.layout.padding.left = taffy::style::LengthPercentage::length(value);
    }
    if let Some(value) = padding_right {
        element.layout.padding.right = taffy::style::LengthPercentage::length(value);
    }
    if let Some(value) = border_left {
        element.layout.border.left = taffy::style::LengthPercentage::length(value);
    }
    if let Some(value) = border_right {
        element.layout.border.right = taffy::style::LengthPercentage::length(value);
    }
    // In-flow horizontal positions are mirrored inside the parent's content box after layout, so
    // an inline-start margin is always the Taffy `left` margin regardless of direction.
    if let Some(value) = logical.margin_start {
        element.layout.margin.left = taffy::style::LengthPercentageAuto::length(value);
    }
    if let Some(value) = logical.margin_end {
        element.layout.margin.right = taffy::style::LengthPercentageAuto::length(value);
    }
}

/// The coordinate frame one element's children are placed in.
///
/// A frame carries everything the three geometry walks (bounds collection, hit-region
/// collection, and paint) need to turn a Taffy-relative child location into painted geometry:
/// the translated origin, whether horizontal positions mirror, the containing block a sticky
/// child may not leave, and the viewport of the nearest ancestor scroll container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct LayoutFrame {
    /// Horizontal origin term. See [`frame_rect`] for how `mirror` changes its meaning.
    pub origin_x: f32,
    pub origin_y: f32,
    /// Whether children are placed right to left inside this frame.
    pub mirror: bool,
    /// Painted content box of the parent; a sticky child never leaves it.
    pub containing_block: Rect,
    /// Painted viewport of the nearest ancestor scroll container, or the window viewport.
    pub scroll_viewport: Rect,
    /// Accumulated window-space transform of the enclosing compositing groups.
    ///
    /// Layout is never transformed; this only maps a painted point back into the coordinate
    /// system that `origin_x`/`origin_y` and every recorded bound are expressed in, so pointer
    /// input can be inverse-mapped through a rotated or scaled subtree.
    pub transform: Transform2D,
}

impl LayoutFrame {
    /// The root frame: no mirroring, the window viewport as both containing block and viewport.
    pub(super) fn root(origin: Point, viewport: Rect) -> Self {
        Self {
            origin_x: origin.x,
            origin_y: origin.y,
            mirror: false,
            containing_block: viewport,
            scroll_viewport: viewport,
            transform: Transform2D::IDENTITY,
        }
    }
}

/// Place one child's Taffy-relative layout box inside its parent frame.
///
/// In a left-to-right frame `origin_x` is the painted position of the parent's border-box left
/// edge (already translated by scrolling) and the child is placed at `origin_x + location.x`. In
/// a right-to-left frame `origin_x` is precomputed by [`child_frame`] so that the same box lands
/// mirrored inside the parent's content box, which is why the child's own width is subtracted
/// instead of added.
pub(super) fn frame_rect(frame: LayoutFrame, layout: &taffy::tree::Layout) -> Rect {
    let x = if frame.mirror {
        frame.origin_x - layout.location.x - layout.size.width
    } else {
        frame.origin_x + layout.location.x
    };
    Rect::new(
        x,
        frame.origin_y + layout.location.y,
        layout.size.width,
        layout.size.height,
    )
}

/// Apply CSS-style sticky offsets to an already-placed element box.
///
/// Sticking is a painted-geometry adjustment only: the element keeps the space it occupies in
/// flow, so scrolling never invalidates layout. The shift is clamped so the element can never
/// leave its containing block, which is what releases a pinned header at the end of its section.
pub(super) fn apply_sticky(element: &Element, natural: Rect, frame: LayoutFrame) -> Rect {
    let Some(insets) = element.sticky else {
        return natural;
    };
    if insets.is_empty() {
        return natural;
    }
    let viewport = frame.scroll_viewport;
    let block = frame.containing_block;
    let mut shift_x = 0.0_f32;
    if let Some(left) = insets.left {
        shift_x = shift_x.max(viewport.x + left - natural.x);
    }
    if let Some(right) = insets.right {
        shift_x = shift_x.min(viewport.right() - right - natural.right());
    }
    if shift_x > 0.0 {
        shift_x = shift_x.min((block.right() - natural.right()).max(0.0));
    } else if shift_x < 0.0 {
        shift_x = shift_x.max((block.x - natural.x).min(0.0));
    }

    let mut shift_y = 0.0_f32;
    if let Some(top) = insets.top {
        shift_y = shift_y.max(viewport.y + top - natural.y);
    }
    if let Some(bottom) = insets.bottom {
        shift_y = shift_y.min(viewport.bottom() - bottom - natural.bottom());
    }
    if shift_y > 0.0 {
        shift_y = shift_y.min((block.bottom() - natural.bottom()).max(0.0));
    } else if shift_y < 0.0 {
        shift_y = shift_y.max((block.y - natural.y).min(0.0));
    }

    Rect::new(
        natural.x + shift_x,
        natural.y + shift_y,
        natural.width,
        natural.height,
    )
}

/// Place one child inside its parent frame, including any sticky offset.
pub(super) fn positioned_rect(
    element: &Element,
    frame: LayoutFrame,
    layout: &taffy::tree::Layout,
) -> Rect {
    apply_sticky(element, frame_rect(frame, layout), frame)
}

/// The painted padding box of an element, given its border-box bounds and Taffy layout.
pub(super) fn padding_box(bounds: Rect, layout: &taffy::tree::Layout) -> Rect {
    Rect::new(
        bounds.x + layout.border.left,
        bounds.y + layout.border.top,
        (bounds.width - layout.border.left - layout.border.right).max(0.0),
        (bounds.height - layout.border.top - layout.border.bottom).max(0.0),
    )
}

/// The painted content box of an element, given its border-box bounds and Taffy layout.
pub(super) fn content_box(bounds: Rect, layout: &taffy::tree::Layout) -> Rect {
    let left = layout.border.left + layout.padding.left;
    let right = layout.border.right + layout.padding.right;
    let top = layout.border.top + layout.padding.top;
    let bottom = layout.border.bottom + layout.padding.bottom;
    Rect::new(
        bounds.x + left,
        bounds.y + top,
        (bounds.width - left - right).max(0.0),
        (bounds.height - top - bottom).max(0.0),
    )
}

/// Build the frame this element's children are placed in.
///
/// `scroll` is the already-clamped scroll translation for a scrolling container, expressed as a
/// distance from the container's inline start edge. `scrolls` says whether this element is the
/// scroll container that owns that offset, which makes its padding box the sticky viewport for
/// everything below it.
pub(super) fn child_frame(
    element: &Element,
    layout: &taffy::tree::Layout,
    bounds: Rect,
    scroll: Vector,
    scrolls: bool,
    parent: LayoutFrame,
) -> LayoutFrame {
    let content = content_box(bounds, layout);
    let origin_x = if element.resolved_direction.is_rtl() {
        // Mirror inside the content box: a child at Taffy offset `location.x` with width `w`
        // lands at `content.left + content.width - (location.x - content_inset) - w`. Folding the
        // constant part into the origin keeps `frame_rect` branch-light. Scrolling an RTL
        // container moves content to the right as the offset grows, because offset zero already
        // rests against the inline start edge on the right.
        let content_inset = content.x - bounds.x;
        bounds.x + 2.0 * content_inset + content.width + scroll.x
    } else {
        bounds.x - scroll.x
    };
    LayoutFrame {
        origin_x,
        origin_y: bounds.y - scroll.y,
        mirror: element.resolved_direction.is_rtl(),
        containing_block: content,
        scroll_viewport: if scrolls {
            padding_box(bounds, layout)
        } else {
            parent.scroll_viewport
        },
        transform: parent.transform,
    }
}

pub(super) fn apply_scroll_end_revision(
    id: ElementId,
    revision: Option<u64>,
    max_y: f32,
    offset: &mut Vector,
    states: &mut HashMap<ElementId, ScrollEndState>,
) {
    let Some(revision) = revision else {
        states.remove(&id);
        return;
    };
    let should_follow = states.get(&id).is_none_or(|state| {
        let declaration_changed =
            state.revision != revision || (state.previous_max_y - max_y).abs() > f32::EPSILON;
        declaration_changed && offset.y >= state.previous_max_y - 1.0
    });
    if should_follow {
        offset.y = max_y;
    }
    states.insert(
        id,
        ScrollEndState {
            revision,
            previous_max_y: max_y,
        },
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn collect_layout_bounds(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
    scroll_offsets: &mut HashMap<ElementId, Vector>,
    scroll_end_states: &mut HashMap<ElementId, ScrollEndState>,
    bounds: &mut HashMap<ElementId, Rect>,
    snap: &mut ScrollSnapGeometry,
    snap_container: Option<ElementId>,
    parent_frame: LayoutFrame,
) -> Result<(), UiError> {
    if element.is_display_none() {
        return Ok(());
    }
    let node = element
        .taffy_node
        .expect("layout nodes are assigned before bounds collection");
    let layout = taffy.layout(node)?;
    let element_bounds = positioned_rect(element, parent_frame, layout);
    bounds.insert(element.runtime_id, element_bounds);

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
        if let Some(request) = element.scroll_request {
            if scroll_end_states
                .get(&element.runtime_id)
                .is_none_or(|state| state.revision != request.revision)
            {
                let origin = request
                    .child
                    .and_then(|index| element.children.get(index))
                    .and_then(|child| child.taffy_node)
                    .and_then(|node| taffy.layout(node).ok())
                    .map_or(Vector::ZERO, |layout| {
                        Vector::new(layout.location.x, layout.location.y)
                    });
                *offset = origin + request.offset;
                scroll_end_states.insert(
                    element.runtime_id,
                    ScrollEndState {
                        revision: request.revision,
                        previous_max_y: max_offset.y,
                    },
                );
            }
        } else {
            apply_scroll_end_revision(
                element.runtime_id,
                element.scroll_to_end_revision,
                max_offset.y,
                offset,
                scroll_end_states,
            );
        }
        offset.x = offset.x.clamp(0.0, max_offset.x);
        offset.y = offset.y.clamp(0.0, max_offset.y);
        scroll = *offset;
    } else if let Some(virtual_scroll) = &element.virtual_scroll {
        let max_offset = virtual_scroll
            .handle
            .max_offset(virtual_scroll.max_offset_y)
            .max(0.0);
        let offset = scroll_offsets.entry(element.runtime_id).or_default();
        offset.x = 0.0;
        offset.y = virtual_scroll.handle.offset().clamp(0.0, max_offset);
        scroll.y =
            virtual_scroll.handle.presented_offset(offset.y) - virtual_scroll.mount.layout_offset_y;
        scroll_end_states.remove(&element.runtime_id);
    } else {
        scroll_end_states.remove(&element.runtime_id);
    }

    if let Some(container) = snap_container
        && let Some(align) = element.snap_align
    {
        snap.push_point(container, align, element.snap_stop_always, element_bounds);
    }
    let child_frame = child_frame(
        element,
        layout,
        element_bounds,
        scroll,
        is_scrollable,
        parent_frame,
    );
    let child_snap_container = if is_scrollable && element.scroll_snap.is_some() {
        snap.push_container(
            element.runtime_id,
            element.scroll_snap.unwrap_or_default(),
            child_frame.scroll_viewport,
            scroll,
            Vector::new(
                (layout.content_size.width - layout.size.width).max(0.0),
                (layout.content_size.height - layout.size.height).max(0.0),
            ),
            element.resolved_direction,
        );
        Some(element.runtime_id)
    } else {
        snap_container
    };
    for child in &element.children {
        collect_layout_bounds(
            child,
            taffy,
            scroll_offsets,
            scroll_end_states,
            bounds,
            snap,
            child_snap_container,
            child_frame,
        )?;
    }
    Ok(())
}

/// Commit variable-list viewport and row measurements from a completed layout.
///
/// This deliberately runs before paint. Measuring while recursively painting a row is too late:
/// its parent has already resolved the retained scroll translation, which can present a stale
/// bottom anchor for one frame when wrapped content changes height during resize.
pub(super) fn report_variable_list_layout_measurements(
    element: &Element,
    taffy: &TaffyTree<MeasureContext>,
) -> Result<bool, UiError> {
    if element.is_display_none() || element.is_visibility_hidden() {
        return Ok(false);
    }
    let node = element
        .taffy_node
        .expect("layout nodes are assigned before list measurement");
    let layout = taffy.layout(node)?;
    let mut changed = false;
    if let Some(virtual_scroll) = &element.virtual_scroll {
        changed |= virtual_scroll
            .handle
            .report_viewport(Size::new(layout.size.width, layout.size.height));
    }
    if let Some(measurement) = &element.list_item_measurement {
        changed |= measurement.report_height(layout.size.height);
    }
    for child in &element.children {
        changed |= report_variable_list_layout_measurements(child, taffy)?;
    }
    Ok(changed)
}

pub(super) fn place_anchored(
    anchor: Rect,
    size: Size,
    viewport: Rect,
    placement: AnchorPlacement,
    gap: f32,
    margin: f32,
) -> Rect {
    resolve_anchored(
        anchor,
        size,
        viewport,
        AnchorGeometry {
            rounding_scale: None,
            placement,
            gap,
            align_offset: 0.0,
            margin,
            flip: true,
            sticky: true,
        },
    )
    .bounds
}

/// The declared geometry one anchored surface is placed with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct AnchorGeometry {
    pub rounding_scale: Option<f32>,
    pub placement: AnchorPlacement,
    pub gap: f32,
    pub align_offset: f32,
    pub margin: f32,
    pub flip: bool,
    pub sticky: bool,
}

impl AnchorGeometry {
    pub(super) fn of(anchor: &AnchorStyle) -> Self {
        Self {
            rounding_scale: anchor.rounding_scale,
            placement: anchor.placement,
            gap: anchor.gap,
            align_offset: anchor.align_offset,
            margin: anchor.viewport_margin,
            flip: anchor.flip,
            sticky: anchor.sticky,
        }
    }
}

/// Place an anchored surface and report which side and alignment the placement actually used.
///
/// [`place_anchored`] keeps the rectangle-only signature its existing callers use; this is the
/// same computation with the resolved placement and remaining room retained, so an application
/// that binds [`crate::AnchorPlacementHandle`] never has to re-derive a flip QuickGUI already made.
pub(super) fn resolve_anchored(
    anchor: Rect,
    size: Size,
    viewport: Rect,
    geometry: AnchorGeometry,
) -> ResolvedAnchorPlacement {
    let AnchorGeometry {
        rounding_scale,
        placement,
        gap,
        align_offset,
        margin,
        flip,
        sticky,
    } = geometry;
    let align_offset = if align_offset.is_finite() {
        align_offset
    } else {
        0.0
    };
    let (preferred_side, preferred_align) = anchor_placement_parts(placement);
    let inner = viewport.inset(Insets::all(margin.max(0.0)));
    let gap = gap.max(0.0);
    let preferred_space = available_anchor_space(anchor, inner, preferred_side, gap);
    let opposite_side = opposite_anchor_side(preferred_side);
    let opposite_space = available_anchor_space(anchor, inner, opposite_side, gap);
    let primary_size = match preferred_side {
        AnchorSide::Top | AnchorSide::Bottom => size.height,
        AnchorSide::Left | AnchorSide::Right => size.width,
    };
    let side = if flip && primary_size > preferred_space && opposite_space > preferred_space {
        opposite_side
    } else {
        preferred_side
    };

    let alignments = match preferred_align {
        AnchorAlign::Start => [AnchorAlign::Start, AnchorAlign::End, AnchorAlign::Center],
        AnchorAlign::Center => [AnchorAlign::Center, AnchorAlign::Start, AnchorAlign::End],
        AnchorAlign::End => [AnchorAlign::End, AnchorAlign::Start, AnchorAlign::Center],
    };
    let align = if !flip {
        preferred_align
    } else {
        alignments
            .into_iter()
            .min_by(|left, right| {
                let left = anchored_origin(anchor, size, side, *left, gap);
                let right = anchored_origin(anchor, size, side, *right, gap);
                cross_axis_overflow(left, size, inner, side)
                    .total_cmp(&cross_axis_overflow(right, size, inner, side))
            })
            .unwrap_or(preferred_align)
    };
    let mut origin = anchored_origin(anchor, size, side, align, gap);
    // The cross-axis offset is declared relative to the anchor, so it is applied before the
    // surface is clamped: an offset can shift a popup along its trigger but never off screen.
    if side.is_vertical() {
        origin.x += align_offset;
    } else {
        origin.y += align_offset;
    }
    if sticky {
        origin.x = clamp_surface_axis(origin.x, size.width, inner.x, inner.right());
        origin.y = clamp_surface_axis(origin.y, size.height, inner.y, inner.bottom());
    }
    if let Some(scale) = rounding_scale {
        let snap = |v: f32| ((v * scale).abs() - 0.5).ceil().copysign(v) / scale;
        let base = Point::new(snap(anchor.x), snap(anchor.y));
        origin.x = base.x + (origin.x - base.x).round();
        origin.y = base.y + (origin.y - base.y).round();
        if sticky {
            origin.x = clamp_surface_axis(origin.x, size.width, viewport.x, viewport.right());
            origin.y = clamp_surface_axis(origin.y, size.height, viewport.y, viewport.bottom());
        }
    }
    let room = available_anchor_space(anchor, inner, side, gap);
    let available = match side {
        AnchorSide::Top | AnchorSide::Bottom => Size::new(inner.width.max(0.0), room),
        AnchorSide::Left | AnchorSide::Right => Size::new(room, inner.height.max(0.0)),
    };
    ResolvedAnchorPlacement {
        placement: crate::anchor_placement(side, align),
        anchor,
        bounds: Rect::new(origin.x, origin.y, size.width, size.height),
        available,
        anchor_hidden: viewport.intersection(anchor).is_none(),
    }
}

pub(super) fn anchor_placement_parts(placement: AnchorPlacement) -> (AnchorSide, AnchorAlign) {
    match placement {
        AnchorPlacement::TopStart => (AnchorSide::Top, AnchorAlign::Start),
        AnchorPlacement::Top => (AnchorSide::Top, AnchorAlign::Center),
        AnchorPlacement::TopEnd => (AnchorSide::Top, AnchorAlign::End),
        AnchorPlacement::BottomStart => (AnchorSide::Bottom, AnchorAlign::Start),
        AnchorPlacement::Bottom => (AnchorSide::Bottom, AnchorAlign::Center),
        AnchorPlacement::BottomEnd => (AnchorSide::Bottom, AnchorAlign::End),
        AnchorPlacement::LeftStart => (AnchorSide::Left, AnchorAlign::Start),
        AnchorPlacement::Left => (AnchorSide::Left, AnchorAlign::Center),
        AnchorPlacement::LeftEnd => (AnchorSide::Left, AnchorAlign::End),
        AnchorPlacement::RightStart => (AnchorSide::Right, AnchorAlign::Start),
        AnchorPlacement::Right => (AnchorSide::Right, AnchorAlign::Center),
        AnchorPlacement::RightEnd => (AnchorSide::Right, AnchorAlign::End),
    }
}

pub(super) fn opposite_anchor_side(side: AnchorSide) -> AnchorSide {
    side.opposite()
}

pub(super) fn available_anchor_space(
    anchor: Rect,
    viewport: Rect,
    side: AnchorSide,
    gap: f32,
) -> f32 {
    match side {
        AnchorSide::Top => anchor.y - viewport.y - gap,
        AnchorSide::Bottom => viewport.bottom() - anchor.bottom() - gap,
        AnchorSide::Left => anchor.x - viewport.x - gap,
        AnchorSide::Right => viewport.right() - anchor.right() - gap,
    }
    .max(0.0)
}

pub(super) fn anchored_origin(
    anchor: Rect,
    size: Size,
    side: AnchorSide,
    align: AnchorAlign,
    gap: f32,
) -> Point {
    let cross_x = match align {
        AnchorAlign::Start => anchor.x,
        AnchorAlign::Center => anchor.x + (anchor.width - size.width) * 0.5,
        AnchorAlign::End => anchor.right() - size.width,
    };
    let cross_y = match align {
        AnchorAlign::Start => anchor.y,
        AnchorAlign::Center => anchor.y + (anchor.height - size.height) * 0.5,
        AnchorAlign::End => anchor.bottom() - size.height,
    };
    match side {
        AnchorSide::Top => Point::new(cross_x, anchor.y - gap - size.height),
        AnchorSide::Bottom => Point::new(cross_x, anchor.bottom() + gap),
        AnchorSide::Left => Point::new(anchor.x - gap - size.width, cross_y),
        AnchorSide::Right => Point::new(anchor.right() + gap, cross_y),
    }
}

pub(super) fn cross_axis_overflow(
    origin: Point,
    size: Size,
    viewport: Rect,
    side: AnchorSide,
) -> f32 {
    match side {
        AnchorSide::Top | AnchorSide::Bottom => {
            (viewport.x - origin.x).max(0.0) + (origin.x + size.width - viewport.right()).max(0.0)
        }
        AnchorSide::Left | AnchorSide::Right => {
            (viewport.y - origin.y).max(0.0) + (origin.y + size.height - viewport.bottom()).max(0.0)
        }
    }
}

pub(super) fn clamp_surface_axis(origin: f32, size: f32, minimum: f32, maximum: f32) -> f32 {
    if size >= maximum - minimum {
        minimum
    } else {
        origin.clamp(minimum, maximum - size)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_element_shadows(
    scene: &mut Scene,
    layer: PaintLayerKey,
    bounds: Rect,
    radius: Corners,
    clip: Rect,
    shadows: &[BoxShadow],
    inset: bool,
) {
    // CSS paints the first declared shadow on top, so display-list insertion is reversed.
    for shadow in shadows
        .iter()
        .rev()
        .filter(|shadow| shadow.is_inset() == inset)
    {
        scene.push_shadow_in(
            layer,
            Shadow::new(bounds, *shadow).corner_radii(radius).clip(clip),
        );
    }
}

pub(super) fn own_text_clip(element: &Element, bounds: Rect, parent_clip: Rect) -> Rect {
    let clips_own_text = element.resolved_typography.line_clamp.is_some()
        || element.layout.overflow.x != Overflow::Visible
        || element.layout.overflow.y != Overflow::Visible;
    if clips_own_text {
        parent_clip.intersection(bounds).unwrap_or(Rect::ZERO)
    } else {
        parent_clip
    }
}

/// Return the box Cosmic Text must use for a text leaf.
///
/// Taffy measures leaf content before adding its resolved padding and border. Painting with the
/// border box would therefore give Cosmic Text a different width from layout: padded wrapped text
/// could alternate between the two line breaks on consecutive resize frames and spill into a
/// neighboring grid cell. Keep shaping, selection, decorations, and paint in the same content box.
pub(super) fn text_content_bounds(bounds: Rect, layout: &taffy::tree::Layout) -> Rect {
    let left = layout.border.left + layout.padding.left;
    let right = layout.border.right + layout.padding.right;
    let top = layout.border.top + layout.padding.top;
    let bottom = layout.border.bottom + layout.padding.bottom;
    Rect::new(
        bounds.x + left,
        bounds.y + top,
        (bounds.width - left - right).max(0.0),
        (bounds.height - top - bottom).max(0.0),
    )
}

pub(super) fn element_has_outset_shadow(element: &Element) -> bool {
    [
        element.visual.shadows.as_deref(),
        element.hover.shadows.as_deref(),
        element.active.shadows.as_deref(),
        element.focus.shadows.as_deref(),
        element.invalid_style.shadows.as_deref(),
        element.disabled_style.shadows.as_deref(),
        element.selected_style.shadows.as_deref(),
        element.dragging.shadows.as_deref(),
        element.drag_over.shadows.as_deref(),
        element.focus_within.shadows.as_deref(),
    ]
    .into_iter()
    .flatten()
    .flatten()
    .any(|shadow| !shadow.is_inset())
        || element
            .group_styles
            .iter()
            .flat_map(|entry| entry.style.shadows.as_deref())
            .flatten()
            .any(|shadow| !shadow.is_inset())
}
