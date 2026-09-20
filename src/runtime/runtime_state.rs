use super::*;

impl Runtime {
    pub(super) fn new(
        startup: RuntimeStartup,
        event_proxy: EventLoopProxy<RuntimeEvent>,
    ) -> Result<Self, AppError> {
        let RuntimeStartup {
            initial_window,
            app_info,
            mut app_paths,
            globals,
            mut keymap,
            menus,
            assets,
            fonts,
            application_callbacks,
            quit_mode,
        } = startup;
        if app_paths.is_none()
            && let Some(info) = &app_info
        {
            app_paths = Some(
                info.paths()
                    .map_err(|error| AppError::Platform(error.to_string()))?,
            );
        }
        #[cfg(target_os = "windows")]
        if let Some(info) = &app_info
            && let Err(error) = windows_shell::set_current_app_id(info.identifier())
        {
            tracing::warn!(%error, "could not apply AppInfo as the Windows application identity");
        }
        let system_info = SystemInfo::current();
        let system_preferences = SystemPreferences::snapshot().unwrap_or_default();
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos")))]
        let pending_initial_open_urls = application_callbacks
            .open_urls
            .is_some()
            .then(deep_link::initial_open_urls)
            .flatten();
        #[cfg(any(target_arch = "wasm32", target_os = "macos"))]
        let pending_initial_open_urls = None;
        let font_system = create_shared_font_system(&assets, &fonts)?;
        let keyboard = KeyboardState::native();
        keymap.set_key_equivalents(keyboard.key_equivalents());
        validate_menus(&menus).map_err(|error| AppError::Platform(error.to_string()))?;
        let menu_actions = collect_menu_actions(&menus);
        let mut pending_windows = VecDeque::with_capacity(2);
        pending_windows.extend(initial_window);
        let image_workers = ImageWorkerPoolHandle::new(event_proxy.clone());
        let background_tasks = BackgroundTaskPoolHandle::new(event_proxy.clone());
        let foreground_tasks = ForegroundTaskSpawner::new(event_proxy.clone());
        let animation_epoch = Instant::now();
        #[cfg(target_os = "windows")]
        let windows_power_monitor = application_callbacks
            .power_event
            .is_some()
            .then(|| power_monitor::WindowsPowerMonitor::start(event_proxy.clone()))
            .transpose()
            .map_err(AppError::Platform)?;
        #[cfg(target_os = "linux")]
        let linux_power_monitor = if application_callbacks.power_event.is_some() {
            match power_monitor::LinuxPowerMonitor::start(event_proxy.clone()) {
                Ok(monitor) => Some(monitor),
                Err(error) => {
                    tracing::warn!(%error, "Linux power monitoring is unavailable");
                    None
                }
            }
        } else {
            None
        };
        #[cfg(target_os = "macos")]
        let mac_application_host = MacApplicationHost::new(
            event_proxy.clone(),
            application_callbacks.open_urls.is_some(),
            application_callbacks.reopen.is_some(),
            application_callbacks.before_quit.is_some()
                || application_callbacks.will_quit.is_some(),
            application_callbacks.system_wake.is_some()
                || application_callbacks.power_event.is_some(),
            application_callbacks.system_notification_response.is_some(),
        )
        .map_err(AppError::Platform)?;
        Ok(Self {
            pending_windows,
            pending_entity_events: VecDeque::with_capacity(8),
            pending_global_notifications: VecDeque::with_capacity(8),
            pending_global_notification_types: HashSet::with_capacity(8),
            pending_all_globals: false,
            targeted_actions: VecDeque::with_capacity(8),
            windows: HashMap::new(),
            pending_window_events: Vec::new(),
            window_handles: HashMap::new(),
            window_registry_cache: RefCell::new(WindowRegistryCache::default()),
            current_window: None,
            active_window: None,
            focus_history: Vec::new(),
            close_requests: Vec::new(),
            focus_requests: Vec::new(),
            invalidate_requests: Vec::new(),
            window_commands: Vec::with_capacity(8),
            external_menus: None,
            external_window_menus: VecDeque::with_capacity(2),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            external_popup_menus: VecDeque::with_capacity(2),
            pending_initial_open_urls,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            pending_native_popup_menus: HashMap::new(),
            platform_requests: VecDeque::with_capacity(8),
            pending_global_shortcut_commands: VecDeque::with_capacity(4),
            pending_tray_commands: VecDeque::with_capacity(4),
            pending_display_events: VecDeque::new(),
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            global_shortcut_state: global_shortcut::GlobalShortcutState::Pending,
            #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
            global_shortcut_state: global_shortcut::GlobalShortcutState::Unavailable,
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            global_shortcuts: HashMap::new(),
            tray_icons: HashMap::new(),
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            single_instance: None,
            image_workers,
            background_tasks,
            foreground_tasks,
            app_info,
            app_paths,
            system_info,
            system_preferences,
            globals,
            assets,
            font_system,
            gpu_contexts: HashMap::new(),
            #[cfg(target_arch = "wasm32")]
            web_canvas: None,
            displays: Displays::default(),
            keyboard,
            #[cfg(target_os = "macos")]
            native_drag_registry: MacTypedDragRegistry::new(),
            #[cfg(target_os = "macos")]
            popover_monitor: MacPopoverMonitor::new(event_proxy.clone()),
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            active_platform_dialogs: HashMap::new(),
            #[cfg(target_os = "macos")]
            automatic_tabbing_baseline: None,
            #[cfg(target_os = "macos")]
            tabbing_window_count: 0,
            #[cfg(target_os = "macos")]
            mac_application_host: Some(mac_application_host),
            #[cfg(target_os = "macos")]
            native_termination_pending: false,
            #[cfg(target_os = "windows")]
            _windows_power_monitor: windows_power_monitor,
            #[cfg(target_os = "linux")]
            _linux_power_monitor: linux_power_monitor,
            application_callbacks,
            quit_mode,
            ready: false,
            opened_window: false,
            exit_requested: false,
            exit_code: None,
            pending_quit: None,
            quit_phase_active: false,
            last_window_quit_prevented: false,
            relaunch_request: None,
            process_services_finalized: false,
            config: WindowOptions::default(),
            keymap,
            menus,
            menu_actions,
            #[cfg(target_os = "macos")]
            dock_menu: None,
            #[cfg(target_os = "macos")]
            dock_menu_actions: Vec::new(),
            #[cfg(target_os = "macos")]
            dock_badge: None,
            #[cfg(target_os = "macos")]
            dock_icon: None,
            #[cfg(target_os = "macos")]
            menu_host: None,
            #[cfg(target_os = "windows")]
            windows_menu_host: None,
            pending_input: None,
            window: None,
            modifiers: Modifiers::default(),
            fatal_error: None,
            event_proxy,
            clipboard: ClipboardService::system(),
            form_submission_depth: 0,
            animation_epoch,
        })
    }

