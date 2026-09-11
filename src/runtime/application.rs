use super::*;

/// A QuickGUI application whose native event loop is advanced by an external runtime.
///
/// Create and pump this value on the platform application thread. Each call still dispatches
/// redraw and lifecycle callbacks synchronously inside Winit, which is required for correct macOS
/// resize behavior. A blocking [`Application::run`] remains the simplest choice for ordinary Rust
/// apps.
#[cfg(not(target_arch = "wasm32"))]
pub struct AppRunner {
    pub(super) event_loop: EventLoop<RuntimeEvent>,
    pub(super) runtime: Runtime,
    pub(super) root_window: WindowHandle,
    pub(super) root_window_pending: bool,
    pub(super) status: AppRunStatus,
    pub(super) relaunched_process: Option<RelaunchedProcess>,
}

#[cfg(not(target_arch = "wasm32"))]
impl AppRunner {
    /// Advance native events until a redraw completes, the timeout elapses, or the app exits.
    ///
    /// `None` may block indefinitely. External runtimes can pair it with [`Self::waker`] so their
    /// command producer interrupts the blocked pump without periodic polling.
    pub fn pump(&mut self, timeout: Option<Duration>) -> Result<AppRunStatus, AppError> {
        if matches!(self.status, AppRunStatus::Exited(_)) {
            return Ok(self.status);
        }
        let status = self.event_loop.pump_app_events(timeout, &mut self.runtime);
        if let Some(error) = self.runtime.fatal_error.take() {
            self.status = AppRunStatus::Exited(1);
            self.runtime.relaunch_request.take();
            self.runtime.finalize_process_services();
            return Err(error);
        }
        self.status = match status {
            PumpStatus::Continue => AppRunStatus::Continue,
            PumpStatus::Exit(code) => AppRunStatus::Exited(code),
        };
        if matches!(self.status, AppRunStatus::Exited(_)) {
            self.runtime.finalize_process_services();
            if let Some(request) = self.runtime.relaunch_request.take() {
                self.relaunched_process = Some(
                    request
                        .spawn()
                        .map_err(|error| AppError::Platform(error.to_string()))?,
                );
            }
        }
        Ok(self.status)
    }

    /// Return a thread-safe handle that interrupts a blocking [`Self::pump`] call.
    pub fn waker(&self) -> AppRunnerWaker {
        AppRunnerWaker {
            proxy: self.runtime.event_proxy.clone(),
        }
    }

    /// Stable handle of the initial application window.
    ///
    /// A windowless [`Application`] reserves this handle until [`Self::open_window`] queues its
    /// first top-level window.
    pub const fn root_window(&self) -> WindowHandle {
        self.root_window
    }

    /// Whether the platform application completed its native initialization.
    ///
    /// A newly created runner becomes ready during its first [`Self::pump`] call, even when it
    /// does not yet own a window. Embedding runtimes should wait for this boundary before opening
    /// their first window.
    pub const fn is_ready(&self) -> bool {
        self.runtime.ready
    }

    /// Window currently activated for a synchronous core callback, if any.
    ///
    /// View and event callbacks should normally use [`ViewContext::window_handle`] and
    /// [`EventContext::window_handle`] directly. This accessor lets bindings project that same
    /// core context into their host language without maintaining separate window identity.
    pub fn current_window(&self) -> Option<WindowHandle> {
        self.runtime.current_handle()
    }

    /// Read the hardware pointer in global logical desktop coordinates.
    pub fn cursor_screen_position(&self) -> Result<Point, PlatformError> {
        cursor_screen_position(&self.runtime.displays)
    }

    /// Immutable package identity supplied before application startup.
    pub fn app_info(&self) -> Option<&AppInfo> {
        self.runtime.app_info.as_ref()
    }

    /// Standard application paths resolved once during startup.
    pub fn app_paths(&self) -> Option<&AppPaths> {
        self.runtime.app_paths.as_ref()
    }

    /// Immutable operating-system and preferred-language snapshot captured at startup.
    pub fn system_info(&self) -> &SystemInfo {
        &self.runtime.system_info
    }

    /// Native integration backends compiled for this target.
    pub const fn desktop_integration_support(&self) -> DesktopIntegrationSupport {
        DesktopIntegrationSupport::current()
    }

