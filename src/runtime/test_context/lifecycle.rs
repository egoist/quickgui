use super::*;

impl TestAppContext {
    /// Run queued callbacks, foreground futures, observation delivery, and dirty declarations.
    pub fn run_until_idle(&mut self) -> Result<(), TestAppError> {
        for _ in 0..MAX_TEST_EFFECT_TURNS {
            let mut progress = false;
            progress |= self.create_pending_windows()?;
            progress |= self.process_one_dispatch()?;
            if self.pending_dispatches.is_empty() {
                progress |= self.process_deferred_effects()?;
            }
            progress |= self.process_foreground_tasks()?;
            // Production delivers queued cross-window actions before applying the close tree from
            // the same callback. Popover commands therefore reach their owner before the popover
            // chain is destroyed; the deterministic runtime preserves that exact ordering.
            progress |= self.close_pending_windows()?;
            progress |= self.rebuild_dirty_windows()?;
            if !progress {
                // Production pumps settled text checks when the event loop is about to wait, so
                // a deadline is timestamped at the end of the turn that accepted the edit.
                let now = self.now.get();
                let mut repaint = false;
                for state in self.windows.values_mut() {
                    if state.ui.advance_spell_check(now).repaint {
                        state.dirty = true;
                        repaint = true;
                    }
                }
                // Production paints every rebuilt frame, and painting is what publishes resolved
                // anchor placements and laid-out bounds. Views that read those back get the same
                // one correcting frame here, so a headless test converges like a real window.
                let observers: Vec<WindowHandle> = self
                    .windows
                    .iter()
                    .filter(|(_, state)| {
                        !state.retained_geometry_ready && state.ui.observes_painted_geometry()
                    })
                    .map(|(handle, _)| *handle)
                    .collect();
                for window in observers {
                    self.prepare_retained_geometry(window)?;
                    if self.window(window)?.dirty {
                        repaint = true;
                    }
                }
                if !repaint {
                    return Ok(());
                }
            }
        }
        Err(TestAppError::EffectTurnLimit)
    }

    /// Advance exact foreground timers and view repaint deadlines without sleeping.
    pub fn advance_time(&mut self, duration: Duration) -> Result<(), TestAppError> {
        let current = self.now.get();
        let now = current.checked_add(duration).unwrap_or(current);
        self.now.set(now);
        self.foreground_tasks.wake_due_timers(now);
        for state in self.windows.values_mut() {
            if state
                .repaint_deadline
                .is_some_and(|deadline| deadline <= now)
            {
                state.repaint_deadline = None;
                state.dirty = true;
            }
            if state.ui.declarative_animation_due(now) {
                state.dirty = true;
            }
            if state.ui.advance_spell_check(now).repaint {
                state.dirty = true;
            }
        }
        self.run_until_idle()
    }

    /// Advance one explicitly requested animation frame. Continuous frames never run implicitly.
    pub fn advance_frame(&mut self) -> Result<usize, TestAppError> {
        let mut scheduled = 0;
        for state in self.windows.values_mut() {
            if state.requested_animation_frame {
                state.dirty = true;
                scheduled += 1;
            }
        }
        self.run_until_idle()?;
        Ok(scheduled)
    }

