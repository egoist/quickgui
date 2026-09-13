use super::*;

impl TestAppContext {
    pub(super) fn render_visual(
        &mut self,
        window: WindowHandle,
        capture: bool,
    ) -> Result<Option<crate::VisualSnapshot>, TestAppError> {
        self.run_until_idle()?;
        let (profile, viewport, scale_factor) = {
            let state = self.window(window)?;
            (
                state.config.performance_profile,
                state.state.viewport_size,
                state.state.scale_factor,
            )
        };
        if capture {
            OffscreenRenderer::validate_snapshot_size(viewport, scale_factor)?;
        }
        let mut renderer = match self.visual_renderer.take() {
            Some(renderer) => renderer,
            None => pollster::block_on(OffscreenRenderer::new(profile, self.font_system.clone()))?,
        };
        let now = self.now.get();
        let result = (|| {
            // A variable list can learn new row heights only after production layout. Rebuild the
            // declaration until its sparse metrics agree, with a hard cap for a pathological view
            // whose item height changes on every render. Ordinary visual tests still take one pass.
            for _ in 0..8 {
                let mut scene = Scene::new();
                let measurements_changed = {
                    let state = self.window_mut(window)?;
                    state
                        .ui
                        .invalidate_layout_measurements_for_test()
                        .map_err(|error| TestAppError::View(error.to_string()))?;
                    state
                        .ui
                        .layout_for_test(viewport, scale_factor, &mut renderer, now)
                        .map_err(|error| TestAppError::View(error.to_string()))?;
                    state.requested_animation_frame |=
                        state.ui.declarative_animation_frame_requested();
                    state.ui.advance_animations(now);
                    state.ui.advance_scrollbars(now);
                    state.ui.advance_caret_blink(now);
                    scene.clear(state.config.background);
                    state
                        .ui
                        .paint_at(&mut scene, &mut renderer, now)
                        .map_err(|error| TestAppError::View(error.to_string()))?;
                    #[cfg(feature = "inspector")]
                    if state.inspector.is_some() {
                        let TestWindow { ui, inspector, .. } = state;
                        let inspector = inspector
                            .as_mut()
                            .expect("inspector presence checked before split borrow");
                        inspector.refresh(
                            ui,
                            FrameMetrics::default(),
                            InspectorFrameDamage::default(),
                            viewport,
                            scale_factor,
                        );
                        inspector
                            .paint(&mut scene, &mut renderer, viewport, scale_factor, now)
                            .map_err(|error| TestAppError::View(error.to_string()))?;
                    }
                    let placement_changed = state.ui.take_anchor_placement_update();
                    state.dirty |= placement_changed;
                    let update = state.ui.take_variable_list_measurement_update();
                    state.dirty |= update.view_dirty;
                    update.changed || placement_changed
                };
                if measurements_changed {
                    self.run_until_idle()?;
                    continue;
                }
                if !capture {
                    return Ok(None);
                }
                #[cfg(target_os = "macos")]
                if !self.window(window)?.ui.native_views().is_empty() {
                    return Err(TestAppError::Visual(
                        crate::VisualTestError::NativeViewUnsupported,
                    ));
                }
                return renderer
                    .render_to_snapshot(&scene, viewport, scale_factor)
                    .map(Some)
                    .map_err(TestAppError::from);
            }
            Err(TestAppError::View(
                "variable-list measurements did not converge within 8 visual layout passes"
                    .to_owned(),
            ))
        })();
        self.visual_renderer = Some(renderer);
        result
    }

    pub(super) fn prepare_retained_geometry(
        &mut self,
        window: WindowHandle,
    ) -> Result<(), TestAppError> {
        if self.window(window)?.retained_geometry_ready {
            return Ok(());
        }
        let now = self.now();
        let (viewport, scale_factor, background) = {
            let state = self.window(window)?;
            (
                state.state.viewport_size,
                state.state.scale_factor,
                state.config.background,
            )
        };
        let mut layout = SemanticTextLayout;
        let mut scene = Scene::new();
        scene.clear(background);
        let state = self.window_mut(window)?;
        state
            .ui
            .layout_for_test(viewport, scale_factor, &mut layout, now)
            .map_err(|error| TestAppError::View(error.to_string()))?;
        state
            .ui
            .paint_at(&mut scene, &mut layout, now)
            .map_err(|error| TestAppError::View(error.to_string()))?;
        // Mirror the production frame: a surface that flipped away from its declared placement
        // publishes the resolved value here, so the owning view redraws once against it.
        state.dirty |= state.ui.take_anchor_placement_update();
        state.retained_geometry_ready = true;
        Ok(())
    }