    /// Current application-wide native menu declaration, including a queued replacement.
    pub fn application_menus(&self) -> &[Menu] {
        self.runtime
            .external_menus
            .as_deref()
            .unwrap_or(&self.runtime.menus)
    }

    /// Effective native menu declaration for one queued or mounted window.
    pub fn window_menus(&self, handle: WindowHandle) -> Option<&[Menu]> {
        let application_menus = self.application_menus();
        if self.runtime.current_handle() == Some(handle) {
            return Some(
                self.runtime
                    .config
                    .window_menus
                    .as_deref()
                    .unwrap_or(application_menus),
            );
        }
        if let Some(window_id) = self.runtime.window_handles.get(&handle)
            && let Some(entry) = self.runtime.windows.get(window_id)
        {
            return Some(
                entry
                    .config
                    .window_menus
                    .as_deref()
                    .unwrap_or(application_menus),
            );
        }
        self.runtime
            .pending_windows
            .iter()
            .find(|request| request.handle == handle)
            .map(|request| {
                request
                    .options
                    .window_menus
                    .as_deref()
                    .unwrap_or(application_menus)
            })
    }

    /// Current bounded system appearance and accessibility-preference snapshot.
    pub const fn system_preferences(&self) -> SystemPreferences {
        self.runtime.system_preferences
    }

    /// Immutable bounded lookup of mounted and queued application windows.
    pub fn window_registry(&self) -> WindowRegistry {
        self.runtime.window_registry()
    }

    pub fn active_window(&self) -> Option<WindowHandle> {
        self.runtime.active_window_handle()
    }

