use super::*;

impl UiTree {
    pub fn paint(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl TextLayoutEngine,
    ) -> Result<(), UiError> {
        self.paint_at(scene, renderer, Instant::now())
    }

    /// An element and its ancestors, innermost first, so a group can tell whether the held press
    /// or the focus sits inside it; empty when there is no such element.
    pub(super) fn ancestor_path(&self, element: Option<ElementId>) -> Vec<ElementId> {
        let mut path = Vec::new();
        let mut current = element;
        while let Some(id) = current
            && path.len() < MAX_MOUSE_EVENT_PATH
        {
            path.push(id);
            current = self.parents.get(&id).copied();
        }
        path
    }

    pub(crate) fn paint_at(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl TextLayoutEngine,
        paint_time: Instant,
    ) -> Result<(), UiError> {
        self.ensure_natural_geometry()?;
        self.begin_cached_paint(paint_time);
        let rebuild_geometry = self.needs_paint_geometry();
        if rebuild_geometry {
            self.element_bounds.clear();
            self.hit_regions.clear();
            self.scroll_regions.clear();
            self.dismiss_regions.clear();
            self.selectable_text_regions.clear();
        }
        #[cfg(target_os = "macos")]
        self.native_views.clear();
        self.text_input_regions.clear();
        self.style_transition_frame_requested = false;
        for playback in self.animations.values_mut() {
            playback.seen = false;
        }
        let Some(root) = &self.root else {
            return Ok(());
        };
        let viewport = Rect::from_size(self.viewport);
        let paint_started = Instant::now();
        let mut source_order = 0;
        let styled_focus = self.styled_focus();
        let pressed_path = self.ancestor_path(self.pressed);
        let focused_path = self.ancestor_path(self.focused);
        let mut transition_context = StyleTransitionPaintContext {
            playbacks: Some(&mut self.style_transitions),
            request_frame: &mut self.style_transition_frame_requested,
            enabled: self.animations_enabled,
            reduce_motion: self.reduce_motion,
            now: paint_time,
        };
        let result = paint_element(
            root,
            &self.taffy,
            &self.natural_bounds,
            &mut self.element_bounds,
            &mut self.scroll_offsets,
            &self.hovered,
            self.pressed,
            self.dragging,
            self.drag_over,
            styled_focus,
            &pressed_path,
            &focused_path,
            None,
            self.scale_factor,
            scene,
            renderer,
            &mut self.text_inputs,
            &mut self.animations,
            self.animations_enabled,
            paint_time,
            &mut transition_context,
            &mut self.hit_regions,
            &mut self.scroll_regions,
            &mut self.scrollbar_states,
            &mut self.dismiss_regions,
            #[cfg(target_os = "macos")]
            &mut self.native_views,
            &mut self.text_input_regions,
            &self.selectable_text_indices,
            &mut self.selectable_text_regions,
            self.static_text_selection,
            LayoutFrame::root(Point::ZERO, viewport),
            viewport,
            viewport,
            PaintLayerKey::default(),
            &mut source_order,
            None,
            &mut self.work,
            rebuild_geometry,
            &mut self.paint_cache,
            false,
        );
        for playback in self.animations.values_mut() {
            playback.finish_visibility();
        }
        result?;
        if let Some(target) = self.visible_tooltip
            && let Some(tooltip) = self.tooltips.get(&target).cloned()
            && let Some(anchor) = self.element_bounds.get(&target).copied()
        {
            let rebuild = self
                .tooltip_overlay
                .as_ref()
                .is_none_or(|overlay| !overlay.matches(target, &tooltip));
            if rebuild {
                self.tooltip_overlay = Some(TooltipOverlay::new(
                    target,
                    &tooltip,
                    self.viewport,
                    self.scale_factor,
                    renderer,
                    paint_time,
                    self.animation_epoch,
                    self.animations_enabled,
                    self.reduce_motion,
                )?);
            }
            if let Some(overlay) = &mut self.tooltip_overlay {
                overlay.paint(
                    anchor,
                    scene,
                    renderer,
                    self.scale_factor,
                    self.animations_enabled,
                    self.reduce_motion,
                    paint_time,
                    viewport,
                    &mut source_order,
                )?;
            }
        }
        if let Some(preview) = &mut self.drag_preview {
            preview.paint(
                scene,
                renderer,
                self.scale_factor,
                self.animations_enabled,
                self.reduce_motion,
                paint_time,
                viewport,
                &mut source_order,
            )?;
        }
        if rebuild_geometry {
            self.hit_regions.sort_by_key(|region| region.order);
            self.scroll_regions.sort_by_key(|region| region.order);
            self.dismiss_regions.sort_by_key(|region| region.order);
        }
        if let Some(drag) = self.scrollbar_drag
            && !self
                .scroll_regions
                .iter()
                .any(|region| region.id == drag.id)
        {
            self.scrollbar_drag = None;
            if let Some(state) = self.scrollbar_states.get_mut(&drag.id) {
                state.dragging = false;
                state.hovered = false;
                state.visible_until = None;
            }
            if self.hovered_scrollbar == Some(drag.id) {
                self.hovered_scrollbar = None;
            }
        }
        #[cfg(target_os = "macos")]
        self.native_views
            .sort_by_key(|region| (region.z_index, region.source_order));
        scene.finish();
        self.finish_paint_geometry();
        self.finish_cached_paint();
        self.work.paint_time += paint_started.elapsed();
        Ok(())
    }

    /// Resolve retained hover against the completed layout before that layout is painted.
    ///
    /// A layout-driven target change is discrete: the old element is no longer beneath the
    /// pointer, so its interaction transition must not be carried to its new screen position.
    pub(crate) fn refresh_hover_after_layout(
        &mut self,
        point: Option<Point>,
    ) -> Result<bool, UiError> {
        self.ensure_natural_geometry()?;
        self.geometry_cache.invalidate_paint();
        self.hit_regions.clear();
        if let Some(root) = &self.root {
            let viewport = Rect::from_size(self.viewport);
            let mut source_order = 0;
            let styled_focus = self.styled_focus();
            let pressed_path = self.ancestor_path(self.pressed);
            let focused_path = self.ancestor_path(self.focused);
            let mut resolved_bounds = HashMap::new();
            collect_layout_hit_regions(
                root,
                &self.taffy,
                &self.natural_bounds,
                &self.style_transitions,
                &mut resolved_bounds,
                &mut self.scroll_offsets,
                &self.selectable_text_indices,
                &mut self.hit_regions,
                &self.hovered,
                self.pressed,
                self.dragging,
                self.drag_over,
                styled_focus,
                &pressed_path,
                &focused_path,
                None,
                LayoutFrame::root(Point::ZERO, viewport),
                viewport,
                viewport,
                PaintLayerKey::default(),
                &mut source_order,
            )?;
            self.hit_regions.sort_by_key(|region| region.order);
        }
        let changed = self.refresh_paint_hover(point, true);
        self.refresh_mouse_hover(point);
        Ok(changed)
    }

