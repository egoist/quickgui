use super::*;
use crate::{MacOsVibrancy, MacOsVisualEffectState};

/// Commands emitted while a [`crate::View`] handles an event.
#[derive(Debug, Default)]
pub struct EventContext {
    pub(crate) globals: GlobalStore,
    pub(crate) app_info: Option<AppInfo>,
    pub(crate) app_paths: Option<AppPaths>,
    pub(crate) system_info: SystemInfo,
    pub(crate) system_preferences: SystemPreferences,
    pub(crate) window_registry: WindowRegistry,
    pub(crate) foreground_tasks: Option<ForegroundTaskSpawner>,
    pub(crate) clipboard: Option<ClipboardService>,
    pub(crate) displays: Displays,
    pub(crate) keyboard_layout: KeyboardLayout,
    pub(crate) assets: Assets,
    pub(crate) window: Option<WindowHandle>,
    pub(crate) parent_window: Option<WindowHandle>,
    pub(crate) popover_owner_window: Option<WindowHandle>,
    pub(crate) popover_root_window: Option<WindowHandle>,
    pub(crate) pointer_position: Option<Point>,
    pub(crate) invalidate: bool,
    pub(crate) exit: bool,
    pub(crate) relaunch: Option<RelaunchRequest>,
    pub(crate) focus: Option<Option<ElementId>>,
    pub(crate) clear_text_selection: bool,
    pub(crate) actions: Vec<AnyAction>,
    pub(crate) targeted_actions: Vec<(WindowHandle, AnyAction)>,
    pub(crate) menus: Option<Vec<Menu>>,
    pub(crate) window_menus: Option<Option<Vec<Menu>>>,
    pub(crate) native_popup_menus: Vec<NativePopupMenuRequest>,
    pub(crate) propagate_action: bool,
    pub(crate) stop_event_propagation: bool,
    pub(crate) prevent_default: bool,
    pub(crate) open_windows: Vec<WindowRequest>,
    pub(crate) close_current_window: bool,
    pub(crate) close_windows: Vec<WindowHandle>,
    pub(crate) focus_windows: Vec<WindowHandle>,
    pub(crate) invalidate_windows: Vec<WindowHandle>,
    pub(crate) window_commands: Vec<WindowCommand>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) constrained_size: Option<Size>,
    pub(crate) constrained_position: Option<Point>,
    pub(crate) platform_requests: Vec<PlatformRequest>,
    pub(crate) prevent_close: bool,
    pub(crate) prevent_quit: bool,
    pub(crate) form_submissions: Vec<ElementId>,
    pub(crate) entity_notifications: Vec<EntityId>,
    pub(crate) notify_all_entities: bool,
    pub(crate) entity_events: Vec<EntityEvent>,
    pub(crate) global_notifications: Vec<TypeId>,
    pub(crate) notify_all_globals: bool,
}

#[derive(Debug)]
pub(crate) struct NativePopupMenuRequest {
    #[cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]
    pub(crate) menu: Menu,
    #[cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]
    pub(crate) position: Option<Point>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EventWindowContext {
    pub(crate) window: Option<WindowHandle>,
    pub(crate) parent: Option<WindowHandle>,
    pub(crate) popover_owner: Option<WindowHandle>,
    pub(crate) popover_root: Option<WindowHandle>,
    pub(crate) pointer_position: Option<Point>,
}

pub(crate) struct EventRuntimeContext {
    pub(crate) globals: GlobalStore,
    pub(crate) foreground_tasks: ForegroundTaskSpawner,
    pub(crate) clipboard: ClipboardService,
    pub(crate) displays: Displays,
    pub(crate) keyboard_layout: KeyboardLayout,
    pub(crate) assets: Assets,
    pub(crate) app_info: Option<AppInfo>,
    pub(crate) app_paths: Option<AppPaths>,
    pub(crate) system_info: SystemInfo,
    pub(crate) system_preferences: SystemPreferences,
    pub(crate) window_registry: WindowRegistry,
    pub(crate) window: EventWindowContext,
}

impl EventContext {
    pub(crate) fn with_runtime(runtime: EventRuntimeContext) -> Self {
        Self {
            globals: runtime.globals,
            app_info: runtime.app_info,
            app_paths: runtime.app_paths,
            system_info: runtime.system_info,
            system_preferences: runtime.system_preferences,
            window_registry: runtime.window_registry,
            foreground_tasks: Some(runtime.foreground_tasks),
            clipboard: Some(runtime.clipboard),
            displays: runtime.displays,
            keyboard_layout: runtime.keyboard_layout,
            assets: runtime.assets,
            window: runtime.window.window,
            parent_window: runtime.window.parent,
            popover_owner_window: runtime.window.popover_owner,
            popover_root_window: runtime.window.popover_root,
            pointer_position: runtime.window.pointer_position,
            invalidate: false,
            exit: false,
            relaunch: None,
            focus: None,
            clear_text_selection: false,
            actions: Vec::new(),
            targeted_actions: Vec::new(),
            menus: None,
            window_menus: None,
            native_popup_menus: Vec::new(),
            propagate_action: false,
            stop_event_propagation: false,
            prevent_default: false,
            open_windows: Vec::new(),
            close_current_window: false,
            close_windows: Vec::new(),
            focus_windows: Vec::new(),
            invalidate_windows: Vec::new(),
            window_commands: Vec::new(),
            exit_code: None,
            constrained_size: None,
            constrained_position: None,
            platform_requests: Vec::new(),
            prevent_close: false,
            prevent_quit: false,
            form_submissions: Vec::new(),
            entity_notifications: Vec::new(),
            notify_all_entities: false,
            entity_events: Vec::new(),
            global_notifications: Vec::new(),
            notify_all_globals: false,
        }
    }

    /// Read the latest bounded display snapshot without polling the operating system.
    pub fn displays(&self) -> &[Display] {
        self.displays.all()
    }

    /// The complete immutable display snapshot, for APIs that need primary and identity lookup.
    pub const fn display_snapshot(&self) -> &Displays {
        &self.displays
    }

    pub fn primary_display(&self) -> Option<&Display> {
        self.displays.primary()
    }

    pub fn find_display(&self, id: DisplayId) -> Option<&Display> {
        self.displays.find(id)
    }

    /// Read the latest immutable keyboard-layout snapshot without querying the platform.
    pub fn keyboard_layout(&self) -> &KeyboardLayout {
        &self.keyboard_layout
    }

    /// Access the application's immutable asset source.
    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    /// GPUI-shaped alias for [`Self::assets`].
    pub fn asset_source(&self) -> &Assets {
        self.assets()
    }

    /// Immutable package identity supplied before application startup.
    pub fn app_info(&self) -> Option<&AppInfo> {
        self.app_info.as_ref()
    }

    /// Standard application paths resolved once during startup.
    pub fn app_paths(&self) -> Option<&AppPaths> {
        self.app_paths.as_ref()
    }

    /// Immutable operating-system and preferred-language snapshot captured at startup.
    pub fn system_info(&self) -> &SystemInfo {
        &self.system_info
    }

    /// Current bounded system appearance and accessibility-preference snapshot.
    pub const fn system_preferences(&self) -> SystemPreferences {
        self.system_preferences
    }

    /// Bounded immutable lookup of mounted and queued application windows at this event boundary.
    pub fn window_registry(&self) -> &WindowRegistry {
        &self.window_registry
    }

    pub fn windows(&self) -> &[WindowHandle] {
        self.window_registry.windows()
    }

    pub const fn active_window(&self) -> Option<WindowHandle> {
        self.window_registry.active_window()
    }

    /// Read the hardware pointer in global logical desktop coordinates.
    pub fn cursor_screen_position(&self) -> Result<Point, PlatformError> {
        crate::runtime::cursor_screen_position(&self.displays)
    }

    /// Read a bounded item from the operating system's general clipboard.
    ///
    /// The operation is synchronous and belongs on QuickGUI's application thread. macOS checks
    /// native NSData lengths before copying text or encoded image bytes into Rust-owned storage.
    pub fn read_from_clipboard(&self) -> Result<Option<ClipboardItem>, ClipboardError> {
        self.clipboard
            .as_ref()
            .ok_or(ClipboardError::Unavailable)?
            .read(ClipboardTarget::General)
    }

    /// Replace the operating system's general clipboard with one validated item.
    ///
    /// Writing [`ClipboardItem::default`] clears the clipboard.
    pub fn write_to_clipboard(&self, item: ClipboardItem) -> Result<(), ClipboardError> {
        self.clipboard
            .as_ref()
            .ok_or(ClipboardError::Unavailable)?
            .write(ClipboardTarget::General, item)
    }