    pub fn simulate_open_urls(
        &mut self,
        urls: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> Result<(), TestAppError> {
        let Some(mut callback) = self.application_callbacks.open_urls.take() else {
            return Ok(());
        };
        let mut retained = Vec::with_capacity(4);
        let mut total = 0_usize;
        for url in urls.into_iter().take(crate::MAX_OPEN_URLS) {
            let url = url.into();
            if url.len() > crate::MAX_PLATFORM_URL_BYTES || url.contains('\0') {
                continue;
            }
            let Some(next_total) = total.checked_add(url.len()) else {
                break;
            };
            if next_total > crate::MAX_OPEN_URLS_TOTAL_BYTES {
                break;
            }
            total = next_total;
            retained.push(url);
        }
        let urls = OpenUrls::from_bounded(retained);
        let mut cx = self.event_context(None);
        callback(urls, &mut cx);
        self.application_callbacks.open_urls = Some(callback);
        self.apply_context(None, cx)?;
        self.run_until_idle()
    }

    pub fn simulate_reopen(&mut self, has_visible_windows: bool) -> Result<(), TestAppError> {
        let Some(mut callback) = self.application_callbacks.reopen.take() else {
            return Ok(());
        };
        let mut cx = self.event_context(None);
        callback(has_visible_windows, &mut cx);
        self.application_callbacks.reopen = Some(callback);
        self.apply_context(None, cx)?;
        self.run_until_idle()
    }

    pub fn simulate_system_wake(&mut self) -> Result<(), TestAppError> {
        let Some(mut callback) = self.application_callbacks.system_wake.take() else {
            return Ok(());
        };
        let mut cx = self.event_context(None);
        callback(&mut cx);
        self.application_callbacks.system_wake = Some(callback);
        self.apply_context(None, cx)?;
        self.run_until_idle()
    }

    /// Deliver one deterministic native power or login-session transition.
    pub fn simulate_power_event(&mut self, event: PowerEvent) -> Result<(), TestAppError> {
        let Some(mut callback) = self.application_callbacks.power_event.take() else {
            return Ok(());
        };
        let mut cx = self.event_context(None);
        callback(event, &mut cx);
        self.application_callbacks.power_event = Some(callback);
        self.apply_context(None, cx)?;
        self.run_until_idle()
    }

    pub fn simulate_system_notification_response(
        &mut self,
        response: SystemNotificationResponse,
    ) -> Result<(), TestAppError> {
        let Some(mut callback) = self
            .application_callbacks
            .system_notification_response
            .take()
        else {
            return Ok(());
        };
        let mut cx = self.event_context(None);
        callback(response, &mut cx);
        self.application_callbacks.system_notification_response = Some(callback);
        self.apply_context(None, cx)?;
        self.run_until_idle()
    }

    /// Deliver a native close request and return whether the window was closed.
    pub fn simulate_close_requested(&mut self, window: WindowHandle) -> Result<bool, TestAppError> {
        self.window(window)?;
        let mut cx = self.event_context(Some(window));
        self.window_mut(window)?
            .view
            .event(&Event::CloseRequested, &mut cx);
        let prevent_close = cx.prevent_close;
        let explicitly_closed = cx.close_current_window;
        self.apply_context(Some(window), cx)?;
        if !prevent_close && !explicitly_closed {
            self.pending_closes.push(window);
        }
        self.run_until_idle()?;
        Ok(!self.is_window_open(window))
    }

    pub(super) fn window(&self, window: WindowHandle) -> Result<&TestWindow, TestAppError> {
        self.windows
            .get(&window)
            .ok_or(TestAppError::UnknownWindow(window))
    }

    pub(super) fn window_mut(
        &mut self,
        window: WindowHandle,
    ) -> Result<&mut TestWindow, TestAppError> {
        self.windows
            .get_mut(&window)
            .ok_or(TestAppError::UnknownWindow(window))
    }

    pub(super) fn require_element(
        &self,
        window: WindowHandle,
        element: ElementId,
    ) -> Result<(), TestAppError> {
        if self.window(window)?.ui.contains_element(element) {
            Ok(())
        } else {
            Err(TestAppError::UnknownElement { window, element })
        }
    }

    pub(super) fn event_context(&self, window: Option<WindowHandle>) -> EventContext {
        let parent = window
            .and_then(|window| self.windows.get(&window))
            .and_then(|window| window.parent);
        let popover_context = window.and_then(|window| self.popover_context(window));
        let pointer_position = window
            .and_then(|window| self.windows.get(&window))
            .and_then(|window| window.pointer);
        EventContext::with_runtime(EventRuntimeContext {
            globals: self.globals.clone(),
            foreground_tasks: self.foreground_tasks.clone(),
            clipboard: self.clipboard.clone(),
            displays: self.displays.clone(),
            keyboard_layout: self.keyboard_layout.clone(),
            assets: self.assets.clone(),
            app_info: self.app_info.clone(),
            app_paths: self.app_paths.clone(),
            system_info: self.system_info.clone(),
            system_preferences: self.system_preferences,
            window_registry: self.window_registry(),
            window: crate::event::EventWindowContext {
                window,
                parent,
                popover_owner: popover_context.map(|context| context.owner),
                popover_root: popover_context.map(|context| context.root),
                pointer_position,
            },
        })
    }

    pub(super) fn popover_context(&self, window: WindowHandle) -> Option<PopoverWindowContext> {
        let current = self.windows.get(&window)?;
        if current.config.kind != WindowKind::SystemPopover {
            return None;
        }
        let mut root = window;
        let mut ancestor = current.parent?;
        loop {
            let window = self.windows.get(&ancestor)?;
            if window.config.kind != WindowKind::SystemPopover {
                return Some(PopoverWindowContext {
                    owner: ancestor,
                    root,
                });
            }
            root = ancestor;
            ancestor = window.parent?;
        }
    }

    pub(super) fn apply_context(
        &mut self,
        origin: Option<WindowHandle>,
        mut cx: EventContext,
    ) -> Result<(), TestAppError> {
        let mut unsupported_platform_request = false;
        for request in std::mem::take(&mut cx.platform_requests) {
            match request {
                PlatformRequest::ShowSystemNotification(notification) => {
                    if self.notification_permission_status == NotificationPermissionStatus::Granted
                    {
                        self.system_notifications
                            .insert(notification.tag.clone(), notification);
                    }
                }
                PlatformRequest::DismissSystemNotification(tag) => {
                    self.system_notifications.remove(&tag);
                }
                PlatformRequest::NotificationPermissionStatus { responder } => {
                    responder.complete(Ok(self.notification_permission_status));
                }
                PlatformRequest::RequestNotificationPermission { responder } => {
                    if self.notification_permission_status
                        == NotificationPermissionStatus::NotDetermined
                    {
                        self.notification_permission_status = NotificationPermissionStatus::Granted;
                    }
                    responder.complete(Ok(self.notification_permission_status));
                }
                PlatformRequest::SetActivationPolicy { policy, responder } => {
                    self.application_shell.activation_policy = policy;
                    self.application_shell.dock_visible = policy == ActivationPolicy::Regular;
                    responder.complete(Ok(()));
                }
                PlatformRequest::ActivateApplication { force } => {
                    self.application_shell.activations += 1;
                    self.application_shell.last_activation_forced = force;
                    self.application_shell.hidden = false;
                }
                PlatformRequest::HideApplication => self.application_shell.hidden = true,
                PlatformRequest::UnhideApplication => self.application_shell.hidden = false,
                PlatformRequest::RequestDockAttention {
                    attention,
                    responder,
                } => {
                    self.application_shell.dock_attention = Some(attention);
                    self.next_dock_attention_id = self.next_dock_attention_id.saturating_add(1);
                    responder.complete(Ok(DockAttentionRequest::new(self.next_dock_attention_id)));
                }
                PlatformRequest::CancelDockAttention(_) => {
                    self.application_shell.dock_attention = None;
                }
                PlatformRequest::SetDockVisible { visible, responder } => {
                    self.application_shell.dock_visible = visible;
                    self.application_shell.activation_policy = if visible {
                        ActivationPolicy::Regular
                    } else {
                        ActivationPolicy::Accessory
                    };
                    responder.complete(Ok(()));
                }
                PlatformRequest::SetSecureKeyboardEntry(enabled) => {
                    self.application_shell.secure_keyboard_entry = enabled;
                }
                PlatformRequest::SetTrayIcon(options) => {
                    self.tray_icons.insert(options.id, options);
                }
                PlatformRequest::RemoveTrayIcon(id) => {
                    self.tray_icons.remove(&id);
                }
                PlatformRequest::ShowTrayMenu(_) => {}
                PlatformRequest::Beep => self.application_shell.beeps += 1,
                PlatformRequest::MoveToApplicationsFolder { responder } => {
                    self.application_shell.applications_folder_moves += 1;
                    responder.complete(Ok(true));
                }
                request => {
                    request.complete_error(PlatformError::Unsupported);
                    unsupported_platform_request = true;
                }
            }
        }
        if unsupported_platform_request {
            return Err(TestAppError::UnsupportedPlatformRequest);
        }
        if !cx.native_popup_menus.is_empty() {
            return Err(TestAppError::UnsupportedPlatformRequest);
        }
        let relaunch_requested = cx.relaunch.is_some();
        if let Some(request) = cx.relaunch.take() {
            self.relaunch_request = Some(request);
        }
        if let Some(code) = cx.exit_code.take() {
            self.exit_code = Some(code);
        }
        if cx.exit && !self.quit_phase_active {
            self.request_quit(if relaunch_requested {
                QuitReason::Relaunch
            } else {
                QuitReason::Explicit
            })?;
        }
        if !enqueue_global_notifications(
            &mut self.pending_global_notifications,
            &mut self.pending_global_notification_types,
            &mut self.pending_all_globals,
            &cx.global_notifications,
            cx.notify_all_globals,
        ) {
            return Err(TestAppError::EffectTurnLimit);
        }
        if !enqueue_entity_events(&mut self.pending_entity_events, &mut cx.entity_events) {
            return Err(TestAppError::EffectTurnLimit);
        }
        if self.exited {
            cx.open_windows.clear();
        } else {
            for request in &mut cx.open_windows {
                let Some(anchor) = request.popover_anchor_element else {
                    continue;
                };
                let origin = origin.ok_or_else(|| {
                    TestAppError::View("a SystemPopover requires a parent test window".to_owned())
                })?;
                // Production pointer events arrive after a presented frame has populated retained
                // geometry. Headless semantic tests intentionally skip paint, so prepare the same
                // CPU-only geometry lazily only when an element-anchored child actually needs it.
                self.prepare_retained_geometry(origin)?;
                let bounds = self.window(origin)?.ui.element_bounds(anchor).ok_or(
                    TestAppError::UnknownElement {
                        window: origin,
                        element: anchor,
                    },
                )?;
                let popover = request.options.popover.as_mut().ok_or_else(|| {
                    TestAppError::View(WindowCommandError::InvalidPopoverConfiguration.to_string())
                })?;
                popover.anchor_rect = bounds;
            }
            self.pending_windows.extend(cx.open_windows.drain(..));
        }
        if cx.close_current_window
            && let Some(origin) = origin
        {
            self.pending_closes.push(origin);
        }
        self.pending_closes.append(&mut cx.close_windows);
        for window in cx.focus_windows.drain(..) {
            if self
                .windows
                .get(&window)
                .is_some_and(|window| window.config.focusable)
            {
                self.set_active_window(Some(window))?;
            }
        }
        for window in cx.invalidate_windows.drain(..) {
            if let Some(state) = self.windows.get_mut(&window) {
                state.dirty = true;
            }
        }
        for command in cx.window_commands.drain(..) {
            self.apply_window_command(command)?;
        }
        if let Some(menus) = cx.menus.take() {
            validate_menus(&menus).map_err(|error| TestAppError::View(error.to_string()))?;
            self.menus = menus;
        }
        if let Some(window_menus) = cx.window_menus.take()
            && let Some(origin) = origin
        {
            if let Some(menus) = window_menus.as_deref() {
                validate_menus(menus).map_err(|error| TestAppError::View(error.to_string()))?;
            }
            self.window_mut(origin)?.config.window_menus = window_menus;
        }

        let mut immediate = Vec::new();
        if let Some(origin) = origin {
            let previous = self.window(origin)?.ui.focused();
            if cx.clear_text_selection && self.window_mut(origin)?.ui.clear_static_text_selection()
            {
                self.window_mut(origin)?.dirty = true;
            }
            if let Some(request) = cx.focus {
                match request {
                    Some(element) if self.window(origin)?.ui.is_focusable(element) => {
                        let state = self.window_mut(origin)?;
                        state.pending_focus = None;
                        state.ui.focus(element);
                    }
                    Some(element) => {
                        let state = self.window_mut(origin)?;
                        state.pending_focus = Some(state.ui.pending_focus(element));
                        state.dirty = true;
                    }
                    None => {
                        let state = self.window_mut(origin)?;
                        state.pending_focus = None;
                        state.ui.blur();
                    }
                }
            }
            if cx.invalidate {
                self.window_mut(origin)?.dirty = true;
            }
            let focused = self.window(origin)?.ui.focused();
            if focused != previous {
                self.window_mut(origin)?.dirty = true;
                immediate.push(TestDispatch::Event(origin, Event::FocusChanged(focused)));
            }
            immediate.extend(
                cx.actions
                    .drain(..)
                    .map(|action| TestDispatch::Action(origin, action)),
            );
            immediate.extend(
                cx.form_submissions
                    .drain(..)
                    .map(|form| TestDispatch::Form(origin, form, None)),
            );
        } else if let Some(active) = self.active_window {
            immediate.extend(
                cx.actions
                    .drain(..)
                    .map(|action| TestDispatch::Action(active, action)),
            );
        }
        immediate.extend(
            cx.targeted_actions
                .drain(..)
                .filter(|(window, _)| self.windows.contains_key(window))
                .map(|(window, action)| TestDispatch::Action(window, action)),
        );
        self.queue_immediate(immediate)?;

        let entities = cx.entity_notifications;
        for state in self.windows.values_mut() {
            state.listeners.scopes.invalidate_entities(&entities);
            state
                .listeners
                .scopes
                .invalidate_globals(&cx.global_notifications);
            if state
                .listeners
                .observes_entity_change(&entities, cx.notify_all_entities)
                || state
                    .listeners
                    .observes_global_change(&cx.global_notifications, cx.notify_all_globals)
            {
                state.dirty = true;
            }
        }
        Ok(())
    }
}