    pub(super) fn queue_dispatch(&mut self, dispatch: TestDispatch) -> Result<(), TestAppError> {
        if self.pending_dispatches.len() == MAX_TEST_PENDING_DISPATCHES {
            return Err(TestAppError::DispatchQueueFull);
        }
        self.pending_dispatches.push_back(dispatch);
        Ok(())
    }

    pub(super) fn queue_immediate(
        &mut self,
        dispatches: impl IntoIterator<Item = TestDispatch>,
    ) -> Result<(), TestAppError> {
        let dispatches = dispatches.into_iter().collect::<Vec<_>>();
        if self.pending_dispatches.len() + dispatches.len() > MAX_TEST_PENDING_DISPATCHES {
            return Err(TestAppError::DispatchQueueFull);
        }
        for dispatch in dispatches.into_iter().rev() {
            self.pending_dispatches.push_front(dispatch);
        }
        Ok(())
    }

    pub(super) fn create_pending_windows(&mut self) -> Result<bool, TestAppError> {
        let mut progress = false;
        while let Some(request) = self.pending_windows.pop_front() {
            validate_window_options(&request.options)?;
            let handle = request.handle;
            let parent_state = request
                .parent
                .and_then(|parent| self.windows.get(&parent))
                .map(|window| window.state);
            let state = test_window_state(
                handle,
                &request.options,
                request.options.focus && request.options.focusable,
                &self.displays,
                parent_state,
            );
            // Match production: a child that never takes key focus must not restore its anchor and
            // overwrite focus movement that already happened in the owner window.
            let restore_focus_on_close = request.popover_anchor_element.filter(|_| {
                request
                    .options
                    .popover
                    .as_ref()
                    .is_some_and(|popover| popover.accepts_key_focus)
            });
            let window = TestWindow {
                parent: request.parent,
                restore_focus_on_close,
                view: request.view,
                ui: UiTree::new_at(self.animation_epoch),
                #[cfg(feature = "inspector")]
                inspector: request
                    .options
                    .inspector
                    .then(|| InspectorState::new(self.animation_epoch)),
                listeners: ListenerRegistry::default(),
                key_dispatch_scratch: Vec::with_capacity(16),
                action_dispatch_scratch: Vec::with_capacity(16),
                touch_captures: HashMap::with_capacity(8),
                config: request.options,
                state,
                system_appearance: WindowAppearance::default(),
                dirty: true,
                pending_focus: None,
                render_count: 0,
                retained_geometry_ready: false,
                requested_animation_frame: false,
                repaint_deadline: None,
                pointer: None,
                first_presented: false,
            };
            self.windows.insert(handle, window);
            self.last_window_quit_prevented = false;
            if (self.active_window.is_none() || self.window(handle)?.config.focus)
                && self.window(handle)?.config.focusable
            {
                self.set_active_window(Some(handle))?;
            }
            progress = true;
        }
        Ok(progress)
    }

    pub(super) fn invoke_quit_callback(
        &mut self,
        request: QuitRequest,
        before: bool,
    ) -> Result<bool, TestAppError> {
        let callback = if before {
            self.application_callbacks.before_quit.take()
        } else {
            self.application_callbacks.will_quit.take()
        };
        let Some(mut callback) = callback else {
            return Ok(true);
        };
        let mut cx = self.event_context(None);
        self.quit_phase_active = true;
        callback(request, &mut cx);
        let prevented = cx.prevent_quit;
        if before {
            self.application_callbacks.before_quit = Some(callback);
        } else {
            self.application_callbacks.will_quit = Some(callback);
        }
        let result = self.apply_context(None, cx);
        self.quit_phase_active = false;
        result?;
        Ok(!prevented)
    }