    /// Read Linux's primary-selection clipboard, commonly pasted with the middle mouse button.
    #[cfg(target_os = "linux")]
    pub fn read_from_selection_clipboard(&self) -> Result<Option<ClipboardItem>, ClipboardError> {
        self.clipboard
            .as_ref()
            .ok_or(ClipboardError::Unavailable)?
            .read(ClipboardTarget::Selection)
    }

    /// Replace Linux's primary-selection clipboard. An empty item clears it.
    #[cfg(target_os = "linux")]
    pub fn write_to_selection_clipboard(&self, item: ClipboardItem) -> Result<(), ClipboardError> {
        self.clipboard
            .as_ref()
            .ok_or(ClipboardError::Unavailable)?
            .write(ClipboardTarget::Selection, item)
    }

    /// Read macOS's shared Find pasteboard without polling it.
    #[cfg(target_os = "macos")]
    pub fn read_from_find_pasteboard(&self) -> Result<Option<ClipboardItem>, ClipboardError> {
        self.clipboard
            .as_ref()
            .ok_or(ClipboardError::Unavailable)?
            .read(ClipboardTarget::Find)
    }

    /// Replace macOS's shared Find pasteboard. An empty item clears it.
    #[cfg(target_os = "macos")]
    pub fn write_to_find_pasteboard(&self, item: ClipboardItem) -> Result<(), ClipboardError> {
        self.clipboard
            .as_ref()
            .ok_or(ClipboardError::Unavailable)?
            .write(ClipboardTarget::Find, item)
    }

    /// Run a non-blocking future for the current window on the application thread.
    ///
    /// Annotate the callback context with the current view type so async updates remain typed:
    /// `cx.spawn(|cx: AsyncViewContext<MyView>| async move { ... })`. Dropping the returned
    /// [`Task`] cancels it; detaching keeps it alive until completion or window close.
    pub fn spawn<V, Build, Fut, R>(&self, build: Build) -> Result<Task<R>, ForegroundTaskSpawnError>
    where
        V: 'static,
        Build: FnOnce(AsyncViewContext<V>) -> Fut,
        Fut: Future<Output = R> + 'static,
        R: 'static,
    {
        let foreground_tasks = self
            .foreground_tasks
            .as_ref()
            .ok_or(ForegroundTaskSpawnError::Unavailable)?;
        let window = self.window.ok_or(ForegroundTaskSpawnError::Unavailable)?;
        foreground_tasks.spawn::<V, _, _, _>(window, build)
    }

    /// Mark the window dirty. Calls are coalesced into a single redraw.
    pub fn invalidate(&mut self) {
        self.invalidate = true;
    }

    /// Notify windows that observed this shared entity during their latest retained render.
    ///
    /// Repeated notifications for the same entity in one callback coalesce. Prefer
    /// [`Entity::update`], which performs the mutation and notification together.
    pub fn notify<T>(&mut self, entity: &Entity<T>) {
        if self.notify_all_entities || self.entity_notifications.contains(&entity.id()) {
            return;
        }
        if self.entity_notifications.len() == MAX_ENTITY_NOTIFICATIONS_PER_EVENT {
            self.entity_notifications.clear();
            self.notify_all_entities = true;
            return;
        }
        self.entity_notifications.push(entity.id());
    }

    /// Emit one typed event from an entity.
    ///
    /// Delivery is deferred until the current callback releases all application borrows, then
    /// runs in FIFO order across subscribing windows. Unlike state notifications, events are not
    /// coalesced. `false` means this callback reached its hard event-count limit.
    pub fn emit<T, E>(&mut self, entity: &Entity<T>, event: E) -> bool
    where
        T: EventEmitter<E>,
        E: Any,
    {
        if self.entity_events.len() == MAX_ENTITY_EVENTS_PER_CALLBACK {
            return false;
        }
        self.entity_events.push(EntityEvent::new(entity, event));
        true
    }

    /// Whether an application-global value of this type has been installed.
    pub fn has_global<G: Global>(&self) -> bool {
        self.globals.has::<G>()
    }

    /// Read an application-global value without subscribing the current window.
    ///
    /// Panics if the value has not been installed. Use [`Self::try_global`] for an optional read.
    pub fn global<G: Global>(&self) -> Ref<'_, G> {
        self.globals.get::<G>()
    }

