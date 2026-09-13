use super::*;

impl Runtime {
    pub(super) fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let mut mounted_focus_previous = None;
        #[cfg(all(target_os = "macos", feature = "swift-ui"))]
        let mut embedded_owner_size_changed = None;
        #[cfg(target_os = "macos")]
        {
            let previous_focus = self.window.as_ref().and_then(|state| state.ui.focused());
            let native_focus_active = self
                .window
                .as_ref()
                .and_then(|state| state.native_host.as_ref())
                .is_some_and(MacNativeHost::native_focus_active);
            if native_focus_active && previous_focus.is_some() {
                if let Some(state) = &mut self.window {
                    state.ui.blur();
                    state.view_dirty = true;
                }
                self.announce_focus_change(event_loop, previous_focus);
            }
        }
        let Some(window_handle) = self.current_handle() else {
            return;
        };
        let Some(window_state) = self.current_window_state() else {
            return;
        };
        let background_tasks = self.background_tasks.clone();
        let foreground_tasks = self.foreground_tasks.clone();
        let globals = self.globals.clone();
        let displays = self.displays.clone();
        let keyboard_layout = self.keyboard.layout().clone();
        let assets = self.assets.clone();
        let font_system = self.font_system.clone();
        let app_info = self.app_info.clone();
        let app_paths = self.app_paths.clone();
        let system_info = self.system_info.clone();
        let system_preferences = self.system_preferences;
        let event_proxy = self.event_proxy.clone();
        #[cfg(target_os = "macos")]
        self.release_focus_to_native_view(event_loop);
        let Some(state) = &mut self.window else {
            return;
        };
        // `NSViewLayerContentsRedrawDuringViewResize` can make AppKit ask for a draw before
        // Winit's frame-change notification reaches `WindowEvent::Resized`. Always reconcile the
        // retained layout with the drawable's current geometry before building that frame, so a
        // new-size surface never presents text or overlays laid out for the preceding size.
        let physical_size = state.window.inner_size();
        let scale_factor = sane_scale_factor(state.window.scale_factor());
        let logical_size = logical_window_size(physical_size, scale_factor);
        if state.scale_factor != scale_factor || state.logical_size != logical_size {
            state.scale_factor = scale_factor;
            state.logical_size = logical_size;
            state
                .renderer
                .resize(physical_size.width, physical_size.height);
            state.layout_dirty = true;
            state.view_dirty |= state.listeners.observes_viewport;
        }
        state.scheduler.begin_redraw();

        let scroll = state.scheduler.take_scroll();
        let scroll_result = if scroll.is_zero() {
            Default::default()
        } else {
            state.ui.scroll_at(state.pointer, scroll, Instant::now())
        };
        if scroll_result.view_dirty {
            state.view_dirty = true;
        }
        let scroll_for_view = (!scroll.is_zero() && !scroll_result.changed).then_some(scroll);
        if let Some(scroll) = scroll_for_view {
            let mut event_cx = self.event_context();
            if let Some(state) = &mut self.window {
                state.view.event(&Event::Scroll(scroll), &mut event_cx);
            }
            if !self.apply_event_context(event_loop, event_cx, false, true) {
                return;
            }
        }

