use super::*;

impl NativeRuntime {
    pub(crate) fn execute_system_command(
        &mut self,
        command: SystemCommand,
    ) -> std::result::Result<SystemCommandResult, String> {
        self.sync_closed_windows();
        match command {
            SystemCommand::ConfigureApp(options) => {
                if self.runner.is_some() {
                    return Err(
                        "application options must be configured before readiness".to_owned()
                    );
                }
                let fonts = native_font_data(&options);
                let (app_info, app_paths, quit_mode) = update_native_app_configuration(
                    self.app_info.clone(),
                    self.app_paths.clone(),
                    self.quit_mode,
                    options,
                )?;
                self.app_info = app_info;
                self.app_paths = app_paths;
                self.quit_mode = quit_mode;
                if let Some(fonts) = fonts {
                    self.fonts = fonts;
                }
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::Exit => Ok(SystemCommandResult::Boolean(
                self.running_runner_mut()?.exit(),
            )),
            SystemCommand::Relaunch(options) => {
                let mut relaunch = RelaunchOptions::new();
                if let Some(executable) = options.executable {
                    relaunch = relaunch.executable(executable);
                }
                if options.clear_arguments.unwrap_or(false) {
                    if options.arguments.is_some() {
                        return Err(
                            "relaunch arguments and clearArguments cannot be combined".to_owned()
                        );
                    }
                    relaunch = relaunch.without_arguments();
                } else if let Some(arguments) = options.arguments {
                    relaunch = relaunch.arguments(arguments);
                }
                if let Some(directory) = options.working_directory {
                    relaunch = relaunch.working_directory(directory);
                }
                self.running_runner_mut()?
                    .relaunch_with(relaunch)
                    .map(SystemCommandResult::Boolean)
                    .map_err(|error| error.to_string())
            }
            SystemCommand::GetAppInfo => Ok(SystemCommandResult::AppInfo(
                self.running_runner()?.app_info().map(Into::into),
            )),
            SystemCommand::GetAppPaths => Ok(SystemCommandResult::AppPaths(
                self.running_runner()?.app_paths().map(Into::into),
            )),
            SystemCommand::GetSystemInfo => Ok(SystemCommandResult::SystemInfo(
                self.running_runner()?.system_info().into(),
            )),
            SystemCommand::GetWindowRegistry => {
                let runner = self.running_runner()?;
                let registry = runner.window_registry();
                let handles = self.handles.borrow();
                Ok(SystemCommandResult::WindowRegistry(NativeWindowRegistry {
                    windows: registry
                        .windows()
                        .iter()
                        .filter_map(|handle| handles.get(handle).copied())
                        .collect(),
                    active_window: runner
                        .active_window()
                        .and_then(|handle| handles.get(&handle).copied()),
                    truncated: registry.is_truncated(),
                }))
            }
            SystemCommand::GetCursorScreenPosition => self
                .running_runner()?
                .cursor_screen_position()
                .map(|point| SystemCommandResult::Point(point.into()))
                .map_err(|error| error.to_string()),
            SystemCommand::GetDesktopIntegrationSupport => {
                Ok(SystemCommandResult::DesktopIntegrationSupport(
                    DesktopIntegrationSupport::current().into(),
                ))
            }
            SystemCommand::GetSystemPreferences => Ok(SystemCommandResult::SystemPreferences(
                self.running_runner()?.system_preferences().into(),
            )),
            SystemCommand::GetDisplays => {
                let runner = self.running_runner()?;
                Ok(SystemCommandResult::Displays(
                    runner
                        .displays()
                        .all()
                        .iter()
                        .map(NativeDisplay::from)
                        .collect(),
                ))
            }
            SystemCommand::GetKeyboardLayout => {
                let layout = self.running_runner()?.keyboard_layout();
                Ok(SystemCommandResult::KeyboardLayout((&layout).into()))
            }
            SystemCommand::GetWindowState(window) => {
                let handle = self.system_window_handle(window)?;
                let state = self
                    .running_runner()?
                    .window_state(handle)
                    .ok_or_else(|| format!("native window {window} is not mounted"))?;
                Ok(SystemCommandResult::WindowState(state.into()))
            }
            SystemCommand::GetWindowFrameMetrics(window) => {
                let handle = self.system_window_handle(window)?;
                let metrics = self
                    .running_runner()?
                    .window_frame_metrics(handle)
                    .ok_or_else(|| format!("native window {window} is not mounted"))?;
                Ok(SystemCommandResult::FrameMetrics(NativeFrameMetrics {
                    frame_number: metrics.frame_number,
                    cpu_milliseconds: metrics.cpu_milliseconds(),
                    smoothed_cpu_milliseconds: metrics.smoothed_cpu_milliseconds(),
                    frame_milliseconds: metrics.frame_milliseconds(),
                    smoothed_frame_milliseconds: metrics.smoothed_frame_milliseconds(),
                }))
            }
            SystemCommand::ReadClipboard => self
                .running_runner()?
                .read_from_clipboard()
                .map(SystemCommandResult::Clipboard)
                .map_err(|error| error.to_string()),
            SystemCommand::WriteClipboard(item) => self
                .running_runner()?
                .write_to_clipboard(item)
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::ShowNotification(notification) => self
                .running_runner_mut()?
                .show_system_notification(notification)
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::DismissNotification(tag) => self
                .running_runner_mut()?
                .dismiss_system_notification(tag)
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::NotificationPermission { request, prompt } => {
                if request == 0
                    || self
                        .pending_notification_permissions
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err(
                        "notification-permission request ids must be nonzero and unique".to_owned(),
                    );
                }
                let runner = self.running_runner_mut()?;
                let response = if prompt {
                    runner.request_notification_permission()
                } else {
                    runner.notification_permission_status()
                }
                .map_err(|error| error.to_string())?;
                self.pending_notification_permissions
                    .push(PendingNotificationPermission::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::SetDockBadge(value) => {
                let runner = self.running_runner_mut()?;
                match value {
                    Some(value) => runner.set_dock_badge(value),
                    None => runner.clear_dock_badge(),
                }
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string())
            }
            SystemCommand::SetDockIcon(value) => {
                let runner = self.running_runner_mut()?;
                match value {
                    Some(icon) => runner.set_dock_icon(icon),
                    None => runner.clear_dock_icon(),
                }
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string())
            }
            SystemCommand::SetDockMenu(value) => {
                let value = dock_menu(value)?;
                let runner = self.running_runner_mut()?;
                match value {
                    Some(menu) => runner.set_dock_menu(menu),
                    None => runner.clear_dock_menu(),
                }
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string())
            }
            SystemCommand::AddRecentDocument(path) => self
                .running_runner_mut()?
                .add_recent_document(path)
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::ClearRecentDocuments => self
                .running_runner_mut()?
                .clear_recent_documents()
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::ShowAboutPanel(options) => self
                .running_runner_mut()?
                .show_about_panel(options)
                .map(|()| SystemCommandResult::Unit)
                .map_err(|error| error.to_string()),
            SystemCommand::FileIcon {
                request,
                path,
                size,
            } => {
                if request == 0
                    || self
                        .pending_file_icons
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err("file-icon request ids must be nonzero and unique".to_owned());
                }
                let response = self
                    .running_runner_mut()?
                    .file_icon(path, size)
                    .map_err(|error| error.to_string())?;
                self.pending_file_icons
                    .push(PendingFileIcon::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::SetUserTasks { request, tasks } => {
                if request == 0
                    || self
                        .pending_user_tasks
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err("user-task request ids must be nonzero and unique".to_owned());
                }
                let response = self
                    .running_runner_mut()?
                    .set_user_tasks(tasks)
                    .map_err(|error| error.to_string())?;
                self.pending_user_tasks
                    .push(PendingUserTasks::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::SetApplicationMenu(json) => {
                let menus = menu::application_menus(&json)?;
                self.running_runner_mut()?
                    .set_application_menus(menus)
                    .map(|()| SystemCommandResult::Unit)
                    .map_err(|error| error.to_string())
            }
            SystemCommand::SetQuitInterception(intercepting) => {
                crate::runtime::set_quit_interception(intercepting);
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::RequestQuit => Ok(SystemCommandResult::Boolean(
                self.running_runner_mut()?.request_quit(),
            )),
            SystemCommand::ReadFindClipboard => {
                #[cfg(target_os = "macos")]
                {
                    self.running_runner()?
                        .read_from_find_pasteboard()
                        .map(SystemCommandResult::Clipboard)
                        .map_err(|error| error.to_string())
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = self.running_runner()?;
                    Ok(SystemCommandResult::Clipboard(None))
                }
            }
            SystemCommand::WriteFindClipboard(item) => {
                #[cfg(target_os = "macos")]
                {
                    self.running_runner()?
                        .write_to_find_pasteboard(item)
                        .map(|()| SystemCommandResult::Unit)
                        .map_err(|error| error.to_string())
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = (self.running_runner()?, item);
                    Err("the Find pasteboard is a macOS integration".to_owned())
                }
            }
            SystemCommand::RequestSingleInstanceLock(identifier) => self
                .running_runner_mut()?
                .request_single_instance_lock(identifier)
                .map(SystemCommandResult::Boolean)
                .map_err(|error| error.to_string()),
            SystemCommand::ReleaseSingleInstanceLock => Ok(SystemCommandResult::Boolean(
                self.running_runner_mut()?.release_single_instance_lock(),
            )),
            SystemCommand::GlobalShortcut { request, action } => {
                if request == 0
                    || self
                        .pending_global_shortcuts
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err(
                        "native global-shortcut request ids must be nonzero and unique".to_owned(),
                    );
                }
                let runner = self.running_runner_mut()?;
                let response = match action {
                    GlobalShortcutAction::Register {
                        registration,
                        accelerator,
                    } => runner.register_global_shortcut(registration, accelerator),
                    GlobalShortcutAction::Unregister { registration } => {
                        runner.unregister_global_shortcut(registration)
                    }
                    GlobalShortcutAction::UnregisterAll => runner.unregister_all_global_shortcuts(),
                }
                .map_err(|error| error.to_string())?;
                self.pending_global_shortcuts
                    .push(PendingGlobalShortcut::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::Tray { request, action } => {
                if request == 0
                    || self
                        .pending_tray
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err("native tray request ids must be nonzero and unique".to_owned());
                }
                let runner = self.running_runner_mut()?;
                let response = match action {
                    tray::TrayAction::Set(options) => runner.set_tray_icon(options),
                    tray::TrayAction::Remove(id) => runner.remove_tray_icon(id),
                    tray::TrayAction::ShowMenu(id) => runner.show_tray_menu(id),
                }
                .map_err(|error| error.to_string())?;
                self.pending_tray.push(PendingTray::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::ExitWithCode(code) => Ok(SystemCommandResult::Boolean(
                self.running_runner_mut()?.exit_with_code(code),
            )),
            SystemCommand::GetApplicationsFolderSupport => {
                let support = self.running_runner()?.applications_folder_support();
                Ok(SystemCommandResult::ApplicationsFolderSupport(
                    NativeApplicationsFolderSupport {
                        supported: support.supported,
                        already_installed: support.already_installed,
                    },
                ))
            }
            SystemCommand::GetWindowRestoreState(window) => {
                let handle = self.system_window_handle(window)?;
                let runner = self.running_runner()?;
                let state = runner
                    .window_state(handle)
                    .ok_or_else(|| format!("native window {window} is not mounted"))?;
                Ok(SystemCommandResult::WindowRestoreState(
                    state.restore_state(&runner.displays()).into(),
                ))
            }
            SystemCommand::AppService { request, action } => {
                if request == 0
                    || self
                        .pending_app_services
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err(
                        "native app-service request ids must be nonzero and unique".to_owned()
                    );
                }
                let runner = self.running_runner_mut()?;
                let response = match action {
                    AppServiceAction::SetActivationPolicy(policy) => runner
                        .set_activation_policy(policy)
                        .map(AppServiceResponse::Unit),
                    AppServiceAction::RequestDockAttention(attention) => runner
                        .request_dock_attention(attention)
                        .map(AppServiceResponse::DockAttention),
                    AppServiceAction::SetDockVisible(visible) => runner
                        .set_dock_visible(visible)
                        .map(AppServiceResponse::Unit),
                    AppServiceAction::MoveToApplicationsFolder => runner
                        .move_to_applications_folder()
                        .map(AppServiceResponse::Boolean),
                }
                .map_err(|error| error.to_string())?;
                self.pending_app_services.push(PendingAppService::new(
                    request,
                    "app-service",
                    response,
                ));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::AppMutation(AppMutationAction::LearnWord(word)) => {
                // Provider state is application-thread local, exactly where this command runs.
                let _ = self.running_runner()?;
                quickgui::spell_check_provider().learn(&word);
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::AppMutation(AppMutationAction::IgnoreWord(word)) => {
                let _ = self.running_runner()?;
                quickgui::spell_check_provider().ignore(&word, quickgui::SpellDocumentTag::NONE);
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::AppMutation(action) => {
                let runner = self.running_runner_mut()?;
                match action {
                    AppMutationAction::Activate(force) => runner.activate_application(force),
                    AppMutationAction::Hide => runner.hide_application(),
                    AppMutationAction::Unhide => runner.unhide_application(),
                    AppMutationAction::CancelDockAttention(id) => {
                        match take_dock_attention_request(id) {
                            Some(request) => runner.cancel_dock_attention(request),
                            // A stale identifier cannot cancel anything; the bounce has ended.
                            None => Ok(()),
                        }
                    }
                    AppMutationAction::SetSecureKeyboardEntry(enabled) => {
                        runner.set_secure_keyboard_entry(enabled)
                    }
                    AppMutationAction::Beep => runner.beep(),
                    // Handled before the runner borrow because they never reach the runner.
                    AppMutationAction::LearnWord(_) | AppMutationAction::IgnoreWord(_) => Ok(()),
                }
                .map_err(|error| error.to_string())?;
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::WindowPopupMenu {
                request,
                window,
                menu,
                position,
            } => {
                if request == 0
                    || self
                        .pending_app_services
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err(
                        "native popup-menu request ids must be nonzero and unique".to_owned()
                    );
                }
                let handle = self.system_window_handle(window)?;
                let menu = menu::popup_menu(&menu)?;
                let response = self
                    .running_runner_mut()?
                    .show_window_popup_menu(handle, menu, position)
                    .map_err(|error| error.to_string())?;
                self.pending_app_services.push(PendingAppService::new(
                    request,
                    "popup-menu",
                    AppServiceResponse::Unit(response),
                ));
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::WindowAction {
                window,
                action: WindowAction::SetMenu(menus),
            } => {
                let handle = self.system_window_handle(window)?;
                let menus = menus
                    .map(|json| menu::application_menus(&json))
                    .transpose()?;
                let runner = self.running_runner_mut()?;
                match menus {
                    Some(menus) => runner.set_window_menus(handle, menus),
                    None => runner.use_application_menus_for_window(handle),
                }
                .map_err(|error| error.to_string())?;
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::WindowAction {
                window,
                action: WindowAction::SetResizePolicy(policy),
            } => {
                // Declared ahead of the native decision: the core answers `WillResize`
                // synchronously and never waits on JavaScript.
                self.system_window_handle(window)?;
                let policy = policy.as_deref().map(parse_resize_policy).transpose()?;
                crate::runtime::set_resize_policy(window, policy);
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::WindowAction {
                window,
                action: WindowAction::SetMovePolicy(policy),
            } => {
                self.system_window_handle(window)?;
                let policy = policy.as_deref().map(parse_move_policy).transpose()?;
                crate::runtime::set_move_policy(window, policy);
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::WindowAction {
                window,
                action: WindowAction::MoveAbove(other),
            } => {
                let handle = self.system_window_handle(window)?;
                let other = self.system_window_handle(other)?;
                self.running_runner_mut()?
                    .move_window_above(handle, other)
                    .map_err(|error| error.to_string())?;
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::WindowAction {
                window,
                action: WindowAction::SetCloseInterception(intercepting),
            } => {
                // Interception is declared ahead of the native decision: the hosted view answers
                // `Event::CloseRequested` from this flag without waiting on JavaScript.
                self.system_window_handle(window)?;
                crate::runtime::set_close_interception(window, intercepting);
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::WindowAction { window, action } => {
                let handle = self.system_window_handle(window)?;
                let runner = self.running_runner_mut()?;
                match action {
                    WindowAction::SetTitle(title) => runner.set_window_title(handle, title),
                    WindowAction::SetBounds(bounds) => runner.set_window_bounds(handle, bounds),
                    WindowAction::Move(position) => runner.move_window(handle, position),
                    WindowAction::Resize(size) => runner.resize_window(handle, size),
                    WindowAction::Minimize => runner.minimize_window(handle),
                    WindowAction::Maximize => runner.maximize_window(handle),
                    WindowAction::Restore => runner.restore_window(handle),
                    WindowAction::SetFullscreen(fullscreen) => {
                        runner.set_window_fullscreen(handle, fullscreen)
                    }
                    WindowAction::SetVisible(visible) => runner.set_window_visible(handle, visible),
                    WindowAction::SetResizable(value) => runner.set_window_resizable(handle, value),
                    WindowAction::SetMovable(value) => runner.set_window_movable(handle, value),
                    WindowAction::SetMinimumSize(value) => {
                        runner.set_window_minimum_size(handle, value)
                    }
                    WindowAction::SetMaximumSize(value) => {
                        runner.set_window_maximum_size(handle, value)
                    }
                    WindowAction::SetMinimizable(value) => {
                        runner.set_window_minimizable(handle, value)
                    }
                    WindowAction::SetMaximizable(value) => {
                        runner.set_window_maximizable(handle, value)
                    }
                    WindowAction::SetClosable(value) => runner.set_window_closable(handle, value),
                    WindowAction::SetDecorated(value) => runner.set_window_decorated(handle, value),
                    WindowAction::SetShadow(value) => runner.set_window_shadow(handle, value),
                    WindowAction::SetContentProtected(value) => {
                        runner.set_window_content_protected(handle, value)
                    }
                    WindowAction::SetWindowLevel(value) => runner.set_window_level(handle, value),
                    WindowAction::SetFocusable(value) => runner.set_window_focusable(handle, value),
                    WindowAction::SetSkipTaskbar(value) => {
                        runner.set_window_skip_taskbar(handle, value)
                    }
                    WindowAction::SetVisibleOnAllWorkspaces(value) => {
                        runner.set_window_visible_on_all_workspaces(handle, value)
                    }
                    WindowAction::SetOpacity(value) => runner.set_window_opacity(handle, value),
                    WindowAction::SetIcon(value) => runner.set_window_icon(handle, value),
                    WindowAction::SetCursorVisible(value) => {
                        runner.set_cursor_visible(handle, value)
                    }
                    WindowAction::SetCursorGrab(value) => runner.set_cursor_grab(handle, value),
                    WindowAction::SetCursorHitTest(value) => {
                        runner.set_cursor_hit_test(handle, value)
                    }
                    WindowAction::SetCursorPosition(value) => {
                        runner.set_cursor_position(handle, value)
                    }
                    WindowAction::SetTaskbarProgress(state, progress) => {
                        runner.set_taskbar_progress(handle, state, progress)
                    }
                    WindowAction::SetTaskbarOverlayIcon(icon, description) => {
                        runner.set_taskbar_overlay_icon(handle, icon, description)
                    }
                    WindowAction::ClearTaskbarOverlayIcon => {
                        runner.clear_taskbar_overlay_icon(handle)
                    }
                    WindowAction::Focus => runner.focus_window(handle),
                    WindowAction::RequestAttention => runner.request_window_attention(handle),
                    WindowAction::SetRepresentedFile(path) => {
                        runner.set_window_represented_file(handle, path)
                    }
                    WindowAction::SetDocumentEdited(edited) => {
                        runner.set_window_document_edited(handle, edited)
                    }
                    WindowAction::SetAppearance(appearance) => {
                        runner.set_window_appearance(handle, appearance)
                    }
                    WindowAction::SetBackgroundAppearance(appearance) => {
                        runner.set_window_background_appearance(handle, appearance)
                    }
                    WindowAction::SetMacOsVibrancy(vibrancy) => {
                        runner.set_macos_window_vibrancy(handle, vibrancy)
                    }
                    WindowAction::SetMacOsVisualEffectState(state) => {
                        runner.set_macos_visual_effect_state(handle, state)
                    }
                    WindowAction::ShowCharacterPalette => runner.show_character_palette(handle),
                    WindowAction::SetTabbingIdentifier(identifier) => {
                        runner.set_window_tabbing_identifier(handle, identifier)
                    }
                    WindowAction::SelectNextTab => runner.select_next_window_tab(handle),
                    WindowAction::SelectPreviousTab => runner.select_previous_window_tab(handle),
                    WindowAction::SelectTab(index) => {
                        runner.select_window_tab(handle, index as usize)
                    }
                    WindowAction::MergeAllWindows => runner.merge_all_windows(handle),
                    WindowAction::MoveTabToNewWindow => {
                        runner.move_window_tab_to_new_window(handle)
                    }
                    WindowAction::ToggleTabBar => runner.toggle_window_tab_bar(handle),
                    WindowAction::ToggleTabOverview => runner.toggle_window_tab_overview(handle),
                    WindowAction::MoveTop => runner.move_window_to_top(handle),
                    WindowAction::SetIgnoreMouseEvents(ignore, forward) => {
                        runner.set_window_ignore_mouse_events(handle, ignore, forward)
                    }
                    WindowAction::SetWindowEnabled(enabled) => {
                        runner.set_window_enabled(handle, enabled)
                    }
                    WindowAction::SetAspectRatio(ratio) => {
                        runner.set_window_aspect_ratio(handle, ratio)
                    }
                    WindowAction::SetWindowButtonVisibility(visible) => {
                        runner.set_window_button_visibility(handle, visible)
                    }
                    WindowAction::SetAlwaysOnTop(flag, level) => {
                        runner.set_window_always_on_top(handle, flag, level)
                    }
                    // Handled before the runner borrow because they never reach the core, or need
                    // a second window handle or a parsed menu declaration first.
                    WindowAction::SetCloseInterception(_)
                    | WindowAction::MoveAbove(_)
                    | WindowAction::SetMenu(_)
                    | WindowAction::SetResizePolicy(_)
                    | WindowAction::SetMovePolicy(_) => Ok(()),
                }
                .map_err(|error| error.to_string())?;
                Ok(SystemCommandResult::Unit)
            }
            SystemCommand::ShellAction { request, action } => {
                if request == 0
                    || self
                        .pending_shell
                        .iter()
                        .any(|pending| pending.request() == request)
                {
                    return Err("native shell request ids must be nonzero and unique".to_owned());
                }
                let runner = self.running_runner_mut()?;
                let response = match action {
                    ShellAction::OpenExternal(url) => runner.open_external(url),
                    ShellAction::OpenPath(path) => runner.open_path(path),
                    ShellAction::RevealPath(path) => runner.reveal_path(path),
                    ShellAction::TrashPath(path) => runner.trash_path(path),
                }
                .map_err(|error| error.to_string())?;
                self.pending_shell
                    .push(PendingShell::new(request, response));
                Ok(SystemCommandResult::Unit)
            }
        }
    }

    pub(crate) fn observe_system_state(&mut self) {
        let Some(runner) = self.runner.as_ref() else {
            return;
        };
        let displays = runner.displays();
        let preferences = runner.system_preferences();
        let window_states = self
            .windows
            .iter()
            .filter_map(|(id, window)| {
                let handle = window.handle?;
                runner.window_state(handle).map(|state| (*id, state))
            })
            .collect::<HashMap<_, _>>();

        if let Some(previous) = &self.system_observation.displays
            && previous != &displays
        {
            crate::enqueue_event(
                &self.events,
                QueuedEvent {
                    kind: "screen-change",
                    window: 0,
                    target: ROOT_NODE,
                    value: None,
                },
            );
        }

        if self
            .system_observation
            .preferences
            .is_some_and(|previous| previous != preferences)
        {
            crate::enqueue_event(
                &self.events,
                QueuedEvent {
                    kind: "system-preferences-change",
                    window: 0,
                    target: ROOT_NODE,
                    value: None,
                },
            );
        }

        for (window, state) in &window_states {
            let Some(previous) = self.system_observation.windows.get(window) else {
                continue;
            };
            if previous != state {
                crate::enqueue_event(
                    &self.events,
                    QueuedEvent {
                        kind: "window-state-change",
                        window: *window,
                        target: ROOT_NODE,
                        value: None,
                    },
                );
            }
            if previous.appearance != state.appearance {
                let value: Arc<str> = match state.appearance {
                    WindowAppearance::Light => Arc::from("light"),
                    WindowAppearance::Dark => Arc::from("dark"),
                };
                crate::enqueue_event(
                    &self.events,
                    QueuedEvent {
                        kind: "appearance-change",
                        window: *window,
                        target: ROOT_NODE,
                        value: Some(value),
                    },
                );
            }
        }

        self.system_observation.displays = Some(displays);
        self.system_observation.windows = window_states;
        self.system_observation.preferences = Some(preferences);
    }

    fn running_runner(&self) -> std::result::Result<&quickgui::AppRunner, String> {
        self.runner
            .as_ref()
            .ok_or_else(|| "the system API requires a running QuickGUI application".to_owned())
    }

    fn running_runner_mut(&mut self) -> std::result::Result<&mut quickgui::AppRunner, String> {
        self.runner
            .as_mut()
            .ok_or_else(|| "the system API requires a running QuickGUI application".to_owned())
    }

    fn system_window_handle(
        &self,
        window: u32,
    ) -> std::result::Result<quickgui::WindowHandle, String> {
        self.windows
            .get(&window)
            .ok_or_else(|| format!("unknown QuickGUI window {window}"))?
            .handle
            .ok_or_else(|| format!("native window {window} is not mounted"))
    }
}

impl From<&Display> for NativeDisplay {
    fn from(display: &Display) -> Self {
        Self {
            id: display.id().get().to_string(),
            uuid: display.uuid().map(|uuid| uuid.to_string()),
            name: display.name().to_owned(),
            bounds: display.bounds().into(),
            work_area: display.visible_bounds().into(),
            scale_factor: f64::from(display.scale_factor()),
            refresh_rate: display
                .refresh_rate_millihertz()
                .map(|rate| f64::from(rate) / 1_000.0),
            primary: display.is_primary(),
        }
    }
}

impl From<quickgui::Rect> for NativeRect {
    fn from(rect: quickgui::Rect) -> Self {
        Self {
            x: f64::from(rect.x),
            y: f64::from(rect.y),
            width: f64::from(rect.width),
            height: f64::from(rect.height),
        }
    }
}

impl From<&KeyboardLayout> for NativeKeyboardLayout {
    fn from(layout: &KeyboardLayout) -> Self {
        Self {
            id: layout.id().to_owned(),
            name: layout.name().to_owned(),
        }
    }
}

impl From<Point> for NativePoint {
    fn from(point: Point) -> Self {
        Self {
            x: f64::from(point.x),
            y: f64::from(point.y),
        }
    }
}

impl From<&AppInfo> for NativeAppInfo {
    fn from(info: &AppInfo) -> Self {
        Self {
            name: info.name().to_owned(),
            version: info.version().to_owned(),
            identifier: info.identifier().to_owned(),
        }
    }
}

impl From<&AppPaths> for NativeAppPaths {
    fn from(paths: &AppPaths) -> Self {
        let optional =
            |path: Option<&std::path::Path>| path.map(|path| path.to_string_lossy().into_owned());
        Self {
            executable: paths.executable().to_string_lossy().into_owned(),
            executable_dir: paths.executable_dir().to_string_lossy().into_owned(),
            resource_dir: paths.resource_dir().to_string_lossy().into_owned(),
            home_dir: optional(paths.home_dir()),
            config_dir: optional(paths.config_dir()),
            data_dir: optional(paths.data_dir()),
            local_data_dir: optional(paths.local_data_dir()),
            cache_dir: optional(paths.cache_dir()),
            log_dir: optional(paths.log_dir()),
            runtime_dir: optional(paths.runtime_dir()),
            temp_dir: paths.temp_dir().to_string_lossy().into_owned(),
            audio_dir: optional(paths.audio_dir()),
            desktop_dir: optional(paths.desktop_dir()),
            document_dir: optional(paths.document_dir()),
            download_dir: optional(paths.download_dir()),
            picture_dir: optional(paths.picture_dir()),
            video_dir: optional(paths.video_dir()),
        }
    }
}

impl From<&SystemInfo> for NativeSystemInfo {
    fn from(info: &SystemInfo) -> Self {
        Self {
            operating_system: info.operating_system().as_str().to_owned(),
            family: info.family().as_str().to_owned(),
            name: info.name().to_owned(),
            version: info.version().map(str::to_owned),
            edition: info.edition().map(str::to_owned),
            codename: info.codename().map(str::to_owned),
            architecture: info.architecture().to_owned(),
            bitness: match info.bitness() {
                quickgui::SystemBitness::X32 => "32",
                quickgui::SystemBitness::X64 => "64",
                quickgui::SystemBitness::Unknown => "unknown",
            }
            .to_owned(),
            hostname: info.hostname().map(str::to_owned),
            locale: info.locale().map(str::to_owned),
            preferred_languages: info
                .preferred_languages()
                .iter()
                .map(ToString::to_string)
                .collect(),
            languages_truncated: info.languages_truncated(),
        }
    }
}

impl From<DesktopIntegrationSupport> for NativeDesktopIntegrationSupport {
    fn from(support: DesktopIntegrationSupport) -> Self {
        Self {
            system_notifications: support.system_notifications,
            scheduled_notifications: support.scheduled_notifications,
            notification_replies: support.notification_replies,
            native_application_menus: support.native_application_menus,
            native_popup_menus: support.native_popup_menus,
            tray_icons: support.tray_icons,
            programmable_tray_popup: support.programmable_tray_popup,
            global_shortcuts: support.global_shortcuts,
            single_instance: support.single_instance,
            dynamic_protocol_registration: support.dynamic_protocol_registration,
            autostart: support.autostart,
            window_icons: support.window_icons,
            window_focusability: support.window_focusability,
            window_opacity: support.window_opacity,
            skip_taskbar: support.skip_taskbar,
            visible_on_all_workspaces: support.visible_on_all_workspaces,
            cursor_control: support.cursor_control,
            cursor_screen_position: support.cursor_screen_position,
            taskbar_progress: support.taskbar_progress,
            taskbar_overlay_icons: support.taskbar_overlay_icons,
            dock_badges: support.dock_badges,
            dock_icons: support.dock_icons,
            dock_menus: support.dock_menus,
            recent_documents: support.recent_documents,
            file_icons: support.file_icons,
            native_about_panel: support.native_about_panel,
            user_tasks: support.user_tasks,
        }
    }
}

impl From<PowerState> for NativePowerState {
    fn from(state: PowerState) -> Self {
        Self {
            source: match state.source() {
                quickgui::PowerSource::Ac => "ac",
                quickgui::PowerSource::Battery => "battery",
                quickgui::PowerSource::Unknown => "unknown",
            }
            .to_owned(),
            battery: state.battery().map(|battery| NativeBatteryState {
                charge_percent: battery.charge_percent().map(u32::from),
                status: match battery.status() {
                    quickgui::BatteryStatus::Charging => "charging",
                    quickgui::BatteryStatus::Discharging => "discharging",
                    quickgui::BatteryStatus::Full => "full",
                    quickgui::BatteryStatus::NotCharging => "not-charging",
                    quickgui::BatteryStatus::Unknown => "unknown",
                }
                .to_owned(),
            }),
            thermal_state: thermal_state_name(state.thermal_state()).to_owned(),
            low_power_mode: state.low_power_mode(),
            cpu_speed_limit_percent: state.cpu_speed_limit_percent().map(u32::from),
        }
    }
}