    /// Queue a new top-level window from an embedding runtime.
    ///
    /// The handle is stable immediately. The platform window is created during the next event
    /// loop turn, so callers can finish installing retained state before pumping again.
    pub fn open_window<V: View>(
        &mut self,
        options: WindowOptions,
        view: V,
    ) -> Result<WindowHandle, AppError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(AppError::Window(
                "cannot open a window after the application event loop exited".to_owned(),
            ));
        }
        validate_window_options(&options).map_err(|error| AppError::Window(error.to_string()))?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| AppError::Window("application event loop is closed".to_owned()))?;
        let request = if self.root_window_pending {
            self.root_window_pending = false;
            WindowRequest::with_handle(view, options, None, self.root_window)
        } else {
            WindowRequest::new(view, options)
        };
        let handle = request.handle;
        self.runtime.pending_windows.push_back(request);
        Ok(handle)
    }

    /// Queue a retained QuickGUI surface whose AppKit view will be mounted by a native host.
    ///
    /// The returned handle is stable immediately, just like [`Self::open_window`]. The hidden
    /// backing NSWindow is never presented; it exists only to preserve Winit's event identity and
    /// the existing renderer, input, IME, and accessibility pipelines while its rendering NSView
    /// is reparented into SwiftUI.
    #[cfg(all(target_os = "macos", feature = "swift-ui"))]
    pub fn open_embedded_view<V: View>(
        &mut self,
        owner: WindowHandle,
        mut options: WindowOptions,
        match_horizontal: bool,
        match_vertical: bool,
        view: V,
    ) -> Result<crate::MacEmbeddedView, AppError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(AppError::Window(
                "cannot open an embedded view after the application event loop exited".to_owned(),
            ));
        }
        let owner_exists = self.runtime.current_handle() == Some(owner)
            || self.runtime.window_handles.contains_key(&owner)
            || self
                .runtime
                .pending_windows
                .iter()
                .any(|request| request.handle == owner);
        if !owner_exists {
            return Err(AppError::Window(
                "an embedded view requires a queued or mounted owner window".to_owned(),
            ));
        }
        options.show = false;
        options.focus = false;
        options.window_bounds = None;
        options.minimum_size = None;
        options.maximum_size = None;
        options.decorated = false;
        options.title_bar_style = TitleBarStyle::Hidden;
        options.shadow = false;
        options.is_movable = false;
        options.is_resizable = false;
        options.is_minimizable = false;
        options.is_maximizable = false;
        options.is_closable = false;
        options.size.width = options.size.width.max(1.0);
        options.size.height = options.size.height.max(1.0);
        validate_window_options(&options).map_err(|error| AppError::Window(error.to_string()))?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| AppError::Window("application event loop is closed".to_owned()))?;
        let handle = WindowHandle::next();
        let embedded = crate::MacEmbeddedView::pending(
            handle,
            owner,
            options.size,
            match_horizontal,
            match_vertical,
        );
        let mut request = WindowRequest::with_handle(view, options, Some(owner), handle);
        request.embedded = Some(embedded.clone());
        self.runtime.pending_windows.push_back(request);
        Ok(embedded)
    }

    /// Queue a native popover anchored to one currently mounted element in a parent window.
    ///
    /// Embedding runtimes may call this before the parent's first presented frame. Resolution is
    /// queued with the child request and occurs at the window-creation boundary, after every
    /// earlier parent request has completed retained layout. This preserves the same display-aware
    /// behavior and core-owned trigger focus restoration as [`EventContext::open_system_popover`]
    /// without polling or a geometry observer.
    pub fn open_system_popover<V: View>(
        &mut self,
        parent: WindowHandle,
        anchor: ElementId,
        options: WindowOptions,
        view: V,
    ) -> Result<WindowHandle, AppError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(AppError::Window(
                "cannot open a popover after the application event loop exited".to_owned(),
            ));
        }
        validate_window_options(&options).map_err(|error| AppError::Window(error.to_string()))?;
        if options.kind != WindowKind::SystemPopover || options.popover.is_none() {
            return Err(AppError::Window(
                WindowCommandError::InvalidPopoverConfiguration.to_string(),
            ));
        }
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| AppError::Window("application event loop is closed".to_owned()))?;
        let mut request = WindowRequest::with_parent(view, options, Some(parent));
        request.popover_anchor_element = Some(anchor);
        let handle = request.handle;
        self.runtime.pending_windows.push_back(request);
        Ok(handle)
    }

    /// Mark one externally owned view dirty and request at most one native redraw.
    ///
    /// A window queued for creation also returns `true`: its first render will read the newest
    /// retained state without scheduling a redundant frame.
    pub fn invalidate_window(&mut self, handle: WindowHandle) -> bool {
        if !matches!(self.status, AppRunStatus::Continue) {
            return false;
        }
        if self
            .runtime
            .pending_windows
            .iter()
            .any(|request| request.handle == handle)
        {
            return true;
        }
        self.runtime.invalidate_external(handle)
    }

    /// Update mounted text or paint properties without rebuilding the view declaration.
    ///
    /// Update the embedding runtime's source state first, so a later ordinary view rebuild
    /// preserves these values. Returns `false` without applying the batch when its targets are
    /// not mounted or are owned by a container-query/animation callback; call
    /// [`Self::invalidate_window`] in that case. A pending view rebuild already reads the latest
    /// source state and accepts the batch without redundant work.
    /// Structural replacements in a scoped renderer return `false`; use
    /// [`Self::invalidate_elements`] so its declarations can update callback ownership too.
    pub fn update_elements(
        &mut self,
        handle: WindowHandle,
        updates: &[crate::ElementUpdate],
    ) -> Result<bool, AppError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Ok(false);
        }
        self.runtime
            .update_external_elements(handle, updates)
            .map_err(|error| AppError::View(error.to_string()))
    }

    /// Re-declare identified component scopes after changing an embedding renderer's source.
    /// Coalesces sibling mutations into one frame and falls back to the root when any requested
    /// scope is absent. Scope callbacks run during redraw, never synchronously in this call.
    pub fn invalidate_elements(&mut self, handle: WindowHandle, ids: &[ElementId]) -> bool {
        if !matches!(self.status, AppRunStatus::Continue) {
            return false;
        }
        self.runtime.invalidate_external_scopes(handle, ids)
    }

    /// Focus one mounted element from an embedding runtime.
    ///
    /// This is the imperative counterpart to [`crate::Element::auto_focus`]. It is intended for
    /// host bindings that expose web-like `element.focus()` behavior after an external event has
    /// returned to the host language. The request is applied synchronously, reasserts native
    /// keyboard ownership, and schedules one redraw even when the element was already logically
    /// focused.
    pub fn focus_element(&mut self, handle: WindowHandle, element: ElementId) -> bool {
        if !matches!(self.status, AppRunStatus::Continue) {
            return false;
        }
        self.runtime.focus_external(handle, element)
    }

    /// Present a platform-native prompt owned by one mounted window.
    ///
    /// This is the embedding-runtime counterpart to [`EventContext::prompt`]. The returned
    /// future remains on the platform thread and resolves after the operating system closes the
    /// prompt; no redraw polling is introduced while it is visible.
    pub fn prompt(
        &mut self,
        window: WindowHandle,
        level: PromptLevel,
        message: impl Into<Arc<str>>,
        detail: Option<&str>,
        buttons: &[PromptButton],
    ) -> Result<PlatformResponse<usize>, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue)
            || !self.runtime.window_handles.contains_key(&window)
        {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) =
            PlatformRequest::prompt(window, level, message, detail.map(Arc::from), buttons)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Present an application-modal native prompt without attaching it to a window.
    pub fn prompt_application(
        &mut self,
        level: PromptLevel,
        message: impl Into<Arc<str>>,
        detail: Option<&str>,
        buttons: &[PromptButton],
    ) -> Result<PlatformResponse<usize>, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) =
            PlatformRequest::application_prompt(level, message, detail.map(Arc::from), buttons)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Present a platform-native open panel owned by one mounted window.
    ///
    /// `Ok(None)` means the user cancelled. The returned future stays on the platform thread and
    /// resolves after the operating system closes the panel.
    pub fn prompt_for_paths(
        &mut self,
        window: WindowHandle,
        options: PathPromptOptions,
    ) -> Result<PathPromptResponse, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue)
            || !self.runtime.window_handles.contains_key(&window)
        {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) = PlatformRequest::open_paths(window, options)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Present an application-modal native open panel without attaching it to a window.
    pub fn prompt_for_paths_application(
        &mut self,
        options: PathPromptOptions,
    ) -> Result<PathPromptResponse, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) = PlatformRequest::application_open_paths(options)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Present a platform-native save panel owned by one mounted window.
    ///
    /// `Ok(None)` means the user cancelled. The returned future stays on the platform thread and
    /// resolves after the operating system closes the panel.
    pub fn prompt_for_new_path(
        &mut self,
        window: WindowHandle,
        options: SavePathOptions,
    ) -> Result<SavePathResponse, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue)
            || !self.runtime.window_handles.contains_key(&window)
        {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) = PlatformRequest::save_path(window, options)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Present an application-modal native save panel without attaching it to a window.
    pub fn prompt_for_new_path_application(
        &mut self,
        options: SavePathOptions,
    ) -> Result<SavePathResponse, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        let (request, response) = PlatformRequest::application_save_path(options)?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(response)
    }

    /// Close a queued or mounted window.
    ///
    /// Returns `false` when the handle is unknown or the application already exited.
    pub fn close_window(&mut self, handle: WindowHandle) -> bool {
        if !matches!(self.status, AppRunStatus::Continue) {
            return false;
        }
        let pending = self
            .runtime
            .pending_windows
            .iter()
            .position(|request| request.handle == handle);
        let mounted = self.runtime.window_handles.contains_key(&handle)
            || self.runtime.current_handle() == Some(handle);
        if pending.is_none() && !mounted {
            return false;
        }
        if self
            .runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .is_err()
        {
            return false;
        }
        if let Some(index) = pending {
            self.runtime.pending_windows.remove(index);
        } else if !self.runtime.close_requests.contains(&handle) {
            self.runtime.close_requests.push(handle);
        }
        true
    }

    /// Mark the root view dirty and request exactly one native redraw.
    ///
    /// Returns `false` only after exit or before the first pump has mounted the root window. State
    /// installed before that first pump is naturally read by the initial render.
    pub fn invalidate_root(&mut self) -> bool {
        self.invalidate_window(self.root_window)
    }

    pub const fn status(&self) -> AppRunStatus {
        self.status
    }

    /// Replacement process spawned after this runner completed orderly teardown, if any.
    pub const fn relaunched_process(&self) -> Option<RelaunchedProcess> {
        self.relaunched_process
    }
}