    pub(super) fn event_context(&self) -> EventContext {
        let parent = self.window.as_ref().and_then(|window| window.parent);
        let popover_context = self.current_popover_context();
        EventContext::with_runtime(EventRuntimeContext {
            globals: self.globals.clone(),
            foreground_tasks: self.foreground_tasks.clone(),
            clipboard: self.clipboard.clone(),
            displays: self.displays.clone(),
            keyboard_layout: self.keyboard.layout().clone(),
            assets: self.assets.clone(),
            app_info: self.app_info.clone(),
            app_paths: self.app_paths.clone(),
            system_info: self.system_info.clone(),
            system_preferences: self.system_preferences,
            window_registry: self.window_registry(),
            window: crate::event::EventWindowContext {
                window: self.current_handle(),
                parent,
                popover_owner: popover_context.map(|context| context.owner),
                popover_root: popover_context.map(|context| context.root),
                pointer_position: self.window.as_ref().and_then(|window| window.pointer),
            },
        })
    }

    pub(super) fn invalidate_external(&mut self, handle: WindowHandle) -> bool {
        if self.current_handle() == Some(handle) {
            let Some(window) = self.window.as_mut() else {
                return false;
            };
            window.view_dirty = true;
            if window.visible && window.scheduler.invalidate() {
                window.window.request_redraw();
            }
            return true;
        }

        let Some(window_id) = self.window_handles.get(&handle).copied() else {
            return false;
        };
        let Some(entry) = self.windows.get_mut(&window_id) else {
            return false;
        };
        entry.state.view_dirty = true;
        if entry.state.visible && entry.state.scheduler.invalidate() {
            entry.state.window.request_redraw();
        }
        true
    }

