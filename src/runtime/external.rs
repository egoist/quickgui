use super::*;

use crate::runtime::effects::queue_deferred_menu_request;

#[cfg(not(target_arch = "wasm32"))]
impl AppRunner {
    /// Request an orderly application exit after native windows and owned resources close.
    pub fn exit(&mut self) -> bool {
        if !matches!(self.status, AppRunStatus::Continue) || self.runtime.exit_requested {
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
        self.runtime.exit_requested = true;
        true
    }

    /// Request a preventable application quit.
    ///
    /// Unlike [`Self::exit`], this runs the `on_before_quit` and `on_will_quit` phases first, so a
    /// declared interception can keep the application alive. Returns `false` when shutdown has
    /// already begun.
    pub fn request_quit(&mut self) -> bool {
        if !matches!(self.status, AppRunStatus::Continue)
            || self.runtime.exit_requested
            || self.runtime.pending_quit.is_some()
        {
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
        self.runtime.pending_quit = Some(QuitReason::Explicit);
        true
    }

    /// Request an orderly relaunch preserving the current process arguments and directory.
    pub fn relaunch(&mut self) -> std::result::Result<bool, crate::SystemIntegrationError> {
        self.relaunch_with(RelaunchOptions::default())
    }

    /// Request an orderly relaunch with explicit process overrides.
    ///
    /// The replacement is spawned only after a later [`Self::pump`] observes event-loop exit and
    /// releases the single-instance guard, native integrations, windows, and foreground work.
    pub fn relaunch_with(
        &mut self,
        options: RelaunchOptions,
    ) -> std::result::Result<bool, crate::SystemIntegrationError> {
        let request = options.prepare()?;
        if !matches!(self.status, AppRunStatus::Continue) || self.runtime.exit_requested {
            return Ok(false);
        }
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| {
                crate::SystemIntegrationError::Platform(Arc::from(
                    "the application event loop is closed",
                ))
            })?;
        self.runtime.relaunch_request = Some(request);
        self.runtime.exit_requested = true;
        Ok(true)
    }

    /// Return the latest bounded display snapshot retained by the application runtime.
    ///
    /// This does not poll the operating system. The snapshot is replaced at native display-change
    /// boundaries and is therefore safe for embedding runtimes to read after each pump.
    pub fn displays(&self) -> Displays {
        self.runtime.displays.clone()
    }

    /// Return the latest immutable keyboard-layout snapshot retained by the runtime.
    pub fn keyboard_layout(&self) -> KeyboardLayout {
        self.runtime.keyboard.layout().clone()
    }

    /// Return a constant-size snapshot of one mounted native window.
    pub fn window_state(&self, handle: WindowHandle) -> Option<WindowState> {
        self.runtime.window_state_for(handle)
    }

    /// Return the latest CPU-side frame telemetry retained for one mounted native window.
    ///
    /// This read does not request a frame. An idle window keeps its last value until it renders
    /// again, and `frame_number` is zero before its first completed frame.
    pub fn window_frame_metrics(&self, handle: WindowHandle) -> Option<crate::FrameMetrics> {
        self.runtime.frame_metrics_for(handle)
    }

    /// Read one bounded item from the operating system's general clipboard.
    pub fn read_from_clipboard(&self) -> Result<Option<ClipboardItem>, crate::ClipboardError> {
        self.runtime.clipboard.read(ClipboardTarget::General)
    }

    /// Atomically replace the operating system's general clipboard with one bounded item.
    pub fn write_to_clipboard(&self, item: ClipboardItem) -> Result<(), crate::ClipboardError> {
        self.runtime.clipboard.write(ClipboardTarget::General, item)
    }

    /// Read Linux's primary-selection clipboard.
    #[cfg(target_os = "linux")]
    pub fn read_from_selection_clipboard(
        &self,
    ) -> Result<Option<ClipboardItem>, crate::ClipboardError> {
        self.runtime.clipboard.read(ClipboardTarget::Selection)
    }

    /// Replace Linux's primary-selection clipboard.
    #[cfg(target_os = "linux")]
    pub fn write_to_selection_clipboard(
        &self,
        item: ClipboardItem,
    ) -> Result<(), crate::ClipboardError> {
        self.runtime
            .clipboard
            .write(ClipboardTarget::Selection, item)
    }

    /// Ask the operating system to open a URL with its registered handler.
    pub fn open_external(
        &mut self,
        url: impl Into<Arc<str>>,
    ) -> Result<ShellResponse, PlatformError> {
        let (request, response) = PlatformRequest::open_url_response(url)?;
        self.queue_platform_response(request)?;
        Ok(response)
    }

    /// Ask the operating system to open a filesystem item with its registered application.
    pub fn open_path(&mut self, path: impl Into<PathBuf>) -> Result<ShellResponse, PlatformError> {
        let (request, response) = PlatformRequest::open_path_response(path)?;
        self.queue_platform_response(request)?;
        Ok(response)
    }

    /// Reveal a filesystem item in the platform file manager.
    pub fn reveal_path(
        &mut self,
        path: impl Into<PathBuf>,
    ) -> Result<ShellResponse, PlatformError> {
        let (request, response) = PlatformRequest::reveal_path_response(path)?;
        self.queue_platform_response(request)?;
        Ok(response)
    }

    /// Move a filesystem item to the operating system trash or recycle bin.
    pub fn trash_path(&mut self, path: impl Into<PathBuf>) -> Result<ShellResponse, PlatformError> {
        let (request, response) = PlatformRequest::trash_path_response(path)?;
        self.queue_platform_response(request)?;
        Ok(response)
    }

    /// Post or replace one operating-system notification.
    pub fn show_system_notification(
        &mut self,
        notification: SystemNotification,
    ) -> Result<(), PlatformError> {
        let request = PlatformRequest::show_system_notification(notification)?;
        #[cfg(target_os = "windows")]
        validate_windows_notification_app_info(self.runtime.app_info.as_ref())?;
        self.queue_platform_request(request)
    }

    /// Dismiss the operating-system notification identified by `tag`, where supported.
    pub fn dismiss_system_notification(
        &mut self,
        tag: impl Into<Arc<str>>,
    ) -> Result<(), PlatformError> {
        let request = PlatformRequest::dismiss_system_notification(tag)?;
        #[cfg(target_os = "windows")]
        validate_windows_notification_app_info(self.runtime.app_info.as_ref())?;
        self.queue_platform_request(request)
    }

    /// Query notification authorization without displaying a prompt.
    pub fn notification_permission_status(
        &mut self,
    ) -> Result<NotificationPermissionResponse, PlatformError> {
        #[cfg(target_os = "windows")]
        validate_windows_notification_app_info(self.runtime.app_info.as_ref())?;
        let (request, response) = PlatformRequest::notification_permission_status();
        self.queue_platform_response(request)?;
        Ok(response)
    }

    /// Explicitly request notification authorization where required by the operating system.
    pub fn request_notification_permission(
        &mut self,
    ) -> Result<NotificationPermissionResponse, PlatformError> {
        #[cfg(target_os = "windows")]
        validate_windows_notification_app_info(self.runtime.app_info.as_ref())?;
        let (request, response) = PlatformRequest::request_notification_permission();
        self.queue_platform_response(request)?;
        Ok(response)
    }

    pub fn set_dock_badge(&mut self, value: impl Into<Arc<str>>) -> Result<(), PlatformError> {
        if !DesktopIntegrationSupport::current().dock_badges {
            return Err(PlatformError::Unsupported);
        }
        let value = value.into();
        self.queue_platform_request(PlatformRequest::set_dock_badge(
            (!value.is_empty()).then_some(value),
        )?)
    }

    pub fn clear_dock_badge(&mut self) -> Result<(), PlatformError> {
        if !DesktopIntegrationSupport::current().dock_badges {
            return Err(PlatformError::Unsupported);
        }
        self.queue_platform_request(PlatformRequest::set_dock_badge(None)?)
    }

    /// Last Dock badge successfully applied by the core runtime.
    pub fn dock_badge(&self) -> Option<&str> {
        #[cfg(target_os = "macos")]
        {
            self.runtime.dock_badge.as_deref()
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    pub fn set_dock_icon(&mut self, icon: Image) -> Result<(), PlatformError> {
        if !DesktopIntegrationSupport::current().dock_icons {
            return Err(PlatformError::Unsupported);
        }
        self.queue_platform_request(PlatformRequest::set_dock_icon(Some(icon)))
    }

    pub fn clear_dock_icon(&mut self) -> Result<(), PlatformError> {
        if !DesktopIntegrationSupport::current().dock_icons {
            return Err(PlatformError::Unsupported);
        }
        self.queue_platform_request(PlatformRequest::set_dock_icon(None))
    }

    pub fn set_dock_menu(&mut self, menu: Menu) -> Result<(), PlatformError> {
        if !DesktopIntegrationSupport::current().dock_menus {
            return Err(PlatformError::Unsupported);
        }
        self.queue_platform_request(PlatformRequest::set_dock_menu(Some(menu))?)
    }

    pub fn clear_dock_menu(&mut self) -> Result<(), PlatformError> {
        if !DesktopIntegrationSupport::current().dock_menus {
            return Err(PlatformError::Unsupported);
        }
        self.queue_platform_request(PlatformRequest::set_dock_menu(None)?)
    }

    /// Current macOS Dock menu declaration, after its request has been processed.
    pub fn dock_menu(&self) -> Option<&Menu> {
        #[cfg(target_os = "macos")]
        {
            self.runtime.dock_menu.as_ref()
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    pub fn add_recent_document(&mut self, path: impl Into<PathBuf>) -> Result<(), PlatformError> {
        if !DesktopIntegrationSupport::current().recent_documents {
            return Err(PlatformError::Unsupported);
        }
        self.queue_platform_request(PlatformRequest::add_recent_document(path)?)
    }

    pub fn clear_recent_documents(&mut self) -> Result<(), PlatformError> {
        if !DesktopIntegrationSupport::current().recent_documents {
            return Err(PlatformError::Unsupported);
        }
        self.queue_platform_request(PlatformRequest::ClearRecentDocuments)
    }

    pub fn show_about_panel(&mut self, options: AboutPanelOptions) -> Result<(), PlatformError> {
        if !DesktopIntegrationSupport::current().native_about_panel {
            return Err(PlatformError::Unsupported);
        }
        self.queue_platform_request(PlatformRequest::show_about_panel(options)?)
    }

    pub fn file_icon(
        &mut self,
        path: impl Into<PathBuf>,
        size: FileIconSize,
    ) -> Result<FileIconResponse, PlatformError> {
        if !DesktopIntegrationSupport::current().file_icons {
            return Err(PlatformError::Unsupported);
        }
        let (request, response) = PlatformRequest::get_file_icon(path, size)?;
        self.queue_platform_response(request)?;
        Ok(response)
    }

    pub fn set_user_tasks(
        &mut self,
        tasks: impl IntoIterator<Item = UserTask>,
    ) -> Result<ShellResponse, PlatformError> {
        if !DesktopIntegrationSupport::current().user_tasks {
            return Err(PlatformError::Unsupported);
        }
        let (request, response) = PlatformRequest::set_user_tasks(tasks.into_iter().collect())?;
        self.queue_platform_response(request)?;
        Ok(response)
    }

    pub fn clear_user_tasks(&mut self) -> Result<ShellResponse, PlatformError> {
        self.set_user_tasks(std::iter::empty())
    }

    /// Replace the complete native application menu set on the next event-loop turn.
    pub fn set_application_menus(
        &mut self,
        menus: impl IntoIterator<Item = Menu>,
    ) -> Result<(), PlatformError> {
        let menus = menus.into_iter().collect::<Vec<_>>();
        crate::menu::validate_menus(&menus).map_err(|_| PlatformError::InvalidMenu)?;
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = menus;
            return Err(PlatformError::Unsupported);
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            if !matches!(self.status, AppRunStatus::Continue) {
                return Err(PlatformError::Unavailable);
            }
            self.runtime
                .event_proxy
                .send_event(RuntimeEvent::ExternalCommandsReady)
                .map_err(|_| PlatformError::Unavailable)?;
            self.runtime.external_menus = Some(menus);
            Ok(())
        }
    }

    /// Remove every application-wide native menu.
    pub fn clear_application_menus(&mut self) -> Result<(), PlatformError> {
        self.set_application_menus(std::iter::empty())
    }

    /// Queue one title mutation for a mounted native window.
    pub fn set_window_title(
        &mut self,
        handle: WindowHandle,
        title: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
        let title = title.into();
        validate_window_title(&title)?;
        self.queue_window_command(handle, WindowCommand::SetTitle(handle, title))
    }

    pub fn set_window_bounds(
        &mut self,
        handle: WindowHandle,
        bounds: WindowBounds,
    ) -> Result<(), WindowCommandError> {
        validate_window_bounds(bounds)?;
        self.queue_window_command(handle, WindowCommand::SetBounds(handle, bounds))
    }

    pub fn move_window(
        &mut self,
        handle: WindowHandle,
        position: Point,
    ) -> Result<(), WindowCommandError> {
        validate_window_position(position)?;
        self.queue_window_command(handle, WindowCommand::Move(handle, position))
    }

    pub fn resize_window(
        &mut self,
        handle: WindowHandle,
        size: Size,
    ) -> Result<(), WindowCommandError> {
        validate_window_size(size)?;
        self.queue_window_command(handle, WindowCommand::Resize(handle, size))
    }

    /// Queue a native minimize request for a mounted window.
    pub fn minimize_window(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::Minimize(handle))
    }

    /// Restore a minimized or maximized native window to its windowed bounds.
    pub fn restore_window(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::Restore(handle))
    }

    /// Toggle the platform-native maximized/zoomed state.
    pub fn maximize_window(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        if self
            .window_state(handle)
            .is_some_and(|state| state.maximized)
        {
            return Ok(());
        }
        self.queue_window_command(handle, WindowCommand::Zoom(handle))
    }

    /// Set the native fullscreen state.
    pub fn set_window_fullscreen(
        &mut self,
        handle: WindowHandle,
        fullscreen: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetFullscreen(handle, fullscreen))
    }

    /// Show or hide a mounted native window.
    pub fn set_window_visible(
        &mut self,
        handle: WindowHandle,
        visible: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetVisible(handle, visible))
    }