/// Live application context supplied after native launch and to application callbacks.
///
/// This is the application-wide form of [`EventContext`]. A launch callback has no current
/// window; windows are created with [`App::open_window`].
pub type App = EventContext;

/// Configures and starts a native application independently from its windows.
///
/// Ordinary Rust applications call [`Self::run`] and open their initial windows from its launch
/// callback. Language bindings and other embedders can instead convert it into an [`AppRunner`],
/// pump once until [`AppRunner::is_ready`] is true, then queue a window with
/// [`AppRunner::open_window`].
pub struct Application {
    pub(super) app_info: Option<AppInfo>,
    pub(super) app_paths: Option<AppPaths>,
    pub(super) keymap: Keymap,
    pub(super) menus: Vec<Menu>,
    pub(super) globals: GlobalStore,
    pub(super) assets: Assets,
    pub(super) fonts: Vec<FontSource>,
    pub(super) application_callbacks: ApplicationCallbacks,
    pub(super) quit_mode: QuitMode,
}

impl Application {
    pub fn new() -> Self {
        Self {
            app_info: None,
            app_paths: None,
            keymap: Keymap::default(),
            menus: Vec::new(),
            globals: GlobalStore::default(),
            assets: Assets::default(),
            fonts: Vec::new(),
            application_callbacks: ApplicationCallbacks::default(),
            quit_mode: QuitMode::Default,
        }
    }