    pub(super) fn request_quit(&mut self, reason: QuitReason) -> Result<(), TestAppError> {
        let request = QuitRequest { reason };
        let accepted = self.invoke_quit_callback(request, true)?
            && self.invoke_quit_callback(request, false)?;
        if !accepted {
            if reason == QuitReason::Relaunch {
                self.relaunch_request = None;
            }
            if reason == QuitReason::LastWindowClosed {
                self.last_window_quit_prevented = true;
            }
            return Ok(());
        }
        self.exited = true;
        self.pending_closes.extend(self.windows.keys().copied());
        Ok(())
    }

    pub(super) fn close_pending_windows(&mut self) -> Result<bool, TestAppError> {
        if self.pending_closes.is_empty() {
            return Ok(false);
        }
        let roots = std::mem::take(&mut self.pending_closes);
        let mut discovered = HashSet::new();
        let mut order = Vec::new();
        let mut stack = roots
            .into_iter()
            .map(|window| (window, false))
            .collect::<Vec<_>>();
        while let Some((window, expanded)) = stack.pop() {
            if expanded {
                order.push(window);
                continue;
            }
            if !discovered.insert(window) {
                continue;
            }
            stack.push((window, true));
            let mut children = self
                .windows
                .iter()
                .filter_map(|(handle, state)| (state.parent == Some(window)).then_some(*handle))
                .collect::<Vec<_>>();
            children.sort_unstable();
            for child in children.into_iter().rev() {
                stack.push((child, false));
            }
        }

        let mut closed = Vec::with_capacity(order.len());
        for window in order {
            let (parent, restore_focus) = self
                .windows
                .get(&window)
                .map(|window| (window.parent, window.restore_focus_on_close))
                .unwrap_or((None, None));
            self.foreground_tasks.cancel_window(window);
            if self.windows.remove(&window).is_some() {
                closed.push(ClosedWindow {
                    handle: window,
                    parent,
                    restore_focus,
                });
            }
        }
        self.focus_history
            .retain(|window| !discovered.contains(window));
        if self
            .active_window
            .is_some_and(|window| discovered.contains(&window))
        {
            self.active_window = self.focus_history.last().copied();
            let _ = self.refresh_window_focus_states();
        }

        for closed in &closed {
            if let Some(parent) = closed.parent
                && self.windows.contains_key(&parent)
            {
                let callbacks = {
                    let listeners = &self.window(parent)?.listeners;
                    let mut callbacks =
                        Vec::with_capacity(listeners.any_child_window_closed.len() + 1);
                    if let Some(callback) =
                        listeners.child_window_closed.get(&closed.handle).cloned()
                    {
                        callbacks.push(callback);
                    }
                    callbacks.extend(listeners.any_child_window_closed.iter().cloned());
                    callbacks
                };
                if !callbacks.is_empty() || closed.restore_focus.is_some() {
                    let mut cx = self.event_context(Some(parent));
                    cx.focus = closed.restore_focus.map(Some);
                    for callback in callbacks {
                        callback(
                            self.window_mut(parent)?.view.as_any_mut(),
                            closed.handle,
                            &mut cx,
                        );
                    }
                    self.apply_context(Some(parent), cx)?;
                }
            }

            if let Some(mut callback) = self.application_callbacks.window_closed.take() {
                let mut cx = self.event_context(None);
                callback(closed.handle, &mut cx);
                self.application_callbacks.window_closed = Some(callback);
                self.apply_context(None, cx)?;
            }
        }

        if !self.exited
            && self.windows.is_empty()
            && self.pending_windows.is_empty()
            && self.quit_mode.quits_when_empty()
            && !self.last_window_quit_prevented
        {
            self.request_quit(QuitReason::LastWindowClosed)?;
        }
        Ok(!closed.is_empty())
    }

    pub(super) fn process_one_dispatch(&mut self) -> Result<bool, TestAppError> {
        let Some(dispatch) = self.pending_dispatches.pop_front() else {
            return Ok(false);
        };
        match dispatch {
            TestDispatch::Event(window, event) => {
                if !self.windows.contains_key(&window) {
                    return Ok(true);
                }
                let mut cx = self.event_context(Some(window));
                self.window_mut(window)?.view.event(&event, &mut cx);
                self.apply_context(Some(window), cx)?;
            }
            TestDispatch::Action(window, action) => {
                if self.windows.contains_key(&window) {
                    self.invoke_action(window, &action)?;
                }
            }
            TestDispatch::Form(window, form, trigger) => {
                if self.windows.contains_key(&window) {
                    self.invoke_form_submission(window, form, trigger)?;
                }
            }
        }
        Ok(true)
    }