    pub(super) fn invalidate_external_scopes(
        &mut self,
        handle: WindowHandle,
        ids: &[ElementId],
    ) -> bool {
        let window = if self.current_handle() == Some(handle) {
            self.window.as_mut()
        } else {
            self.window_handles
                .get(&handle)
                .and_then(|id| self.windows.get_mut(id))
                .map(|entry| &mut entry.state)
        };
        let Some(window) = window else {
            return false;
        };
        if ids.is_empty() {
            return true;
        }
        window.view_dirty |= !window.listeners.scopes.invalidate(ids);
        if window.visible && window.scheduler.invalidate() {
            window.window.request_redraw();
        }
        true
    }

    pub(super) fn update_external_elements(
        &mut self,
        handle: WindowHandle,
        updates: &[crate::ElementUpdate],
    ) -> Result<bool, crate::ui_tree::UiError> {
        let window = if self.current_handle() == Some(handle) {
            self.window.as_mut()
        } else {
            self.window_handles
                .get(&handle)
                .and_then(|id| self.windows.get_mut(id))
                .map(|entry| &mut entry.state)
        };
        let Some(window) = window else {
            return Ok(false);
        };
        if window.view_dirty {
            return Ok(true);
        }
        if window.listeners.needs_scoped_replacement(updates) {
            return Ok(false);
        }
        let Some(kind) = window.ui.update_elements(updates)? else {
            return Ok(false);
        };
        window.layout_dirty |= kind == crate::ui_tree::ElementUpdateKind::Layout;
        if kind != crate::ui_tree::ElementUpdateKind::None
            && window.visible
            && window.scheduler.invalidate()
        {
            window.window.request_redraw();
        }
        Ok(true)
    }

    pub(super) fn focus_external(&mut self, handle: WindowHandle, element: ElementId) -> bool {
        if self.current_handle() == Some(handle) {
            let Some(window) = self.window.as_mut() else {
                return false;
            };
            if !window.ui.is_focusable(element) {
                if !window.view_dirty {
                    return false;
                }
                window.pending_focus = Some(window.ui.pending_focus(element));
                if window.visible && window.scheduler.invalidate() {
                    window.window.request_redraw();
                }
                return true;
            }
            let changed = window.ui.focus(element);
            window.pending_focus = None;
            if changed {
                self.pending_input = None;
            }
            #[cfg(target_os = "macos")]
            if let Some(host) = &window.native_host {
                host.focus_framework();
            }
            window.view_dirty = true;
            if window.visible && window.scheduler.invalidate() {
                window.window.request_redraw();
            }
            return true;
        }

        let Some(window_id) = self.window_handles.get(&handle).copied() else {
            return false;
        };
        let Some(entry) = self.windows.get_mut(&window_id) else {
            return false;
        };
        if !entry.state.ui.is_focusable(element) {
            if !entry.state.view_dirty {
                return false;
            }
            entry.state.pending_focus = Some(entry.state.ui.pending_focus(element));
            if entry.state.visible && entry.state.scheduler.invalidate() {
                entry.state.window.request_redraw();
            }
            return true;
        }
        let changed = entry.state.ui.focus(element);
        entry.state.pending_focus = None;
        if changed {
            entry.pending_input = None;
        }
        #[cfg(target_os = "macos")]
        if let Some(host) = &entry.state.native_host {
            host.focus_framework();
        }
        entry.state.view_dirty = true;
        if entry.state.visible && entry.state.scheduler.invalidate() {
            entry.state.window.request_redraw();
        }
        true
    }

    pub(super) fn current_popover_context(&self) -> Option<PopoverWindowContext> {
        if self.config.kind != WindowKind::SystemPopover {
            return None;
        }
        let mut root = self.current_handle()?;
        let mut ancestor = self.window.as_ref()?.parent?;
        loop {
            let window_id = *self.window_handles.get(&ancestor)?;
            let entry = self.windows.get(&window_id)?;
            if entry.config.kind != WindowKind::SystemPopover {
                return Some(PopoverWindowContext {
                    owner: ancestor,
                    root,
                });
            }
            root = ancestor;
            ancestor = entry.state.parent?;
        }
    }