    /// Returns true when paint-only hover state changed.
    pub fn pointer_moved(&mut self, point: Point, renderer: &mut impl TextLayoutEngine) -> bool {
        let now = Instant::now();
        let hover_changed = self.refresh_retained_hover(Some(point));

        let mut selection_changed = false;
        if let Some(id) = self.selecting_input
            && let Some(mut region) = self
                .text_input_regions
                .iter()
                .rev()
                .find(|region| region.id == id)
                .cloned()
        {
            let offset = self.scroll_offsets.entry(id).or_default();
            let previous = *offset;
            if point.x < region.clip.x {
                offset.x -= region.clip.x - point.x;
            } else if point.x > region.clip.right() {
                offset.x += point.x - region.clip.right();
            }
            if point.y < region.clip.y {
                offset.y -= region.clip.y - point.y;
            } else if point.y > region.clip.bottom() {
                offset.y += point.y - region.clip.bottom();
            }
            offset.x = offset.x.clamp(0.0, region.max_scroll.x);
            offset.y = offset.y.clamp(0.0, region.max_scroll.y);
            if previous != *offset {
                let state = self.scrollbar_states.entry(id).or_default();
                state.axis = ScrollbarAxis::for_change(previous, *offset);
                if !state.hovered && !state.dragging {
                    state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
                }
            }
            region.scroll = *offset;
            let index = text_input_index_at(&region, point, self.scale_factor, renderer);
            let moved = self
                .text_inputs
                .get_mut(&id)
                .is_some_and(|state| state.move_to(index, true));
            selection_changed = previous != *offset || moved;
        }

        if let Some(gesture) = self.static_text_gesture {
            let next = self.selectable_text_position_at(point, true, renderer);
            if let Some(next) = next {
                let (target_start, target_end) = static_selection_unit_range(
                    next,
                    gesture.unit,
                    &self.selectable_texts,
                    &self.selectable_text_indices,
                )
                .unwrap_or((next, next));
                let extends_backward =
                    static_position_key(target_start, &self.selectable_text_indices)
                        < static_position_key(gesture.base_start, &self.selectable_text_indices);
                let next_selection = if extends_backward {
                    StaticTextSelection {
                        anchor: gesture.base_end,
                        focus: target_start,
                    }
                } else {
                    StaticTextSelection {
                        anchor: gesture.base_start,
                        focus: target_end,
                    }
                };
                if self.static_text_selection != Some(next_selection) {
                    self.static_text_selection = Some(next_selection);
                    selection_changed = true;
                }
                let delta = point - gesture.origin;
                if !gesture.moved
                    && delta.x * delta.x + delta.y * delta.y >= 4.0
                    && let Some(gesture) = self.static_text_gesture.as_mut()
                {
                    gesture.moved = true;
                }
            }
        }

        let tooltip_changed = self.update_tooltip_hover(Some(point), now);
        hover_changed || selection_changed || tooltip_changed
    }

    pub fn pointer_left(&mut self) -> bool {
        let hover_changed = self.refresh_retained_hover(None);
        hover_changed | self.update_tooltip_hover(None, Instant::now())
    }

    /// Recompute retained paint and listener hover state without changing selection, scrolling, or
    /// tooltip timers.
    ///
    /// Redraw calls this after layout so moving an element beneath a stationary pointer produces
    /// the same visual and entry/exit transitions as web hover.
    pub(crate) fn refresh_retained_hover(&mut self, point: Option<Point>) -> bool {
        let paint_hover_changed = self.refresh_paint_hover(point, false);
        self.refresh_mouse_hover(point);
        paint_hover_changed
    }

    pub(super) fn refresh_paint_hover(
        &mut self,
        point: Option<Point>,
        snap_transition: bool,
    ) -> bool {
        self.hover_scratch.clear();
        if self.dragging.is_none()
            && !self.external_drag_active
            && let Some(point) = point
        {
            for region in self.hit_regions.iter().rev() {
                if region.stateful && region.contains(point) {
                    self.hover_scratch.insert(region.id);
                }
                if (region.blocks_pointer || region.pointer_listener) && region.contains(point) {
                    break;
                }
            }
        }
        let paint_hover_changed = self.hover_scratch != self.hovered;
        if paint_hover_changed {
            if snap_transition {
                for id in &self.hovered {
                    if !self.hover_scratch.contains(id) {
                        self.style_transitions.remove(id);
                    }
                }
                for id in &self.hover_scratch {
                    if !self.hovered.contains(id) {
                        self.style_transitions.remove(id);
                    }
                }
            }
            std::mem::swap(&mut self.hovered, &mut self.hover_scratch);
        }
        self.hover_scratch.clear();
        paint_hover_changed
    }

    pub(crate) fn refresh_mouse_hover(&mut self, point: Option<Point>) {
        self.mouse_hover_path_scratch.clear();
        if self.dragging.is_none()
            && !self.external_drag_active
            && let Some(point) = point
            && let Some(mut current) = self
                .hit_regions
                .iter()
                .rev()
                .find(|region| region.contains(point))
                .map(|region| region.id)
        {
            let mut depth = 0;
            loop {
                if depth == MAX_MOUSE_EVENT_PATH {
                    self.mouse_hover_path_scratch.clear();
                    break;
                }
                depth += 1;
                if self
                    .mouse_listener_ranges
                    .get(&current)
                    .is_some_and(|range| {
                        self.mouse_listener_bindings[range.clone()]
                            .iter()
                            .any(|binding| binding.kind == MouseListenerKind::Hover)
                    })
                {
                    self.mouse_hover_path_scratch.push(current);
                }
                let Some(parent) = self.parents.get(&current).copied() else {
                    break;
                };
                current = parent;
            }
        }

        for index in 0..self.mouse_hover_path.len() {
            let id = self.mouse_hover_path[index];
            let exited = !self.mouse_hover_path_scratch.contains(&id);
            if exited {
                self.push_mouse_hover_changes(id, false);
            }
        }
        for index in (0..self.mouse_hover_path_scratch.len()).rev() {
            let id = self.mouse_hover_path_scratch[index];
            let entered = !self.mouse_hover_path.contains(&id);
            if entered {
                self.push_mouse_hover_changes(id, true);
            }
        }
        std::mem::swap(
            &mut self.mouse_hover_path,
            &mut self.mouse_hover_path_scratch,
        );
        self.mouse_hover_path_scratch.clear();
    }

