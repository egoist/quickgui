use super::*;

impl UiTree {
    pub(crate) fn take_work(&mut self) -> crate::PipelineMetrics {
        std::mem::take(&mut self.work)
    }

    pub fn new() -> Self {
        Self::new_at(Instant::now())
    }

    pub(crate) fn new_at(animation_epoch: Instant) -> Self {
        Self {
            work: crate::PipelineMetrics::default(),
            geometry_cache: retained::GeometryCache::default(),
            paint_cache: paint_cache::PaintCache::default(),
            root: None,
            taffy: TaffyTree::with_capacity(256),
            layout_nodes: LayoutNodeCache::default(),
            root_node: None,
            mounted_state_dirty: false,
            retained_semantics_dirty: false,
            retained_placement_dirty: false,
            seen_ids: HashSet::with_capacity(256),
            displayed_ids: HashSet::with_capacity(256),
            visible_ids: HashSet::with_capacity(256),
            scroll_offsets: HashMap::new(),
            scroll_end_states: HashMap::with_capacity(8),
            virtual_scroll_handles: HashMap::with_capacity(8),
            anchor_placement_handles: Vec::new(),
            layout_bounds_handles: Vec::new(),
            natural_bounds: HashMap::with_capacity(256),
            element_bounds: HashMap::with_capacity(256),
            hit_regions: Vec::with_capacity(128),
            drop_predicates: HashMap::with_capacity(8),
            context_menu_ids: HashSet::with_capacity(8),
            mouse_listener_bindings: Vec::with_capacity(16),
            mouse_listener_ranges: HashMap::with_capacity(8),
            mouse_listener_elements: Vec::with_capacity(8),
            key_listener_bindings: Vec::with_capacity(16),
            key_listener_ranges: HashMap::with_capacity(8),
            action_listener_bindings: Vec::with_capacity(16),
            action_listener_ranges: HashMap::with_capacity(8),
            scroll_wheel_ids: HashSet::with_capacity(8),
            touch_ids: HashSet::with_capacity(8),
            mouse_pressure_ids: HashSet::with_capacity(8),
            pinch_ids: HashSet::with_capacity(8),
            rotation_ids: HashSet::with_capacity(8),
            smart_magnify_ids: HashSet::with_capacity(8),
            tooltips: HashMap::with_capacity(8),
            pointer_tooltip: None,
            hovered_tooltip: None,
            pending_tooltip: None,
            visible_tooltip: None,
            tooltip_overlay: None,
            scroll_regions: Vec::with_capacity(8),
            scroll_snap_geometry: ScrollSnapGeometry::default(),
            scroll_snap: ScrollSnapState::default(),
            scrollbar_drag: None,
            scrollbar_states: HashMap::with_capacity(8),
            hovered_scrollbar: None,
            dismiss_regions: Vec::with_capacity(4),
            focus_restorations: Vec::with_capacity(4),
            #[cfg(target_os = "macos")]
            native_views: Vec::with_capacity(4),
            text_input_regions: Vec::with_capacity(8),
            text_inputs: HashMap::with_capacity(8),
            selectable_texts: Vec::with_capacity(64),
            selectable_text_indices: HashMap::with_capacity(64),
            selectable_text_regions: Vec::with_capacity(64),
            static_text_selection: None,
            static_text_gesture: None,
            last_static_text_click: None,
            accessibility_text_ids: HashMap::with_capacity(8),
            accessibility_snapshot: std::cell::RefCell::new(None),
            next_accessibility_text_id: ACCESSIBILITY_ROOT_ID.0 - 1,
            animations: HashMap::with_capacity(8),
            animation_ids: HashSet::with_capacity(8),
            declarative_animations: HashMap::with_capacity(8),
            declarative_time_animation_ids: HashSet::with_capacity(8),
            declarative_springs: HashMap::with_capacity(8),
            declarative_spring_ids: HashSet::with_capacity(8),
            declarative_animation_ids: HashSet::with_capacity(8),
            declarative_animation_frame_requested: false,
            declarative_animation_deadline: None,
            style_transitions: HashMap::with_capacity(8),
            style_transition_ids: HashSet::with_capacity(8),
            style_transition_frame_requested: false,
            animation_epoch,
            animations_enabled: true,
            reduce_motion: false,
            selecting_input: None,
            hovered: HashSet::with_capacity(8),
            hover_scratch: HashSet::with_capacity(8),
            mouse_hover_path: Vec::with_capacity(8),
            mouse_hover_path_scratch: Vec::with_capacity(8),
            pending_mouse_hover_changes: Vec::with_capacity(8),
            pressed: None,
            dragging: None,
            external_drag_active: false,
            drag_over: None,
            drag_preview: None,
            focused: None,
            // Nothing has been clicked yet, so an initial or programmatic focus shows its ring the
            // way a browser paints an autofocused control before any pointer interaction.
            focus_visible: true,
            input_modality: None,
            active_focus_trap: None,
            focusable_ids: HashSet::with_capacity(32),
            clickable_ids: HashSet::with_capacity(32),
            activation_targets: HashMap::with_capacity(8),
            invalid_ids: HashSet::with_capacity(8),
            form_ids: HashSet::with_capacity(4),
            form_submitter_ids: HashSet::with_capacity(4),
            focus_order: Vec::with_capacity(32),
            parents: HashMap::with_capacity(256),
            key_contexts: HashMap::with_capacity(32),
            focus_initialized: false,
            validation_announcement: None,
            viewport: Size::ZERO,
            scale_factor: 1.0,
        }
    }