    pub(super) fn current_never_key_popover_children(&self) -> Vec<WindowHandle> {
        let Some(owner) = self.current_handle() else {
            return Vec::new();
        };
        self.windows
            .values()
            .filter_map(|entry| {
                (entry.state.visible
                    && entry.state.parent == Some(owner)
                    && window_is_never_key_popover(&entry.config))
                .then_some(entry.handle)
            })
            .collect()
    }

    #[cfg(target_os = "macos")]
    pub(super) fn popovers_to_close_after_application_deactivation(&self) -> Vec<WindowHandle> {
        let candidates = self
            .windows
            .values()
            .filter_map(|entry| {
                (entry.state.visible
                    && window_dismisses_system_popover_on_pointer_outside(&entry.config))
                .then_some(entry.handle)
            })
            .collect::<Vec<_>>();

        // One close request tears down its whole child tree. Keep only the highest dismissible
        // ancestor so nested system popovers cannot produce duplicate close callbacks.
        let mut roots = candidates
            .iter()
            .copied()
            .filter(|handle| {
                !candidates.iter().copied().any(|ancestor| {
                    ancestor != *handle && self.window_is_ancestor(ancestor, *handle)
                })
            })
            .collect::<Vec<_>>();
        roots.sort_unstable();
        roots
    }