    pub(super) fn push_mouse_hover_changes(&mut self, id: ElementId, hovered: bool) {
        let Some(range) = self.mouse_listener_ranges.get(&id).cloned() else {
            return;
        };
        for binding in self.mouse_listener_bindings[range].iter().copied() {
            if binding.kind == MouseListenerKind::Hover
                && self.pending_mouse_hover_changes.len() < crate::MAX_MOUSE_LISTENERS_PER_WINDOW
            {
                self.pending_mouse_hover_changes.push(MouseHoverChange {
                    key: binding.key,
                    hovered,
                });
            }
        }
    }

    pub(crate) fn take_mouse_hover_changes(&mut self, output: &mut Vec<MouseHoverChange>) {
        output.append(&mut self.pending_mouse_hover_changes);
    }

    /// Advance exactly one pending tooltip deadline.
    pub(crate) fn advance_tooltips(&mut self, now: Instant) -> bool {
        let Some(pending) = self.pending_tooltip else {
            return false;
        };
        if pending.show_at > now {
            return false;
        }
        self.pending_tooltip = None;
        if self.hovered_tooltip == Some(pending.target)
            && self.tooltips.contains_key(&pending.target)
        {
            self.visible_tooltip = Some(pending.target);
            self.tooltip_overlay = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn next_tooltip_deadline(&self) -> Option<Instant> {
        self.pending_tooltip.map(|pending| pending.show_at)
    }

    /// Hide any pending or visible tooltip after presses, scrolling, or drag initiation.
    pub(crate) fn clear_tooltip(&mut self) -> bool {
        self.pointer_tooltip = None;
        self.hovered_tooltip = None;
        self.pending_tooltip = None;
        let changed = self.visible_tooltip.take().is_some();
        self.tooltip_overlay = None;
        changed
    }

    pub(super) fn update_tooltip_hover(&mut self, point: Option<Point>, now: Instant) -> bool {
        self.pointer_tooltip = point.and_then(|point| self.tooltip_target_at(point));
        self.reconcile_tooltip(now)
    }

    pub(super) fn reconcile_tooltip(&mut self, now: Instant) -> bool {
        let next = self.pointer_tooltip.or_else(|| {
            self.focused
                .and_then(|focused| self.tooltip_ancestor(focused))
        });
        if next == self.hovered_tooltip {
            return false;
        }
        let was_visible = self.visible_tooltip.take().is_some();
        self.tooltip_overlay = None;
        self.pending_tooltip = None;
        self.hovered_tooltip = next;
        let mut immediately_visible = false;
        if let Some(target) = next
            && let Some(tooltip) = self.tooltips.get(&target)
        {
            if tooltip.delay.is_zero() {
                self.visible_tooltip = Some(target);
                immediately_visible = true;
            } else if let Some(show_at) = now.checked_add(tooltip.delay) {
                self.pending_tooltip = Some(PendingTooltip { target, show_at });
            }
        }
        was_visible || immediately_visible
    }

    pub(super) fn tooltip_ancestor(&self, mut current: ElementId) -> Option<ElementId> {
        loop {
            if self.tooltips.contains_key(&current) {
                return Some(current);
            }
            current = self.parents.get(&current).copied()?;
        }
    }

    /// Clear press/selection state when a platform pointer sequence is cancelled.
    pub(crate) fn cancel_pointer_interaction(&mut self) -> bool {
        self.selecting_input = None;
        let static_gesture_changed = self.static_text_gesture.take().is_some();
        let scrollbar_changed = self.scrollbar_drag.take().is_some();
        if scrollbar_changed {
            for state in self.scrollbar_states.values_mut() {
                state.dragging = false;
                state.hovered = false;
                state.visible_until = None;
            }
            self.hovered_scrollbar = None;
        }
        self.pressed.take().is_some()
            | scrollbar_changed
            | static_gesture_changed
            | self.clear_tooltip()
    }

    /// Mark an internal drag active and suppress the source click/ordinary hover states.
    pub(crate) fn begin_drag(&mut self, source: ElementId) -> bool {
        let changed = (self.dragging != Some(source))
            | self.drag_over.take().is_some()
            | self.pressed.take().is_some()
            | !self.hovered.is_empty()
            | self.clear_tooltip();
        self.dragging = Some(source);
        self.external_drag_active = false;
        self.hovered.clear();
        self.selecting_input = None;
        self.static_text_gesture = None;
        changed
    }

    /// Update only when the compatible target changes, avoiding redundant repaints while moving
    /// within one drop zone.
    pub(crate) fn set_drag_over(&mut self, target: Option<ElementId>) -> bool {
        if self.drag_over == target {
            false
        } else {
            self.drag_over = target;
            true
        }
    }

    pub(crate) fn begin_external_drag(&mut self) -> bool {
        let changed = !self.external_drag_active
            | self.dragging.take().is_some()
            | self.pressed.take().is_some()
            | !self.hovered.is_empty()
            | self.clear_tooltip();
        self.external_drag_active = true;
        self.hovered.clear();
        self.selecting_input = None;
        self.static_text_gesture = None;
        changed
    }

    pub(crate) fn end_drag(&mut self) -> bool {
        self.dragging.take().is_some()
            | std::mem::take(&mut self.external_drag_active)
            | self.drag_over.take().is_some()
    }

    pub(crate) fn element_bounds(&self, id: ElementId) -> Option<Rect> {
        self.element_bounds.get(&id).copied()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn scroll_offset(&self, id: ElementId) -> Option<Vector> {
        self.scroll_offsets.get(&id).copied()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn set_drag_preview(
        &mut self,
        preview: Option<Element>,
        source: ElementId,
        origin: Point,
        position: Point,
        cursor_offset: Option<Point>,
        renderer: &mut impl TextLayoutEngine,
        now: Instant,
    ) -> Result<bool, UiError> {
        let Some(mut root) = preview else {
            return Ok(self.drag_preview.take().is_some());
        };
        if let Some(source_bounds) = self.element_bounds(source) {
            if absolute_length(root.layout.size.width).is_none() {
                root.layout.size.width = Dimension::length(source_bounds.width);
            }
            if absolute_length(root.layout.size.height).is_none() {
                root.layout.size.height = Dimension::length(source_bounds.height);
            }
        }
        let mut tree = DetachedTree::new(
            root,
            ElementId::new(0xd4a6_31f8_6c92_7b05),
            self.viewport,
            self.scale_factor,
            renderer,
            now,
            self.animation_epoch,
            self.animations_enabled,
            self.reduce_motion,
        )?;
        // Drag creation happens between frames, unlike tooltip creation during paint. Sample an
        // unthrottled entrance again at the first actual presentation time.
        tree.motion.needs_resolve |= tree.motion.frame_requested;
        let default_offset = self
            .element_bounds(source)
            .map(|bounds| Point::new(origin.x - bounds.x, origin.y - bounds.y))
            .unwrap_or(Point::ZERO);
        self.drag_preview = Some(DragPreview {
            tree,
            cursor_offset: cursor_offset.unwrap_or(default_offset),
            position,
        });
        Ok(true)
    }

    pub(crate) fn move_drag_preview(&mut self, position: Point) -> bool {
        let Some(preview) = &mut self.drag_preview else {
            return false;
        };
        if preview.position == position {
            false
        } else {
            preview.position = position;
            true
        }
    }

    pub(crate) fn clear_drag_preview(&mut self) -> bool {
        self.drag_preview.take().is_some()
    }

    /// Start a captured drag on the topmost built-in scrollbar under `point`.
    ///
    /// The full 12-point track is interactive even though the painted thumb stays visually slim.
    /// Pressing the track first centers the thumb at the pointer, then continues as a drag.
    pub(crate) fn begin_scrollbar_drag(&mut self, point: Option<Point>) -> Option<bool> {
        let point = point?;
        let (region, axis, geometry) = self.scrollbar_at(point)?;
        let offset = self.scroll_offsets.entry(region.id).or_default();
        let previous = *offset;
        if !geometry.thumb.contains(point) && geometry.travel > 0.0 {
            let track_start = axis.point(Point::new(geometry.track.x, geometry.track.y));
            let thumb_length = match axis {
                ScrollbarAxis::Horizontal => geometry.thumb.width,
                ScrollbarAxis::Vertical => geometry.thumb.height,
            };
            let thumb_origin =
                (axis.point(point) - track_start - thumb_length * 0.5).clamp(0.0, geometry.travel);
            let next = thumb_origin / geometry.travel * axis.component(region.max_offset);
            axis.set_component(offset, next);
        }
        let view_dirty = axis == ScrollbarAxis::Vertical
            && region.virtual_scroll
            && *offset != previous
            && self
                .virtual_scroll_handles
                .get(&region.id)
                .is_none_or(|binding| binding.update_from_input(offset.y, region.bounds.height));
        let linked_offset_y = offset.y;
        let scroll_origin = axis.component(*offset);
        if axis == ScrollbarAxis::Vertical && region.virtual_scroll {
            self.sync_linked_virtual_scroll_offsets(region.id, linked_offset_y, None);
        }
        if axis == ScrollbarAxis::Vertical
            && let Some(binding) = self.virtual_scroll_handles.get(&region.id)
        {
            binding.handle.scrollbar_drag_started();
        }
        self.scrollbar_drag = Some(ScrollbarDrag {
            id: region.id,
            axis,
            pointer_origin: axis.point(point),
            scroll_origin,
        });
        let state = self.scrollbar_states.entry(region.id).or_default();
        state.axis = axis;
        state.hovered = true;
        state.dragging = true;
        state.visible_until = None;
        self.hovered_scrollbar = Some(region.id);
        self.selecting_input = None;
        self.static_text_gesture = None;
        self.pressed = None;
        Some(view_dirty)
    }

    /// Update an active scrollbar drag. Returns whether the retained scroll offset changed.
    pub(crate) fn drag_scrollbar(&mut self, point: Point) -> ScrollResult {
        let Some(drag) = self.scrollbar_drag else {
            return ScrollResult::default();
        };
        let Some(region) = self
            .scroll_regions
            .iter()
            .find(|region| region.id == drag.id)
            .copied()
        else {
            self.scrollbar_drag = None;
            return ScrollResult::default();
        };
        let Some(geometry) = scrollbar_geometry(region, drag.axis, drag.scroll_origin) else {
            self.scrollbar_drag = None;
            return ScrollResult::default();
        };
        if geometry.travel <= 0.0 {
            return ScrollResult::default();
        }
        let maximum = drag.axis.component(region.max_offset);
        let next = (drag.scroll_origin
            + (drag.axis.point(point) - drag.pointer_origin) * maximum / geometry.travel)
            .clamp(0.0, maximum);
        let offset = self.scroll_offsets.entry(region.id).or_default();
        let state = self.scrollbar_states.entry(region.id).or_default();
        state.axis = drag.axis;
        state.dragging = true;
        state.visible_until = None;
        if drag.axis.component(*offset) == next {
            ScrollResult::default()
        } else {
            drag.axis.set_component(offset, next);
            let view_dirty = drag.axis == ScrollbarAxis::Vertical
                && region.virtual_scroll
                && self
                    .virtual_scroll_handles
                    .get(&region.id)
                    .is_none_or(|binding| binding.update_from_input(next, region.bounds.height));
            if drag.axis == ScrollbarAxis::Vertical && region.virtual_scroll {
                self.sync_linked_virtual_scroll_offsets(region.id, next, None);
            }
            ScrollResult {
                changed: true,
                view_dirty,
            }
        }
    }

    pub(crate) fn end_scrollbar_drag(&mut self, now: Instant) -> ScrollResult {
        let Some(drag) = self.scrollbar_drag.take() else {
            return ScrollResult::default();
        };
        let state = self.scrollbar_states.entry(drag.id).or_default();
        state.dragging = false;
        if !state.hovered {
            state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
        }
        let view_dirty = drag.axis == ScrollbarAxis::Vertical
            && self
                .virtual_scroll_handles
                .get(&drag.id)
                .is_some_and(|binding| binding.handle.scrollbar_drag_ended());
        // Releasing a scrollbar thumb ends a scroll just as a momentum phase does.
        if self.scroll_snap_geometry.container(drag.id).is_some() {
            self.snap_scroll_container(drag.id, now);
        }
        ScrollResult {
            changed: true,
            view_dirty,
        }
    }

    pub(crate) fn scrollbar_drag_active(&self) -> bool {
        self.scrollbar_drag.is_some()
    }

    pub(crate) fn is_over_scrollbar(&self, point: Point) -> bool {
        self.scrollbar_at(point).is_some()
    }

    /// Resolve the topmost explicit web-style app-region declaration at `point`.
    ///
    /// The built-in overlay scrollbar always wins over an ancestor drag region.
    pub(crate) fn is_app_region_drag(&self, point: Point) -> bool {
        if self.is_over_scrollbar(point) {
            return false;
        }
        self.hit_regions
            .iter()
            .rev()
            .filter(|region| region.contains(point))
            .find_map(|region| region.app_region)
            == Some(AppRegion::Drag)
    }

    /// Update the topmost scrollbar edge under the pointer.
    ///
    /// The 12-point hit track remains available while the thumb itself is hidden. Only state
    /// transitions repaint, so stationary pointer events do not create extra frames.
    pub(crate) fn update_scrollbar_hover(&mut self, point: Option<Point>, now: Instant) -> bool {
        let next = point
            .and_then(|point| self.scrollbar_at(point))
            .map(|(region, axis, _)| (region.id, axis));
        if next.is_some_and(|(id, axis)| {
            self.hovered_scrollbar == Some(id)
                && self
                    .scrollbar_states
                    .get(&id)
                    .is_some_and(|state| state.axis == axis)
        }) {
            return false;
        }

        if let Some(previous) = self.hovered_scrollbar.take() {
            let state = self.scrollbar_states.entry(previous).or_default();
            state.hovered = false;
            if !state.dragging {
                state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
            }
        }
        if let Some((next, axis)) = next {
            let state = self.scrollbar_states.entry(next).or_default();
            state.axis = axis;
            state.hovered = true;
            state.visible_until = None;
            self.hovered_scrollbar = Some(next);
        }
        true
    }

    pub(super) fn scrollbar_at(
        &self,
        point: Point,
    ) -> Option<(ScrollRegion, ScrollbarAxis, ScrollbarGeometry)> {
        let blocker = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.blocks_pointer && region.contains(point))
            .map(|region| region.order);
        for region in self.scroll_regions.iter().rev() {
            if !region.clip.contains(point)
                || blocker.is_some_and(|blocker| blocker < region.scrollbar_order)
            {
                continue;
            }
            let offset = self
                .scroll_offsets
                .get(&region.id)
                .copied()
                .unwrap_or_default();
            for axis in [ScrollbarAxis::Vertical, ScrollbarAxis::Horizontal] {
                if axis == ScrollbarAxis::Vertical && !region.vertical_scrollbar {
                    continue;
                }
                let Some(geometry) = scrollbar_geometry(*region, axis, axis.component(offset))
                else {
                    continue;
                };
                if geometry.track.contains(point) {
                    return Some((*region, axis, geometry));
                }
            }
        }
        None
    }

    pub fn pointer_button(
        &mut self,
        point: Option<Point>,
        pressed: bool,
        extend_selection: bool,
        now: Instant,
        renderer: &mut impl TextLayoutEngine,
    ) -> PointerResult {
        let tooltip_repaint = pressed && self.clear_tooltip();
        if pressed
            && let Some(dismiss) = self.dismiss_regions.last().copied()
            && dismiss.policy.on_pointer_outside()
            && point.is_none_or(|point| !dismiss.contains(point))
        {
            self.selecting_input = None;
            self.static_text_gesture = None;
            let repaint = self.pressed.take().is_some() | tooltip_repaint;
            return PointerResult {
                repaint,
                clicked: None,
                dismissed: Some(DismissRequest {
                    id: dismiss.id,
                    restore_focus: dismiss.restore_focus,
                }),
                pointer_listener: None,
                drag_source: None,
            };
        }

        let static_position = if pressed {
            point.and_then(|point| self.selectable_text_position_at(point, false, renderer))
        } else {
            None
        };
        let pointer_listener = static_position
            .is_none()
            .then(|| point.and_then(|point| self.pointer_listener_at(point)))
            .flatten();
        let drag_source = pointer_listener
            .is_none()
            .then(|| {
                static_position
                    .is_none()
                    .then(|| point.and_then(|point| self.drag_source_at(point)))
                    .flatten()
            })
            .flatten();
        let region = point.and_then(|point| self.interactive_region_at(point));
        let target = region
            .filter(|region| region.clickable)
            .map(|region| region.id);
        if pressed {
            let focus_changed = if let Some(position) = static_position {
                if let Some(id) = self.nearest_focusable_ancestor(position.id) {
                    self.focus_from_pointer(id)
                } else {
                    self.blur()
                }
            } else {
                region
                    .filter(|region| region.focusable)
                    .is_some_and(|region| self.focus_from_pointer(region.id))
            };
            let selection_changed;
            self.selecting_input = None;
            if let (Some(point), Some(position)) = (point, static_position) {
                let unit = self.static_text_click_unit(point, position, now, extend_selection);
                let existing_anchor = if extend_selection {
                    self.static_text_selection
                        .map(|selection| selection.anchor)
                        .unwrap_or(position)
                } else {
                    position
                };
                let (base_start, base_end) = if extend_selection {
                    (existing_anchor, existing_anchor)
                } else {
                    static_selection_unit_range(
                        position,
                        unit,
                        &self.selectable_texts,
                        &self.selectable_text_indices,
                    )
                    .unwrap_or((position, position))
                };
                let next = if extend_selection {
                    StaticTextSelection {
                        anchor: existing_anchor,
                        focus: position,
                    }
                } else {
                    StaticTextSelection {
                        anchor: base_start,
                        focus: base_end,
                    }
                };
                selection_changed = self.static_text_selection != Some(next);
                self.static_text_selection = Some(next);
                self.static_text_gesture = Some(StaticTextGesture {
                    origin: point,
                    moved: unit != StaticTextSelectionUnit::Character,
                    unit,
                    base_start,
                    base_end,
                });
            } else if let (Some(point), Some(region)) = (point, region)
                && self.text_inputs.contains_key(&region.id)
                && let Some(input_region) = self
                    .text_input_regions
                    .iter()
                    .rev()
                    .find(|input| input.id == region.id)
                    .cloned()
            {
                let index = text_input_index_at(&input_region, point, self.scale_factor, renderer);
                let input_selection_changed = self
                    .text_inputs
                    .get_mut(&region.id)
                    .is_some_and(|state| state.move_to(index, extend_selection));
                self.selecting_input = Some(region.id);
                selection_changed =
                    input_selection_changed | self.static_text_selection.take().is_some();
                self.static_text_gesture = None;
            } else {
                selection_changed = self.static_text_selection.take().is_some();
                self.static_text_gesture = None;
            }
            let repaint =
                self.pressed != target || focus_changed || selection_changed || tooltip_repaint;
            self.pressed = target;
            PointerResult {
                repaint,
                clicked: None,
                dismissed: None,
                pointer_listener,
                drag_source,
            }
        } else {
            self.selecting_input = None;
            let suppress_click = self
                .static_text_gesture
                .take()
                .is_some_and(|gesture| gesture.moved);
            let clicked = self
                .pressed
                .filter(|pressed_id| !suppress_click && Some(*pressed_id) == target);
            let repaint = self.pressed.take().is_some();
            PointerResult {
                repaint,
                clicked,
                dismissed: None,
                pointer_listener: None,
                drag_source: None,
            }
        }
    }

    /// Apply platform content-motion deltas to the deepest scrollable region under the pointer.
    pub fn scroll_at(&mut self, point: Option<Point>, delta: Vector, now: Instant) -> ScrollResult {
        let Some(point) = point else {
            return ScrollResult::default();
        };
        let tooltip_changed = self.clear_tooltip();
        let blocker = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.blocks_pointer && region.contains(point))
            .map(|region| region.order);
        for region in self.scroll_regions.iter().rev().copied() {
            if !region.bounds.contains(point) {
                continue;
            }
            if blocker.is_some_and(|blocker| region.order < blocker) {
                continue;
            }
            let offset = self.scroll_offsets.entry(region.id).or_default();
            // A right-to-left container keeps offset zero against its right edge, so a physical
            // horizontal delta moves it the other way along the stored inline axis.
            let horizontal = if region.rtl { -delta.x } else { delta.x };
            let previous = *offset;
            let next = Vector::new(
                (offset.x - horizontal).clamp(0.0, region.max_offset.x),
                (offset.y - delta.y).clamp(0.0, region.max_offset.y),
            );
            if next != *offset {
                *offset = next;
                let active_axis = ScrollbarAxis::for_change(previous, next);
                let state = self.scrollbar_states.entry(region.id).or_default();
                state.axis = active_axis;
                if !state.hovered && !state.dragging {
                    state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
                }
                let view_dirty = region.virtual_scroll
                    && self
                        .virtual_scroll_handles
                        .get(&region.id)
                        .is_none_or(|binding| {
                            binding.update_from_input(next.y, region.bounds.height)
                        });
                if region.virtual_scroll && next.y != previous.y {
                    self.sync_linked_virtual_scroll_offsets(
                        region.id,
                        next.y,
                        if active_axis == ScrollbarAxis::Vertical {
                            now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY)
                        } else {
                            None
                        },
                    );
                }
                // Wheels carry no phase on most platforms. Arm one bounded settle deadline that
                // the next delta pushes back; it is the only timer scroll snapping ever creates.
                if self.scroll_snap_geometry.container(region.id).is_some() {
                    self.arm_scroll_snap(region.id, previous, now);
                }
                return ScrollResult {
                    changed: true,
                    view_dirty,
                };
            }
        }
        ScrollResult {
            changed: tooltip_changed,
            view_dirty: false,
        }
    }

    fn sync_linked_virtual_scroll_offsets(
        &mut self,
        source: ElementId,
        offset_y: f32,
        visible_until: Option<Instant>,
    ) {
        let Some(source_handle) = self
            .virtual_scroll_handles
            .get(&source)
            .map(|binding| binding.handle.clone())
        else {
            return;
        };
        let handles = &self.virtual_scroll_handles;
        let offsets = &mut self.scroll_offsets;
        let states = &mut self.scrollbar_states;
        for (id, binding) in handles {
            if !binding.handle.shares_state(&source_handle) {
                continue;
            }
            offsets.entry(*id).or_default().y = offset_y;
            if let Some(deadline) = visible_until {
                let state = states.entry(*id).or_default();
                state.axis = ScrollbarAxis::Vertical;
                if !state.hovered && !state.dragging {
                    state.visible_until = Some(deadline);
                }
            }
        }
    }

    /// Return the topmost declared or inferred cursor at `point`.
    ///
    /// An explicit arrow is retained as `Some(Arrow)`, allowing a foreground element to reset a
    /// cursor inherited from a lower hit region without becoming a pointer blocker.
    pub(crate) fn cursor_style_at(&self, point: Point) -> Option<CursorStyle> {
        let inert_background_below = self
            .dismiss_regions
            .last()
            .filter(|region| region.order.layer.plane == crate::ScenePlane::Overlay)
            .map(|region| region.order);
        for region in self.hit_regions.iter().rev() {
            if inert_background_below.is_some_and(|order| region.order < order) {
                // Outside a dismissible overlay, the first press dismisses instead of activating
                // anything painted beneath its surface. Keep the platform arrow there instead of
                // advertising a stale text, link, or resize cursor from inert background content.
                return Some(CursorStyle::Arrow);
            }
            if !region.contains(point) {
                continue;
            }
            let interaction_cursor = if self.drag_over == Some(region.id) {
                region.cursor_states.drag_over
            } else if self.dragging == Some(region.id) {
                region.cursor_states.dragging
            } else if self.pressed == Some(region.id) {
                region.cursor_states.active
            } else if self.dragging.is_none() && !self.external_drag_active {
                region.cursor_states.hover
            } else {
                None
            };
            let cursor = region
                .cursor_states
                .selected
                .or(interaction_cursor)
                .or(region.cursor_states.invalid)
                .or_else(|| {
                    (self.focused == Some(region.id))
                        .then_some(region.cursor_states.focus)
                        .flatten()
                })
                .or(region.cursor_style);
            if let Some(cursor) = cursor {
                return Some(cursor);
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        inert_background_below.map(|_| CursorStyle::Arrow)
    }

    pub(super) fn selectable_text_position_at(
        &self,
        point: Point,
        allow_nearest: bool,
        renderer: &mut impl TextLayoutEngine,
    ) -> Option<StaticTextPosition> {
        let blocker = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| {
                (region.blocks_pointer || region.pointer_listener) && region.contains(point)
            })
            .map(|region| region.order);
        let exact = self
            .selectable_text_regions
            .iter()
            .filter(|region| {
                region.clip.contains(point)
                    && region.bounds.contains(point)
                    && blocker.is_none_or(|blocker| blocker < region.order)
            })
            .max_by_key(|region| region.order);
        let region = exact.or_else(|| {
            allow_nearest
                .then(|| nearest_selectable_text_region(&self.selectable_text_regions, point))?
        })?;
        let entry = self.selectable_texts.get(region.document_index)?;
        let local = Point::new(
            (point.x - region.bounds.x).clamp(0.0, region.bounds.width),
            (point.y - region.bounds.y).clamp(0.0, region.bounds.height),
        );
        let offset = renderer.text_index_for_point_with_highlights(
            TextId::new(entry.id.value()),
            &entry.content,
            &region.style,
            region.highlights.as_ref(),
            region.bounds.width,
            self.scale_factor,
            local,
        );
        Some(StaticTextPosition {
            id: entry.id,
            offset: boundary_at_or_before(&entry.content, offset),
        })
    }

    pub(super) fn static_text_click_unit(
        &mut self,
        point: Point,
        position: StaticTextPosition,
        now: Instant,
        extend: bool,
    ) -> StaticTextSelectionUnit {
        if extend {
            self.last_static_text_click = None;
            return StaticTextSelectionUnit::Character;
        }
        let count = self
            .last_static_text_click
            .filter(|click| {
                click.id == position.id
                    && now.saturating_duration_since(click.at) <= STATIC_TEXT_MULTI_CLICK_INTERVAL
                    && {
                        let delta = point - click.position;
                        delta.x * delta.x + delta.y * delta.y
                            <= STATIC_TEXT_MULTI_CLICK_DISTANCE * STATIC_TEXT_MULTI_CLICK_DISTANCE
                    }
            })
            .map_or(
                1,
                |click| if click.count >= 3 { 1 } else { click.count + 1 },
            );
        self.last_static_text_click = Some(StaticTextClick {
            position: point,
            id: position.id,
            at: now,
            count,
        });
        match count {
            2 => StaticTextSelectionUnit::Word,
            3 => StaticTextSelectionUnit::Line,
            _ => StaticTextSelectionUnit::Character,
        }
    }

    pub fn dismiss_topmost(&self) -> Option<DismissRequest> {
        self.dismiss_regions
            .last()
            .filter(|region| region.policy.on_escape())
            .map(|region| DismissRequest {
                id: region.id,
                restore_focus: region.restore_focus,
            })
    }

    pub(crate) fn dismiss_request_for_pointer(
        &self,
        point: Option<Point>,
    ) -> Option<DismissRequest> {
        let dismiss = self.dismiss_regions.last().copied()?;
        if !dismiss.policy.on_pointer_outside() {
            return None;
        }
        point
            .is_none_or(|point| !dismiss.contains(point))
            .then_some(DismissRequest {
                id: dismiss.id,
                restore_focus: dismiss.restore_focus,
            })
    }

    #[cfg(target_os = "macos")]
    pub fn native_views(&self) -> &[NativeViewPlacement] {
        &self.native_views
    }

    #[cfg(target_os = "macos")]
    pub fn overlay_input_active(&self) -> bool {
        self.hit_regions.iter().any(|region| {
            region.order.layer.plane == crate::ScenePlane::Overlay && region.blocks_pointer
        })
    }

    pub(super) fn interactive_region_at(&self, point: Point) -> Option<HitRegion> {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if region.clickable || region.focusable {
                return Some(*region);
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        None
    }

    pub(crate) fn pointer_listener_at(&self, point: Point) -> Option<ElementId> {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if region.pointer_listener {
                return Some(region.id);
            }
            if region.blocks_pointer {
                return None;
            }
        }
        None
    }

    pub(crate) fn context_menu_listener_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.context_menu_ids.contains(&id))
    }