    /// Configure when closing the final window terminates the application.
    pub fn quit_mode(mut self, mode: QuitMode) -> Self {
        self.quit_mode = mode;
        self
    }

    /// GPUI-compatible alias for [`Self::quit_mode`].
    pub fn with_quit_mode(self, mode: QuitMode) -> Self {
        self.quit_mode(mode)
    }

    /// Install the immutable package identity exposed by every core context.
    ///
    /// Standard application paths are resolved from its identifier at startup unless
    /// [`Self::app_paths`] supplies an explicit snapshot.
    pub fn app_info(mut self, info: AppInfo) -> Self {
        self.app_info = Some(info);
        self
    }

    pub fn with_app_info(self, info: AppInfo) -> Self {
        self.app_info(info)
    }

    /// Override the standard path snapshot retained by the application core.
    pub fn app_paths(mut self, paths: AppPaths) -> Self {
        self.app_paths = Some(paths);
        self
    }

    pub fn with_app_paths(self, paths: AppPaths) -> Self {
        self.app_paths(paths)
    }

    /// Install the immutable application asset source used by every window.
    pub fn with_assets(mut self, source: impl crate::AssetSource) -> Self {
        self.assets = Assets::new(source);
        self
    }

    /// Install an already shared application asset handle.
    pub fn assets(mut self, assets: Assets) -> Self {
        self.assets = assets;
        self
    }

    /// Register one custom OpenType font file or asset path before launch.
    pub fn font(mut self, font: impl Into<FontSource>) -> Self {
        self.fonts.push(font.into());
        self
    }

    /// Register custom fonts in declaration order before launch.
    pub fn fonts(mut self, fonts: impl IntoIterator<Item = impl Into<FontSource>>) -> Self {
        self.fonts.extend(fonts.into_iter().map(Into::into));
        self
    }

    /// Add application key bindings. Later bindings take precedence at equal context depth.
    pub fn bind_keys(mut self, bindings: impl IntoIterator<Item = KeyBinding>) -> Self {
        self.keymap.add_bindings(bindings);
        self
    }