    pub(super) fn fail(&mut self, event_loop: &ActiveEventLoop, error: AppError) {
        tracing::error!(%error, "QuickGUI is exiting after a fatal error");
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::error_1(&error.to_string().into());
            if let Some(window) = web_sys::window() {
                if let Ok(event) = web_sys::CustomEvent::new("quickgui:error") {
                    let _ = window.dispatch_event(&event);
                }
            }
        }
        self.fatal_error = Some(error);
        event_loop.exit();
    }

    pub(super) fn activate_window(&mut self, window_id: WindowId) -> bool {
        debug_assert!(self.window.is_none());
        debug_assert!(self.current_window.is_none());
        let Some(entry) = self.windows.remove(&window_id) else {
            return false;
        };
        self.current_window = Some((window_id, entry.handle));
        self.config = entry.config;
        self.pending_input = entry.pending_input;
        self.modifiers = entry.modifiers;
        self.window = Some(entry.state);
        true
    }

    pub(super) fn deactivate_window(&mut self) {
        let Some((window_id, handle)) = self.current_window.take() else {
            return;
        };
        let state = self
            .window
            .take()
            .expect("an activated window always owns runtime state");
        let entry = WindowEntry {
            handle,
            config: std::mem::take(&mut self.config),
            pending_input: self.pending_input.take(),
            modifiers: std::mem::take(&mut self.modifiers),
            state,
        };
        let previous = self.windows.insert(window_id, entry);
        debug_assert!(previous.is_none());
    }

    pub(super) fn current_handle(&self) -> Option<WindowHandle> {
        self.current_window.map(|(_, handle)| handle)
    }

    /// Whether `handle` names a window whose platform creation is still queued.
    pub(super) fn window_is_pending(&self, handle: WindowHandle) -> bool {
        self.pending_windows
            .iter()
            .any(|request| request.handle == handle)
    }

    pub(super) fn active_window_handle(&self) -> Option<WindowHandle> {
        let active = self.active_window?;
        if self.current_window.is_some_and(|(id, _)| id == active) {
            return self.current_handle();
        }
        self.windows.get(&active).map(|entry| entry.handle)
    }

    pub(super) fn window_registry(&self) -> WindowRegistry {
        let total = self.window_handles.len() + self.pending_windows.len();
        let mut cache = self.window_registry_cache.borrow_mut();
        let cache_matches = total <= MAX_APPLICATION_WINDOWS
            && cache.handles.len() == total
            && self
                .window_handles
                .keys()
                .all(|handle| cache.handles.binary_search(handle).is_ok())
            && self
                .pending_windows
                .iter()
                .all(|request| cache.handles.binary_search(&request.handle).is_ok());
        if !cache_matches {
            let mut handles = Vec::with_capacity(total.min(MAX_APPLICATION_WINDOWS));
            handles.extend(
                self.window_handles
                    .keys()
                    .copied()
                    .take(MAX_APPLICATION_WINDOWS),
            );
            handles.extend(
                self.pending_windows
                    .iter()
                    .take(MAX_APPLICATION_WINDOWS.saturating_sub(handles.len()))
                    .map(|request| request.handle),
            );
            handles.sort_unstable();
            handles.dedup();
            handles.truncate(MAX_APPLICATION_WINDOWS);
            cache.handles = handles.into();
            cache.truncated = total > MAX_APPLICATION_WINDOWS;
        } else {
            cache.truncated = false;
        }
        let active_window = self.active_window_handle();
        if let Some(active) = active_window
            && cache.handles.binary_search(&active).is_err()
        {
            cache.truncated = true;
            let mut handles = cache.handles.to_vec();
            if let Some(last) = handles.last_mut() {
                *last = active;
                handles.sort_unstable();
                cache.handles = handles.into();
            }
        }
        WindowRegistry::new(cache.handles.clone(), active_window, cache.truncated)
    }

    pub(super) fn window_state_for(&self, handle: WindowHandle) -> Option<WindowState> {
        if self.current_handle() == Some(handle) {
            return self.current_window_state();
        }
        let window_id = self.window_handles.get(&handle)?;
        let entry = self.windows.get(window_id)?;
        Some(runtime_window_state(handle, &entry.config, &entry.state))
    }

    pub(super) fn frame_metrics_for(&self, handle: WindowHandle) -> Option<FrameMetrics> {
        if self.current_handle() == Some(handle) {
            return self.window.as_ref().map(|state| state.metrics.current());
        }
        let window_id = self.window_handles.get(&handle)?;
        let entry = self.windows.get(window_id)?;
        Some(entry.state.metrics.current())
    }

    pub(super) fn current_window_state(&self) -> Option<WindowState> {
        let handle = self.current_handle()?;
        let state = self.window.as_ref()?;
        Some(runtime_window_state(handle, &self.config, state))
    }

    #[cfg(target_os = "macos")]
    pub(super) fn register_native_tabbing(&mut self, event_loop: &ActiveEventLoop) {
        if self.tabbing_window_count == 0 {
            let baseline = event_loop.allows_automatic_window_tabbing();
            self.automatic_tabbing_baseline = Some(baseline);
            if !baseline {
                event_loop.set_allows_automatic_window_tabbing(true);
            }
        }
        self.tabbing_window_count = self.tabbing_window_count.saturating_add(1);
    }

    #[cfg(target_os = "macos")]
    pub(super) fn unregister_native_tabbing(&mut self, event_loop: &ActiveEventLoop) {
        self.tabbing_window_count = self.tabbing_window_count.saturating_sub(1);
        if self.tabbing_window_count == 0
            && let Some(baseline) = self.automatic_tabbing_baseline.take()
        {
            event_loop.set_allows_automatic_window_tabbing(baseline);
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn restore_native_tabbing_baseline(&mut self, event_loop: &ActiveEventLoop) {
        self.tabbing_window_count = 0;
        if let Some(baseline) = self.automatic_tabbing_baseline.take() {
            event_loop.set_allows_automatic_window_tabbing(baseline);
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn refresh_native_tab_states(&mut self) {
        for entry in self.windows.values_mut() {
            if entry.config.tabbing_identifier.is_none()
                && entry.state.native_tabs == WindowTabState::default()
            {
                continue;
            }
            let Ok(tabs) = window_tab_state(&entry.state.window) else {
                continue;
            };
            if entry.state.native_tabs != tabs {
                entry.state.native_tabs = tabs;
                if entry.state.listeners.observes_window_state {
                    entry.state.view_dirty = true;
                    if entry.state.visible && entry.state.scheduler.invalidate() {
                        entry.state.window.request_redraw();
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn refresh_current_native_tab_state(&mut self) {
        if self.config.tabbing_identifier.is_none()
            && self
                .window
                .as_ref()
                .is_none_or(|state| state.native_tabs == WindowTabState::default())
        {
            return;
        }
        let Some(state) = self.window.as_mut() else {
            return;
        };
        let Ok(tabs) = window_tab_state(&state.window) else {
            return;
        };
        if state.native_tabs != tabs {
            state.native_tabs = tabs;
            if state.listeners.observes_window_state {
                state.view_dirty = true;
                if state.visible && state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
            }
        }
    }

    /// Refresh the bounded monitor snapshot only at a native lifecycle boundary.
    ///
    /// This is intentionally absent from `about_to_wait`: unchanged applications retain no
    /// monitor polling cost and no display-owned native handles in public state.
    pub(super) fn refresh_displays(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let displays = crate::display::native_displays(event_loop);
        let snapshot_changed = self.displays != displays;
        if snapshot_changed && self.application_callbacks.display_event.is_some() {
            self.pending_display_events
                .extend(self.displays.diff(&displays));
        }
        self.displays = displays;

        let refresh_window = |state: &mut RuntimeWindow| {
            let previous = state.display_id;
            state.display_id = runtime_window_display_id(state, &self.displays);
            let display_changed = previous != state.display_id;
            if snapshot_changed && state.listeners.observes_displays
                || display_changed && state.listeners.observes_window_state
            {
                state.view_dirty = true;
                if state.visible && state.scheduler.invalidate() {
                    state.window.request_redraw();
                }
            }
        };

        for entry in self.windows.values_mut() {
            refresh_window(&mut entry.state);
        }
        if let Some(state) = &mut self.window {
            refresh_window(state);
        }
        snapshot_changed
    }

    #[cfg(target_os = "macos")]
    pub(super) fn refresh_keyboard_layout(&mut self) -> bool {
        let keyboard = KeyboardState::native();
        if self.keyboard == keyboard {
            return false;
        }
        self.keyboard = keyboard;
        self.keymap
            .set_key_equivalents(self.keyboard.key_equivalents());

        // A prefix cannot safely span two command layouts. Key releases remain independently
        // translated by their native event, while future presses use the new immutable table.
        self.pending_input = None;
        for entry in self.windows.values_mut() {
            entry.pending_input = None;
            if entry.state.listeners.observes_keyboard_layout {
                entry.state.view_dirty = true;
                if entry.state.visible && entry.state.scheduler.invalidate() {
                    entry.state.window.request_redraw();
                }
            }
        }
        if let Some(state) = &mut self.window
            && state.listeners.observes_keyboard_layout
        {
            state.view_dirty = true;
            if state.visible && state.scheduler.invalidate() {
                state.window.request_redraw();
            }
        }
        true
    }

    pub(super) fn refresh_system_preferences(&mut self, preferences: SystemPreferences) -> bool {
        if self.system_preferences == preferences {
            return false;
        }
        self.system_preferences = preferences;
        let now = Instant::now();

        let refresh_window =
            |state: &mut RuntimeWindow, config: &WindowOptions, preferences: SystemPreferences| {
                let reduce_motion = config.reduce_motion
                    || preferences.reduce_motion().is_some_and(|enabled| enabled);
                if state.reduce_motion != reduce_motion {
                    state.reduce_motion = reduce_motion;
                    state.ui.set_reduce_motion(reduce_motion);
                    state
                        .ui
                        .set_animations_enabled(!state.occluded && !reduce_motion, now);
                }
                if state.listeners.observes_system_preferences {
                    state.view_dirty = true;
                    if state.visible && state.scheduler.invalidate() {
                        state.window.request_redraw();
                    }
                }
            };

        for entry in self.windows.values_mut() {
            refresh_window(&mut entry.state, &entry.config, preferences);
        }
        if let Some(state) = &mut self.window {
            refresh_window(state, &self.config, preferences);
        }
        true
    }

    pub(super) fn note_window_focused(&mut self, window_id: WindowId) {
        self.focus_history
            .retain(|candidate| *candidate != window_id);
        self.focus_history.push(window_id);
        self.active_window = Some(window_id);
    }

    pub(super) fn process_entity_events(
        &mut self,
        event_loop: &ActiveEventLoop,
        deliveries: &mut usize,
    ) -> bool {
        debug_assert!(self.current_window.is_none());
        debug_assert!(self.window.is_none());

        while let Some(event) = self.pending_entity_events.pop_front() {
            let mut targets = self
                .windows
                .iter()
                .filter_map(|(window_id, entry)| {
                    entry
                        .state
                        .listeners
                        .has_entity_event_subscribers(event.source, event.event_type)
                        .then_some((entry.handle, *window_id))
                })
                .collect::<Vec<_>>();
            targets.sort_unstable_by_key(|(handle, _)| *handle);

            for (_, window_id) in targets {
                if !self.activate_window(window_id) {
                    continue;
                }
                let callbacks = self
                    .window
                    .as_ref()
                    .map(|window| {
                        window
                            .listeners
                            .entity_event_callbacks(event.source, event.event_type)
                    })
                    .unwrap_or_default();

                for callback in callbacks {
                    if !reserve_entity_event_delivery(deliveries) {
                        self.deactivate_window();
                        self.fail(
                            event_loop,
                            AppError::View(format!(
                                "one effect cycle exceeded {MAX_ENTITY_EVENT_DELIVERIES_PER_TURN} entity-event callback deliveries"
                            )),
                        );
                        return false;
                    }
                    let mut cx = self.event_context();
                    if let Some(window) = &mut self.window {
                        callback.borrow_mut()(
                            window.view.as_any_mut(),
                            event.value.as_ref(),
                            &mut cx,
                        );
                    }
                    if !self.apply_event_context(event_loop, cx, false, true) {
                        self.deactivate_window();
                        return false;
                    }
                }
                self.deactivate_window();
            }
        }
        true
    }

    pub(super) fn process_global_notifications(
        &mut self,
        event_loop: &ActiveEventLoop,
        deliveries: &mut usize,
    ) -> bool {
        debug_assert!(self.current_window.is_none());
        debug_assert!(self.window.is_none());

        loop {
            let global_type = if self.pending_all_globals {
                self.pending_all_globals = false;
                self.pending_global_notifications.clear();
                self.pending_global_notification_types.clear();
                None
            } else if let Some(global_type) = self.pending_global_notifications.pop_front() {
                self.pending_global_notification_types.remove(&global_type);
                Some(global_type)
            } else {
                break;
            };

            let mut targets = self
                .windows
                .iter()
                .filter_map(|(window_id, entry)| {
                    entry
                        .state
                        .listeners
                        .has_global_subscribers(global_type)
                        .then_some((entry.handle, *window_id))
                })
                .collect::<Vec<_>>();
            targets.sort_unstable_by_key(|(handle, _)| *handle);

            for (_, window_id) in targets {
                if !self.activate_window(window_id) {
                    continue;
                }
                let subscriptions = self
                    .window
                    .as_ref()
                    .map(|window| window.listeners.global_subscriptions(global_type))
                    .unwrap_or_default();
                for subscription in subscriptions {
                    // A preceding callback may have dropped this subscription from the same view.
                    if !subscription.is_active() {
                        continue;
                    }
                    if !reserve_global_observer_delivery(deliveries) {
                        self.deactivate_window();
                        self.fail(
                            event_loop,
                            AppError::View(format!(
                                "one effect cycle exceeded {MAX_GLOBAL_OBSERVER_DELIVERIES_PER_TURN} global observer callback deliveries"
                            )),
                        );
                        return false;
                    }
                    let mut cx = self.event_context();
                    if let Some(window) = &mut self.window {
                        subscription.callback.borrow_mut()(window.view.as_any_mut(), &mut cx);
                    }
                    if !self.apply_event_context(event_loop, cx, false, true) {
                        self.deactivate_window();
                        return false;
                    }
                }
                self.deactivate_window();
            }
        }
        true
    }

    pub(super) fn process_deferred_effects(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let mut global_deliveries = 0_usize;
        let mut entity_deliveries = 0_usize;
        loop {
            if !self.process_global_notifications(event_loop, &mut global_deliveries)
                || !self.process_entity_events(event_loop, &mut entity_deliveries)
            {
                return false;
            }
            if !self.pending_all_globals
                && self.pending_global_notifications.is_empty()
                && self.pending_entity_events.is_empty()
            {
                return true;
            }
        }
    }
}