    /// Copy the current painted application tree into one reusable, bounded inspector snapshot.
    ///
    /// This method exists only in explicit `inspector` builds. Ordinary builds contain neither
    /// the traversal nor any snapshot storage or paint-path branch.
    #[cfg(feature = "inspector")]
    pub(crate) fn inspector_snapshot_into(
        &self,
        snapshot: &mut crate::inspector::InspectorSnapshot,
        hit_lookup: &mut HashMap<ElementId, HitRegion>,
    ) {
        snapshot.nodes.clear();
        snapshot.focus_path.clear();
        snapshot.nodes_truncated = false;
        snapshot.hit_regions_truncated =
            self.hit_regions.len() > crate::inspector::MAX_INSPECTOR_NODES;

        hit_lookup.clear();
        hit_lookup.extend(
            self.hit_regions
                .iter()
                .copied()
                .take(crate::inspector::MAX_INSPECTOR_NODES)
                .map(|region| (region.id, region)),
        );

        snapshot.focus_path.extend(self.focus_path());
        let Some(root) = self.root.as_ref() else {
            return;
        };
        let viewport = Rect::from_size(self.viewport);
        let mut source_order = 0;
        collect_inspector_nodes(
            root,
            None,
            0,
            PaintLayerKey::default(),
            viewport,
            viewport,
            &self.element_bounds,
            hit_lookup,
            self.focused,
            &mut source_order,
            snapshot,
        );
    }

    #[cfg(test)]
    pub(super) fn set_root(
        &mut self,
        root: Element,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
    ) -> Result<(), UiError> {
        self.set_root_with_prepare(root, viewport, scale_factor, renderer, |_| {})
    }

    /// Mount a declaration while preparing each size-dependent subtree before its first layout.
    ///
    /// The production runtime uses this hook to resolve image resources returned by a query
    /// callback in the same cache frame as the surrounding view. Ordinary callers use
    /// [`UiTree::set_root`], whose preparation hook is a no-op.
    pub(crate) fn set_root_with_prepare(
        &mut self,
        root: Element,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
        mut prepare: impl FnMut(&mut Element),
    ) -> Result<(), UiError> {
        let now = Instant::now();
        // Retain the previous playback registry until query callbacks have declared their current
        // descendants. This preserves an animation keyed inside a stable query across view
        // rebuilds instead of restarting it during the intermediate empty-shell layout.
        self.set_root_unlaid(root, viewport, scale_factor, now, false)?;
        self.layout_with_prepare_at(viewport, scale_factor, renderer, now, &mut prepare)
    }

    pub(super) fn set_root_unlaid(
        &mut self,
        mut root: Element,
        viewport: Size,
        scale_factor: f32,
        now: Instant,
        retain_motion_registry: bool,
    ) -> Result<(), UiError> {
        if self
            .root
            .as_ref()
            .is_none_or(|old| old.layout_rounding != root.layout_rounding)
        {
            if root.layout_rounding {
                self.taffy.enable_rounding();
            } else {
                self.taffy.disable_rounding();
            }
            if let Some(node) = self.root_node {
                self.taffy.mark_dirty(node)?;
            }
        }
        self.declarative_animation_ids.clear();
        self.declarative_time_animation_ids.clear();
        self.declarative_spring_ids.clear();
        self.declarative_animation_frame_requested = false;
        self.declarative_animation_deadline = None;
        let mut resolved_motion_ids = Vec::new();
        resolve_declarative_animations(
            &mut root,
            &mut self.declarative_animations,
            &mut self.declarative_animation_ids,
            &mut self.declarative_time_animation_ids,
            &mut self.declarative_springs,
            &mut self.declarative_spring_ids,
            &mut self.declarative_animation_frame_requested,
            &mut self.declarative_animation_deadline,
            now,
            self.animation_epoch,
            self.animations_enabled,
            self.reduce_motion,
            &mut resolved_motion_ids,
        )?;
        if retain_motion_registry {
            self.finalize_declarative_motion_registry();
        }
        self.build_resolved_root(root, viewport, scale_factor)?;
        if retain_motion_registry {
            self.sync_mounted_root(now)?;
        }
        Ok(())
    }

