use super::*;

impl Runtime {
    pub(super) fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if !self.activate_window(window_id) {
            return;
        }
        #[cfg(target_os = "macos")]
        let (event, platform_click_count) = match event {
            WindowEvent::MouseInputWithClickCount {
                device_id,
                state,
                button,
                click_count,
            } => (
                WindowEvent::MouseInput {
                    device_id,
                    state,
                    button,
                },
                Some(usize::from(click_count)),
            ),
            event => (event, None),
        };
        #[cfg(not(target_os = "macos"))]
        let platform_click_count: Option<usize> = None;
        (|| {
            if let Some(state) = self.window.as_mut()
                && state.window.id() == window_id
            {
                state.accessibility.process_event(&state.window, &event);
            }
            let Some(state) = self.window.as_ref() else {
                return;
            };
            if state.window.id() != window_id {
                return;
            }

            match event {
                WindowEvent::CloseRequested => {
                    let mut cx = self.event_context();
                    if let Some(window) = &mut self.window {
                        window.view.event(&Event::CloseRequested, &mut cx);
                    }
                    let prevent_close = cx.prevent_close;
                    let explicitly_closed = cx.close_current_window;
                    if !self.apply_event_context(event_loop, cx, false, true) {
                        return;
                    }
                    if !prevent_close
                        && !explicitly_closed
                        && let Some(handle) = self.current_handle()
                    {
                        self.close_requests.push(handle);
                    }
                }
                WindowEvent::Moved(physical) => {
                    let proposed = {
                        let state = self.window.as_ref().expect("window checked above");
                        Point::new(
                            physical.x as f32 / state.scale_factor,
                            physical.y as f32 / state.scale_factor,
                        )
                    };
                    if !self.apply_move_constraint(event_loop, proposed) {
                        return;
                    }
                    let Some(state) = self.window.as_mut() else {
                        return;
                    };
                    state.logical_position = proposed;
                    state.display_id = runtime_window_display_id(state, &self.displays);
                    state.maximized = runtime_window_is_maximized(state, &self.config);
                    if !runtime_window_is_fullscreen(state) && !state.maximized {
                        state.restore_bounds.x = state.logical_position.x;
                        state.restore_bounds.y = state.logical_position.y;
                    }
                    let logical_position = state.logical_position;
                    let scale_factor = state.scale_factor;
                    let observe = state.listeners.observes_window_state;
                    #[cfg(target_os = "macos")]
                    self.refresh_current_native_tab_state();
                    if !self.refresh_window_lifecycle(event_loop) {
                        return;
                    }
                    self.dispatch(
                        event_loop,
                        Event::Moved {
                            logical_position,
                            scale_factor,
                        },
                        observe,
                    );
                }
                WindowEvent::ThemeChanged(theme) => {
                    let appearance = map_window_appearance(theme);
                    let color_scheme = match appearance {
                        WindowAppearance::Light => ColorScheme::Light,
                        WindowAppearance::Dark => ColorScheme::Dark,
                    };
                    self.refresh_system_preferences(SystemPreferences::snapshot().unwrap_or_else(
                        |error| {
                            tracing::warn!(%error, "could not refresh native system colors");
                            self.system_preferences.with_color_scheme(color_scheme)
                        },
                    ));
                    if self.config.preferred_appearance.is_none() {
                        let state = self.window.as_mut().expect("window checked above");
                        if state.appearance != appearance {
                            state.appearance = appearance;
                            let observe = state.listeners.observes_window_state;
                            self.dispatch(
                                event_loop,
                                Event::AppearanceChanged(appearance),
                                observe,
                            );
                        }
                    }
                }
                WindowEvent::Resized(physical) => {
                    let proposed = {
                        let state = self.window.as_ref().expect("window checked above");
                        logical_window_size(physical, state.scale_factor)
                    };
                    if !self.apply_resize_constraint(event_loop, proposed) {
                        return;
                    }
                    let Some(state) = self.window.as_mut() else {
                        return;
                    };
                    state.renderer.resize(physical.width, physical.height);
                    state.logical_size = logical_window_size(physical, state.scale_factor);
                    state.display_id = runtime_window_display_id(state, &self.displays);
                    state.maximized = runtime_window_is_maximized(state, &self.config);
                    if !runtime_window_is_fullscreen(state) && !state.maximized {
                        state.restore_bounds.width = state.logical_size.width;
                        state.restore_bounds.height = state.logical_size.height;
                    }
                    #[cfg(target_os = "macos")]
                    if let Some(position) = self.config.traffic_light_position
                        && let Err(error) = position_traffic_lights(&state.window, position)
                    {
                        tracing::warn!(%error, "could not restore the configured traffic-light position");
                    }
                    #[cfg(target_os = "macos")]
                    if let Err(error) =
                        set_window_movable(&state.window, implicit_native_movable(&self.config))
                    {
                        tracing::warn!(%error, "could not restore native window movability");
                    }
                    let logical_size = state.logical_size;
                    let scale_factor = state.scale_factor;
                    state.layout_dirty = true;
                    state.view_dirty |= state.listeners.observes_viewport;
                    #[cfg(target_os = "macos")]
                    self.refresh_current_native_tab_state();
                    if !self.refresh_window_lifecycle(event_loop) {
                        return;
                    }
                    self.dispatch(
                        event_loop,
                        Event::Resized {
                            logical_size,
                            scale_factor,
                        },
                        true,
                    );
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    let state = self.window.as_mut().expect("window checked above");
                    state.scale_factor = sane_scale_factor(scale_factor);
                    let physical = state.window.inner_size();
                    state.renderer.resize(physical.width, physical.height);
                    state.logical_size = logical_window_size(physical, state.scale_factor);
                    if let Some(position) =
                        logical_window_position(&state.window, state.scale_factor)
                    {
                        state.logical_position = position;
                    }
                    state.display_id = runtime_window_display_id(state, &self.displays);
                    state.maximized = runtime_window_is_maximized(state, &self.config);
                    if !runtime_window_is_fullscreen(state) && !state.maximized {
                        state.restore_bounds = Rect::new(
                            state.logical_position.x,
                            state.logical_position.y,
                            state.logical_size.width,
                            state.logical_size.height,
                        );
                    }
                    #[cfg(target_os = "macos")]
                    if let Some(position) = self.config.traffic_light_position
                        && let Err(error) = position_traffic_lights(&state.window, position)
                    {
                        tracing::warn!(%error, "could not restore the configured traffic-light position");
                    }
                    #[cfg(target_os = "macos")]
                    if let Err(error) =
                        set_window_movable(&state.window, implicit_native_movable(&self.config))
                    {
                        tracing::warn!(%error, "could not restore native window movability");
                    }
                    let logical_size = state.logical_size;
                    let scale_factor = state.scale_factor;
                    state.layout_dirty = true;
                    state.view_dirty |= state.listeners.observes_viewport;
                    #[cfg(target_os = "macos")]
                    self.refresh_current_native_tab_state();
                    self.dispatch(
                        event_loop,
                        Event::Resized {
                            logical_size,
                            scale_factor,
                        },
                        true,
                    );
                }
                WindowEvent::Occluded(false) => {
                    let state = self.window.as_mut().expect("window checked above");
                    let changed = state.occluded;
                    state.occluded = false;
                    if state.listeners.observes_window_state
                        || state.ui.has_declarative_animations()
                    {
                        state.view_dirty = true;
                    }
                    state.ui.set_reduce_motion(state.reduce_motion);
                    state
                        .ui
                        .set_animations_enabled(!state.reduce_motion, Instant::now());
                    state.scheduler.invalidate();
                    state.window.request_redraw();
                    if changed && !self.dispatch(event_loop, Event::OcclusionChanged(false), false)
                    {
                        return;
                    }
                    self.refresh_window_lifecycle(event_loop);
                }
                WindowEvent::Occluded(true) => {
                    let state = self.window.as_mut().expect("window checked above");
                    let changed = !state.occluded;
                    state.occluded = true;
                    state.ui.set_animations_enabled(false, Instant::now());
                    if changed && !self.dispatch(event_loop, Event::OcclusionChanged(true), false) {
                        return;
                    }
                    // AppKit reports miniaturization as occlusion rather than as a Winit window
                    // event, so this is where a Dock minimize becomes `Event::Minimized`.
                    self.refresh_window_lifecycle(event_loop);
                }
                WindowEvent::RedrawRequested => {
                    #[cfg(target_os = "windows")]
                    if let Some(state) = &mut self.window
                        && state.visible
                        && !state.taskbar_state_applied
                        && state.taskbar_apply_attempts < 3
                    {
                        state.taskbar_apply_attempts += 1;
                        let progress = windows_window::set_taskbar_progress(
                            &state.window,
                            self.config.taskbar_progress_state,
                            self.config.taskbar_progress,
                        );
                        let overlay = windows_window::set_taskbar_overlay_icon(
                            &state.window,
                            self.config.taskbar_overlay_icon.as_ref(),
                            self.config.taskbar_overlay_description.as_deref(),
                        );
                        state.taskbar_state_applied = progress.is_ok() && overlay.is_ok();
                        if !state.taskbar_state_applied {
                            if state.taskbar_apply_attempts < 3 {
                                state.window.request_redraw();
                            } else {
                                let error =
                                    progress.err().or_else(|| overlay.err()).unwrap_or_default();
                                tracing::warn!(%error, "could not apply retained taskbar state");
                            }
                        }
                    }
                    self.redraw(event_loop)
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let scale = self
                        .window
                        .as_ref()
                        .expect("window checked above")
                        .scale_factor;
                    let point = Point::new(position.x as f32 / scale, position.y as f32 / scale);
                    if let Some(state) = &mut self.window {
                        state.pointer = Some(point);
                    }
                    #[cfg(feature = "inspector")]
                    {
                        let consumed = self.window.as_ref().is_some_and(|state| {
                            state
                                .inspector
                                .as_ref()
                                .is_some_and(|inspector| inspector.captures_pointer(point))
                        });
                        if consumed {
                            let state = self.window.as_mut().expect("window checked above");
                            let inspector_changed = state
                                .inspector
                                .as_mut()
                                .is_some_and(|inspector| inspector.pointer_moved(point));
                            let app_changed = state.ui.pointer_left()
                                | state.ui.update_scrollbar_hover(None, Instant::now());
                            let cursor =
                                state
                                    .inspector
                                    .as_ref()
                                    .map_or(CursorIcon::Default, |inspector| {
                                        if inspector.mode() == InspectorMode::Picking
                                            && !inspector.panel_contains(point)
                                        {
                                            CursorIcon::Crosshair
                                        } else {
                                            CursorIcon::Default
                                        }
                                    });
                            set_cursor_if_changed(state, cursor);
                            if (inspector_changed || app_changed) && state.scheduler.invalidate() {
                                state.window.request_redraw();
                            }
                            if !self.invoke_pending_mouse_hover(event_loop) {
                                return;
                            }
                            return;
                        }
                    }
                    let drag_start = self.window.as_ref().and_then(|state| {
                        let candidate = state.drag_candidate?;
                        let delta = point - candidate.origin;
                        (state.drag_session.is_none()
                            && state.pointer_capture.is_none()
                            && delta.x.abs().max(delta.y.abs()) >= DRAG_THRESHOLD)
                            .then_some((candidate.source, candidate.origin))
                    });
                    if let Some((source, origin)) = drag_start
                        && !self.invoke_drag_start(
                            event_loop,
                            source,
                            DragStartEvent {
                                origin,
                                position: point,
                                modifiers: self.modifiers,
                            },
                        )
                    {
                        return;
                    }
                    #[cfg(target_os = "macos")]
                    if self.window.as_ref().is_some_and(|state| {
                        state.drag_session.is_some()
                            && point_outside_viewport(point, state.logical_size)
                    }) && self.promote_external_drag()
                    {
                        if let Some(state) = &mut self.window {
                            state.pointer = None;
                        }
                        self.dispatch(event_loop, Event::PointerLeft, false);
                        return;
                    }
                    let target_changed = self.update_drag_target(point);
                    let state = self.window.as_mut().expect("window checked above");
                    if let Some(drag) = &mut state.drag_session {
                        drag.position = point;
                    }
                    let preview_changed = state.ui.move_drag_preview(point);
                    let now = Instant::now();
                    let captured = state.pointer_capture.as_mut().map(|capture| {
                        let delta = point - capture.position;
                        capture.position = point;
                        (
                            capture.target,
                            PointerEvent {
                                // The captured element localizes the event before delivery.
                                size: Size::ZERO,
                                phase: PointerPhase::Move,
                                position: point,
                                origin: capture.origin,
                                local_position: point,
                                local_origin: capture.origin,
                                delta,
                                button: capture.button,
                                modifiers: self.modifiers,
                            },
                        )
                    });
                    let drag_active = state.any_drag_active();
                    let scrollbar_dragging = !drag_active && state.ui.scrollbar_drag_active();
                    let (over_scrollbar, app_region_drag, repaint) = if drag_active {
                        let hover_changed = state.ui.update_scrollbar_hover(None, now);
                        let RuntimeWindow { ui, renderer, .. } = state;
                        (
                            false,
                            false,
                            target_changed
                                | preview_changed
                                | hover_changed
                                | ui.pointer_moved(point, renderer),
                        )
                    } else if scrollbar_dragging {
                        let scroll_result = state.ui.drag_scrollbar(point);
                        if scroll_result.view_dirty {
                            state.view_dirty = true;
                        }
                        let repaint = scroll_result.changed | state.ui.pointer_left();
                        (true, false, repaint)
                    } else if captured.is_none() {
                        let hover_changed = state.ui.update_scrollbar_hover(Some(point), now);
                        let over_scrollbar = state.ui.is_over_scrollbar(point);
                        let app_region_drag = !over_scrollbar && state.ui.is_app_region_drag(point);
                        let pointer_changed = if over_scrollbar || app_region_drag {
                            state.ui.pointer_left()
                        } else {
                            let RuntimeWindow { ui, renderer, .. } = state;
                            ui.pointer_moved(point, renderer)
                        };
                        (
                            over_scrollbar,
                            app_region_drag,
                            hover_changed | pointer_changed,
                        )
                    } else {
                        let hover_changed = state.ui.update_scrollbar_hover(None, now);
                        let RuntimeWindow { ui, renderer, .. } = state;
                        (
                            false,
                            false,
                            hover_changed | ui.pointer_moved(point, renderer),
                        )
                    };
                    let cursor = desired_cursor(state, point);
                    set_cursor_if_changed(state, cursor);
                    let pressed_button = state.pressed_mouse_buttons.current();
                    if repaint && state.scheduler.invalidate() {
                        state.window.request_redraw();
                    }
                    if !self.invoke_pending_mouse_hover(event_loop) {
                        return;
                    }
                    if over_scrollbar || scrollbar_dragging || app_region_drag {
                        // Keep window-level pointer tracking coherent while element-level hit
                        // testing remains occluded by the scrollbar or drag region.
                        self.dispatch(event_loop, Event::PointerMoved(point), false);
                        return;
                    }
                    if let Some((target, event)) = captured
                        && !self.invoke_pointer(event_loop, target, event)
                    {
                        return;
                    }
                    if self
                        .invoke_mouse_event_at(
                            event_loop,
                            point,
                            MouseListenerKind::Move,
                            None,
                            MouseListenerEvent::Move(MouseMoveEvent {
                                position: point,
                                pressed_button,
                                modifiers: self.modifiers,
                            }),
                        )
                        .is_none()
                    {
                        return;
                    }
                    self.dispatch(event_loop, Event::PointerMoved(point), false);
                }
                WindowEvent::CursorLeft { .. } => {
                    // Preserve the last retained in-window point for element-level exit dispatch.
                    // The macOS external-drag probe below samples the hardware boundary and may
                    // temporarily replace `state.pointer` with an out-of-window coordinate.
                    let exit_position = self
                        .window
                        .as_ref()
                        .and_then(|state| state.pointer)
                        .unwrap_or(Point::ZERO);
                    #[cfg(target_os = "macos")]
                    {
                        let boundary_point = self
                            .window
                            .as_ref()
                            .and_then(|state| current_pointer_position(&state.window));
                        if let Some(point) = boundary_point {
                            match self.promote_external_drag_at_boundary(event_loop, point) {
                                Some(true) => return,
                                Some(false) => {}
                                None => return,
                            }
                        } else {
                            self.promote_external_drag();
                        }
                    }
                    let state = self.window.as_mut().expect("window checked above");
                    let pressed_button = state.pressed_mouse_buttons.current();
                    state.pointer = None;
                    let cursor = state
                        .pointer_capture
                        .map_or(CursorIcon::Default, |capture| capture.cursor);
                    set_cursor_if_changed(state, cursor);
                    let repaint = state.ui.pointer_left()
                        | state.ui.update_scrollbar_hover(None, Instant::now())
                        | state.ui.set_drag_over(None)
                        | {
                            #[cfg(feature = "inspector")]
                            {
                                state
                                    .inspector
                                    .as_mut()
                                    .is_some_and(InspectorState::pointer_left)
                            }
                            #[cfg(not(feature = "inspector"))]
                            {
                                false
                            }
                        };
                    if repaint && state.scheduler.invalidate() {
                        state.window.request_redraw();
                    }
                    if !self.invoke_pending_mouse_hover(event_loop) {
                        return;
                    }
                    if self
                        .invoke_mouse_event_at(
                            event_loop,
                            exit_position,
                            MouseListenerKind::Exit,
                            None,
                            MouseListenerEvent::Exit(MouseExitEvent {
                                position: exit_position,
                                pressed_button,
                                modifiers: self.modifiers,
                            }),
                        )
                        .is_none()
                    {
                        return;
                    }
                    self.dispatch(event_loop, Event::PointerLeft, false);
                }
                WindowEvent::HoveredFile(path) => {
                    self.refresh_native_file_pointer();
                    self.hover_native_file(path);
                }
                WindowEvent::DroppedFile(path) => {
                    self.refresh_native_file_pointer();
                    self.drop_native_file(event_loop, path);
                }
                WindowEvent::HoveredFileCancelled => {
                    self.cancel_native_file_hover(event_loop);
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let pressed = state == ElementState::Pressed;
                    let button = map_mouse_button(button);
                    // Every focus change this press or release causes — a listener's `cx.focus`,
                    // the retained press default, the click it completes — is pointer-driven, so
                    // one scope covers the whole arm and each early exit leaves through the block
                    // instead of returning past the scope's end.
                    let scope = self.begin_input_dispatch(InputModality::Pointer);
                    'mouse_input: {
                        #[cfg(feature = "inspector")]
                        {
                            let (consumed, mut repaint, action) = {
                                let window = self.window.as_mut().expect("window checked above");
                                let point = window.pointer.unwrap_or(Point::ZERO);
                                window.inspector.as_mut().map_or(
                                    (false, false, InspectorPointerAction::None),
                                    |inspector| {
                                        inspector.pointer_button(
                                            point,
                                            pressed,
                                            button == MouseButton::Left,
                                        )
                                    },
                                )
                            };
                            if consumed {
                                match action {
                                    InspectorPointerAction::None => {}
                                    InspectorPointerAction::StartPicking => {
                                        repaint |= self
                                            .window
                                            .as_mut()
                                            .and_then(|window| window.inspector.as_mut())
                                            .is_some_and(InspectorState::start_picking);
                                    }
                                    InspectorPointerAction::Close => {
                                        self.config.inspector = false;
                                        if let Some(window) = &mut self.window {
                                            window.inspector = None;
                                            reconcile_inspector_pointer_state(window);
                                            if window.listeners.observes_window_state {
                                                window.view_dirty = true;
                                            }
                                        }
                                        repaint = true;
                                    }
                                }
                                let window = self.window.as_mut().expect("window retained");
                                if repaint && window.scheduler.invalidate() {
                                    window.window.request_redraw();
                                }
                                break 'mouse_input;
                            }
                        }
                        #[cfg(target_os = "macos")]
                        let native_click_count = platform_click_count;
                        #[cfg(not(target_os = "macos"))]
                        let native_click_count = platform_click_count;
                        let (mouse_position, click_count, first_mouse) = {
                            let window = self.window.as_mut().expect("window checked above");
                            let position = window.pointer;
                            let click_position = position.unwrap_or(Point::ZERO);
                            let first_mouse = !window.focused;
                            let click_count = if pressed {
                                window.pressed_mouse_buttons.press(button);
                                window.mouse_clicks.press(
                                    button,
                                    click_position,
                                    Instant::now(),
                                    native_click_count,
                                )
                            } else {
                                let count = window.mouse_clicks.release(button, native_click_count);
                                window.pressed_mouse_buttons.release(button);
                                count
                            };
                            (position, click_count, first_mouse)
                        };
                        #[cfg(target_os = "macos")]
                        let suppress_external_release = if button == MouseButton::Left {
                            let window = self.window.as_mut().expect("window checked above");
                            if pressed {
                                window.suppress_external_drag_release = false;
                                if let Some(monitor) = &window.external_drag_monitor {
                                    monitor.disarm();
                                }
                                window.external_drag_mouse_down = None;
                                false
                            } else {
                                window.external_drag_mouse_down = None;
                                if let Some(monitor) = &window.external_drag_monitor {
                                    monitor.disarm();
                                }
                                std::mem::take(&mut window.suppress_external_drag_release)
                            }
                        } else {
                            false
                        };
                        #[cfg(target_os = "macos")]
                        if suppress_external_release {
                            // Preserve raw button symmetry for application event handlers, but do not
                            // route the native drag's terminal release back through retained hit
                            // testing where it could activate a control under the drop position.
                            self.dispatch(
                                event_loop,
                                Event::MouseButton { button, pressed },
                                false,
                            );
                            break 'mouse_input;
                        }
                        if pressed {
                            let window = self.window.as_mut().expect("window checked above");
                            if window.ui.clear_tooltip() && window.scheduler.invalidate() {
                                window.window.request_redraw();
                            }
                        }
                        let scrollbar_consumed = {
                            let window = self.window.as_mut().expect("window checked above");
                            if window.pointer_capture.is_some() || window.any_drag_active() {
                                false
                            } else {
                                let now = Instant::now();
                                let consumed = if button == MouseButton::Left
                                    && window.ui.scrollbar_drag_active()
                                {
                                    if !pressed {
                                        let result = window.ui.end_scrollbar_drag(now);
                                        window.view_dirty |= result.view_dirty;
                                        window.ui.update_scrollbar_hover(window.pointer, now);
                                    }
                                    true
                                } else if button == MouseButton::Left && pressed {
                                    if let Some(view_dirty) =
                                        window.ui.begin_scrollbar_drag(window.pointer)
                                    {
                                        window.view_dirty |= view_dirty;
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    window
                                        .pointer
                                        .is_some_and(|point| window.ui.is_over_scrollbar(point))
                                };
                                if consumed && window.scheduler.invalidate() {
                                    window.window.request_redraw();
                                }
                                consumed
                            }
                        };
                        if scrollbar_consumed {
                            break 'mouse_input;
                        }
                        let app_region_consumed = self.window.as_ref().is_some_and(|window| {
                            window.pointer_capture.is_none()
                                && !window.any_drag_active()
                                && window
                                    .pointer
                                    .is_some_and(|point| window.ui.is_app_region_drag(point))
                        });
                        if app_region_consumed {
                            if button == MouseButton::Left
                                && pressed
                                && self.config.is_movable
                                && let Some(window) = self.window.as_ref()
                            {
                                #[cfg(target_os = "macos")]
                                let result = perform_window_drag(
                                    &window.window,
                                    self.config.title_bar_style != TitleBarStyle::Default,
                                );
                                #[cfg(not(target_os = "macos"))]
                                let result = window
                                    .window
                                    .drag_window()
                                    .map_err(|error| error.to_string());
                                if let Err(error) = result {
                                    tracing::warn!(%error, "could not start window drag from app region");
                                }
                                #[cfg(target_os = "macos")]
                                if let Some(window) = &mut self.window {
                                    window.external_drag_mouse_down = None;
                                    if let Some(monitor) = &window.external_drag_monitor {
                                        monitor.disarm();
                                    }
                                }
                            }
                            break 'mouse_input;
                        }
                        // A direct captured-pointer release is terminal cleanup, not a preventable
                        // default. Deliver it before general mouse-up callbacks so closing or
                        // preventing from those callbacks cannot strand capture.
                        let terminal_capture = if pressed {
                            None
                        } else {
                            let window = self.window.as_mut().expect("window checked above");
                            window.drag_candidate = None;
                            window
                                .pointer_capture
                                .filter(|capture| capture.button == button)
                                .map(|capture| {
                                    window.pointer_capture = None;
                                    let position = window.pointer.unwrap_or(capture.position);
                                    (
                                        capture.target,
                                        PointerEvent {
                                            // The captured element localizes the event before delivery.
                                            size: Size::ZERO,
                                            phase: PointerPhase::Up,
                                            position,
                                            origin: capture.origin,
                                            local_position: position,
                                            local_origin: capture.origin,
                                            delta: position - capture.position,
                                            button,
                                            modifiers: self.modifiers,
                                        },
                                    )
                                })
                        };
                        if let Some((target, event)) = terminal_capture
                            && !self.invoke_pointer(event_loop, target, event)
                        {
                            break 'mouse_input;
                        }
                        if terminal_capture.is_some()
                            && let Some(window) = &mut self.window
                        {
                            let cursor = window
                                .pointer
                                .map_or(CursorIcon::Default, |point| desired_cursor(window, point));
                            set_cursor_if_changed(window, cursor);
                        }

                        let default_prevented = if let Some(position) = mouse_position {
                            let result = if pressed {
                                self.invoke_mouse_event_at(
                                    event_loop,
                                    position,
                                    MouseListenerKind::Down,
                                    Some(button),
                                    MouseListenerEvent::Down(MouseDownEvent {
                                        button,
                                        position,
                                        modifiers: self.modifiers,
                                        click_count,
                                        first_mouse,
                                    }),
                                )
                            } else {
                                self.invoke_mouse_event_at(
                                    event_loop,
                                    position,
                                    MouseListenerKind::Up,
                                    Some(button),
                                    MouseListenerEvent::Up(MouseUpEvent {
                                        button,
                                        position,
                                        modifiers: self.modifiers,
                                        click_count,
                                    }),
                                )
                            };
                            let Some(default_prevented) = result else {
                                break 'mouse_input;
                            };
                            default_prevented
                        } else {
                            false
                        };

                        let internal_drag_release = (button == MouseButton::Left && !pressed)
                            .then(|| {
                                self.window
                                    .as_ref()
                                    .filter(|window| window.drag_session.is_some())
                                    .and_then(|window| {
                                        window.pointer.or_else(|| {
                                            window.drag_session.as_ref().map(|drag| drag.position)
                                        })
                                    })
                            })
                            .flatten();
                        if let Some(position) = internal_drag_release {
                            if !self.finish_internal_drag(event_loop, position) {
                                break 'mouse_input;
                            }
                            self.dispatch(
                                event_loop,
                                Event::MouseButton { button, pressed },
                                false,
                            );
                            break 'mouse_input;
                        }
                        if default_prevented {
                            if !pressed {
                                let window = self.window.as_mut().expect("window checked above");
                                if window.ui.cancel_pointer_interaction()
                                    && window.scheduler.invalidate()
                                {
                                    window.window.request_redraw();
                                }
                            }
                            self.dispatch(
                                event_loop,
                                Event::MouseButton { button, pressed },
                                false,
                            );
                            break 'mouse_input;
                        }
                        if button == MouseButton::Right && pressed {
                            let (target, position, dismiss) = self
                                .window
                                .as_ref()
                                .and_then(|window| {
                                    let position = window.pointer?;
                                    Some((
                                        window.ui.context_menu_listener_at(position),
                                        position,
                                        window.ui.dismiss_request_for_pointer(Some(position)),
                                    ))
                                })
                                .unwrap_or((None, Point::ZERO, None));
                            if let Some(dismiss) = dismiss {
                                self.invoke_dismiss(event_loop, dismiss);
                                if self.window.is_none() {
                                    break 'mouse_input;
                                }
                            }
                            if let Some(target) = target {
                                self.dispatch(
                                    event_loop,
                                    Event::MouseButton { button, pressed },
                                    false,
                                );
                                self.invoke_context_menu(
                                    event_loop,
                                    ContextMenuEvent {
                                        target,
                                        position,
                                        modifiers: self.modifiers,
                                    },
                                );
                                break 'mouse_input;
                            }
                        }
                        let (pointer_result, previous_focus, captured) = {
                            let window = self.window.as_mut().expect("window checked above");
                            let previous_focus = window.ui.focused();
                            let result = if button == MouseButton::Left {
                                let RuntimeWindow { ui, renderer, .. } = window;
                                ui.pointer_button(
                                    window.pointer,
                                    pressed,
                                    self.modifiers.contains(Modifiers::SHIFT),
                                    Instant::now(),
                                    renderer,
                                )
                            } else {
                                crate::ui_tree::PointerResult {
                                    open_url: None,
                                    repaint: false,
                                    clicked: None,
                                    dismissed: None,
                                    pointer_listener: window
                                        .pointer
                                        .and_then(|point| window.ui.pointer_listener_at(point)),
                                    drag_source: None,
                                }
                            };
                            let captured = if pressed {
                                if window.pointer_capture.is_none() {
                                    result.pointer_listener.and_then(|target| {
                                        let position = window.pointer?;
                                        let capture = PointerCapture {
                                            target,
                                            button,
                                            origin: position,
                                            position,
                                            cursor: window
                                                .ui
                                                .cursor_style_at(position)
                                                .map_or(CursorIcon::Default, platform_cursor),
                                        };
                                        window.pointer_capture = Some(capture);
                                        Some((
                                            target,
                                            PointerEvent {
                                                // The captured element localizes the event before delivery.
                                                size: Size::ZERO,
                                                phase: PointerPhase::Down,
                                                position,
                                                origin: position,
                                                local_position: position,
                                                local_origin: position,
                                                delta: Vector::ZERO,
                                                button,
                                                modifiers: self.modifiers,
                                            },
                                        ))
                                    })
                                } else {
                                    None
                                }
                            } else {
                                window.drag_candidate = None;
                                window
                                    .pointer_capture
                                    .filter(|capture| capture.button == button)
                                    .map(|capture| {
                                        window.pointer_capture = None;
                                        let position = window.pointer.unwrap_or(capture.position);
                                        (
                                            capture.target,
                                            PointerEvent {
                                                // The captured element localizes the event before delivery.
                                                size: Size::ZERO,
                                                phase: PointerPhase::Up,
                                                position,
                                                origin: capture.origin,
                                                local_position: position,
                                                local_origin: capture.origin,
                                                delta: position - capture.position,
                                                button,
                                                modifiers: self.modifiers,
                                            },
                                        )
                                    })
                            };
                            if button == MouseButton::Left && pressed {
                                window.drag_candidate = result.drag_source.and_then(|source| {
                                    window
                                        .pointer
                                        .map(|origin| DragCandidate { source, origin })
                                });
                            }
                            if result.repaint && window.scheduler.invalidate() {
                                window.window.request_redraw();
                            }
                            if let Some(point) = window.pointer {
                                let cursor = desired_cursor(window, point);
                                set_cursor_if_changed(window, cursor);
                            }
                            (result, previous_focus, captured)
                        };
                        #[cfg(target_os = "macos")]
                        if button == MouseButton::Left
                            && pressed
                            && pointer_result.drag_source.is_some()
                        {
                            if let Some(window) = &mut self.window {
                                window.external_drag_mouse_down = capture_left_mouse_down();
                            }
                            self.arm_external_drag_monitor();
                        }
                        self.announce_focus_change(event_loop, previous_focus);
                        if let Some((target, event)) = captured
                            && !self.invoke_pointer(event_loop, target, event)
                        {
                            break 'mouse_input;
                        }
                        self.dispatch(event_loop, Event::MouseButton { button, pressed }, false);
                        if let Some(request) = pointer_result.dismissed {
                            self.invoke_dismiss(event_loop, request);
                        }
                        if let Some(url) = pointer_result.open_url
                            && let Ok(request) = PlatformRequest::open_url(url)
                        {
                            enqueue_platform_requests(
                                &mut self.platform_requests,
                                &mut vec![request],
                            );
                            self.process_platform_requests();
                        }
                        if let Some(id) = pointer_result.clicked {
                            self.invoke_click(event_loop, id);
                        }
                    }
                    self.end_input_dispatch(scope);
                }
                WindowEvent::Touch(touch) => {
                    let event = {
                        let state = self.window.as_ref().expect("window checked above");
                        TouchEvent {
                            id: TouchId(touch.id),
                            phase: map_touch_phase(touch.phase),
                            position: Point::new(
                                touch.location.x as f32 / state.scale_factor,
                                touch.location.y as f32 / state.scale_factor,
                            ),
                            force: touch.force.map(bounded_touch_force),
                        }
                        .bounded()
                    };
                    let target = {
                        let state = self.window.as_mut().expect("window checked above");
                        match event.phase {
                            TouchPhase::Started => {
                                state.touch_captures.remove(&event.id);
                                let target = state.ui.touch_listener_at(event.position);
                                if let Some(target) = target
                                    && state.touch_captures.len() < MAX_ACTIVE_TOUCHES_PER_WINDOW
                                {
                                    state.touch_captures.insert(
                                        event.id,
                                        TouchCapture {
                                            target,
                                            last_event: event,
                                        },
                                    );
                                    Some(target)
                                } else {
                                    None
                                }
                            }
                            TouchPhase::Moved => {
                                state.touch_captures.get_mut(&event.id).map(|capture| {
                                    capture.last_event = event;
                                    capture.target
                                })
                            }
                            TouchPhase::Ended | TouchPhase::Cancelled => state
                                .touch_captures
                                .remove(&event.id)
                                .map(|capture| capture.target),
                        }
                    };
                    self.invoke_touch(event_loop, target, event);
                }
                WindowEvent::MouseWheel { delta, phase, .. } => {
                    let (position, scale_factor, target) = {
                        let state = self.window.as_ref().expect("window checked above");
                        if state
                            .pointer
                            .is_some_and(|point| state.ui.is_app_region_drag(point))
                        {
                            return;
                        }
                        let position = state.pointer.unwrap_or(Point::ZERO);
                        let target = state
                            .pointer
                            .and_then(|point| state.ui.scroll_wheel_listener_at(point));
                        (position, state.scale_factor, target)
                    };
                    let delta = match delta {
                        MouseScrollDelta::LineDelta(x, y) => ScrollDelta::Lines(Vector::new(x, y)),
                        MouseScrollDelta::PixelDelta(delta) => ScrollDelta::Pixels(Vector::new(
                            delta.x as f32 / scale_factor,
                            delta.y as f32 / scale_factor,
                        )),
                    };
                    let event = ScrollWheelEvent {
                        position,
                        delta,
                        phase: map_gesture_phase(phase),
                        modifiers: self.modifiers,
                    }
                    .bounded();
                    #[cfg(feature = "inspector")]
                    if let Some((consumed, changed)) = self.window.as_mut().and_then(|state| {
                        let inspector = state.inspector.as_mut()?;
                        Some(inspector.scroll(
                            position,
                            event.delta.pixel_delta(self.config.line_scroll_pixels),
                        ))
                    }) && consumed
                    {
                        let state = self.window.as_mut().expect("window retained");
                        if changed && state.scheduler.invalidate() {
                            state.window.request_redraw();
                        }
                        return;
                    }
                    if let Some(target) = target {
                        let Some(default_prevented) =
                            self.invoke_scroll_wheel(event_loop, target, event)
                        else {
                            return;
                        };
                        if default_prevented {
                            return;
                        }
                    }
                    // A momentum end phase is the exact moment a scroll-snap container must
                    // resolve. Requesting the settle here shortens the bounded fallback deadline
                    // instead of adding a second timer.
                    if matches!(event.phase, GesturePhase::Ended | GesturePhase::Cancelled) {
                        let state = self
                            .window
                            .as_mut()
                            .expect("window retained after callback");
                        if state.ui.scroll_gesture_ended(Instant::now())
                            && state.scheduler.invalidate()
                        {
                            state.window.request_redraw();
                        }
                    }
                    let delta = event.delta.pixel_delta(self.config.line_scroll_pixels);
                    if delta.is_zero() {
                        return;
                    }
                    let state = self
                        .window
                        .as_mut()
                        .expect("window retained after callback");
                    if state.scheduler.accumulate_scroll(delta) {
                        state.window.request_redraw();
                    }
                }
                WindowEvent::TouchpadPressure {
                    pressure, stage, ..
                } => {
                    let (position, target) = self
                        .window
                        .as_ref()
                        .and_then(|window| {
                            let position = window.pointer?;
                            Some((position, window.ui.mouse_pressure_listener_at(position)))
                        })
                        .unwrap_or((Point::ZERO, None));
                    let default_allowed = self.invoke_mouse_pressure(
                        event_loop,
                        target,
                        MousePressureEvent {
                            position,
                            pressure: bounded_pressure(pressure),
                            stage: map_pressure_stage(stage),
                            modifiers: self.modifiers,
                        },
                    );
                    let force_click = self.window.as_mut().is_some_and(|window| {
                        let entered_force = window.pressure_stage < 2 && stage >= 2;
                        window.pressure_stage = stage;
                        entered_force
                    });
                    if default_allowed && force_click {
                        // Opted-in text inputs show the dictionary definition once per force click.
                        if let Some(window) = &self.window {
                            let _ = window.ui.input_force_click_definition();
                        }
                    }
                }
                WindowEvent::PinchGesture { delta, phase, .. } => {
                    let (position, target) = self
                        .window
                        .as_ref()
                        .and_then(|window| {
                            let position = window.pointer?;
                            Some((position, window.ui.pinch_listener_at(position)))
                        })
                        .unwrap_or((Point::ZERO, None));
                    self.invoke_pinch(
                        event_loop,
                        target,
                        PinchEvent {
                            position,
                            delta: bounded_gesture_delta(delta, MAX_PINCH_DELTA_PER_EVENT),
                            phase: map_gesture_phase(phase),
                            modifiers: self.modifiers,
                        },
                    );
                }
                WindowEvent::RotationGesture { delta, phase, .. } => {
                    let (position, target) = self
                        .window
                        .as_ref()
                        .and_then(|window| {
                            let position = window.pointer?;
                            Some((position, window.ui.rotation_listener_at(position)))
                        })
                        .unwrap_or((Point::ZERO, None));
                    self.invoke_rotation(
                        event_loop,
                        target,
                        RotationEvent {
                            position,
                            delta: bounded_gesture_delta(
                                f64::from(delta),
                                MAX_ROTATION_DEGREES_PER_EVENT,
                            ),
                            phase: map_gesture_phase(phase),
                            modifiers: self.modifiers,
                        },
                    );
                }
                WindowEvent::DoubleTapGesture { .. } => {
                    let (position, target) = self
                        .window
                        .as_ref()
                        .and_then(|window| {
                            let position = window.pointer?;
                            Some((position, window.ui.smart_magnify_listener_at(position)))
                        })
                        .unwrap_or((Point::ZERO, None));
                    self.invoke_smart_magnify(
                        event_loop,
                        target,
                        SmartMagnifyEvent {
                            position,
                            modifiers: self.modifiers,
                        },
                    );
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    self.modifiers = map_modifiers(modifiers.state());
                    self.dispatch(event_loop, Event::ModifiersChanged(self.modifiers), false);
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    let stroke = self.keyboard.keystroke(&event, self.modifiers);
                    let key = stroke.key.clone();
                    // Every focus change this key causes is keyboard-driven, so one scope covers
                    // the whole arm and each early exit leaves through the block. Tab and the
                    // arrows reveal focus up front, so the focused control shows its ring even
                    // when the press turns out to move nothing.
                    let scope = self.begin_input_dispatch(InputModality::Keyboard);
                    if event.state == ElementState::Pressed && key.reveals_focus() {
                        self.reveal_focus();
                    }
                    'keyboard_input: {
                        #[cfg(feature = "inspector")]
                        if event.state == ElementState::Pressed
                            && key == Key::Escape
                            && self
                                .window
                                .as_ref()
                                .is_some_and(|state| state.inspector.is_some())
                        {
                            self.config.inspector = false;
                            let state = self.window.as_mut().expect("window checked above");
                            state.inspector = None;
                            reconcile_inspector_pointer_state(state);
                            if state.listeners.observes_window_state {
                                state.view_dirty = true;
                            }
                            if state.scheduler.invalidate() {
                                state.window.request_redraw();
                            }
                            break 'keyboard_input;
                        }
                        if event.state == ElementState::Pressed {
                            if key == Key::Escape
                                && window_dismisses_system_popover_on_escape(&self.config)
                            {
                                if let Some(handle) = self.current_handle() {
                                    self.close_requests.push(handle);
                                }
                                break 'keyboard_input;
                            }
                            if key == Key::Escape && self.cancel_internal_drag() {
                                if self
                                    .invoke_key_event(
                                        event_loop,
                                        KeyListenerEvent::Down(KeyDownEvent {
                                            key: key.clone(),
                                            key_char: stroke.key_char.clone(),
                                            text: event.text.as_ref().map(ToString::to_string),
                                            modifiers: stroke.modifiers,
                                            repeat: event.repeat,
                                        }),
                                    )
                                    .is_none()
                                {
                                    break 'keyboard_input;
                                }
                                self.dispatch(
                                    event_loop,
                                    Event::KeyDown {
                                        key,
                                        key_char: stroke.key_char,
                                        modifiers: stroke.modifiers,
                                        repeat: event.repeat,
                                    },
                                    false,
                                );
                                break 'keyboard_input;
                            }
                            self.handle_pressed_key(
                                event_loop,
                                PendingKey {
                                    stroke,
                                    repeat: event.repeat,
                                    text: event.text.map(|text| text.to_string()),
                                },
                            );
                        } else {
                            let Keystroke {
                                key,
                                key_char,
                                modifiers,
                            } = stroke;
                            if self
                                .invoke_key_event(
                                    event_loop,
                                    KeyListenerEvent::Up(KeyUpEvent {
                                        key: key.clone(),
                                        key_char: key_char.clone(),
                                        modifiers,
                                    }),
                                )
                                .is_none()
                            {
                                break 'keyboard_input;
                            }
                            self.dispatch(
                                event_loop,
                                Event::KeyUp {
                                    key,
                                    key_char,
                                    modifiers,
                                },
                                false,
                            );
                        }
                    }
                    self.end_input_dispatch(scope);
                }
                WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
                    let result = self
                        .window
                        .as_mut()
                        .map(|window| window.ui.input_preedit(&text, cursor))
                        .unwrap_or_default();
                    self.apply_input_result(event_loop, result, false);
                }
                WindowEvent::Ime(Ime::Commit(text)) => {
                    let result = self
                        .window
                        .as_mut()
                        .map(|window| window.ui.input_replace(&text))
                        .unwrap_or_default();
                    let changed = result.change.is_some();
                    if self.apply_input_result(event_loop, result, true) && changed {
                        self.dispatch(event_loop, Event::TextInput(text), false);
                    }
                }
                WindowEvent::Ime(Ime::Disabled) => {
                    let result = self
                        .window
                        .as_mut()
                        .map(|window| window.ui.input_preedit("", None))
                        .unwrap_or_default();
                    self.apply_input_result(event_loop, result, false);
                }
                WindowEvent::Ime(Ime::Enabled) => {}
                WindowEvent::Focused(focused) => {
                    let was_focused = self.window.as_ref().is_some_and(|state| state.focused);
                    let never_key_popovers_to_close = if !focused && was_focused {
                        self.current_never_key_popover_children()
                    } else {
                        Vec::new()
                    };
                    #[cfg(not(target_os = "macos"))]
                    let popover_root_to_close = (!focused
                        && was_focused
                        && window_dismisses_system_popover_on_pointer_outside(&self.config))
                    .then(|| self.current_handle())
                    .flatten();
                    if let Some(state) = &mut self.window {
                        let rebuild = state
                            .listeners
                            .requires_window_state_rebuild(state.focused != focused);
                        state.focused = focused;
                        state.view_dirty |= rebuild;
                    }
                    if focused {
                        self.note_window_focused(window_id);
                        #[cfg(target_os = "macos")]
                        if !self.install_active_mac_menu(event_loop) {
                            return;
                        }
                    }
                    if !focused {
                        #[cfg(not(target_os = "macos"))]
                        if let Some(root) = popover_root_to_close {
                            self.close_requests.push(root);
                        }
                        self.close_requests.extend(never_key_popovers_to_close);
                        let cancelled = self.window.as_mut().and_then(|state| {
                            let capture = state.pointer_capture.take();
                            let internal_drag = state.drag_session.take().is_some();
                            state.drag_candidate = None;
                            state.pressed_mouse_buttons = PressedMouseButtons::default();
                            state.mouse_clicks.cancel();
                            #[cfg(target_os = "macos")]
                            {
                                state.external_drag_mouse_down = None;
                                if let Some(monitor) = &state.external_drag_monitor {
                                    monitor.disarm();
                                }
                                if state.outbound_external_drag.is_none() {
                                    state.suppress_external_drag_release = false;
                                }
                            }
                            let repaint = state.ui.cancel_pointer_interaction()
                                | (internal_drag && state.ui.end_drag())
                                | (internal_drag && state.ui.clear_drag_preview());
                            let cursor = state
                                .pointer
                                .map_or(CursorIcon::Default, |point| desired_cursor(state, point));
                            set_cursor_if_changed(state, cursor);
                            if repaint && state.scheduler.invalidate() {
                                state.window.request_redraw();
                            }
                            let capture = capture?;
                            Some((
                                capture.target,
                                PointerEvent {
                                    // The captured element localizes the event before delivery.
                                    size: Size::ZERO,
                                    phase: PointerPhase::Cancel,
                                    position: capture.position,
                                    origin: capture.origin,
                                    local_position: capture.position,
                                    local_origin: capture.origin,
                                    delta: Vector::ZERO,
                                    button: capture.button,
                                    modifiers: self.modifiers,
                                },
                            ))
                        });
                        if let Some((target, event)) = cancelled
                            && !self.invoke_pointer(event_loop, target, event)
                        {
                            return;
                        }
                        loop {
                            let cancelled_touch = self.window.as_mut().and_then(|state| {
                                let id = state.touch_captures.keys().next().copied()?;
                                let capture = state.touch_captures.remove(&id)?;
                                let mut event = capture.last_event;
                                event.phase = TouchPhase::Cancelled;
                                Some((capture.target, event))
                            });
                            let Some((target, event)) = cancelled_touch else {
                                break;
                            };
                            if !self.invoke_touch(event_loop, Some(target), event) {
                                return;
                            }
                        }
                    }
                    if focused {
                        let reduce_motion = self.config.reduce_motion
                            || self
                                .system_preferences
                                .reduce_motion()
                                .is_some_and(|enabled| enabled);
                        let state = self.window.as_mut().expect("window checked above");
                        state.reduce_motion = reduce_motion;
                        state.ui.set_reduce_motion(reduce_motion);
                        state.ui.set_animations_enabled(
                            !state.occluded && !reduce_motion,
                            Instant::now(),
                        );
                    }
                    #[cfg(target_os = "macos")]
                    self.refresh_current_native_tab_state();
                    self.dispatch(event_loop, Event::Focused(focused), true);
                }
                WindowEvent::Destroyed => {
                    if let Some(handle) = self.current_handle() {
                        self.close_requests.push(handle);
                    }
                }
                _ => {}
            }
        })();
        self.deactivate_window();
        self.process_window_commands(event_loop);
    }
}