    pub(crate) fn scroll_wheel_listener_at(&self, point: Point) -> Option<ElementId> {
        if self.scroll_wheel_ids.is_empty() {
            return None;
        }
        let target = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.contains(point))
            .map(|region| region.id)?;
        self.scroll_wheel_listener_for_target(target)
    }

    pub(crate) fn scroll_wheel_listener_for_target(&self, mut id: ElementId) -> Option<ElementId> {
        loop {
            if self.scroll_wheel_ids.contains(&id) {
                return Some(id);
            }
            id = self.parents.get(&id).copied()?;
        }
    }

    pub(crate) fn parent_scroll_wheel_listener(&self, id: ElementId) -> Option<ElementId> {
        let mut current = self.parents.get(&id).copied();
        while let Some(id) = current {
            if self.scroll_wheel_ids.contains(&id) {
                return Some(id);
            }
            current = self.parents.get(&id).copied();
        }
        None
    }

    pub(crate) fn touch_listener_at(&self, point: Point) -> Option<ElementId> {
        if self.touch_ids.is_empty() {
            return None;
        }
        self.ancestor_at(point, |id| self.touch_ids.contains(&id))
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn touch_listener_for_target(&self, mut id: ElementId) -> Option<ElementId> {
        loop {
            if self.touch_ids.contains(&id) {
                return Some(id);
            }
            id = self.parents.get(&id).copied()?;
        }
    }

    pub(crate) fn parent_touch_listener(&self, id: ElementId) -> Option<ElementId> {
        let mut current = self.parents.get(&id).copied();
        while let Some(id) = current {
            if self.touch_ids.contains(&id) {
                return Some(id);
            }
            current = self.parents.get(&id).copied();
        }
        None
    }

    pub(crate) fn mouse_pressure_listener_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.mouse_pressure_ids.contains(&id))
    }

    pub(crate) fn pinch_listener_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.pinch_ids.contains(&id))
    }

    pub(crate) fn rotation_listener_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.rotation_ids.contains(&id))
    }

    pub(crate) fn smart_magnify_listener_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.smart_magnify_ids.contains(&id))
    }

    pub(super) fn tooltip_target_at(&self, point: Point) -> Option<ElementId> {
        self.ancestor_at(point, |id| self.tooltips.contains_key(&id))
    }

    pub(super) fn ancestor_at(
        &self,
        point: Point,
        mut predicate: impl FnMut(ElementId) -> bool,
    ) -> Option<ElementId> {
        let mut current = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.contains(point))
            .map(|region| region.id)?;
        loop {
            if predicate(current) {
                return Some(current);
            }
            current = self.parents.get(&current).copied()?;
        }
    }

    /// Fill a bounded target-to-root path for the topmost retained hit region at `point`.
    ///
    /// A tree deeper than the public dispatch bound fails closed and produces no partial event.
    pub(crate) fn mouse_event_path_at(&self, point: Point, output: &mut Vec<ElementId>) -> bool {
        output.clear();
        let Some(mut current) = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.contains(point))
            .map(|region| region.id)
        else {
            return true;
        };
        loop {
            if output.len() == MAX_MOUSE_EVENT_PATH {
                output.clear();
                return false;
            }
            output.push(current);
            let Some(parent) = self.parents.get(&current).copied() else {
                return true;
            };
            current = parent;
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn mouse_event_path_for_target(
        &self,
        mut target: ElementId,
        output: &mut Vec<ElementId>,
    ) -> bool {
        output.clear();
        loop {
            if output.len() == MAX_MOUSE_EVENT_PATH {
                output.clear();
                return false;
            }
            output.push(target);
            let Some(parent) = self.parents.get(&target).copied() else {
                return true;
            };
            target = parent;
        }
    }

    /// Collect outside capture handlers, then root-to-target capture and target-to-root bubble.
    pub(crate) fn collect_mouse_dispatch(
        &self,
        path: &[ElementId],
        kind: MouseListenerKind,
        button: Option<MouseButton>,
        output: &mut Vec<MouseListenerKey>,
    ) {
        output.clear();

        // Outside listeners behave like document capture: topmost/deepest declarations get the
        // first chance to close transient UI before the in-path target sees the press.
        if matches!(kind, MouseListenerKind::Down | MouseListenerKind::Up) {
            if self.hit_regions.is_empty() {
                for id in self.mouse_listener_elements.iter().rev().copied() {
                    if path.contains(&id) {
                        continue;
                    }
                    self.extend_mouse_dispatch_for(
                        id,
                        kind,
                        DispatchPhase::Capture,
                        button,
                        true,
                        output,
                    );
                }
            } else {
                for id in self.hit_regions.iter().rev().map(|region| region.id) {
                    if path.contains(&id) {
                        continue;
                    }
                    self.extend_mouse_dispatch_for(
                        id,
                        kind,
                        DispatchPhase::Capture,
                        button,
                        true,
                        output,
                    );
                }
            }
        }

        for id in path.iter().rev().copied() {
            self.extend_mouse_dispatch_for(id, kind, DispatchPhase::Capture, button, false, output);
        }
        for id in path.iter().copied() {
            self.extend_mouse_dispatch_for(id, kind, DispatchPhase::Bubble, button, false, output);
        }
    }

    pub(super) fn extend_mouse_dispatch_for(
        &self,
        id: ElementId,
        kind: MouseListenerKind,
        phase: DispatchPhase,
        button: Option<MouseButton>,
        outside: bool,
        output: &mut Vec<MouseListenerKey>,
    ) {
        let Some(range) = self.mouse_listener_ranges.get(&id).cloned() else {
            return;
        };
        for binding in self.mouse_listener_bindings[range].iter().copied() {
            if binding.kind == kind
                && binding.phase == phase
                && binding.outside == outside
                && binding
                    .button
                    .is_none_or(|expected| button == Some(expected))
            {
                output.push(binding.key);
            }
        }
    }

    pub(crate) fn drag_source_at(&self, point: Point) -> Option<ElementId> {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if region.drag_source {
                return Some(region.id);
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        None
    }

    pub(crate) fn drop_target_at(
        &self,
        point: Point,
        accepts: impl Fn(ElementId) -> bool,
    ) -> Option<ElementId> {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if region.drop_target && accepts(region.id) {
                return Some(region.id);
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        None
    }

    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(crate) fn drop_offer_target_at<'a, I>(
        &self,
        point: Point,
        offers: I,
        accepts: impl Fn(ElementId, TypeId, &dyn Any) -> bool,
    ) -> Option<(ElementId, usize)>
    where
        I: Iterator<Item = (TypeId, &'a dyn Any)> + Clone,
    {
        for region in self.hit_regions.iter().rev() {
            if !region.contains(point) {
                continue;
            }
            if region.drop_target {
                for (index, (value_type, value)) in offers.clone().enumerate() {
                    if accepts(region.id, value_type, value) {
                        return Some((region.id, index));
                    }
                }
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        None
    }

    pub(crate) fn can_drop(&self, target: ElementId, value_type: TypeId, value: &dyn Any) -> bool {
        self.drop_predicates
            .get(&(target, value_type))
            .is_none_or(|predicate| predicate(value))
    }

    /// Refresh the platform-facing typed drop topology without retaining the full UI tree.
    ///
    /// The snapshot follows the same topmost-first blocker semantics as [`Self::drop_target_at`]
    /// and reuses its allocations across damage frames. Listener order is supplied by the view's
    /// stable callback registry so reaching a bound stays deterministic.
    #[cfg(target_os = "macos")]
    pub(crate) fn update_external_drop_snapshot(
        &self,
        snapshot: &mut ExternalDropSnapshot,
        listeners: &[(ElementId, TypeId)],
    ) {
        snapshot.clear();
        snapshot.acceptances_truncated = listeners.len() > MAX_EXTERNAL_DROP_ACCEPTANCES;
        for &(id, value_type) in listeners.iter().take(MAX_EXTERNAL_DROP_ACCEPTANCES) {
            snapshot.target_ids.insert(id);
            snapshot.acceptances.insert(
                (id, value_type),
                drop_acceptance(&self.drop_predicates, id, value_type),
            );
        }
        for region in self.hit_regions.iter().rev() {
            let relevant = (region.drop_target && snapshot.target_ids.contains(&region.id))
                || region.blocks_pointer
                || region.pointer_listener;
            if !relevant {
                continue;
            }
            if snapshot.regions.len() == MAX_EXTERNAL_DROP_HIT_REGIONS {
                snapshot.regions_truncated = true;
                break;
            }
            snapshot.regions.push(ExternalDropHitRegion {
                id: region.id,
                bounds: region.bounds,
                clip: region.clip,
                drop_target: region.drop_target,
                blocks_pointer: region.blocks_pointer,
                pointer_listener: region.pointer_listener,
            });
        }
    }
}