    pub(super) fn build_resolved_root(
        &mut self,
        mut root: Element,
        viewport: Size,
        scale_factor: f32,
    ) -> Result<(), UiError> {
        let started = Instant::now();
        let reconciled_before = self.layout_nodes.reconciled_nodes;
        self.seen_ids.clear();
        self.seen_ids
            .insert(ElementId::new(ACCESSIBILITY_ROOT_ID.0));
        let mut style_transition_count = 0;
        validate_style_transition_count(&root, &mut style_transition_count)?;
        validate_container_query_limits(&root)?;
        validate_sticky_limits(&root)?;
        collect_explicit_ids(&root, &mut self.seen_ids)?;
        self.update_layout_scale_factor(scale_factor)?;
        self.viewport = viewport;
        let inherited = TextStyle::default();
        let root_node = build_retained_layout_node(
            &mut self.taffy,
            &mut self.layout_nodes,
            &mut self.seen_ids,
            &mut root,
            ElementId::new(0x9e37_79b9_7f4a_7c15),
            0,
            &inherited,
            true,
            Direction::Ltr,
        )?;
        self.layout_nodes.commit_children(&mut self.taffy)?;
        self.root = Some(root);
        self.root_node = Some(root_node);
        self.geometry_cache.invalidate();
        self.geometry_cache.state_transforms =
            retained::has_state_transform(self.root.as_ref().unwrap());
        self.mounted_state_dirty = true;
        self.work.reconciliation_time += started.elapsed();
        self.work.reconciled_nodes += self.layout_nodes.reconciled_nodes - reconciled_before;
        Ok(())
    }