        let Some(state) = &mut self.window else {
            return;
        };
        let started = FrameTimer::start();
        let mut declaration_time = Duration::ZERO;
        let mut scope_animation_frame = false;
        if !state.view_dirty && state.listeners.scopes.pending() {
            let previous_focus = state.ui.focused();
            let declaration_started = Instant::now();
            let mut cx = ViewContext::<()> {
                size: state.logical_size,
                scale_factor: state.scale_factor,
                metrics: state.metrics.current(),
                focused: state.ui.focused(),
                focused_path: state.ui.focus_path(),
                request_animation_frame: false,
                repaint_deadline: None,
                listeners: &mut state.listeners,
                window: window_handle,
                window_state,
                displays: &displays,
                keyboard_layout: &keyboard_layout,
                font_system: &font_system,
                assets: &assets,
                app_info: app_info.as_ref(),
                app_paths: app_paths.as_ref(),
                system_info: &system_info,
                system_preferences: &system_preferences,
                background_tasks: Some(&background_tasks),
                foreground_tasks: &foreground_tasks,
                globals: &globals,
                event_proxy: Some(&event_proxy),
                marker: PhantomData,
            };
            let updates = state.view.render_scopes(&mut cx);
            scope_animation_frame = cx.request_animation_frame;
            state.view_deadline = state
                .view_deadline
                .into_iter()
                .chain(cx.repaint_deadline)
                .min();
            declaration_time += declaration_started.elapsed();
            if let Some(mut updates) = updates {
                for update in &mut updates {
                    if let crate::ElementUpdate::Replace { element, .. } = update {
                        state.image_assets.resolve_subtree(element);
                    }
                }
                match state.ui.update_elements(&updates) {
                    Ok(Some(kind)) => {
                        state.layout_dirty |= kind == crate::ui_tree::ElementUpdateKind::Layout;
                        if state.layout_dirty {
                            mounted_focus_previous = Some(previous_focus);
                        }
                    }
                    Ok(None) => state.view_dirty = true,
                    Err(error) => {
                        self.fail(event_loop, AppError::View(error.to_string()));
                        return;
                    }
                }
            } else {
                state.view_dirty = true;
            }
        }
        let retained_scroll_only = scroll_result.changed
            && !scroll_result.view_dirty
            && !state.view_dirty
            && !state.layout_dirty;
        let retained_layout_only = state.layout_dirty && !state.view_dirty;
        let placement_changed = state.ui.take_retained_placement_dirty();
        let retained_geometry_changed =
            state.view_dirty || state.layout_dirty || scroll_result.changed || placement_changed;
        let retained_semantics_changed = state.ui.take_retained_semantics_dirty();
        if retained_semantics_changed {
            state.accessibility_updates.semantic_change();
        }
        let accessibility_geometry = if retained_semantics_changed {
            // A patched label changes its accessible value (and possibly its parent's name),
            // even though its frame takes the retained-layout path. Do not debounce semantics
            // as if this were only a live resize.
            None
        } else if retained_layout_only || placement_changed {
            Some(AccessibilityUpdateKind::LayoutGeometry)
        } else if retained_scroll_only {
            Some(AccessibilityUpdateKind::ScrollGeometry)
        } else {
            None
        };
        #[cfg(feature = "inspector")]
        let view_rebuilt = state.view_dirty || state.layout_dirty;
        let mut request_animation_frame = scope_animation_frame;
        if state.view_dirty {
            let declaration_started = Instant::now();
            let previous_mounted_focus = state.ui.focused();
            let focused_path = state.ui.focus_path();
            let (mut root, requested, repaint_deadline) = state.view.render(
                state.logical_size,
                state.scale_factor,
                state.metrics.current(),
                state.ui.focused(),
                focused_path,
                &mut state.listeners,
                window_handle,
                window_state,
                &displays,
                &keyboard_layout,
                &font_system,
                &assets,
                app_info.as_ref(),
                app_paths.as_ref(),
                &system_info,
                &system_preferences,
                Some(&background_tasks),
                &foreground_tasks,
                &globals,
                Some(&event_proxy),
            );
            declaration_time += declaration_started.elapsed();
            request_animation_frame = requested;
            state.view_deadline = repaint_deadline;
            state.image_assets.begin_resolve_tree(&mut root);
            let logical_size = state.logical_size;
            let scale_factor = state.scale_factor;
            let image_assets = &mut state.image_assets;
            let ui = &mut state.ui;
            let renderer = &mut state.renderer;
            if let Err(error) =
                ui.set_root_with_prepare(root, logical_size, scale_factor, renderer, |subtree| {
                    image_assets.resolve_subtree(subtree)
                })
            {
                self.fail(event_loop, AppError::View(error.to_string()));
                return;
            }
            let image_resolution_changed = image_assets.finish_resolve_frame();
            request_animation_frame |= image_resolution_changed;
            if let Some(request) = state.pending_focus.take()
                && state.ui.is_focusable(request.element)
            {
                state.ui.focus_as(request.element, request.modality);
            }
            if previous_mounted_focus != state.ui.focused() {
                mounted_focus_previous = Some(previous_mounted_focus);
            }
            state.layout_dirty = false;
            state.view_dirty = image_resolution_changed;
        } else if state.layout_dirty {
            let logical_size = state.logical_size;
            let scale_factor = state.scale_factor;
            let image_assets = &mut state.image_assets;
            let ui = &mut state.ui;
            let renderer = &mut state.renderer;
            if let Err(error) =
                ui.relayout_with_prepare(logical_size, scale_factor, renderer, |subtree| {
                    image_assets.resolve_subtree(subtree)
                })
            {
                self.fail(event_loop, AppError::View(error.to_string()));
                return;
            }
            request_animation_frame |= image_assets.finish_resolve_frame();
            state.layout_dirty = false;
        }
        #[cfg(all(target_os = "macos", feature = "swift-ui"))]
        if let Some(embedded) = state.embedded.clone() {
            let (match_horizontal, match_vertical) = embedded.match_axes();
            if match_horizontal || match_vertical {
                let measured = match state.ui.measure_intrinsic_content(
                    state.logical_size,
                    state.scale_factor,
                    match_horizontal,
                    match_vertical,
                    &mut state.renderer,
                ) {
                    Ok(size) => size,
                    Err(error) => {
                        self.fail(event_loop, AppError::View(error.to_string()));
                        return;
                    }
                };
                if embedded.update_content_size(measured) {
                    embedded_owner_size_changed = Some(embedded.owner);
                }
            }
        }
        let ime_target = state.ui.focused_text_input();
        if ime_target != state.ime_target {
            let previous_target = state.ime_target;
            if let Some(previous_target) = previous_target {
                state.ui.input_cancel_preedit(previous_target);
            }
            state.ime_target = ime_target;
            if previous_target.is_some() != ime_target.is_some() {
                state.window.set_ime_allowed(ime_target.is_some());
            }
        }