    pub fn set_window_resizable(
        &mut self,
        handle: WindowHandle,
        resizable: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetResizable(handle, resizable))
    }

    pub fn set_window_movable(
        &mut self,
        handle: WindowHandle,
        movable: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetMovable(handle, movable))
    }

    pub fn set_window_minimum_size(
        &mut self,
        handle: WindowHandle,
        minimum: Option<Size>,
    ) -> Result<(), WindowCommandError> {
        if let Some(minimum) = minimum {
            validate_window_size(minimum)?;
        }
        self.queue_window_command(handle, WindowCommand::SetMinimumSize(handle, minimum))
    }

    pub fn set_window_maximum_size(
        &mut self,
        handle: WindowHandle,
        maximum: Option<Size>,
    ) -> Result<(), WindowCommandError> {
        if let Some(maximum) = maximum {
            validate_window_size(maximum)?;
        }
        self.queue_window_command(handle, WindowCommand::SetMaximumSize(handle, maximum))
    }

    pub fn set_window_minimizable(
        &mut self,
        handle: WindowHandle,
        minimizable: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetMinimizable(handle, minimizable))
    }

    pub fn set_window_maximizable(
        &mut self,
        handle: WindowHandle,
        maximizable: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetMaximizable(handle, maximizable))
    }

    pub fn set_window_closable(
        &mut self,
        handle: WindowHandle,
        closable: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetClosable(handle, closable))
    }

    pub fn set_window_decorated(
        &mut self,
        handle: WindowHandle,
        decorated: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetDecorated(handle, decorated))
    }

    pub fn set_window_shadow(
        &mut self,
        handle: WindowHandle,
        shadow: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetShadow(handle, shadow))
    }

    pub fn set_window_content_protected(
        &mut self,
        handle: WindowHandle,
        protected: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(
            handle,
            WindowCommand::SetContentProtected(handle, protected),
        )
    }

    pub fn set_window_level(
        &mut self,
        handle: WindowHandle,
        level: Option<WindowLevel>,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetWindowLevel(handle, level))
    }

    pub fn set_window_focusable(
        &mut self,
        handle: WindowHandle,
        focusable: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetFocusable(handle, focusable))
    }

    pub fn set_window_skip_taskbar(
        &mut self,
        handle: WindowHandle,
        skip: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetSkipTaskbar(handle, skip))
    }

    pub fn set_window_visible_on_all_workspaces(
        &mut self,
        handle: WindowHandle,
        visible: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(
            handle,
            WindowCommand::SetVisibleOnAllWorkspaces(handle, visible),
        )
    }

    pub fn set_window_opacity(
        &mut self,
        handle: WindowHandle,
        opacity: f32,
    ) -> Result<(), WindowCommandError> {
        validate_window_opacity(opacity)?;
        self.queue_window_command(handle, WindowCommand::SetOpacity(handle, opacity))
    }

    pub fn set_window_icon(
        &mut self,
        handle: WindowHandle,
        icon: Option<Image>,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetIcon(handle, icon))
    }

    pub fn set_cursor_visible(
        &mut self,
        handle: WindowHandle,
        visible: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetCursorVisible(handle, visible))
    }

    pub fn set_cursor_grab(
        &mut self,
        handle: WindowHandle,
        mode: CursorGrabMode,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetCursorGrab(handle, mode))
    }

    pub fn set_cursor_hit_test(
        &mut self,
        handle: WindowHandle,
        hit_test: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetCursorHitTest(handle, hit_test))
    }

    pub fn set_cursor_position(
        &mut self,
        handle: WindowHandle,
        position: Point,
    ) -> Result<(), WindowCommandError> {
        validate_window_position(position)?;
        self.queue_window_command(handle, WindowCommand::SetCursorPosition(handle, position))
    }

    pub fn set_taskbar_progress(
        &mut self,
        handle: WindowHandle,
        state: TaskbarProgressState,
        progress: f32,
    ) -> Result<(), WindowCommandError> {
        validate_taskbar_progress(progress)?;
        self.queue_window_command(
            handle,
            WindowCommand::SetTaskbarProgress(handle, state, progress),
        )
    }

    pub fn set_taskbar_overlay_icon(
        &mut self,
        handle: WindowHandle,
        icon: Image,
        description: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
        let description = description.into();
        validate_taskbar_overlay_description(Some(&description))?;
        self.queue_window_command(
            handle,
            WindowCommand::SetTaskbarOverlayIcon(handle, Some(icon), Some(description)),
        )
    }

    pub fn clear_taskbar_overlay_icon(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(
            handle,
            WindowCommand::SetTaskbarOverlayIcon(handle, None, None),
        )
    }

    /// Bring a mounted native window to the front and give it keyboard focus.
    pub fn focus_window(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.ensure_window_command_target(handle)?;
        if self.runtime.focus_requests.len() == MAX_PENDING_WINDOW_COMMANDS {
            return Err(WindowCommandError::QueueFull);
        }
        self.wake_for_external_command()?;
        if !self.runtime.focus_requests.contains(&handle) {
            self.runtime.focus_requests.push(handle);
        }
        Ok(())
    }

    /// Request informational attention for a mounted native window.
    pub fn request_window_attention(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::RequestAttention(handle))
    }

    /// Set or clear the represented document path in native window chrome.
    pub fn set_window_represented_file(
        &mut self,
        handle: WindowHandle,
        path: Option<PathBuf>,
    ) -> Result<(), WindowCommandError> {
        if let Some(path) = &path {
            validate_window_document_path(path)?;
        }
        self.queue_window_command(handle, WindowCommand::SetRepresentedFile(handle, path))
    }

    /// Set the native unsaved-document indicator for a mounted window.
    pub fn set_window_document_edited(
        &mut self,
        handle: WindowHandle,
        edited: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetDocumentEdited(handle, edited))
    }

    /// Force one native window to light/dark appearance, or follow the operating system with
    /// `None`.
    pub fn set_window_appearance(
        &mut self,
        handle: WindowHandle,
        appearance: Option<WindowAppearance>,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetAppearance(handle, appearance))
    }

    pub fn set_window_background_appearance(
        &mut self,
        handle: WindowHandle,
        appearance: WindowBackgroundAppearance,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(
            handle,
            WindowCommand::SetBackgroundAppearance(handle, appearance),
        )
    }

    pub fn set_macos_window_vibrancy(
        &mut self,
        handle: WindowHandle,
        vibrancy: Option<MacOsVibrancy>,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetMacOsVibrancy(handle, vibrancy))
    }

    pub fn set_macos_visual_effect_state(
        &mut self,
        handle: WindowHandle,
        state: MacOsVisualEffectState,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(
            handle,
            WindowCommand::SetMacOsVisualEffectState(handle, state),
        )
    }

    /// Present AppKit's character palette above one mounted window.
    pub fn show_character_palette(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::ShowCharacterPalette(handle))
    }

    /// Show the platform dictionary definition for one mounted window's focused text selection.
    pub fn show_definition_for_selection(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::LookUpSelection(handle))
    }

    /// Opt one mounted window into a named native system-tab group, or leave it with `None`.
    pub fn set_window_tabbing_identifier(
        &mut self,
        handle: WindowHandle,
        identifier: Option<String>,
    ) -> Result<(), WindowCommandError> {
        if let Some(identifier) = &identifier {
            validate_window_tabbing_identifier(identifier)?;
        }
        self.queue_window_command(
            handle,
            WindowCommand::SetTabbingIdentifier(handle, identifier),
        )
    }

    /// Select the next tab in one window's native tab group.
    pub fn select_next_window_tab(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SelectNextTab(handle))
    }

    /// Select the previous tab in one window's native tab group.
    pub fn select_previous_window_tab(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SelectPreviousTab(handle))
    }

    /// Select one tab by index in a window's native tab group.
    pub fn select_window_tab(
        &mut self,
        handle: WindowHandle,
        index: usize,
    ) -> Result<(), WindowCommandError> {
        if index >= MAX_SYSTEM_WINDOW_TABS {
            return Err(WindowCommandError::InvalidTabIndex);
        }
        self.queue_window_command(handle, WindowCommand::SelectTab(handle, index))
    }

    /// Merge every window of this application into one native tab group.
    pub fn merge_all_windows(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::MergeAllWindows(handle))
    }

    /// Move one window's active native tab into a window of its own.
    pub fn move_window_tab_to_new_window(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::MoveTabToNewWindow(handle))
    }

    /// Toggle one window's native tab bar.
    pub fn toggle_window_tab_bar(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::ToggleTabBar(handle))
    }

    /// Toggle one window's native tab overview.
    pub fn toggle_window_tab_overview(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::ToggleTabOverview(handle))
    }

    /// Read macOS's shared Find pasteboard.
    #[cfg(target_os = "macos")]
    pub fn read_from_find_pasteboard(
        &self,
    ) -> Result<Option<ClipboardItem>, crate::ClipboardError> {
        self.runtime.clipboard.read(ClipboardTarget::Find)
    }

    /// Atomically replace macOS's shared Find pasteboard. An empty item clears it.
    #[cfg(target_os = "macos")]
    pub fn write_to_find_pasteboard(
        &self,
        item: ClipboardItem,
    ) -> Result<(), crate::ClipboardError> {
        self.runtime.clipboard.write(ClipboardTarget::Find, item)
    }

    /// Raise one mounted window to the front of its stacking level without activating the app.
    pub fn move_window_to_top(&mut self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::MoveToTop(handle))
    }

    /// Order one mounted window immediately above another retained window.
    pub fn move_window_above(
        &mut self,
        handle: WindowHandle,
        other: WindowHandle,
    ) -> Result<(), WindowCommandError> {
        if handle == other {
            return Err(WindowCommandError::InvalidWindowOrder);
        }
        self.ensure_window_command_target(other)?;
        self.queue_window_command(handle, WindowCommand::MoveAbove(handle, other))
    }

    /// Let clicks pass through one window to whatever is behind it.
    ///
    /// `forward` keeps pointer motion flowing to the window; it is ignored when `ignore` is false.
    pub fn set_window_ignore_mouse_events(
        &mut self,
        handle: WindowHandle,
        ignore: bool,
        forward: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(
            handle,
            WindowCommand::SetIgnoreMouseEvents(handle, ignore, ignore && forward),
        )
    }

    /// Block or restore every native input event for one window.
    pub fn set_window_enabled(
        &mut self,
        handle: WindowHandle,
        enabled: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(handle, WindowCommand::SetWindowEnabled(handle, enabled))
    }

    /// Constrain one window's live resizing to a `width:height` content ratio.
    pub fn set_window_aspect_ratio(
        &mut self,
        handle: WindowHandle,
        ratio: Option<Size>,
    ) -> Result<(), WindowCommandError> {
        validate_window_aspect_ratio(ratio)?;
        self.queue_window_command(handle, WindowCommand::SetAspectRatio(handle, ratio))
    }

    /// Show or hide the macOS close/minimize/zoom buttons on one window.
    pub fn set_window_button_visibility(
        &mut self,
        handle: WindowHandle,
        visible: bool,
    ) -> Result<(), WindowCommandError> {
        self.queue_window_command(
            handle,
            WindowCommand::SetWindowButtonVisibility(handle, visible),
        )
    }

    /// Raise or restore one window's stacking level.
    ///
    /// `level` names the level applied while `always_on_top` is true; `None` uses
    /// [`WindowLevel::AlwaysOnTop`]. Turning it off returns the window to [`WindowLevel::Normal`].
    pub fn set_window_always_on_top(
        &mut self,
        handle: WindowHandle,
        always_on_top: bool,
        level: Option<WindowLevel>,
    ) -> Result<(), WindowCommandError> {
        let level = if always_on_top {
            level.unwrap_or(WindowLevel::AlwaysOnTop)
        } else {
            WindowLevel::Normal
        };
        self.queue_window_command(handle, WindowCommand::SetWindowLevel(handle, Some(level)))
    }

    /// Change how the application appears in the Dock and application switcher.
    pub fn set_activation_policy(
        &mut self,
        policy: crate::ActivationPolicy,
    ) -> Result<PlatformResponse<()>, PlatformError> {
        let (request, response) = PlatformRequest::set_activation_policy(policy);
        self.queue_platform_request(request)?;
        Ok(response)
    }

    /// Bring the application forward, optionally stealing focus from the frontmost application.
    pub fn activate_application(&mut self, force: bool) -> Result<(), PlatformError> {
        self.queue_platform_request(PlatformRequest::ActivateApplication { force })
    }

    /// Hide every window of this application.
    pub fn hide_application(&mut self) -> Result<(), PlatformError> {
        self.queue_platform_request(PlatformRequest::HideApplication)
    }

    /// Reveal an application hidden by [`Self::hide_application`].
    pub fn unhide_application(&mut self) -> Result<(), PlatformError> {
        self.queue_platform_request(PlatformRequest::UnhideApplication)
    }

    /// Ask for the user's attention, bouncing the macOS Dock tile.
    pub fn request_dock_attention(
        &mut self,
        attention: crate::DockAttention,
    ) -> Result<PlatformResponse<crate::DockAttentionRequest>, PlatformError> {
        let (request, response) = PlatformRequest::request_dock_attention(attention);
        self.queue_platform_request(request)?;
        Ok(response)
    }

    /// Stop an in-flight critical Dock bounce.
    pub fn cancel_dock_attention(
        &mut self,
        request: crate::DockAttentionRequest,
    ) -> Result<(), PlatformError> {
        self.queue_platform_request(PlatformRequest::CancelDockAttention(request))
    }

    /// Show or hide the Dock tile.
    pub fn set_dock_visible(
        &mut self,
        visible: bool,
    ) -> Result<PlatformResponse<()>, PlatformError> {
        let (request, response) = PlatformRequest::set_dock_visible(visible);
        self.queue_platform_request(request)?;
        Ok(response)
    }

    /// Route keystrokes straight to this process, bypassing input monitoring.
    pub fn set_secure_keyboard_entry(&mut self, enabled: bool) -> Result<(), PlatformError> {
        self.queue_platform_request(PlatformRequest::SetSecureKeyboardEntry(enabled))
    }

    /// Play the operating system's alert sound.
    pub fn beep(&mut self) -> Result<(), PlatformError> {
        self.queue_platform_request(PlatformRequest::Beep)
    }

    /// Whether this process can relocate its bundle into an `/Applications` directory.
    pub fn applications_folder_support(&self) -> crate::ApplicationsFolderSupport {
        #[cfg(target_os = "macos")]
        {
            crate::macos_shell::applications_folder_support()
        }
        #[cfg(not(target_os = "macos"))]
        {
            crate::ApplicationsFolderSupport::default()
        }
    }

    /// Move the running application bundle into `/Applications`.
    pub fn move_to_applications_folder(&mut self) -> Result<PlatformResponse<bool>, PlatformError> {
        let (request, response) = PlatformRequest::move_to_applications_folder();
        self.queue_platform_request(request)?;
        Ok(response)
    }

    /// Whether this process is running from an installed application bundle.
    pub fn is_application_packaged(&self) -> bool {
        crate::is_application_packaged()
    }

    /// Request an orderly application exit that ends with an explicit process exit code.
    pub fn exit_with_code(&mut self, code: i32) -> bool {
        if !self.exit() {
            return false;
        }
        self.runtime.exit_code = Some(code);
        true
    }

    /// Replace one window's native menu declaration on the next event-loop turn.
    ///
    /// The replacement is applied inside the runtime's window-scoped effect cycle, exactly where
    /// `EventContext::set_window_menus` applies one, so macOS menu-bar installation and Windows
    /// per-window attachment follow the identical path.
    pub fn set_window_menus(
        &mut self,
        handle: WindowHandle,
        menus: impl IntoIterator<Item = Menu>,
    ) -> Result<(), PlatformError> {
        let menus = menus.into_iter().collect::<Vec<_>>();
        validate_menus(&menus).map_err(|_| PlatformError::InvalidMenu)?;
        self.queue_window_menus(handle, Some(menus))
    }

    /// Remove one window's override and inherit the application's native menus again.
    pub fn use_application_menus_for_window(
        &mut self,
        handle: WindowHandle,
    ) -> Result<(), PlatformError> {
        self.queue_window_menus(handle, None)
    }

    /// Open a platform-native popup menu owned by one mounted window.
    ///
    /// `position` is in window-local logical pixels from the top-left; `None` uses the current
    /// native cursor position. The returned response completes once the popup closes, whether an
    /// item was chosen or the user dismissed it.
    pub fn show_window_popup_menu(
        &mut self,
        handle: WindowHandle,
        menu: Menu,
        position: Option<Point>,
    ) -> Result<PlatformResponse<()>, PlatformError> {
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (handle, menu, position);
            Err(PlatformError::Unsupported)
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            if !matches!(self.status, AppRunStatus::Continue)
                || !self.runtime.window_handles.contains_key(&handle)
            {
                return Err(PlatformError::Unavailable);
            }
            if position.is_some_and(|position| validate_window_position(position).is_err()) {
                return Err(PlatformError::InvalidMenuPosition);
            }
            validate_menus(std::slice::from_ref(&menu)).map_err(|_| PlatformError::InvalidMenu)?;
            let (responder, response) = crate::platform::response_channel();
            queue_deferred_menu_request(
                &mut self.runtime.external_popup_menus,
                ExternalPopupMenuRequest {
                    window: handle,
                    menu,
                    position,
                    responder,
                },
            )?;
            self.runtime
                .event_proxy
                .send_event(RuntimeEvent::ExternalCommandsReady)
                .map_err(|_| PlatformError::Unavailable)?;
            Ok(response)
        }
    }

    fn queue_window_menus(
        &mut self,
        handle: WindowHandle,
        menus: Option<Vec<Menu>>,
    ) -> Result<(), PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue)
            || !self.runtime.window_handles.contains_key(&handle)
        {
            return Err(PlatformError::Unavailable);
        }
        queue_deferred_menu_request(
            &mut self.runtime.external_window_menus,
            ExternalWindowMenus {
                window: handle,
                menus,
            },
        )?;
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        Ok(())
    }

    fn queue_window_command(
        &mut self,
        handle: WindowHandle,
        command: WindowCommand,
    ) -> Result<(), WindowCommandError> {
        self.ensure_window_command_target(handle)?;
        if self.runtime.window_commands.len() == MAX_PENDING_WINDOW_COMMANDS {
            return Err(WindowCommandError::QueueFull);
        }
        self.wake_for_external_command()?;
        self.runtime.window_commands.push(command);
        Ok(())
    }

    fn queue_platform_response(&mut self, request: PlatformRequest) -> Result<(), PlatformError> {
        self.queue_platform_request(request)
    }

    fn queue_platform_request(&mut self, request: PlatformRequest) -> Result<(), PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.platform_requests.len() == crate::MAX_PENDING_PLATFORM_REQUESTS {
            return Err(PlatformError::PendingQueueFull);
        }
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime.platform_requests.push_back(request);
        Ok(())
    }

    fn ensure_window_command_target(&self, handle: WindowHandle) -> Result<(), WindowCommandError> {
        // A handle is a valid target from the moment `open_window` returned it: the platform
        // window is created on the next event-loop turn, and a command queued before then waits
        // for it rather than being refused for the few milliseconds of that gap.
        if !matches!(self.status, AppRunStatus::Continue)
            || !(self.runtime.window_handles.contains_key(&handle)
                || self.runtime.current_handle() == Some(handle)
                || self.runtime.window_is_pending(handle))
        {
            Err(WindowCommandError::Unavailable)
        } else {
            Ok(())
        }
    }

    fn wake_for_external_command(&self) -> Result<(), WindowCommandError> {
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| WindowCommandError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_window_commands_keep_public_validation() {
        assert_eq!(
            validate_window_title(&"x".repeat(MAX_WINDOW_TITLE_BYTES + 1)),
            Err(WindowCommandError::TitleTooLong)
        );
        assert_eq!(
            validate_window_document_path(Path::new("")),
            Err(WindowCommandError::InvalidDocumentPath)
        );
    }
}