    pub(super) fn process_deferred_effects(&mut self) -> Result<bool, TestAppError> {
        let mut progress = false;
        let mut global_deliveries = 0;
        let mut entity_deliveries = 0;
        loop {
            let global_type = if self.pending_all_globals {
                self.pending_all_globals = false;
                self.pending_global_notifications.clear();
                self.pending_global_notification_types.clear();
                Some(None)
            } else {
                self.pending_global_notifications
                    .pop_front()
                    .map(|global_type| {
                        self.pending_global_notification_types.remove(&global_type);
                        Some(global_type)
                    })
            };
            if let Some(global_type) = global_type {
                let targets = self.windows();
                for window in targets {
                    if !self
                        .window(window)?
                        .listeners
                        .has_global_subscribers(global_type)
                    {
                        continue;
                    }
                    let subscriptions = self
                        .window(window)?
                        .listeners
                        .global_subscriptions(global_type);
                    for subscription in subscriptions {
                        if !subscription.is_active() {
                            continue;
                        }
                        if !reserve_global_observer_delivery(&mut global_deliveries) {
                            return Err(TestAppError::EffectTurnLimit);
                        }
                        let mut cx = self.event_context(Some(window));
                        subscription.callback.borrow_mut()(
                            self.window_mut(window)?.view.as_any_mut(),
                            &mut cx,
                        );
                        self.apply_context(Some(window), cx)?;
                        progress = true;
                    }
                }
                continue;
            }

            let Some(event) = self.pending_entity_events.pop_front() else {
                break;
            };
            for window in self.windows() {
                if !self
                    .window(window)?
                    .listeners
                    .has_entity_event_subscribers(event.source, event.event_type)
                {
                    continue;
                }
                let callbacks = self
                    .window(window)?
                    .listeners
                    .entity_event_callbacks(event.source, event.event_type);
                for callback in callbacks {
                    if !reserve_entity_event_delivery(&mut entity_deliveries) {
                        return Err(TestAppError::EffectTurnLimit);
                    }
                    let mut cx = self.event_context(Some(window));
                    callback.borrow_mut()(
                        self.window_mut(window)?.view.as_any_mut(),
                        event.value.as_ref(),
                        &mut cx,
                    );
                    self.apply_context(Some(window), cx)?;
                    progress = true;
                }
            }
        }
        Ok(progress)
    }

    pub(super) fn process_foreground_tasks(&mut self) -> Result<bool, TestAppError> {
        let mut batch = self.foreground_tasks.take_ready_batch();
        let progress = !batch.is_empty();
        while let Some(ScheduledForegroundTask {
            task,
            window,
            runnable,
        }) = batch.pop_front()
        {
            if !self.foreground_tasks.owns(task, window) {
                drop(runnable);
                continue;
            }
            if window.is_some_and(|window| !self.windows.contains_key(&window)) {
                self.foreground_tasks.cancel_task(task);
                drop(runnable);
                continue;
            }
            if catch_unwind(AssertUnwindSafe(|| runnable.run())).is_err() {
                self.foreground_tasks.cancel_task(task);
                return Err(TestAppError::ForegroundTaskPanicked);
            }
            let mut updates = self.foreground_tasks.take_updates(task);
            debug_assert!(window.is_some() || updates.is_empty());
            while let Some(update) = updates.pop_front() {
                let window = window.expect("view updates require a window-owned task");
                let mut cx = self.event_context(Some(window));
                update(self.window_mut(window)?.view.as_any_mut(), &mut cx);
                self.apply_context(Some(window), cx)?;
            }
        }
        Ok(progress)
    }