        state.ui.advance_animations(Instant::now());
        state.ui.advance_scrollbars(Instant::now());
        #[cfg(feature = "inspector")]
        let retained_hover_pointer = state.pointer.filter(|point| {
            !state
                .inspector
                .as_ref()
                .is_some_and(|inspector| inspector.captures_pointer(*point))
        });
        #[cfg(not(feature = "inspector"))]
        let retained_hover_pointer = state.pointer;
        if retained_geometry_changed
            && let Err(error) = state.ui.refresh_hover_after_layout(retained_hover_pointer)
        {
            self.fail(event_loop, AppError::View(error.to_string()));
            return;
        }
        state.scene.clear(self.config.background);
        if let Err(error) = state.ui.paint(&mut state.scene, &mut state.renderer) {
            self.fail(event_loop, AppError::View(error.to_string()));
            return;
        }
        state.ui.refresh_mouse_hover(retained_hover_pointer);
        // Paint rebuilds the retained hit stack even when only scrolling moved content. Resolve
        // once during this already-damaged frame so a stationary pointer cannot keep the cursor
        // belonging to the element that used to be underneath it.
        if let Some(point) = state.pointer {
            let cursor = desired_cursor(state, point);
            set_cursor_if_changed(state, cursor);
        }
        // Anchored placement is resolved while painting. When a surface actually flipped, the
        // application's arrow, transform origin, or available-space sizing is one frame behind, so
        // request exactly one correcting frame; an unchanged placement requests none.
        if state.ui.take_anchor_placement_update() {
            state.view_dirty = true;
        }
        let variable_list_measurement_update = state.ui.take_variable_list_measurement_update();
        let variable_list_measurements_changed = variable_list_measurement_update.changed;
        let declarative_animation_frame_requested =
            state.ui.declarative_animation_frame_requested();
        let detached_animation_frame_requested = state.ui.detached_animation_frame_requested();
        let style_transition_frame_requested = state.ui.style_transition_frame_requested();
        if variable_list_measurement_update.view_dirty {
            state.view_dirty = true;
        }
        #[cfg(feature = "inspector")]
        if state.inspector.is_some() {
            let metrics = state.metrics.current();
            let viewport = state.logical_size;
            let scale_factor = state.scale_factor;
            let damage = InspectorFrameDamage {
                view_rebuilt,
                retained_scroll_changed: scroll_result.changed,
                variable_measurements_changed: variable_list_measurements_changed,
                animation_requested: request_animation_frame,
                declarative_animation_requested: declarative_animation_frame_requested,
                detached_animation_requested: detached_animation_frame_requested,
                style_transition_requested: style_transition_frame_requested,
            };
            let RuntimeWindow {
                ui,
                inspector,
                scene,
                renderer,
                ..
            } = state;
            let inspector = inspector
                .as_mut()
                .expect("inspector presence checked before split borrow");
            inspector.refresh(ui, metrics, damage, viewport, scale_factor);
            if let Err(error) =
                inspector.paint(scene, renderer, viewport, scale_factor, Instant::now())
            {
                self.fail(event_loop, AppError::View(error.to_string()));
                return;
            }
        }
        #[cfg(target_os = "macos")]
        {
            let text_type = TypeId::of::<ExternalDragText>();
            let url_type = TypeId::of::<ExternalDragUrl>();
            let has_text = state
                .listeners
                .drop_order
                .iter()
                .any(|(_, value_type)| *value_type == text_type);
            let has_url = state
                .listeners
                .drop_order
                .iter()
                .any(|(_, value_type)| *value_type == url_type);
            let has_typed = !state.listeners.drop_order.is_empty();
            let RuntimeWindow {
                native_drop_host,
                ui,
                listeners,
                ..
            } = state;
            native_drop_host.update(has_text, has_url, has_typed, |snapshot| {
                ui.update_external_drop_snapshot(snapshot, &listeners.drop_order);
            });

            let has_native_views = !state.ui.native_views().is_empty();
            if has_native_views && state.native_host.is_none() {
                let host = match MacNativeHost::new(&state.window) {
                    Ok(host) => host,
                    Err(error) => {
                        self.fail(event_loop, AppError::View(error));
                        return;
                    }
                };
                if let Err(error) = state
                    .renderer
                    .enable_native_composition(host.overlay_pointer())
                {
                    self.fail(event_loop, AppError::Render(error.to_string()));
                    return;
                }
                state.native_host = Some(host);
            }
            if let Some(host) = &mut state.native_host {
                if let Err(error) = host.reconcile(state.ui.native_views()) {
                    self.fail(event_loop, AppError::View(error));
                    return;
                }
                let overlay_active = has_native_views
                    && (state.scene.has_content_in_plane(crate::ScenePlane::Overlay)
                        || state.ui.overlay_input_active());
                host.set_overlay_active(overlay_active);
                if let Err(error) = state.renderer.set_native_overlay_active(overlay_active) {
                    self.fail(event_loop, AppError::Render(error.to_string()));
                    return;
                }
            }
            state
                .renderer
                .set_native_composition_active(has_native_views);
            if let Some(guard) = state.first_frame_guard.as_ref() {
                guard.cover();
            }
        }
        let platform_content_attached = runtime_window_content_attached(state);
        if platform_content_attached
            && ime_target.is_some()
            && let Some(caret) = state.ui.ime_cursor_area()
        {
            state.window.set_ime_cursor_area(
                LogicalPosition::new(caret.x as f64, caret.y as f64),
                LogicalSize::new(caret.width.max(1.0) as f64, caret.height.max(1.0) as f64),
            );
        }
        let accessibility_started = Instant::now();
        if let Some(update_kind) = state
            .accessibility_updates
            .should_update(accessibility_geometry, Instant::now())
        {
            let window_title = self.config.title.as_str();
            let RuntimeWindow {
                accessibility, ui, ..
            } = state;
            accessibility.update_if_active(|| match update_kind {
                AccessibilityUpdateKind::ScrollGeometry => ui.accessibility_scroll_update(),
                AccessibilityUpdateKind::Full | AccessibilityUpdateKind::LayoutGeometry => {
                    ui.accessibility_update(window_title)
                }
            });
        }
        let accessibility_time = accessibility_started.elapsed();
        let render_started = Instant::now();
        match state.renderer.render(&state.scene, state.scale_factor) {
            Ok(RenderOutcome::Presented(mut stats)) => {
                #[cfg(target_os = "macos")]
                if state.first_frame_guard.is_some()
                    && let Err(error) = state.renderer.wait_for_submitted_work()
                {
                    self.fail(event_loop, AppError::Render(error.to_string()));
                    return;
                }
                let image_assets = state.image_assets.stats();
                stats.cpu_image_cache_bytes = image_assets.decoded_bytes;
                stats.image_resource_entries = image_assets.entries;
                stats.image_resources_loading = image_assets.loading;
                stats.image_resources_failed = image_assets.failed;
                (stats.animated_images, stats.active_animations) = state.ui.animation_counts();
                let mut pipeline = state.ui.take_work();
                pipeline.declaration_time = declaration_time;
                pipeline.accessibility_time = accessibility_time;
                pipeline.render_time = render_started.elapsed();
                state.metrics.record(started.elapsed(), stats, pipeline);
                #[cfg(target_os = "macos")]
                if let Some(guard) = state.first_frame_guard.take() {
                    guard.reveal();
                }
                if (request_animation_frame
                    || declarative_animation_frame_requested
                    || detached_animation_frame_requested
                    || style_transition_frame_requested
                    || variable_list_measurements_changed)
                    && state.scheduler.invalidate()
                {
                    state.view_dirty |=
                        request_animation_frame || declarative_animation_frame_requested;
                    state.window.request_redraw();
                }
            }
            Ok(RenderOutcome::Retry) => {
                state.view_dirty |=
                    request_animation_frame || declarative_animation_frame_requested;
                if state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
            }
            Ok(RenderOutcome::Occluded) => {
                // Wait for the platform to expose or resize the window; do not spin while hidden.
                // Keep view-owned frame requests dirty so the first exposed frame (including the
                // detached macOS first-present pass) can resume them instead of silently losing
                // an animation that was declared while the surface was unavailable.
                state.view_dirty |=
                    request_animation_frame || declarative_animation_frame_requested;
            }
            Err(error) => self.fail(event_loop, AppError::Render(error.to_string())),
        }
        if let Some(state) = self.window.as_mut()
            && !state.first_presented
            && state.metrics.current().frame_number > 0
        {
            state.first_presented = true;
            // Ready-to-show: the first frame is on screen, so a window created with
            // `WindowOptions::show(false)` can be revealed without a blank flash.
            #[cfg(target_arch = "wasm32")]
            if let Some(window) = web_sys::window() {
                if let Ok(event) = web_sys::CustomEvent::new("quickgui:ready") {
                    let _ = window.dispatch_event(&event);
                }
            }
            if !self.dispatch(event_loop, Event::FirstPresented, false) {
                return;
            }
        }
        if !self.invoke_pending_mouse_hover(event_loop) {
            return;
        }
        #[cfg(all(target_os = "macos", feature = "swift-ui"))]
        if let Some(owner) = embedded_owner_size_changed
            && !self.invalidate_requests.contains(&owner)
        {
            self.invalidate_requests.push(owner);
        }
        if let Some(previous) = mounted_focus_previous {
            self.announce_focus_change(event_loop, previous);
        } else {
            #[cfg(target_os = "macos")]
            self.sync_native_menu_state();
        }
    }
}