    /// Reconcile retained interaction and playback state exactly once against a fully expanded
    /// declaration. Intermediate container-query shells deliberately skip this phase so they
    /// cannot transiently unmount stable text input, scroll, transition, or image state.
    pub(super) fn sync_mounted_root(&mut self, now: Instant) -> Result<(), UiError> {
        self.geometry_cache.invalidate();
        self.geometry_cache.state_transforms = self
            .root
            .as_ref()
            .is_some_and(retained::has_state_transform);
        self.paint_cache.mount(self.root.as_ref());
        // Query callbacks may temporarily replace their descendants with an empty shell. Keep
        // their layout nodes until the complete declaration has converged, just like mount state.
        self.layout_nodes.retain(&mut self.taffy, &self.seen_ids)?;
        let root = self
            .root
            .as_ref()
            .expect("mounted state is synchronized after a resolved root is built");
        self.displayed_ids.clear();
        collect_displayed_ids(root, &mut self.displayed_ids);
        self.visible_ids.clear();
        collect_visible_ids(root, &mut self.visible_ids);
        if let Some(drag) = self.scrollbar_drag
            && !self.visible_ids.contains(&drag.id)
        {
            if let Some(binding) = self.virtual_scroll_handles.get(&drag.id) {
                binding.handle.scrollbar_drag_ended();
            }
            self.scrollbar_drag = None;
        }
        self.style_transition_ids.clear();
        collect_style_transition_ids(root, &mut self.style_transition_ids);
        self.style_transitions
            .retain(|id, _| self.style_transition_ids.contains(id));
        self.animation_ids.clear();
        sync_animations(root, &mut self.animations, &mut self.animation_ids, now);
        self.animations
            .retain(|id, _| self.animation_ids.contains(id));
        if let Some(root) = &self.root {
            let mut tooltips = HashMap::with_capacity(self.tooltips.capacity().max(8));
            collect_tooltips(root, &mut tooltips)?;
            self.tooltips = tooltips;
            self.tooltip_overlay = None;
            if self
                .hovered_tooltip
                .is_some_and(|target| !self.tooltips.contains_key(&target))
            {
                self.hovered_tooltip = None;
                self.pointer_tooltip = None;
                self.pending_tooltip = None;
                self.visible_tooltip = None;
            }
            self.anchor_placement_handles.clear();
            collect_anchor_placement_handles(root, &mut self.anchor_placement_handles);
            self.layout_bounds_handles.clear();
            collect_layout_bounds_handles(root, &mut self.layout_bounds_handles);
            self.virtual_scroll_handles.clear();
            sync_virtual_scrolls(
                root,
                &mut self.virtual_scroll_handles,
                &mut self.scroll_offsets,
                &mut self.scrollbar_states,
                now,
            );
            let mut input_ids = HashSet::new();
            sync_text_inputs(root, &mut self.text_inputs, &mut input_ids);
            self.text_inputs.retain(|id, _| input_ids.contains(id));

            let previous_selectable_texts = std::mem::take(&mut self.selectable_texts);
            self.selectable_texts = Vec::with_capacity(previous_selectable_texts.len());
            let mut previous_selectable_texts = previous_selectable_texts
                .into_iter()
                .map(|entry| (entry.id, entry))
                .collect::<HashMap<_, _>>();
            self.selectable_text_indices.clear();
            collect_selectable_texts(
                root,
                &mut self.selectable_texts,
                &mut self.selectable_text_indices,
                &mut previous_selectable_texts,
            );
            sync_static_text_selection(
                &mut self.static_text_selection,
                &self.selectable_texts,
                &self.selectable_text_indices,
            );
            if self.static_text_gesture.is_some() && self.static_text_selection.is_none() {
                self.static_text_gesture = None;
            }
            if self
                .last_static_text_click
                .is_some_and(|click| !self.selectable_text_indices.contains_key(&click.id))
            {
                self.last_static_text_click = None;
            }
            let mut accessible_text_ids = input_ids.clone();
            accessible_text_ids.extend(self.selectable_text_indices.keys().copied());
            sync_accessibility_text_ids(
                &accessible_text_ids,
                &self.seen_ids,
                &mut self.accessibility_text_ids,
                &mut self.next_accessibility_text_id,
            );
            if self
                .selecting_input
                .is_some_and(|id| !input_ids.contains(&id))
            {
                self.selecting_input = None;
            }
        }
        let previous_focused = self.focused;
        let previous_focus_restorations = std::mem::take(&mut self.focus_restorations);
        self.rebuild_focus_index();
        let restore_focus = previous_focus_restorations
            .iter()
            .rev()
            .find_map(|restoration| {
                (!self.seen_ids.contains(&restoration.surface)
                    && self.focusable_ids.contains(&restoration.target))
                .then_some(restoration.target)
            });
        if let Some(root) = &self.root {
            collect_focus_restorations(
                root,
                &self.displayed_ids,
                &previous_focus_restorations,
                previous_focused,
                &mut self.focus_restorations,
            );
        }
        self.rebuild_dispatch_index();
        self.rebuild_drop_predicates();
        self.scroll_offsets
            .retain(|id, _| self.displayed_ids.contains(id));
        self.scroll_end_states
            .retain(|id, _| self.displayed_ids.contains(id));
        self.scrollbar_states
            .retain(|id, _| self.displayed_ids.contains(id));
        for (id, state) in &mut self.scrollbar_states {
            if !self.visible_ids.contains(id) {
                state.hovered = false;
                state.dragging = false;
                state.visible_until = None;
            }
        }
        if self
            .hovered_scrollbar
            .is_some_and(|id| !self.visible_ids.contains(&id))
        {
            self.hovered_scrollbar = None;
        }
        self.hovered.retain(|id| self.visible_ids.contains(id));
        if self
            .pressed
            .is_some_and(|id| !self.visible_ids.contains(&id))
        {
            self.pressed = None;
        }
        if self
            .dragging
            .is_some_and(|id| !self.visible_ids.contains(&id))
        {
            self.dragging = None;
            self.drag_over = None;
        }
        if self
            .drag_over
            .is_some_and(|id| !self.visible_ids.contains(&id))
        {
            self.drag_over = None;
        }
        if let Some(target) = restore_focus {
            self.focus(target);
        } else if self
            .focused
            .is_some_and(|id| !self.focusable_ids.contains(&id))
        {
            self.focused = None;
        }
        self.initialize_focus_if_needed();
        if self.pointer_tooltip.is_none() {
            self.reconcile_tooltip(now);
        }
        self.mounted_state_dirty = false;
        Ok(())
    }

    pub(super) fn finalize_declarative_motion_registry(&mut self) {
        self.declarative_animations
            .retain(|id, _| self.declarative_time_animation_ids.contains(id));
        self.declarative_springs
            .retain(|id, _| self.declarative_spring_ids.contains(id));
        self.recompute_declarative_motion_schedule();
    }