    /// Read an application-global value if one has been installed.
    pub fn try_global<G: Global>(&self) -> Option<Ref<'_, G>> {
        self.globals.try_get::<G>()
    }

    /// Mutably access an application-global value and notify its observers after this callback.
    ///
    /// Like GPUI, requesting mutable access is treated as a change. The returned guard is
    /// main-thread-only and fails loudly if application code attempts an overlapping global borrow.
    pub fn global_mut<G: Global>(&mut self) -> RefMut<'_, G> {
        self.note_global_changed(TypeId::of::<G>());
        self.globals.get_mut::<G>()
    }

    /// Mutably access a global, inserting its default value if it is not installed yet.
    pub fn default_global<G: Global + Default>(&mut self) -> RefMut<'_, G> {
        self.note_global_changed(TypeId::of::<G>());
        self.globals.default_mut::<G>()
    }

    /// Install or replace one application-global value and notify observers after this callback.
    pub fn set_global<G: Global>(&mut self, global: G) {
        self.globals.set(global);
        self.note_global_changed(TypeId::of::<G>());
    }

    /// Mutate one global inside a scoped borrow and return the callback result.
    pub fn update_global<G: Global, R>(&mut self, update: impl FnOnce(&mut G) -> R) -> R {
        self.note_global_changed(TypeId::of::<G>());
        let mut global = self.globals.get_mut::<G>();
        update(&mut global)
    }

    /// Remove and return one application-global value, notifying observers after this callback.
    pub fn remove_global<G: Global>(&mut self) -> G {
        let global = self.globals.remove::<G>();
        self.note_global_changed(TypeId::of::<G>());
        global
    }

    pub(super) fn note_global_changed(&mut self, global_type: TypeId) {
        if self.notify_all_globals || self.global_notifications.contains(&global_type) {
            return;
        }
        if self.global_notifications.len() == MAX_GLOBAL_NOTIFICATIONS_PER_EVENT {
            self.global_notifications.clear();
            self.notify_all_globals = true;
            return;
        }
        self.global_notifications.push(global_type);
    }

    /// Ask the application event loop to exit cleanly.
    ///
    /// Every owned native window is torn down child-first, foreground work is cancelled, and the
    /// application-level window-closed callback runs before the event loop terminates.
    pub fn exit(&mut self) {
        self.exit = true;
    }

    /// Relaunch the current executable after orderly application teardown.
    ///
    /// The current arguments and working directory are preserved. Preparing the request happens
    /// synchronously so an invalid process environment cannot turn into a silent post-exit error.
    pub fn relaunch(&mut self) -> Result<(), SystemIntegrationError> {
        self.relaunch_with(RelaunchOptions::default())
    }

    /// Relaunch with explicit process overrides after orderly application teardown.
    pub fn relaunch_with(
        &mut self,
        options: RelaunchOptions,
    ) -> Result<(), SystemIntegrationError> {
        self.relaunch = Some(options.prepare()?);
        self.exit = true;
        Ok(())
    }

    /// Create another native window hosting an independently retained view.
    ///
    /// The stable handle is available immediately, before the platform window is mounted.
    pub fn open_window<V: View>(&mut self, options: WindowOptions, view: V) -> WindowHandle {
        let request = WindowRequest::with_parent(view, options, self.window);
        let handle = request.handle;
        self.open_windows.push(request);
        handle
    }

    /// Open a parent-owned native popover anchored to the latest retained bounds of an element.
    ///
    /// Unlike an in-window overlay, this popover owns a separate native window and WGPU surface, so
    /// it may extend beyond the parent window while the platform constrains it to the display work
    /// area. The anchor is resolved after the listener returns and before any invalidated rebuild;
    /// it also remains the parent's focus-restoration target until the child closes. No geometry
    /// observer, polling task, or hard-coded duplicate rectangle is required.
    pub fn open_system_popover<V: View>(
        &mut self,
        anchor: impl Into<ElementId>,
        options: WindowOptions,
        view: V,
    ) -> Result<WindowHandle, WindowCommandError> {
        if self.window.is_none() {
            return Err(WindowCommandError::Unavailable);
        }
        if options.kind != crate::WindowKind::SystemPopover || options.popover.is_none() {
            return Err(WindowCommandError::InvalidPopoverConfiguration);
        }
        let mut request = WindowRequest::with_parent(view, options, self.window);
        request.popover_anchor_element = Some(anchor.into());
        let handle = request.handle;
        self.open_windows.push(request);
        Ok(handle)
    }

    pub fn window_handle(&self) -> Option<WindowHandle> {
        self.window
    }

    /// Latest logical pointer position in the current native window, when the pointer is inside.
    ///
    /// The value is captured from the input event being delivered and never polls the platform.
    /// It is therefore safe to use from hover callbacks, whose compact payload contains only the
    /// entered/exited state.
    pub const fn pointer_position(&self) -> Option<Point> {
        self.pointer_position
    }

    fn current_window_handle(&self) -> Result<WindowHandle, WindowCommandError> {
        self.window.ok_or(WindowCommandError::Unavailable)
    }

    fn push_window_command(&mut self, command: WindowCommand) -> Result<(), WindowCommandError> {
        if self.window_commands.len() == crate::MAX_WINDOW_COMMANDS_PER_EVENT {
            return Err(WindowCommandError::QueueFull);
        }
        self.window_commands.push(command);
        Ok(())
    }

    fn push_platform_request(&mut self, request: PlatformRequest) -> Result<(), PlatformError> {
        if self.platform_requests.len() == crate::MAX_PLATFORM_REQUESTS_PER_EVENT {
            return Err(PlatformError::QueueFull);
        }
        self.platform_requests.push(request);
        Ok(())
    }

    fn ensure_platform_capacity(&self) -> Result<(), PlatformError> {
        if self.platform_requests.len() == crate::MAX_PLATFORM_REQUESTS_PER_EVENT {
            Err(PlatformError::QueueFull)
        } else {
            Ok(())
        }
    }

    /// Present a native prompt owned by the current window.
    ///
    /// The returned future resolves to the selected button index. Construct it in an event
    /// callback, then await it from [`Self::spawn`]; the native sheet performs no redraw polling.
    pub fn prompt(
        &mut self,
        level: PromptLevel,
        message: impl Into<Arc<str>>,
        detail: Option<&str>,
        buttons: &[PromptButton],
    ) -> Result<PlatformResponse<usize>, PlatformError> {
        self.ensure_platform_capacity()?;
        let window = self.window.ok_or(PlatformError::Unavailable)?;
        let (request, response) =
            PlatformRequest::prompt(window, level, message, detail.map(Arc::from), buttons)?;
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Present a native open panel owned by the current window.
    ///
    /// `Ok(None)` means the user cancelled. Selected paths retain their platform-native bytes and
    /// are bounded by [`crate::MAX_SELECTED_PATHS`] and
    /// [`crate::MAX_SELECTED_PATHS_TOTAL_BYTES`].
    pub fn prompt_for_paths(
        &mut self,
        options: PathPromptOptions,
    ) -> Result<PathPromptResponse, PlatformError> {
        self.ensure_platform_capacity()?;
        let window = self.window.ok_or(PlatformError::Unavailable)?;
        let (request, response) = PlatformRequest::open_paths(window, options)?;
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Present a native save panel owned by the current window.
    ///
    /// `Ok(None)` means the user cancelled.
    pub fn prompt_for_new_path(
        &mut self,
        options: SavePathOptions,
    ) -> Result<SavePathResponse, PlatformError> {
        self.ensure_platform_capacity()?;
        let window = self.window.ok_or(PlatformError::Unavailable)?;
        let (request, response) = PlatformRequest::save_path(window, options)?;
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Present a native message box with an optional checkbox, custom icon, and key buttons.
    ///
    /// The returned future resolves to the chosen button index and the final checkbox state, so a
    /// "do not ask again" affordance needs no second round trip. When this context owns a window
    /// the box is presented as that window's sheet; otherwise it is application-modal.
    ///
    /// Only macOS implements the checkbox and the custom icon. The portable backend rejects those
    /// two options with [`PlatformError::Unsupported`] rather than dropping them silently.
    pub fn message_box(
        &mut self,
        options: MessageBoxOptions,
    ) -> Result<MessageBoxResponseFuture, PlatformError> {
        self.ensure_platform_capacity()?;
        let (request, response) = match self.window {
            Some(window) => PlatformRequest::message_box(window, options)?,
            None => PlatformRequest::application_message_box(options)?,
        };
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Show the operating system's file preview panel for one path.
    ///
    /// `display_name` replaces the panel's title and is bounded by
    /// [`crate::MAX_FILE_PREVIEW_NAME_BYTES`]. macOS implements this with Quick Look; other
    /// desktops report [`PlatformError::Unsupported`].
    pub fn preview_file(
        &mut self,
        path: impl Into<PathBuf>,
        display_name: Option<&str>,
    ) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().file_previews {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::preview_file(path, display_name.map(Arc::from))?;
        self.push_platform_request(request)
    }

    /// Show the file preview panel and observe whether the platform accepted the request.
    pub fn preview_file_response(
        &mut self,
        path: impl Into<PathBuf>,
        display_name: Option<&str>,
    ) -> Result<ShellResponse, PlatformError> {
        self.ensure_platform_capacity()?;
        let (request, response) =
            PlatformRequest::preview_file_response(path, display_name.map(Arc::from))?;
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Close the operating system's file preview panel if it is open.
    pub fn close_file_preview(&mut self) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().file_previews {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::close_file_preview())
    }

    /// Close the file preview panel and observe whether the platform accepted the request.
    pub fn close_file_preview_response(&mut self) -> Result<ShellResponse, PlatformError> {
        self.ensure_platform_capacity()?;
        let (request, response) = PlatformRequest::close_file_preview_response();
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Present the system color panel and observe the user's selection.
    ///
    /// Install the observer with `Application::on_color_panel_change` before showing the panel.
    /// The panel is a shared system resource: showing it again re-seeds its color and replaces the
    /// reporting mode. macOS implements this; other desktops report [`PlatformError::Unsupported`].
    pub fn show_color_panel(
        &mut self,
        initial: Color,
        mode: ColorPanelMode,
    ) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().color_panel {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::show_color_panel(initial, mode))
    }

    /// Dismiss the system color panel and stop observing it.
    pub fn close_color_panel(&mut self) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().color_panel {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::close_color_panel())
    }

    /// Present the system font panel seeded with `font`.
    ///
    /// Install the observer with `Application::on_font_panel_change` before showing the panel.
    pub fn show_font_panel(&mut self, font: Font) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().font_panel {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::show_font_panel(font))
    }

    /// Present the operating system's share sheet anchored inside the current window.
    ///
    /// `anchor` is in the window's logical content coordinates. The item list is bounded by
    /// [`crate::MAX_SHARE_ITEMS`] and every text or URL item by
    /// [`crate::MAX_SHARE_ITEM_TEXT_BYTES`].
    pub fn share_items(&mut self, items: &[ShareItem], anchor: Rect) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().share_sheet {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        let window = Some(self.window.ok_or(PlatformError::Unavailable)?);
        self.push_platform_request(PlatformRequest::share_items(window, items, anchor)?)
    }

    /// Present the share sheet and observe whether the platform accepted the request.
    pub fn share_items_response(
        &mut self,
        items: &[ShareItem],
        anchor: Rect,
    ) -> Result<ShellResponse, PlatformError> {
        self.ensure_platform_capacity()?;
        let window = Some(self.window.ok_or(PlatformError::Unavailable)?);
        let (request, response) = PlatformRequest::share_items_response(window, items, anchor)?;
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Ask the operating system to authenticate the current user with biometrics.
    ///
    /// The future resolves to `true` when the user authenticated and `false` when the attempt was
    /// cancelled or rejected. Platforms without a biometric service, and machines whose hardware
    /// or policy makes the check unavailable, report [`PlatformError::Unsupported`]. `reason` is
    /// shown by the operating system and is bounded by [`crate::MAX_BIOMETRIC_REASON_BYTES`].
    pub fn authenticate_with_biometrics(
        &mut self,
        reason: impl Into<Arc<str>>,
    ) -> Result<PlatformResponse<bool>, PlatformError> {
        if !crate::DesktopIntegrationSupport::current().biometric_authentication {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        let (request, response) = PlatformRequest::authenticate_with_biometrics(reason)?;
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Post or replace an operating-system notification.
    ///
    /// The request is rejected before retention if any text or action exceeds its public bound.
    /// Use [`Self::request_notification_permission`] when the application wants to control the
    /// authorization prompt instead of relying on the backend's first-post behavior.
    pub fn show_system_notification(
        &mut self,
        notification: SystemNotification,
    ) -> Result<(), PlatformError> {
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::show_system_notification(notification)?;
        #[cfg(target_os = "windows")]
        crate::runtime::validate_windows_notification_app_info(self.app_info.as_ref())?;
        self.push_platform_request(request)
    }

    /// Remove a pending or delivered operating-system notification by its stable tag.
    pub fn dismiss_system_notification(
        &mut self,
        tag: impl Into<Arc<str>>,
    ) -> Result<(), PlatformError> {
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::dismiss_system_notification(tag)?;
        #[cfg(target_os = "windows")]
        crate::runtime::validate_windows_notification_app_info(self.app_info.as_ref())?;
        self.push_platform_request(request)
    }

    /// Query notification authorization without displaying a prompt.
    pub fn notification_permission_status(
        &mut self,
    ) -> Result<NotificationPermissionResponse, PlatformError> {
        self.ensure_platform_capacity()?;
        #[cfg(target_os = "windows")]
        crate::runtime::validate_windows_notification_app_info(self.app_info.as_ref())?;
        let (request, response) = PlatformRequest::notification_permission_status();
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Explicitly request notification authorization where the operating system requires it.
    pub fn request_notification_permission(
        &mut self,
    ) -> Result<NotificationPermissionResponse, PlatformError> {
        self.ensure_platform_capacity()?;
        #[cfg(target_os = "windows")]
        crate::runtime::validate_windows_notification_app_info(self.app_info.as_ref())?;
        let (request, response) = PlatformRequest::request_notification_permission();
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Ask the operating system to open a URL with its registered application.
    pub fn open_url(&mut self, url: impl Into<Arc<str>>) -> Result<(), PlatformError> {
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::open_url(url)?;
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            self.push_platform_request(request)
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        )))]
        {
            let _ = request;
            Err(PlatformError::Unsupported)
        }
    }

    /// Ask the operating system to open a filesystem path with its default application.
    pub fn open_path(&mut self, path: impl Into<PathBuf>) -> Result<(), PlatformError> {
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::open_path(path)?;
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            self.push_platform_request(request)
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        )))]
        {
            let _ = request;
            Err(PlatformError::Unsupported)
        }
    }

    /// Reveal a filesystem path in the operating system's file browser.
    pub fn reveal_path(&mut self, path: impl Into<PathBuf>) -> Result<(), PlatformError> {
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::reveal_path(path)?;
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            self.push_platform_request(request)
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        )))]
        {
            let _ = request;
            Err(PlatformError::Unsupported)
        }
    }

    /// Move a filesystem path to the operating system's trash or recycle bin.
    pub fn trash_path(&mut self, path: impl Into<PathBuf>) -> Result<(), PlatformError> {
        self.ensure_platform_capacity()?;
        let request = PlatformRequest::trash_path(path)?;
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            self.push_platform_request(request)
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        )))]
        {
            let _ = request;
            Err(PlatformError::Unsupported)
        }
    }

    /// Set the macOS Dock badge label. An empty value clears the badge.
    pub fn set_dock_badge(&mut self, value: impl Into<Arc<str>>) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().dock_badges {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        let value = value.into();
        let request = PlatformRequest::set_dock_badge((!value.is_empty()).then_some(value))?;
        self.push_platform_request(request)
    }

    pub fn clear_dock_badge(&mut self) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().dock_badges {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::set_dock_badge(None)?)
    }

    /// Replace the macOS Dock icon for this process.
    pub fn set_dock_icon(&mut self, icon: Image) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().dock_icons {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::set_dock_icon(Some(icon)))
    }

    pub fn clear_dock_icon(&mut self) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().dock_icons {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::set_dock_icon(None))
    }

    /// Create or replace a native tray / menu-bar extra icon.
    pub fn set_tray_icon(&mut self, options: TrayIconOptions) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().tray_icons {
            return Err(PlatformError::Unsupported);
        }
        validate_tray_options(&options)?;
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::set_tray_icon(options))
    }

    /// Remove one native tray icon. Removing an unknown id succeeds.
    pub fn remove_tray_icon(&mut self, id: u32) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().tray_icons {
            return Err(PlatformError::Unsupported);
        }
        if id == 0 {
            return Err(PlatformError::Platform(
                "a tray icon id must be nonzero".into(),
            ));
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::remove_tray_icon(id))
    }

    /// Open a tray icon's context menu at the current cursor position.
    ///
    /// Linux StatusNotifierItem hosts own menu presentation and do not expose this operation.
    pub fn show_tray_menu(&mut self, id: u32) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().programmable_tray_popup {
            return Err(PlatformError::Unsupported);
        }
        if id == 0 {
            return Err(PlatformError::Platform(
                "a tray icon id must be nonzero".into(),
            ));
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::show_tray_menu(id))
    }

    /// Replace the macOS Dock context menu.
    pub fn set_dock_menu(&mut self, menu: Menu) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().dock_menus {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::set_dock_menu(Some(menu))?)
    }

    pub fn clear_dock_menu(&mut self) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().dock_menus {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::set_dock_menu(None)?)
    }

    /// Add one path to the operating system's recent-document list.
    pub fn add_recent_document(&mut self, path: impl Into<PathBuf>) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().recent_documents {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::add_recent_document(path)?)
    }

    pub fn clear_recent_documents(&mut self) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().recent_documents {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::ClearRecentDocuments)
    }

    /// Present the operating system's standard About UI.
    pub fn show_about_panel(&mut self, options: AboutPanelOptions) -> Result<(), PlatformError> {
        if !crate::DesktopIntegrationSupport::current().native_about_panel {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        self.push_platform_request(PlatformRequest::show_about_panel(options)?)
    }

    /// Resolve the native icon for a filesystem item.
    pub fn file_icon(
        &mut self,
        path: impl Into<PathBuf>,
        size: FileIconSize,
    ) -> Result<FileIconResponse, PlatformError> {
        if !crate::DesktopIntegrationSupport::current().file_icons {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        let (request, response) = PlatformRequest::get_file_icon(path, size)?;
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Replace the complete Windows Jump List user-task section.
    pub fn set_user_tasks(
        &mut self,
        tasks: impl IntoIterator<Item = UserTask>,
    ) -> Result<ShellResponse, PlatformError> {
        if !crate::DesktopIntegrationSupport::current().user_tasks {
            return Err(PlatformError::Unsupported);
        }
        self.ensure_platform_capacity()?;
        let (request, response) = PlatformRequest::set_user_tasks(tasks.into_iter().collect())?;
        self.push_platform_request(request)?;
        Ok(response)
    }

    pub fn clear_user_tasks(&mut self) -> Result<ShellResponse, PlatformError> {
        self.set_user_tasks(std::iter::empty())
    }

    pub fn set_window_title(&mut self, title: impl Into<String>) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_title_handle(handle, title)
    }

    pub fn set_window_title_handle(
        &mut self,
        handle: WindowHandle,
        title: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
        let title = title.into();
        validate_window_title(&title)?;
        self.push_window_command(WindowCommand::SetTitle(handle, title))
    }

    /// Represent a file in the current window's native document chrome.
    pub fn set_represented_file(
        &mut self,
        path: impl Into<PathBuf>,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_represented_file_handle(handle, path)
    }

    /// Represent a file in a target window's native document chrome.
    pub fn set_represented_file_handle(
        &mut self,
        handle: WindowHandle,
        path: impl Into<PathBuf>,
    ) -> Result<(), WindowCommandError> {
        let path = path.into();
        validate_window_document_path(&path)?;
        self.push_window_command(WindowCommand::SetRepresentedFile(handle, Some(path)))
    }

    /// GPUI-compatible alias for [`Self::set_represented_file`].
    pub fn set_document_path(&mut self, path: impl AsRef<Path>) -> Result<(), WindowCommandError> {
        self.set_represented_file(path.as_ref().to_path_buf())
    }

    /// GPUI-compatible target-window alias for [`Self::set_represented_file_handle`].
    pub fn set_document_path_handle(
        &mut self,
        handle: WindowHandle,
        path: impl AsRef<Path>,
    ) -> Result<(), WindowCommandError> {
        self.set_represented_file_handle(handle, path.as_ref().to_path_buf())
    }

    pub fn clear_represented_file(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.clear_represented_file_handle(handle)
    }

    pub fn clear_represented_file_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetRepresentedFile(handle, None))
    }

    /// Set the current window's native unsaved-document indication.
    pub fn set_window_edited(&mut self, edited: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_edited_handle(handle, edited)
    }

    pub fn set_window_edited_handle(
        &mut self,
        handle: WindowHandle,
        edited: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetDocumentEdited(handle, edited))
    }

    /// Alias for [`Self::set_window_edited`].
    pub fn set_document_edited(&mut self, edited: bool) -> Result<(), WindowCommandError> {
        self.set_window_edited(edited)
    }

    /// Present AppKit's character palette for the current window.
    pub fn show_character_palette(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.show_character_palette_handle(handle)
    }

    pub fn show_character_palette_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::ShowCharacterPalette(handle))
    }

    /// Show the platform dictionary definition for the focused text input's selection or caret
    /// word in the current window.
    ///
    /// The request is applied when the runtime processes window commands; a window without a
    /// focused text input, an empty target, or a platform without a definition service ignores it.
    pub fn show_definition_for_selection(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.show_definition_for_selection_handle(handle)
    }

    pub fn show_definition_for_selection_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::LookUpSelection(handle))
    }

    /// Opt the current window into native system tabbing.
    pub fn set_tabbing_identifier(
        &mut self,
        identifier: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_tabbing_identifier_handle(handle, identifier)
    }

    pub fn set_tabbing_identifier_handle(
        &mut self,
        handle: WindowHandle,
        identifier: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
        let identifier = identifier.into();
        validate_window_tabbing_identifier(&identifier)?;
        self.push_window_command(WindowCommand::SetTabbingIdentifier(
            handle,
            Some(identifier),
        ))
    }

    pub fn clear_tabbing_identifier(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.clear_tabbing_identifier_handle(handle)
    }

    pub fn clear_tabbing_identifier_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetTabbingIdentifier(handle, None))
    }

    pub fn select_next_tab(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.select_next_tab_handle(handle)
    }

    pub fn select_next_tab_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SelectNextTab(handle))
    }

    pub fn select_previous_tab(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.select_previous_tab_handle(handle)
    }

    pub fn select_previous_tab_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SelectPreviousTab(handle))
    }

    pub fn select_tab(&mut self, index: usize) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.select_tab_handle(handle, index)
    }

    pub fn select_tab_handle(
        &mut self,
        handle: WindowHandle,
        index: usize,
    ) -> Result<(), WindowCommandError> {
        if index >= MAX_SYSTEM_WINDOW_TABS {
            return Err(WindowCommandError::InvalidTabIndex);
        }
        self.push_window_command(WindowCommand::SelectTab(handle, index))
    }

    pub fn merge_all_windows(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.merge_all_windows_handle(handle)
    }

    pub fn merge_all_windows_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::MergeAllWindows(handle))
    }

    pub fn move_tab_to_new_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.move_tab_to_new_window_handle(handle)
    }

    pub fn move_tab_to_new_window_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::MoveTabToNewWindow(handle))
    }

    pub fn toggle_tab_bar(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.toggle_tab_bar_handle(handle)
    }

    pub fn toggle_tab_bar_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::ToggleTabBar(handle))
    }

    pub fn toggle_tab_overview(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.toggle_tab_overview_handle(handle)
    }

    pub fn toggle_tab_overview_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::ToggleTabOverview(handle))
    }

    pub fn set_window_bounds(
        &mut self,
        bounds: crate::WindowBounds,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_bounds_handle(handle, bounds)
    }

    pub fn set_window_bounds_handle(
        &mut self,
        handle: WindowHandle,
        bounds: crate::WindowBounds,
    ) -> Result<(), WindowCommandError> {
        validate_window_bounds(bounds)?;
        self.push_window_command(WindowCommand::SetBounds(handle, bounds))
    }

    pub fn move_window(&mut self, position: Point) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.move_window_handle(handle, position)
    }

    pub fn move_window_handle(
        &mut self,
        handle: WindowHandle,
        position: Point,
    ) -> Result<(), WindowCommandError> {
        validate_window_position(position)?;
        self.push_window_command(WindowCommand::Move(handle, position))
    }

    pub fn resize_window(&mut self, size: Size) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.resize_window_handle(handle, size)
    }

    pub fn resize_window_handle(
        &mut self,
        handle: WindowHandle,
        size: Size,
    ) -> Result<(), WindowCommandError> {
        validate_window_size(size)?;
        self.push_window_command(WindowCommand::Resize(handle, size))
    }

    pub fn minimize_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.minimize_window_handle(handle)
    }

    pub fn minimize_window_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::Minimize(handle))
    }

    pub fn restore_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.restore_window_handle(handle)
    }

    pub fn restore_window_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::Restore(handle))
    }

    pub fn zoom_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.zoom_window_handle(handle)
    }

    pub fn zoom_window_handle(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::Zoom(handle))
    }

    pub fn toggle_fullscreen(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.toggle_fullscreen_handle(handle)
    }

    pub fn toggle_fullscreen_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::ToggleFullscreen(handle))
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_fullscreen_handle(handle, fullscreen)
    }

    pub fn set_fullscreen_handle(
        &mut self,
        handle: WindowHandle,
        fullscreen: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetFullscreen(handle, fullscreen))
    }

    pub fn show_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.show_window_handle(handle)
    }

    pub fn show_window_handle(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetVisible(handle, true))
    }

    pub fn hide_window(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.hide_window_handle(handle)
    }

    pub fn hide_window_handle(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetVisible(handle, false))
    }

    pub fn set_window_movable(&mut self, movable: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_movable_handle(handle, movable)
    }

    pub fn set_window_movable_handle(
        &mut self,
        handle: WindowHandle,
        movable: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetMovable(handle, movable))
    }

    pub fn set_window_resizable(&mut self, resizable: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_resizable_handle(handle, resizable)
    }

    pub fn set_window_resizable_handle(
        &mut self,
        handle: WindowHandle,
        resizable: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetResizable(handle, resizable))
    }

    /// Set the current window's minimum logical inner size.
    pub fn set_window_minimum_size(&mut self, size: Size) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_minimum_size_handle(handle, size)
    }

    /// Set a target window's minimum logical inner size.
    pub fn set_window_minimum_size_handle(
        &mut self,
        handle: WindowHandle,
        size: Size,
    ) -> Result<(), WindowCommandError> {
        validate_window_size(size)?;
        self.push_window_command(WindowCommand::SetMinimumSize(handle, Some(size)))
    }

    /// Remove the current window's minimum-size constraint.
    pub fn clear_window_minimum_size(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.clear_window_minimum_size_handle(handle)
    }

    /// Remove a target window's minimum-size constraint.
    pub fn clear_window_minimum_size_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetMinimumSize(handle, None))
    }

    /// Set the current window's maximum logical inner size.
    pub fn set_window_maximum_size(&mut self, size: Size) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_maximum_size_handle(handle, size)
    }

    pub fn set_window_maximum_size_handle(
        &mut self,
        handle: WindowHandle,
        size: Size,
    ) -> Result<(), WindowCommandError> {
        validate_window_size(size)?;
        self.push_window_command(WindowCommand::SetMaximumSize(handle, Some(size)))
    }

    pub fn clear_window_maximum_size(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.clear_window_maximum_size_handle(handle)
    }

    pub fn clear_window_maximum_size_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetMaximumSize(handle, None))
    }

    pub fn set_window_minimizable(&mut self, minimizable: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_minimizable_handle(handle, minimizable)
    }

    pub fn set_window_minimizable_handle(
        &mut self,
        handle: WindowHandle,
        minimizable: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetMinimizable(handle, minimizable))
    }

    pub fn set_window_maximizable(&mut self, maximizable: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_maximizable_handle(handle, maximizable)
    }

    pub fn set_window_maximizable_handle(
        &mut self,
        handle: WindowHandle,
        maximizable: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetMaximizable(handle, maximizable))
    }

    pub fn set_window_closable(&mut self, closable: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_closable_handle(handle, closable)
    }

    pub fn set_window_closable_handle(
        &mut self,
        handle: WindowHandle,
        closable: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetClosable(handle, closable))
    }

    pub fn set_window_decorated(&mut self, decorated: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_decorated_handle(handle, decorated)
    }

    pub fn set_window_decorated_handle(
        &mut self,
        handle: WindowHandle,
        decorated: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetDecorated(handle, decorated))
    }

    pub fn set_window_shadow(&mut self, shadow: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_shadow_handle(handle, shadow)
    }

    pub fn set_window_shadow_handle(
        &mut self,
        handle: WindowHandle,
        shadow: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetShadow(handle, shadow))
    }

    pub fn set_window_content_protected(
        &mut self,
        protected: bool,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_content_protected_handle(handle, protected)
    }

    pub fn set_window_content_protected_handle(
        &mut self,
        handle: WindowHandle,
        protected: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetContentProtected(handle, protected))
    }

    pub fn set_window_level(&mut self, level: WindowLevel) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_level_handle(handle, level)
    }

    pub fn set_window_level_handle(
        &mut self,
        handle: WindowHandle,
        level: WindowLevel,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetWindowLevel(handle, Some(level)))
    }

    pub fn use_automatic_window_level(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.use_automatic_window_level_handle(handle)
    }

    pub fn use_automatic_window_level_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetWindowLevel(handle, None))
    }

    /// Change how the application appears in the Dock and application switcher.
    ///
    /// macOS applies `NSApplicationActivationPolicy`. Other platforms complete the response with
    /// [`PlatformError::Unsupported`].
    pub fn set_activation_policy(
        &mut self,
        policy: ActivationPolicy,
    ) -> Result<PlatformResponse<()>, PlatformError> {
        let (request, response) = PlatformRequest::set_activation_policy(policy);
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Bring the application forward.
    ///
    /// `force` uses AppKit's ignore-other-apps activation, which steals focus from the frontmost
    /// application. Prefer `false` unless the user just asked for the application explicitly.
    pub fn activate_application(&mut self, force: bool) -> Result<(), PlatformError> {
        self.push_platform_request(PlatformRequest::ActivateApplication { force })
    }

    /// Hide every window of this application.
    pub fn hide_application(&mut self) -> Result<(), PlatformError> {
        self.push_platform_request(PlatformRequest::HideApplication)
    }

    /// Reveal an application hidden by [`Self::hide_application`].
    pub fn unhide_application(&mut self) -> Result<(), PlatformError> {
        self.push_platform_request(PlatformRequest::UnhideApplication)
    }

    /// Ask for the user's attention, bouncing the macOS Dock tile.
    ///
    /// [`DockAttention::Critical`] keeps bouncing until the application is activated or the
    /// returned request is cancelled; [`DockAttention::Informational`] bounces once.
    pub fn request_dock_attention(
        &mut self,
        attention: DockAttention,
    ) -> Result<PlatformResponse<DockAttentionRequest>, PlatformError> {
        let (request, response) = PlatformRequest::request_dock_attention(attention);
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Stop an in-flight [`DockAttention::Critical`] request.
    pub fn cancel_dock_attention(
        &mut self,
        request: DockAttentionRequest,
    ) -> Result<(), PlatformError> {
        self.push_platform_request(PlatformRequest::CancelDockAttention(request))
    }

    /// Show or hide the Dock tile, keeping the application's windows usable either way.
    pub fn set_dock_visible(
        &mut self,
        visible: bool,
    ) -> Result<PlatformResponse<()>, PlatformError> {
        let (request, response) = PlatformRequest::set_dock_visible(visible);
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Route keystrokes straight to this process, bypassing input monitoring.
    ///
    /// macOS uses `EnableSecureEventInput`/`DisableSecureEventInput`. QuickGUI reads the current
    /// state first, so repeated calls cannot unbalance the system-wide counter.
    pub fn set_secure_keyboard_entry(&mut self, enabled: bool) -> Result<(), PlatformError> {
        self.push_platform_request(PlatformRequest::SetSecureKeyboardEntry(enabled))
    }

    /// Play the operating system's alert sound.
    pub fn beep(&mut self) -> Result<(), PlatformError> {
        self.push_platform_request(PlatformRequest::Beep)
    }

    /// Whether this process can relocate its bundle into an `/Applications` directory.
    pub fn applications_folder_support(&self) -> ApplicationsFolderSupport {
        #[cfg(target_os = "macos")]
        {
            crate::macos_shell::applications_folder_support()
        }
        #[cfg(not(target_os = "macos"))]
        {
            ApplicationsFolderSupport::default()
        }
    }

    /// Move the running application bundle into `/Applications`.
    ///
    /// Resolves to `false` when the bundle is already installed there. QuickGUI never restarts the
    /// process on its own; call [`Self::relaunch`] after a successful move.
    pub fn move_to_applications_folder(&mut self) -> Result<PlatformResponse<bool>, PlatformError> {
        let (request, response) = PlatformRequest::move_to_applications_folder();
        self.push_platform_request(request)?;
        Ok(response)
    }

    /// Whether this process is running from an installed application bundle.
    ///
    /// See [`crate::is_application_packaged`] for the exact per-platform heuristic.
    pub fn is_application_packaged(&self) -> bool {
        crate::is_application_packaged()
    }

    /// Exit the application with an explicit process exit code.
    ///
    /// Teardown is identical to [`Self::exit`]: windows close child-first and the quit callbacks
    /// still run. A non-zero code is applied once the event loop has fully unwound.
    pub fn exit_with_code(&mut self, code: i32) {
        self.exit_code = Some(code);
        self.exit();
    }

    /// Raise the current window to the front of its stacking level without activating the app.
    pub fn move_window_top(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.move_window_top_handle(handle)
    }

    pub fn move_window_top_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::MoveToTop(handle))
    }

    /// Order the current window immediately above another retained window.
    pub fn move_window_above(&mut self, other: WindowHandle) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.move_window_above_handle(handle, other)
    }

    pub fn move_window_above_handle(
        &mut self,
        handle: WindowHandle,
        other: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        if handle == other {
            return Err(WindowCommandError::InvalidWindowOrder);
        }
        self.push_window_command(WindowCommand::MoveAbove(handle, other))
    }

    /// Let clicks pass through the current window to whatever is behind it.
    ///
    /// `forward` keeps pointer motion and hover events flowing to this window; it is ignored when
    /// `ignore` is `false`. [`Self::set_cursor_hit_test`] remains the simple all-or-nothing form.
    pub fn set_ignore_mouse_events(
        &mut self,
        ignore: bool,
        forward: bool,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_ignore_mouse_events_handle(handle, ignore, forward)
    }

    pub fn set_ignore_mouse_events_handle(
        &mut self,
        handle: WindowHandle,
        ignore: bool,
        forward: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetIgnoreMouseEvents(
            handle,
            ignore,
            ignore && forward,
        ))
    }

    /// Block or restore every native input event for the current window.
    ///
    /// A disabled window stays visible and keeps rendering; it simply stops receiving pointer and
    /// keyboard input, which is the native way to express an application-modal owner.
    pub fn set_window_enabled(&mut self, enabled: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_enabled_handle(handle, enabled)
    }

    pub fn set_window_enabled_handle(
        &mut self,
        handle: WindowHandle,
        enabled: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetWindowEnabled(handle, enabled))
    }

    /// Constrain the current window's live resizing to one `width:height` content ratio.
    pub fn set_aspect_ratio(&mut self, ratio: Option<Size>) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_aspect_ratio_handle(handle, ratio)
    }

    pub fn set_aspect_ratio_handle(
        &mut self,
        handle: WindowHandle,
        ratio: Option<Size>,
    ) -> Result<(), WindowCommandError> {
        validate_window_aspect_ratio(ratio)?;
        self.push_window_command(WindowCommand::SetAspectRatio(handle, ratio))
    }

    pub fn clear_aspect_ratio(&mut self) -> Result<(), WindowCommandError> {
        self.set_aspect_ratio(None)
    }

    pub fn clear_aspect_ratio_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.set_aspect_ratio_handle(handle, None)
    }

    /// Show or hide the macOS close/minimize/zoom buttons on the current window.
    pub fn set_window_button_visibility(
        &mut self,
        visible: bool,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_button_visibility_handle(handle, visible)
    }

    pub fn set_window_button_visibility_handle(
        &mut self,
        handle: WindowHandle,
        visible: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetWindowButtonVisibility(handle, visible))
    }

    /// Narrow the inner size proposed by [`Event::WillResize`](crate::Event::WillResize).
    ///
    /// Only meaningful while handling that event. The runtime issues at most one corrective
    /// native resize per event and never loops.
    pub fn constrain_resize(&mut self, size: Size) -> Result<(), WindowCommandError> {
        validate_window_size(size)?;
        self.constrained_size = Some(size);
        Ok(())
    }

    /// Replace the position proposed by [`Event::WillMove`](crate::Event::WillMove).
    pub fn constrain_move(&mut self, position: Point) -> Result<(), WindowCommandError> {
        validate_window_position(position)?;
        self.constrained_position = Some(position);
        Ok(())
    }

    pub fn set_window_always_on_top(
        &mut self,
        always_on_top: bool,
    ) -> Result<(), WindowCommandError> {
        self.set_window_level(if always_on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        })
    }

    pub fn set_window_always_on_top_handle(
        &mut self,
        handle: WindowHandle,
        always_on_top: bool,
    ) -> Result<(), WindowCommandError> {
        self.set_window_level_handle(
            handle,
            if always_on_top {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            },
        )
    }

    pub fn set_window_focusable(&mut self, focusable: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_focusable_handle(handle, focusable)
    }

    pub fn set_window_focusable_handle(
        &mut self,
        handle: WindowHandle,
        focusable: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetFocusable(handle, focusable))
    }

    pub fn set_window_skip_taskbar(&mut self, skip: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_skip_taskbar_handle(handle, skip)
    }

    pub fn set_window_skip_taskbar_handle(
        &mut self,
        handle: WindowHandle,
        skip: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetSkipTaskbar(handle, skip))
    }

    pub fn set_window_visible_on_all_workspaces(
        &mut self,
        visible: bool,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_visible_on_all_workspaces_handle(handle, visible)
    }

    pub fn set_window_visible_on_all_workspaces_handle(
        &mut self,
        handle: WindowHandle,
        visible: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetVisibleOnAllWorkspaces(handle, visible))
    }

    pub fn set_window_opacity(&mut self, opacity: f32) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_opacity_handle(handle, opacity)
    }

    pub fn set_window_opacity_handle(
        &mut self,
        handle: WindowHandle,
        opacity: f32,
    ) -> Result<(), WindowCommandError> {
        validate_window_opacity(opacity)?;
        self.push_window_command(WindowCommand::SetOpacity(handle, opacity))
    }

    pub fn set_window_icon(&mut self, icon: Image) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_icon_handle(handle, icon)
    }

    pub fn set_window_icon_handle(
        &mut self,
        handle: WindowHandle,
        icon: Image,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetIcon(handle, Some(icon)))
    }

    pub fn clear_window_icon(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.clear_window_icon_handle(handle)
    }

    pub fn clear_window_icon_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetIcon(handle, None))
    }

    /// Set the current window's native taskbar progress indicator.
    pub fn set_taskbar_progress(
        &mut self,
        state: TaskbarProgressState,
        progress: f32,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_taskbar_progress_handle(handle, state, progress)
    }

    pub fn set_taskbar_progress_handle(
        &mut self,
        handle: WindowHandle,
        state: TaskbarProgressState,
        progress: f32,
    ) -> Result<(), WindowCommandError> {
        validate_taskbar_progress(progress)?;
        self.push_window_command(WindowCommand::SetTaskbarProgress(handle, state, progress))
    }

    /// Install a Windows taskbar overlay icon for the current window.
    pub fn set_taskbar_overlay_icon(
        &mut self,
        icon: Image,
        description: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_taskbar_overlay_icon_handle(handle, icon, description)
    }

    pub fn set_taskbar_overlay_icon_handle(
        &mut self,
        handle: WindowHandle,
        icon: Image,
        description: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
        let description = description.into();
        validate_taskbar_overlay_description(Some(&description))?;
        self.push_window_command(WindowCommand::SetTaskbarOverlayIcon(
            handle,
            Some(icon),
            Some(description),
        ))
    }

    pub fn clear_taskbar_overlay_icon(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.clear_taskbar_overlay_icon_handle(handle)
    }

    pub fn clear_taskbar_overlay_icon_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetTaskbarOverlayIcon(handle, None, None))
    }

    pub fn set_cursor_visible(&mut self, visible: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_cursor_visible_handle(handle, visible)
    }

    pub fn set_cursor_visible_handle(
        &mut self,
        handle: WindowHandle,
        visible: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetCursorVisible(handle, visible))
    }

    pub fn set_cursor_grab(&mut self, mode: CursorGrabMode) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_cursor_grab_handle(handle, mode)
    }

    pub fn set_cursor_grab_handle(
        &mut self,
        handle: WindowHandle,
        mode: CursorGrabMode,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetCursorGrab(handle, mode))
    }

    pub fn set_cursor_hit_test(&mut self, hit_test: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_cursor_hit_test_handle(handle, hit_test)
    }

    pub fn set_cursor_hit_test_handle(
        &mut self,
        handle: WindowHandle,
        hit_test: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetCursorHitTest(handle, hit_test))
    }

    pub fn set_cursor_position(&mut self, position: Point) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_cursor_position_handle(handle, position)
    }

    pub fn set_cursor_position_handle(
        &mut self,
        handle: WindowHandle,
        position: Point,
    ) -> Result<(), WindowCommandError> {
        validate_window_position(position)?;
        self.push_window_command(WindowCommand::SetCursorPosition(handle, position))
    }

    /// Force the current window's native chrome to one light/dark appearance.
    pub fn set_window_appearance(
        &mut self,
        appearance: WindowAppearance,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_appearance_handle(handle, appearance)
    }

    /// Force a target window's native chrome to one light/dark appearance.
    pub fn set_window_appearance_handle(
        &mut self,
        handle: WindowHandle,
        appearance: WindowAppearance,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetAppearance(handle, Some(appearance)))
    }

    /// Return the current window to the operating system's effective appearance.
    pub fn follow_system_window_appearance(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.follow_system_window_appearance_handle(handle)
    }

    /// Return a target window to the operating system's effective appearance.
    pub fn follow_system_window_appearance_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetAppearance(handle, None))
    }

    /// Change how the native compositor treats transparent pixels in the current window.
    pub fn set_window_background_appearance(
        &mut self,
        appearance: WindowBackgroundAppearance,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_window_background_appearance_handle(handle, appearance)
    }

    /// Change how the native compositor treats transparent pixels in a target window.
    pub fn set_window_background_appearance_handle(
        &mut self,
        handle: WindowHandle,
        appearance: WindowBackgroundAppearance,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetBackgroundAppearance(handle, appearance))
    }

    /// Set or remove the current window's macOS semantic vibrancy material.
    pub fn set_macos_window_vibrancy(
        &mut self,
        vibrancy: Option<MacOsVibrancy>,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_macos_window_vibrancy_handle(handle, vibrancy)
    }

    /// Set or remove a target window's macOS semantic vibrancy material.
    pub fn set_macos_window_vibrancy_handle(
        &mut self,
        handle: WindowHandle,
        vibrancy: Option<MacOsVibrancy>,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetMacOsVibrancy(handle, vibrancy))
    }

    /// Change how the current macOS vibrancy material follows window activity.
    pub fn set_macos_visual_effect_state(
        &mut self,
        state: MacOsVisualEffectState,
    ) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_macos_visual_effect_state_handle(handle, state)
    }

    /// Change how a target window's macOS vibrancy material follows window activity.
    pub fn set_macos_visual_effect_state_handle(
        &mut self,
        handle: WindowHandle,
        state: MacOsVisualEffectState,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetMacOsVisualEffectState(handle, state))
    }

    /// Open or close the retained-tree inspector for the current window.
    #[cfg(feature = "inspector")]
    pub fn set_inspector(&mut self, open: bool) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.set_inspector_handle(handle, open)
    }

    /// Open or close the retained-tree inspector for a target window.
    #[cfg(feature = "inspector")]
    pub fn set_inspector_handle(
        &mut self,
        handle: WindowHandle,
        open: bool,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::SetInspector(handle, open))
    }

    /// Toggle the retained-tree inspector for the current window.
    #[cfg(feature = "inspector")]
    pub fn toggle_inspector(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.toggle_inspector_handle(handle)
    }

    /// Toggle the retained-tree inspector for a target window.
    #[cfg(feature = "inspector")]
    pub fn toggle_inspector_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::ToggleInspector(handle))
    }

    pub fn request_window_attention(&mut self) -> Result<(), WindowCommandError> {
        let handle = self.current_window_handle()?;
        self.request_window_attention_handle(handle)
    }

    pub fn request_window_attention_handle(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.push_window_command(WindowCommand::RequestAttention(handle))
    }

    /// Close the window currently delivering this event.
    pub fn close_window(&mut self) {
        self.close_current_window = true;
    }

    /// Close the complete system-popover chain containing the current window.
    ///
    /// Closing the first popover lets the runtime tear down every descendant child-first and gives
    /// native keyboard focus back to the nearest non-popover owner. Outside a system popover this
    /// is a no-op and returns `false`.
    pub fn close_popover_chain(&mut self) -> bool {
        let Some(root) = self.popover_root_window else {
            return false;
        };
        if let Some(owner) = self.popover_owner_window
            && !self.focus_windows.contains(&owner)
        {
            self.focus_windows.push(owner);
        }
        if Some(root) == self.window {
            self.close_current_window = true;
        } else {
            self.close_windows.push(root);
        }
        true
    }

    /// Close a window previously returned by [`Self::open_window`].
    pub fn close_window_handle(&mut self, handle: WindowHandle) {
        self.close_windows.push(handle);
    }

    /// Bring a window to the front and give it native keyboard focus.
    pub fn focus_window(&mut self, handle: WindowHandle) {
        self.focus_windows.push(handle);
    }

    /// Mark another window's view dirty and schedule one coalesced redraw.
    pub fn invalidate_window(&mut self, handle: WindowHandle) {
        self.invalidate_windows.push(handle);
    }

    /// Keep the current window open after receiving [`Event::CloseRequested`].
    pub fn prevent_close(&mut self) {
        self.prevent_close = true;
    }

    /// Cancel the active application before-quit or will-quit phase.
    ///
    /// Calling this outside those application callbacks has no effect.
    pub fn prevent_quit(&mut self) {
        self.prevent_quit = true;
    }

    /// Move keyboard focus to a stable element handle.
    ///
    /// If the target is introduced by the view invalidation from this same event, QuickGUI keeps
    /// the request through exactly that next rebuild. A missing target is then discarded rather
    /// than becoming a persistent focus trap.
    pub fn focus(&mut self, handle: FocusHandle) {
        self.focus = Some(Some(handle.id()));
    }

    /// Clear keyboard focus within the window.
    pub fn blur(&mut self) {
        self.focus = Some(None);
    }

    /// Clear the retained selection painted across immutable selectable text.
    ///
    /// Custom text surfaces such as terminals and editors should call this after accepting input
    /// so an earlier pointer selection does not remain highlighted while the content changes.
    pub fn clear_text_selection(&mut self) {
        self.clear_text_selection = true;
    }

    /// Validate and submit a mounted form after the current callback completes.
    ///
    /// Repeated requests for the same form in one callback coalesce. The bounded queue prevents a
    /// callback from retaining unbounded work; `false` reports that the limit was reached.
    pub fn submit_form(&mut self, form: impl Into<ElementId>) -> bool {
        let form = form.into();
        if self.form_submissions.contains(&form) {
            return true;
        }
        if self.form_submissions.len() == MAX_FORM_SUBMISSIONS_PER_EVENT {
            return false;
        }
        self.form_submissions.push(form);
        true
    }

    /// Dispatch a typed action through the currently focused element path.
    ///
    /// This is useful for buttons, menus, command palettes, and native menu items that should use
    /// exactly the same command handlers as keyboard bindings.
    pub fn dispatch_action<A: Action>(&mut self, action: A) {
        self.actions.push(AnyAction::new(action));
    }

    /// Dispatch a previously type-erased action through the focused element path.
    ///
    /// Command registries and pickers can retain heterogeneous actions as [`AnyAction`] values,
    /// then restore the intended focus and dispatch the original concrete payload without a type
    /// switch or a parallel command system.
    pub fn dispatch_any_action(&mut self, action: AnyAction) {
        self.actions.push(action);
    }

    /// Dispatch a typed action through another window's focused retained path.
    ///
    /// Delivery is deferred until the current callback releases its view borrow. `false` means
    /// this callback reached the hard cross-window action bound; a target that closes before
    /// delivery is ignored safely.
    pub fn dispatch_action_to_window<A: Action>(
        &mut self,
        window: WindowHandle,
        action: A,
    ) -> bool {
        self.dispatch_any_action_to_window(window, AnyAction::new(action))
    }

    /// Dispatch a previously type-erased action through another window's focused retained path.
    pub fn dispatch_any_action_to_window(
        &mut self,
        window: WindowHandle,
        action: AnyAction,
    ) -> bool {
        if self.targeted_actions.len() == MAX_TARGETED_ACTIONS_PER_EVENT {
            return false;
        }
        self.targeted_actions.push((window, action));
        true
    }

    /// Dispatch a typed action to this native child window's parent.
    ///
    /// Returns `false` when the context is not attached to a child or the callback reached the
    /// cross-window action bound.
    pub fn dispatch_action_to_parent<A: Action>(&mut self, action: A) -> bool {
        let Some(parent) = self.parent_window else {
            return false;
        };
        self.dispatch_action_to_window(parent, action)
    }

    /// Dispatch a previously type-erased action to this native child window's parent.
    pub fn dispatch_any_action_to_parent(&mut self, action: AnyAction) -> bool {
        let Some(parent) = self.parent_window else {
            return false;
        };
        self.dispatch_any_action_to_window(parent, action)
    }

    /// The parent of the current native child window, when one exists.
    pub const fn parent_window_handle(&self) -> Option<WindowHandle> {
        self.parent_window
    }

    /// Dispatch a typed action to the nearest non-popover owner of this system popover chain.
    ///
    /// This differs from [`Self::dispatch_action_to_parent`] for nested menus: a submenu's direct
    /// parent is another popover window, while commands should reach the application window that
    /// opened the popover chain.
    pub fn dispatch_action_to_popover_owner<A: Action>(&mut self, action: A) -> bool {
        let Some(owner) = self.popover_owner_window else {
            return false;
        };
        self.dispatch_action_to_window(owner, action)
    }

    /// Dispatch a previously type-erased action to the nearest non-popover owner.
    pub fn dispatch_any_action_to_popover_owner(&mut self, action: AnyAction) -> bool {
        let Some(owner) = self.popover_owner_window else {
            return false;
        };
        self.dispatch_any_action_to_window(owner, action)
    }

    /// The nearest non-popover owner of the current system popover chain, when one exists.
    pub const fn popover_owner_window_handle(&self) -> Option<WindowHandle> {
        self.popover_owner_window
    }

    /// The first system popover below the non-popover owner of the current popover chain.
    pub const fn popover_root_window_handle(&self) -> Option<WindowHandle> {
        self.popover_root_window
    }

    /// Replace the application's native menu declaration.
    ///
    /// Use this after state changes that affect labels, checked state, or static availability.
    /// Focused action-handler availability and contextual key equivalents update automatically.
    pub fn set_menus(&mut self, menus: impl IntoIterator<Item = Menu>) {
        self.menus = Some(menus.into_iter().collect());
    }

    /// Remove every application-wide native menu.
    pub fn clear_menus(&mut self) {
        self.menus = Some(Vec::new());
    }

    /// Replace the current window's native menu declaration.
    ///
    /// On macOS this becomes the process menu bar while the window is active. On Windows it is
    /// attached only to this window. Other desktop targets retain the declaration but may report
    /// native menu presentation as unsupported.
    pub fn set_window_menus(&mut self, menus: impl IntoIterator<Item = Menu>) {
        self.window_menus = Some(Some(menus.into_iter().collect()));
    }

    /// Keep a window-specific empty native menu instead of inheriting the application menu.
    pub fn clear_window_menus(&mut self) {
        self.window_menus = Some(Some(Vec::new()));
    }

    /// Remove the current window's override and inherit the application's native menus again.
    pub fn use_application_menus(&mut self) {
        self.window_menus = Some(None);
    }

    /// Open a platform-native popup menu owned by the current window.
    ///
    /// `position` is in window-local logical pixels from the top-left. `None` uses the current
    /// native cursor position. Menu actions follow the same focused typed-action and OS-role path
    /// as application menu items.
    pub fn show_native_popup_menu(
        &mut self,
        menu: Menu,
        position: Option<Point>,
    ) -> Result<(), PlatformError> {
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (menu, position);
            return Err(PlatformError::Unsupported);
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            if self.window.is_none() {
                return Err(PlatformError::Unavailable);
            }
            if self.native_popup_menus.len() == MAX_NATIVE_POPUP_MENUS_PER_EVENT {
                return Err(PlatformError::QueueFull);
            }
            if position.is_some_and(|position| validate_window_position(position).is_err()) {
                return Err(PlatformError::InvalidMenuPosition);
            }
            validate_menus(std::slice::from_ref(&menu)).map_err(|_| PlatformError::InvalidMenu)?;
            self.native_popup_menus
                .push(NativePopupMenuRequest { menu, position });
            Ok(())
        }
    }

    /// Allow the current action to continue bubbling to the next ancestor handler.
    ///
    /// Action handlers consume by default, matching GPUI's command dispatch behavior. For input
    /// and capture-phase action events that propagate by default, this also cancels an earlier
    /// [`Self::stop_propagation`] call made during the same callback.
    pub fn propagate(&mut self) {
        self.propagate_action = true;
        self.stop_event_propagation = false;
    }

    /// Stop the current input event before it reaches another listening ancestor.
    ///
    /// Input events bubble by default. Capture-phase action listeners also use this method.
    /// Stopping propagation does not suppress native default behavior; use
    /// [`Self::prevent_default`] separately when replacing retained mouse, focus, selection,
    /// drag, click, scroll, or key behavior.
    pub fn stop_propagation(&mut self) {
        self.stop_event_propagation = true;
    }

    /// Suppress the framework's default behavior for the current input event.
    ///
    /// For a [`ScrollWheelEvent`] this prevents retained scrolling. For a targeted desktop mouse
    /// press or release it suppresses the framework's focus, text-selection, click, context-menu,
    /// dismissal, and drag-start defaults. For a [`KeyDownEvent`] it suppresses text editing,
    /// focus traversal, focused activation, dismissal, and the default macOS close shortcut.
    /// Terminal pointer capture and drag cleanup still run. This does not stop propagation to
    /// another listener.
    pub fn prevent_default(&mut self) {
        self.prevent_default = true;
    }
}