    /// Replace the complete application keymap.
    pub fn keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = keymap;
        self
    }

    /// Append one declarative application menu.
    pub fn menu(mut self, menu: Menu) -> Self {
        self.menus.push(menu);
        self
    }

    /// Replace the complete declarative application menu set.
    pub fn menus(mut self, menus: impl IntoIterator<Item = Menu>) -> Self {
        self.menus = menus.into_iter().collect();
        self
    }

    /// Handle a typed action after the focused window's listeners, or without a window.
    ///
    /// Application handlers keep commands such as Open available after the last window closes.
    /// Registering another handler for the same action type replaces the previous one.
    pub fn on_action<A: Action>(
        mut self,
        mut callback: impl FnMut(&A, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.actions.insert(
            TypeId::of::<A>(),
            Box::new(move |action, cx| {
                callback(
                    action.downcast_ref::<A>().expect("registered action type"),
                    cx,
                );
            }),
        );
        self
    }

    /// Install or replace one main-thread application-global value before launch.
    pub fn global<G: Global>(self, global: G) -> Self {
        self.globals.set(global);
        self
    }

    /// Handle URLs supplied by the operating system, including `file:` URLs.
    pub fn on_open_urls(
        mut self,
        callback: impl FnMut(OpenUrls, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.open_urls = Some(Box::new(callback));
        self
    }

    /// Handle a Dock/Finder request to reopen an already-running macOS application.
    pub fn on_reopen(mut self, callback: impl FnMut(bool, &mut EventContext) + 'static) -> Self {
        self.application_callbacks.reopen = Some(Box::new(callback));
        self
    }

    /// Handle the operating system waking from sleep.
    pub fn on_system_wake(mut self, callback: impl FnMut(&mut EventContext) + 'static) -> Self {
        self.application_callbacks.system_wake = Some(Box::new(callback));
        self
    }

    /// Handle a native keyboard-layout change.
    pub fn on_keyboard_layout_change(
        mut self,
        callback: impl FnMut(&KeyboardLayout, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.keyboard_layout = Some(Box::new(callback));
        self
    }

    /// Handle activation of a delivered system notification or action button.
    pub fn on_system_notification_response(
        mut self,
        callback: impl FnMut(SystemNotificationResponse, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.system_notification_response = Some(Box::new(callback));
        self
    }

    /// Handle a registered system-wide keyboard shortcut when it is pressed.
    pub fn on_global_shortcut(
        mut self,
        callback: impl FnMut(GlobalShortcutEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.global_shortcut = Some(Box::new(callback));
        self
    }

    /// Handle arguments and the working directory forwarded by a later process.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn on_second_instance(
        mut self,
        callback: impl FnMut(SecondInstanceEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.second_instance = Some(Box::new(callback));
        self
    }

    /// Handle native power, thermal, shutdown, and login-session transitions.
    pub fn on_power_event(
        mut self,
        callback: impl FnMut(PowerEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.power_event = Some(Box::new(callback));
        self
    }

    /// Handle clicks, scrolling, and native menu actions from application tray icons.
    pub fn on_tray_event(
        mut self,
        callback: impl FnMut(TrayEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.tray_event = Some(Box::new(callback));
        self
    }

    /// Handle the application becoming the frontmost application.
    ///
    /// macOS delivers this from `NSApplicationDidBecomeActiveNotification`. It reuses the
    /// application observer QuickGUI already installs, so it adds no additional native observer,
    /// timer, or polling pass. Other platforms never invoke it today.
    pub fn on_did_become_active(
        mut self,
        callback: impl FnMut(&mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.did_become_active = Some(Box::new(callback));
        self
    }

    /// Handle the application losing frontmost status.
    ///
    /// macOS delivers this from `NSApplicationDidResignActiveNotification`, the same observer that
    /// already dismisses grabbing system popovers. Other platforms never invoke it today.
    pub fn on_did_resign_active(
        mut self,
        callback: impl FnMut(&mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.did_resign_active = Some(Box::new(callback));
        self
    }

    /// Handle granular display additions, removals, and metric changes.
    ///
    /// The coarse snapshot exposed by [`EventContext::displays`] keeps working unchanged. This
    /// callback only describes what moved between two consecutive snapshots, so an application can
    /// react without re-scanning every display. Events are delivered in ascending
    /// [`crate::DisplayId`] order and are produced only when the operating system reports a
    /// reconfiguration; no polling or timer is installed.
    pub fn on_display_event(
        mut self,
        callback: impl FnMut(DisplayEvent, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.display_event = Some(Box::new(callback));
        self
    }

    /// Handle a color chosen in the system color panel.
    ///
    /// [`crate::ColorPanelMode::Continuous`] reports every intermediate color while the user drags
    /// inside the panel; [`crate::ColorPanelMode::OnClose`] reports only the final color once the
    /// panel is dismissed. No timer or observer runs while the panel is closed.
    pub fn on_color_panel_change(
        mut self,
        callback: impl FnMut(Color, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.color_panel_change = Some(Box::new(callback));
        self
    }

    /// Handle a font chosen in the system font panel.
    ///
    /// QuickGUI's inherited [`Font`] carries no point size, so the reported value describes the
    /// chosen family, weight, and slant.
    pub fn on_font_panel_change(
        mut self,
        callback: impl FnMut(Font, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.font_panel_change = Some(Box::new(callback));
        self
    }

    /// Run after a native window and its owned resources have been removed.
    pub fn on_window_closed(
        mut self,
        callback: impl FnMut(WindowHandle, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.window_closed = Some(Box::new(callback));
        self
    }

    /// Run the first preventable phase of an orderly application quit.
    pub fn on_before_quit(
        mut self,
        callback: impl FnMut(QuitRequest, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.before_quit = Some(Box::new(callback));
        self
    }

    /// Run the final preventable phase immediately before owned windows are torn down.
    pub fn on_will_quit(
        mut self,
        callback: impl FnMut(QuitRequest, &mut EventContext) + 'static,
    ) -> Self {
        self.application_callbacks.will_quit = Some(Box::new(callback));
        self
    }

    /// Apply identity and bundled fonts written by `quickgui` packaging, when present.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn apply_packaged_cli_metadata_from(&mut self, path: &Path) -> Result<(), AppError> {
        let bytes = std::fs::read(path).map_err(|error| {
            AppError::Platform(format!(
                "could not read packaged QuickGUI metadata {}: {error}",
                path.display()
            ))
        })?;
        let metadata: PackagedCliMetadata = serde_json::from_slice(&bytes).map_err(|error| {
            AppError::Platform(format!(
                "invalid packaged QuickGUI metadata {}: {error}",
                path.display()
            ))
        })?;
        self.app_info = Some(
            AppInfo::new(metadata.name, metadata.version, metadata.identifier)
                .map_err(|error| AppError::Platform(error.to_string()))?,
        );
        let resource_dir = path.parent().unwrap_or_else(|| Path::new("."));
        for relative in metadata.fonts {
            let font_path = resource_dir.join(&relative);
            let font = std::fs::read(&font_path).map_err(|error| {
                AppError::Platform(format!(
                    "could not read packaged font {}: {error}",
                    font_path.display()
                ))
            })?;
            self.fonts.push(FontSource::from(font));
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn apply_packaged_cli_metadata(&mut self) -> Result<(), AppError> {
        let Some(path) = packaged_cli_metadata_path() else {
            return Ok(());
        };
        self.apply_packaged_cli_metadata_from(&path)
    }

    /// Convert this windowless application into an externally pumped native event loop.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn into_runner(mut self) -> Result<AppRunner, AppError> {
        self.apply_packaged_cli_metadata()?;
        let event_loop = EventLoop::with_user_event().build()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let root_window = WindowHandle::next();
        let runtime = Runtime::new(
            RuntimeStartup {
                initial_window: None,
                app_info: self.app_info,
                app_paths: self.app_paths,
                globals: self.globals,
                keymap: self.keymap,
                menus: self.menus,
                assets: self.assets,
                fonts: self.fonts,
                application_callbacks: self.application_callbacks,
                quit_mode: self.quit_mode,
            },
            event_loop.create_proxy(),
        )?;
        Ok(AppRunner {
            event_loop,
            runtime,
            root_window,
            root_window_pending: true,
            status: AppRunStatus::Continue,
            relaunched_process: None,
        })
    }

    /// Start the native event loop and invoke `on_finish_launching` once application-wide native
    /// initialization is complete.
    ///
    /// The callback receives a windowless [`App`] context. Open the initial window there with
    /// [`App::open_window`].
    pub fn run(
        mut self,
        on_finish_launching: impl FnOnce(&mut App) + 'static,
    ) -> Result<(), AppError> {
        self.application_callbacks.finish_launching = Some(Box::new(on_finish_launching));

        #[cfg(not(target_arch = "wasm32"))]
        {
            let AppRunner {
                event_loop,
                mut runtime,
                ..
            } = self.into_runner()?;
            let run_result = event_loop.run_app(&mut runtime);
            let fatal_error = runtime.fatal_error.take();
            let relaunch = runtime.relaunch_request.take();
            let exit_code = runtime.exit_code.take();
            runtime.finalize_process_services();
            drop(runtime);
            run_result?;
            if let Some(error) = fatal_error {
                return Err(error);
            }
            if let Some(request) = relaunch {
                request
                    .spawn()
                    .map_err(|error| AppError::Platform(error.to_string()))?;
            }
            // Structured teardown already ran; only the process status is left to apply.
            if let Some(code) = exit_code.filter(|code| *code != 0) {
                std::process::exit(code);
            }
            Ok(())
        }

        #[cfg(target_arch = "wasm32")]
        Err(AppError::Platform(
            "use Application::run_web for asynchronous browser startup".to_owned(),
        ))
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(serde::Deserialize)]
struct PackagedCliMetadata {
    name: String,
    version: String,
    identifier: String,
    #[serde(default)]
    fonts: Vec<String>,
}

#[cfg(not(target_arch = "wasm32"))]
fn packaged_cli_metadata_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("QUICKGUI_METADATA") {
        return Some(PathBuf::from(path));
    }
    let executable = std::env::current_exe().ok()?;
    let resource_dir = packaged_resource_dir(&executable);
    let path = resource_dir.join("quickgui.json");
    path.is_file().then_some(path)
}

#[cfg(not(target_arch = "wasm32"))]
fn packaged_resource_dir(executable: &Path) -> PathBuf {
    let executable_dir = executable.parent().unwrap_or_else(|| Path::new("."));
    #[cfg(target_os = "macos")]
    if executable_dir
        .file_name()
        .is_some_and(|name| name == "MacOS")
        && let Some(contents) = executable_dir.parent()
        && contents.file_name().is_some_and(|name| name == "Contents")
    {
        return contents.join("Resources");
    }
    executable_dir.to_path_buf()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod packaged_cli_tests {
    use super::*;

    #[test]
    fn packaged_cli_metadata_overrides_app_info() {
        let unique = std::process::id();
        let dir = std::env::temp_dir().join(format!("quickgui-cli-metadata-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("quickgui.json");
        std::fs::create_dir_all(dir.join("fonts")).unwrap();
        std::fs::write(dir.join("fonts/bundled.ttf"), b"font-bytes").unwrap();
        std::fs::write(
            &path,
            r#"{"name":"Packaged","version":"2.0.0","identifier":"dev.quickgui.packaged","fonts":["fonts/bundled.ttf"]}"#,
        )
        .unwrap();
        let mut application = Application::new()
            .app_info(AppInfo::new("Original", "0.0.1", "dev.quickgui.original").unwrap());
        application.apply_packaged_cli_metadata_from(&path).unwrap();
        let info = application.app_info.as_ref().unwrap();
        assert_eq!(info.name(), "Packaged");
        assert_eq!(info.version(), "2.0.0");
        assert_eq!(info.identifier(), "dev.quickgui.packaged");
        assert_eq!(application.fonts.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(target_arch = "wasm32")]
impl Application {
    /// Start the retained runtime on a browser canvas. GPU initialization is
    /// asynchronous; subsequent window setup reuses this ready device and never
    /// blocks the browser event loop on a GPU promise.
    pub async fn run_web(
        mut self,
        canvas: web_sys::HtmlCanvasElement,
        on_finish_launching: impl FnOnce(&mut App) + 'static,
    ) -> Result<(), AppError> {
        use winit::platform::web::EventLoopExtWebSys;
        let profile = PerformanceProfile::Balanced;
        let gpu = GpuContext::for_canvas(canvas.clone(), profile)
            .await
            .map_err(|error| AppError::GraphicsInitialization(error.to_string()))?;
        self.application_callbacks.finish_launching = Some(Box::new(on_finish_launching));
        let event_loop = EventLoop::with_user_event().build()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let mut runtime = Runtime::new(
            RuntimeStartup {
                initial_window: None,
                app_info: self.app_info,
                app_paths: self.app_paths,
                globals: self.globals,
                keymap: self.keymap,
                menus: self.menus,
                assets: self.assets,
                fonts: self.fonts,
                application_callbacks: self.application_callbacks,
                quit_mode: self.quit_mode,
            },
            event_loop.create_proxy(),
        )?;
        runtime.web_canvas = Some(canvas);
        runtime.gpu_contexts.insert(profile, gpu);
        event_loop.spawn_app(runtime);
        Ok(())
    }
}