    pub(super) fn initialize_focus_if_needed(&mut self) {
        if self.focused.is_none() && !self.focus_initialized {
            self.focused = self
                .root
                .as_ref()
                .and_then(find_auto_focus)
                .filter(|id| self.focusable_ids.contains(id));
        }
        self.focus_initialized = true;
    }

    pub(super) fn recompute_declarative_motion_schedule(&mut self) {
        self.declarative_animation_frame_requested = self
            .declarative_time_animation_ids
            .iter()
            .filter_map(|id| self.declarative_animations.get(id))
            .any(|playback| playback.active && playback.next_frame_at.is_none())
            || self
                .declarative_spring_ids
                .iter()
                .filter_map(|id| self.declarative_springs.get(id))
                .any(|playback| playback.active);
        self.declarative_animation_deadline = self
            .declarative_time_animation_ids
            .iter()
            .filter_map(|id| self.declarative_animations.get(id))
            .filter(|playback| playback.active)
            .filter_map(|playback| playback.next_frame_at)
            .min();
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn set_root_for_test(
        &mut self,
        root: Element,
        viewport: Size,
        scale_factor: f32,
        now: Instant,
    ) -> Result<(), UiError> {
        self.set_root_unlaid(root, viewport, scale_factor, now, false)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn contains_element(&self, id: ElementId) -> bool {
        self.seen_ids.contains(&id)
    }

    pub(crate) fn is_clickable(&self, id: ElementId) -> bool {
        self.clickable_ids.contains(&id)
    }

    pub fn set_animations_enabled(&mut self, enabled: bool, now: Instant) {
        if self.animations_enabled == enabled {
            return;
        }
        if !enabled {
            for playback in self.declarative_animations.values_mut() {
                playback.pause(now);
            }
            for playback in self.declarative_springs.values_mut() {
                playback.pause(now);
            }
            for playback in self.style_transitions.values_mut() {
                playback.pause(now);
            }
            for playback in self.animations.values_mut() {
                playback.advance(now);
                playback.active = false;
            }
            if let Some(preview) = &mut self.drag_preview {
                preview.tree.pause(now);
            }
            if let Some(tooltip) = &mut self.tooltip_overlay {
                tooltip.tree.pause(now);
            }
        } else {
            for playback in self.declarative_animations.values_mut() {
                playback.resume(now);
            }
            for playback in self.declarative_springs.values_mut() {
                playback.resume(now);
            }
            for playback in self.style_transitions.values_mut() {
                playback.resume(now);
            }
            if let Some(preview) = &mut self.drag_preview {
                preview.tree.resume(now);
            }
            if let Some(tooltip) = &mut self.tooltip_overlay {
                tooltip.tree.resume(now);
            }
        }
        self.animations_enabled = enabled;
    }

    pub(crate) fn set_reduce_motion(&mut self, reduce_motion: bool) {
        if self.reduce_motion == reduce_motion {
            return;
        }
        self.reduce_motion = reduce_motion;
        if let Some(preview) = &mut self.drag_preview {
            preview.tree.motion.mark_needs_resolve();
        }
        if let Some(tooltip) = &mut self.tooltip_overlay {
            tooltip.tree.motion.mark_needs_resolve();
        }
    }

    pub(crate) fn declarative_animation_frame_requested(&self) -> bool {
        self.animations_enabled && !self.reduce_motion && self.declarative_animation_frame_requested
    }

    pub(crate) fn style_transition_frame_requested(&self) -> bool {
        self.animations_enabled && !self.reduce_motion && self.style_transition_frame_requested
    }

    pub(crate) fn detached_animation_frame_requested(&self) -> bool {
        self.animations_enabled
            && !self.reduce_motion
            && (self
                .drag_preview
                .as_ref()
                .is_some_and(|preview| preview.tree.motion.frame_requested)
                || self
                    .tooltip_overlay
                    .as_ref()
                    .is_some_and(|tooltip| tooltip.tree.motion.frame_requested))
    }

    pub(crate) fn has_declarative_animations(&self) -> bool {
        !self.declarative_animations.is_empty() || !self.declarative_springs.is_empty()
    }

    pub(crate) fn declarative_animation_due(&mut self, now: Instant) -> bool {
        if !self.animations_enabled || self.reduce_motion {
            return false;
        }
        if self
            .declarative_animation_deadline
            .is_some_and(|deadline| deadline <= now)
        {
            self.declarative_animation_deadline = None;
            true
        } else {
            false
        }
    }

    pub fn advance_animations(&mut self, now: Instant) -> bool {
        if !self.animations_enabled {
            return false;
        }
        let mut changed = false;
        for playback in self.animations.values_mut() {
            changed |= playback.advance(now);
        }
        for playback in self.style_transitions.values_mut() {
            changed |= playback.deadline_due(now);
        }
        if let Some(preview) = &mut self.drag_preview {
            changed |= preview.tree.advance_animations(now);
        }
        if let Some(tooltip) = &mut self.tooltip_overlay {
            changed |= tooltip.tree.advance_animations(now);
        }
        changed
    }

    pub fn next_animation_deadline(&self) -> Option<Instant> {
        if !self.animations_enabled {
            return None;
        }
        self.animations
            .values()
            .filter_map(AnimationPlayback::deadline)
            .chain(self.declarative_animation_deadline)
            .chain(
                self.style_transitions
                    .values()
                    .filter_map(StyleTransitionPlayback::deadline),
            )
            .chain(
                self.drag_preview
                    .iter()
                    .filter_map(|preview| preview.tree.next_animation_deadline()),
            )
            .chain(
                self.tooltip_overlay
                    .iter()
                    .filter_map(|tooltip| tooltip.tree.next_animation_deadline()),
            )
            .min()
    }

    /// Advance the one-shot native scrollbar visibility deadline.
    ///
    /// This never drives an animation loop: it returns true once, when a visible scrollbar needs
    /// its final hide repaint.
    pub(crate) fn advance_scrollbars(&mut self, now: Instant) -> bool {
        // Scroll snapping shares this one-shot deadline pump: it is not an animation loop, and
        // both its settle and its bounded travel clear themselves once resolved.
        let mut changed = self.advance_scroll_snap(now);
        for state in self.scrollbar_states.values_mut() {
            if !state.hovered
                && !state.dragging
                && state.visible_until.is_some_and(|deadline| deadline <= now)
            {
                state.visible_until = None;
                changed = true;
            }
        }
        changed
    }

    /// Whether the focused input's caret toggled since it was last painted.
    ///
    /// This is a one-shot check per toggle, not an animation loop: the paint that follows records
    /// the phase it drew, and nothing is due again until the next half period elapses.
    pub(crate) fn advance_caret_blink(&self, now: Instant) -> bool {
        self.focused
            .and_then(|focused| self.text_inputs.get(&focused))
            .is_some_and(|input| input.caret_toggle_due(now))
    }

    /// When the focused input's caret next toggles, if any input is focused and blinking.
    pub(crate) fn next_caret_blink_deadline(&self, now: Instant) -> Option<Instant> {
        self.focused
            .and_then(|focused| self.text_inputs.get(&focused))
            .and_then(|input| input.next_caret_toggle(now))
    }

    pub(crate) fn next_scrollbar_deadline(&self) -> Option<Instant> {
        self.scrollbar_states
            .values()
            .filter(|state| !state.hovered && !state.dragging)
            .filter_map(|state| state.visible_until)
            .chain(self.next_scroll_snap_deadline())
            .min()
    }

    pub fn animation_counts(&self) -> (usize, usize) {
        let preview = self
            .drag_preview
            .as_ref()
            .map_or((0, 0), |preview| preview.tree.animation_counts());
        let tooltip = self
            .tooltip_overlay
            .as_ref()
            .map_or((0, 0), |tooltip| tooltip.tree.animation_counts());
        (
            self.animations.len() + preview.0 + tooltip.0,
            self.animations
                .values()
                .filter(|state| state.active)
                .count()
                + self
                    .declarative_animations
                    .values()
                    .filter(|state| state.active)
                    .count()
                + self
                    .declarative_springs
                    .values()
                    .filter(|state| state.active)
                    .count()
                + self
                    .style_transitions
                    .values()
                    .filter(|state| state.active)
                    .count()
                + preview.1
                + tooltip.1,
        )
    }

    /// Whether a mounted variable-height list learned geometry after this declaration was built.
    ///
    /// The runtime uses this edge-triggered revision comparison to request one correcting frame.
    /// A view rebuild is needed only when the measured mounted slice no longer covers the viewport;
    /// unchanged retained lists create no redraw source.
    pub(crate) fn take_variable_list_measurement_update(&mut self) -> ScrollResult {
        let mut result = ScrollResult::default();
        for binding in self.virtual_scroll_handles.values_mut() {
            let revision = binding.handle.measurement_revision();
            if revision == binding.measurement_revision {
                continue;
            }
            binding.measurement_revision = revision;
            result.changed = true;
            result.view_dirty |= !binding
                .handle
                .refresh_mount_after_measurement(&mut binding.mount);
        }
        result
    }

    /// Whether any bound anchored element resolved to a different placement during the last paint.
    ///
    /// The runtime uses this edge-triggered revision comparison to request one correcting frame so
    /// an application that draws from [`crate::AnchorPlacementHandle`] converges immediately after
    /// a flip. An unchanged placement reports nothing and creates no redraw source.
    /// Whether any mounted element reads back geometry that only painting resolves.
    pub(crate) fn observes_painted_geometry(&self) -> bool {
        !self.layout_bounds_handles.is_empty()
    }

    pub(crate) fn take_anchor_placement_update(&mut self) -> bool {
        let mut changed = false;
        for binding in &mut self.anchor_placement_handles {
            let revision = binding.handle.revision();
            if revision == binding.revision {
                continue;
            }
            binding.revision = revision;
            changed = true;
        }
        // Painted bounds follow the same edge-triggered contract: a handle whose bounds moved
        // earns one correcting frame, and an unchanged one earns nothing.
        for binding in &mut self.layout_bounds_handles {
            let revision = binding.handle.revision();
            if revision == binding.revision {
                continue;
            }
            binding.revision = revision;
            changed = true;
        }
        changed
    }

    #[cfg(test)]
    pub(super) fn layout(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
    ) -> Result<(), UiError> {
        self.layout_with_prepare_at(
            viewport,
            scale_factor,
            renderer,
            Instant::now(),
            &mut |_| {},
        )
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn layout_for_test(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
        now: Instant,
    ) -> Result<(), UiError> {
        self.layout_with_prepare_at(viewport, scale_factor, renderer, now, &mut |_| {})
    }

    /// Recompute layout for the mounted declaration without rebuilding the application view.
    ///
    /// Resize-only frames use this retained path. Size-dependent container-query callbacks still
    /// receive their preparation hook, while stable element, text-input, scroll, and accessibility
    /// state remains mounted.
    pub(crate) fn relayout_with_prepare(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
        mut prepare: impl FnMut(&mut Element),
    ) -> Result<(), UiError> {
        self.layout_with_prepare_at(
            viewport,
            scale_factor,
            renderer,
            Instant::now(),
            &mut prepare,
        )
    }

    fn update_layout_scale_factor(&mut self, scale_factor: f32) -> Result<(), UiError> {
        if self.scale_factor != scale_factor {
            // Display scale participates in text shaping but not in Taffy's constraint key.
            self.layout_nodes.invalidate_measurements(&mut self.taffy)?;
            self.scale_factor = scale_factor;
        }
        Ok(())
    }

    /// Measure the mounted root with max-content constraints on selected axes, then restore its
    /// ordinary finite viewport layout. Embedded native hosts use this bounded two-pass path so
    /// intrinsic SwiftUI sizing does not create a second renderer or a parallel layout engine.
    #[cfg(all(target_os = "macos", feature = "swift-ui"))]
    pub(crate) fn measure_intrinsic_content(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        match_horizontal: bool,
        match_vertical: bool,
        renderer: &mut impl TextLayoutEngine,
    ) -> Result<Size, UiError> {
        let Some(root) = self.root_node else {
            return Ok(Size::new(1.0, 1.0));
        };
        compute_detached_layout_available(
            &mut self.taffy,
            root,
            TaffySize {
                width: if match_horizontal {
                    AvailableSpace::MaxContent
                } else {
                    AvailableSpace::Definite(viewport.width)
                },
                height: if match_vertical {
                    AvailableSpace::MaxContent
                } else {
                    AvailableSpace::Definite(viewport.height)
                },
            },
            scale_factor,
            renderer,
        )?;
        let measured = self.taffy.layout(root)?.size;
        compute_detached_layout(&mut self.taffy, root, viewport, scale_factor, renderer)?;
        Ok(Size::new(
            if match_horizontal {
                measured.width.max(1.0)
            } else {
                viewport.width.max(1.0)
            },
            if match_vertical {
                measured.height.max(1.0)
            } else {
                viewport.height.max(1.0)
            },
        ))
    }

    /// Discard CPU-only semantic measurement caches before an exact offscreen visual layout.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn invalidate_layout_measurements_for_test(&mut self) -> Result<(), UiError> {
        if let Some(root) = &self.root {
            mark_layout_nodes_dirty(root, &mut self.taffy)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn layout_with_prepare_at(
        &mut self,
        viewport: Size,
        scale_factor: f32,
        renderer: &mut impl TextLayoutEngine,
        now: Instant,
        prepare: &mut impl FnMut(&mut Element),
    ) -> Result<(), UiError> {
        let started = Instant::now();
        let passes_before = self.layout_nodes.layout_passes;
        let measured_before = self.layout_nodes.measured_nodes;
        self.update_layout_scale_factor(scale_factor)?;
        self.viewport = viewport;
        let mut converged = self.root_node.is_none();
        let mut needs_outer_layout = true;
        for _ in 0..=MAX_CONTAINER_QUERY_DEPTH {
            let Some(root_node) = self.root_node else {
                converged = true;
                break;
            };
            if needs_outer_layout {
                self.layout_nodes.compute_layout(
                    &mut self.taffy,
                    root_node,
                    viewport,
                    scale_factor,
                    renderer,
                )?;
            }
            if let Some(root) = &self.root {
                compute_container_query_child_layouts(
                    root,
                    &mut self.taffy,
                    scale_factor,
                    renderer,
                    Some(&mut self.layout_nodes),
                )?;
            }
            let resolution = self.resolve_container_queries(now, prepare)?;
            if !resolution.changed {
                converged = true;
                break;
            }
            self.mounted_state_dirty = true;

            let requires_full_rebuild = resolution.replaced_existing_subtree
                || self.build_pending_container_query_subtrees()?;
            if requires_full_rebuild {
                let root = self
                    .root
                    .take()
                    .expect("a laid out query declaration retains its element root");
                self.build_resolved_root(root, viewport, scale_factor)?;
                needs_outer_layout = true;
            } else {
                // The query boxes already have final parent geometry. Newly declared child roots
                // can be laid out directly inside those boxes without recomputing the outer tree.
                needs_outer_layout = false;
            }
        }
        if !converged {
            return Err(UiError::ContainerQueryDidNotConverge);
        }
        self.finalize_declarative_motion_registry();
        if self.mounted_state_dirty {
            self.sync_mounted_root(now)?;
        }
        if let Some(root) = &self.root {
            // Variable-list geometry comes from the completed Taffy layout. Commit it before
            // paint chooses the retained scroll translation so a width-dependent row (notably a
            // wrapped Markdown message) cannot expose one frame at the previous tail offset.
            report_variable_list_layout_measurements(root, &self.taffy)?;
        }
        if let Some(preview) = &mut self.drag_preview {
            preview.tree.layout(viewport, scale_factor, renderer)?;
        }
        if let Some(tooltip) = &mut self.tooltip_overlay {
            tooltip.tree.layout(viewport, scale_factor, renderer)?;
        }
        self.work.layout_time += started.elapsed();
        self.work.layout_passes += self.layout_nodes.layout_passes - passes_before;
        self.work.measured_nodes += self.layout_nodes.measured_nodes - measured_before;
        if self.layout_nodes.layout_passes != passes_before {
            self.geometry_cache.invalidate();
        }
        Ok(())
    }

    pub(super) fn resolve_container_queries(
        &mut self,
        now: Instant,
        prepare: &mut impl FnMut(&mut Element),
    ) -> Result<ContainerQueryResolution, UiError> {
        let Some(root) = &mut self.root else {
            return Ok(ContainerQueryResolution::default());
        };
        let mut context = ContainerQueryResolveContext {
            taffy: &self.taffy,
            prepare,
            animations: &mut self.declarative_animations,
            motion_ids: &mut self.declarative_animation_ids,
            time_animation_ids: &mut self.declarative_time_animation_ids,
            springs: &mut self.declarative_springs,
            spring_ids: &mut self.declarative_spring_ids,
            request_frame: &mut self.declarative_animation_frame_requested,
            deadline: &mut self.declarative_animation_deadline,
            now,
            animation_epoch: self.animation_epoch,
            enabled: self.animations_enabled,
            reduce_motion: self.reduce_motion,
            sanitize_detached: false,
        };
        context.resolve(root)
    }

    /// Build callback roots into the existing Taffy arena when every changed query was previously
    /// empty. Returns `true` when an explicit ID collides with an already assigned generated ID;
    /// a full rebuild then preserves the normal explicit-ID-first assignment contract.
    pub(super) fn build_pending_container_query_subtrees(&mut self) -> Result<bool, UiError> {
        let Some(root) = &mut self.root else {
            return Ok(false);
        };
        let mut style_transition_count = 0;
        validate_style_transition_count(root, &mut style_transition_count)?;
        validate_container_query_limits(root)?;
        let rebuild = build_pending_container_query_subtrees(
            root,
            &mut self.taffy,
            &mut self.layout_nodes,
            &mut self.seen_ids,
        )?;
        self.layout_nodes.commit_children(&mut self.taffy)?;
        Ok(rebuild)
    }
}