    pub(super) fn rebuild_dirty_windows(&mut self) -> Result<bool, TestAppError> {
        let windows = self
            .windows()
            .into_iter()
            .filter(|window| {
                self.windows[window].dirty || self.windows[window].listeners.scopes.pending()
            })
            .collect::<Vec<_>>();
        if windows.is_empty() {
            return Ok(false);
        }
        let now = self.now.get();
        for window in windows {
            let globals = self.globals.clone();
            let foreground_tasks = self.foreground_tasks.clone();
            let displays = self.displays.clone();
            let keyboard_layout = self.keyboard_layout.clone();
            let assets = self.assets.clone();
            let font_system = self.font_system.clone();
            let app_info = self.app_info.clone();
            let app_paths = self.app_paths.clone();
            let system_info = self.system_info.clone();
            let system_preferences = self.system_preferences;
            let state_snapshot = self.window(window)?.state;
            if !self.window(window)?.dirty {
                let previous_focus = self.window(window)?.ui.focused();
                let state = self.window_mut(window)?;
                let mut cx = ViewContext::<()> {
                    size: state.state.viewport_size,
                    scale_factor: state.state.scale_factor,
                    metrics: FrameMetrics::default(),
                    focused: state.ui.focused(),
                    focused_path: state.ui.focus_path(),
                    request_animation_frame: false,
                    repaint_deadline: None,
                    listeners: &mut state.listeners,
                    window,
                    window_state: state_snapshot,
                    displays: &displays,
                    keyboard_layout: &keyboard_layout,
                    font_system: &font_system,
                    assets: &assets,
                    app_info: app_info.as_ref(),
                    app_paths: app_paths.as_ref(),
                    system_info: &system_info,
                    system_preferences: &system_preferences,
                    background_tasks: None,
                    foreground_tasks: &foreground_tasks,
                    globals: &globals,
                    event_proxy: None,
                    marker: PhantomData,
                };
                let updates = state.view.render_scopes(&mut cx);
                state.requested_animation_frame |= cx.request_animation_frame;
                state.repaint_deadline = state
                    .repaint_deadline
                    .into_iter()
                    .chain(cx.repaint_deadline)
                    .min();
                let applied = match updates {
                    Some(updates) => state
                        .ui
                        .update_elements(&updates)
                        .map_err(|error| TestAppError::View(error.to_string()))?
                        .is_some(),
                    None => false,
                };
                if applied {
                    state.retained_geometry_ready = false;
                    self.prepare_retained_geometry(window)?;
                    self.focus_changed(window, previous_focus)?;
                    continue;
                }
            }
            let (root, requested, repaint_deadline) = {
                let state = self.window_mut(window)?;
                state.view.render(
                    state.state.viewport_size,
                    state.state.scale_factor,
                    FrameMetrics::default(),
                    state.ui.focused(),
                    state.ui.focus_path(),
                    &mut state.listeners,
                    window,
                    state_snapshot,
                    &displays,
                    &keyboard_layout,
                    &font_system,
                    &assets,
                    app_info.as_ref(),
                    app_paths.as_ref(),
                    &system_info,
                    &system_preferences,
                    None,
                    &foreground_tasks,
                    &globals,
                    None,
                )
            };
            let previous_focus = self.window(window)?.ui.focused();
            {
                let state = self.window_mut(window)?;
                let reduce_motion = state.config.reduce_motion
                    || system_preferences
                        .reduce_motion()
                        .is_some_and(|enabled| enabled);
                state.ui.set_reduce_motion(reduce_motion);
                state.ui.set_animations_enabled(!reduce_motion, now);
                state
                    .ui
                    .set_root_for_test(
                        root,
                        state.state.viewport_size,
                        state.state.scale_factor,
                        now,
                    )
                    .map_err(|error| TestAppError::View(error.to_string()))?;
                let mut semantic_layout = SemanticTextLayout;
                state
                    .ui
                    .layout_for_test(
                        state.state.viewport_size,
                        state.state.scale_factor,
                        &mut semantic_layout,
                        now,
                    )
                    .map_err(|error| TestAppError::View(error.to_string()))?;
                if let Some(request) = state.pending_focus.take()
                    && state.ui.is_focusable(request.element)
                {
                    state.ui.focus_as(request.element, request.modality);
                }
                state.dirty = false;
                state.render_count += 1;
                state.retained_geometry_ready = false;
                state.requested_animation_frame =
                    requested || state.ui.declarative_animation_frame_requested();
                state.repaint_deadline = repaint_deadline;
            }
            self.focus_changed(window, previous_focus)?;
        }
        Ok(true)
    }
}
