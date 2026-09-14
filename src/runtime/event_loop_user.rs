use super::*;

impl Runtime {
    pub(super) fn handle_user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeEvent) {
        if matches!(&event, RuntimeEvent::ExternalCommandsReady) {
            self.process_window_commands(event_loop);
            return;
        }
        if let RuntimeEvent::InvalidateWindow(handle) = &event {
            self.invalidate_external(*handle);
            return;
        }
        if let RuntimeEvent::InvalidateElement(handle, element) = &event {
            self.invalidate_external_scopes(*handle, &[*element]);
            return;
        }
        if matches!(&event, RuntimeEvent::ForegroundTasksReady) {
            self.process_foreground_tasks(event_loop);
            return;
        }
        // Application menus outlive their windows. Only activate a window when one exists;
        // otherwise typed application handlers still receive Open, Help, and similar commands.
        if matches!(
            &event,
            RuntimeEvent::MenuWillOpen | RuntimeEvent::MenuAction(_)
        ) {
            let target = self.active_window;
            if let Some(target) = target
                && !self.activate_window(target)
            {
                return;
            }
            self.pending_input = None;
            if let RuntimeEvent::MenuAction(action_id) = event {
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                {
                    self.menu_actions = collect_menu_actions(self.active_menu_declaration());
                }
                let item = self
                    .menu_actions
                    .get(action_id)
                    .filter(|item| !item.disabled && !item.hidden)
                    .map(|item| (item.action.clone(), item.os_action));
                if let Some((action, os_action)) = item {
                    let handled = action.as_ref().is_some_and(|action| {
                        self.invoke_action(event_loop, action).unwrap_or(true)
                    });
                    if !handled && let Some(os_action) = os_action {
                        self.invoke_os_action(event_loop, os_action);
                    }
                }
            }
            #[cfg(target_os = "macos")]
            self.sync_native_menu_state();
            if target.is_some() {
                self.deactivate_window();
            }
            self.process_window_commands(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::DockMenuAction(action_id) = &event {
            self.invoke_dock_menu_action(event_loop, *action_id);
            return;
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let RuntimeEvent::NativePopupMenuClosed(popup_id) = &event {
            if let Some(responder) = self
                .pending_native_popup_menus
                .remove(popup_id)
                .and_then(|popup| popup.responder)
            {
                responder.complete(Ok(()));
            }
            return;
        }
        if let RuntimeEvent::OpenUrls(urls) = &event {
            self.invoke_open_urls(event_loop, urls.clone());
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::Reopen {
            has_visible_windows,
        } = &event
        {
            self.invoke_reopen(event_loop, *has_visible_windows);
            return;
        }
        #[cfg(target_os = "macos")]
        if matches!(&event, RuntimeEvent::QuitRequested) {
            self.native_termination_pending = true;
            self.pending_quit = Some(QuitReason::OperatingSystem);
            self.process_window_commands(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if matches!(&event, RuntimeEvent::SystemWake) {
            self.invoke_system_wake(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::NativePanel(panel_event) = event {
            self.invoke_native_panel_event(event_loop, panel_event);
            return;
        }
        #[cfg(target_os = "macos")]
        if matches!(&event, RuntimeEvent::DisplaysChanged) {
            self.refresh_displays(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if matches!(&event, RuntimeEvent::ApplicationActivated) {
            self.invoke_did_become_active(event_loop);
            self.process_window_commands(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if matches!(&event, RuntimeEvent::ApplicationDeactivated) {
            let popovers = self.popovers_to_close_after_application_deactivation();
            self.close_requests.extend(popovers);
            self.invoke_did_resign_active(event_loop);
            self.process_window_commands(event_loop);
            return;
        }
        #[cfg(target_os = "macos")]
        if matches!(&event, RuntimeEvent::KeyboardLayoutChanged) {
            if self.refresh_keyboard_layout() {
                self.invoke_keyboard_layout_change(event_loop);
                self.sync_active_native_menu_state();
            }
            return;
        }
        if let RuntimeEvent::SystemPreferencesChanged(preferences) = &event {
            self.refresh_system_preferences(*preferences);
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::SystemNotificationAuthorization { granted, error } = &event {
            if let Some(host) = self.mac_application_host.as_mut() {
                host.complete_system_notification_authorization(*granted, error.clone());
            }
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::SystemNotificationPermissionStatus(status) = &event {
            if let Some(host) = self.mac_application_host.as_mut() {
                host.complete_system_notification_permission_status(*status);
            }
            return;
        }
        if let RuntimeEvent::SystemNotificationResponse(response) = &event {
            self.invoke_system_notification_response(event_loop, response.clone());
            return;
        }
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        if let RuntimeEvent::GlobalShortcut(hotkey_id) = &event {
            self.invoke_global_shortcut(event_loop, *hotkey_id);
            return;
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        if let RuntimeEvent::SecondInstance(second_instance) = &event {
            self.invoke_second_instance(event_loop, second_instance.clone());
            return;
        }
        if let RuntimeEvent::Power(power_event) = &event {
            self.invoke_power_event(event_loop, *power_event);
            return;
        }
        if let RuntimeEvent::Tray(tray_event) = &event {
            self.invoke_tray_event(event_loop, tray_event.clone());
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::PopoverPointerDismissRequested(handle) = &event {
            let should_close = self
                .window_handles
                .get(handle)
                .and_then(|window_id| self.windows.get(window_id))
                .is_some_and(|entry| {
                    entry.state.visible
                        && window_dismisses_system_popover_on_pointer_outside(&entry.config)
                });
            if should_close {
                self.close_requests.push(*handle);
                self.process_window_commands(event_loop);
            }
            return;
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        if let RuntimeEvent::PlatformDialogClosed(owner, id) = &event {
            if self
                .active_platform_dialogs
                .get(owner)
                .is_some_and(|dialog| dialog.id == *id)
                && let Some(dialog) = self.active_platform_dialogs.remove(owner)
            {
                #[cfg(target_os = "macos")]
                if let Some(focus) = dialog.focus
                    && !focus.restore()
                {
                    tracing::warn!("AppKit rejected the platform dialog's saved first responder");
                }
                #[cfg(not(target_os = "macos"))]
                drop(dialog);
            }
            return;
        }
        #[cfg(target_os = "macos")]
        if let RuntimeEvent::PlatformDialogCancelled(owner, id) = &event {
            if self
                .active_platform_dialogs
                .get(owner)
                .is_some_and(|dialog| dialog.id == *id)
                && let Some(dialog) = self.active_platform_dialogs.get(owner)
            {
                dialog.native.cancel();
            }
            return;
        }
        let released_image_capacity = matches!(&event, RuntimeEvent::ImageLoaded(_, _));
        let target = match &event {
            RuntimeEvent::ExternalCommandsReady => unreachable!("handled before target routing"),
            RuntimeEvent::InvalidateWindow(_) | RuntimeEvent::InvalidateElement(_, _) => {
                unreachable!("handled before target routing")
            }
            RuntimeEvent::Accessibility(event) => Some(event.window_id),
            RuntimeEvent::ImageLoaded(handle, _) => self.window_handles.get(handle).copied(),
            RuntimeEvent::BackgroundCompleted(completion) => {
                self.window_handles.get(&completion.window).copied()
            }
            RuntimeEvent::ForegroundTasksReady => unreachable!("handled before target routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::ExternalDragBoundary(handle, _) => {
                self.window_handles.get(handle).copied()
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::ExternalDragEnded(handle, _) => self.window_handles.get(handle).copied(),
            #[cfg(target_os = "macos")]
            RuntimeEvent::NativeDropChanged(handle) => self.window_handles.get(handle).copied(),
            #[cfg(target_os = "macos")]
            RuntimeEvent::ApplicationActivated | RuntimeEvent::ApplicationDeactivated => {
                unreachable!("handled before routing")
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::PopoverPointerDismissRequested(_) => {
                unreachable!("handled before routing")
            }
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            RuntimeEvent::PlatformDialogClosed(_, _) => unreachable!("handled before routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::PlatformDialogCancelled(_, _) => unreachable!("handled before routing"),
            RuntimeEvent::SystemNotificationResponse(_) => {
                unreachable!("handled before window routing")
            }
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            RuntimeEvent::GlobalShortcut(_) => unreachable!("handled before window routing"),
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            RuntimeEvent::SecondInstance(_) => unreachable!("handled before window routing"),
            RuntimeEvent::SystemPreferencesChanged(_) => {
                unreachable!("handled before window routing")
            }
            RuntimeEvent::Power(_) => unreachable!("handled before window routing"),
            RuntimeEvent::Tray(_) => unreachable!("handled before window routing"),
            RuntimeEvent::OpenUrls(_) => unreachable!("handled before window routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::Reopen { .. }
            | RuntimeEvent::QuitRequested
            | RuntimeEvent::SystemWake
            | RuntimeEvent::DisplaysChanged
            | RuntimeEvent::KeyboardLayoutChanged
            | RuntimeEvent::NativePanel(_)
            | RuntimeEvent::SystemNotificationPermissionStatus(_)
            | RuntimeEvent::SystemNotificationAuthorization { .. } => {
                unreachable!("handled before window routing")
            }
            RuntimeEvent::MenuWillOpen | RuntimeEvent::MenuAction(_) => {
                unreachable!("handled before window routing")
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::DockMenuAction(_) => unreachable!("handled before window routing"),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            RuntimeEvent::NativePopupMenuAction(popup_id, _) => self
                .pending_native_popup_menus
                .get(popup_id)
                .and_then(|popup| self.window_handles.get(&popup.window).copied()),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            RuntimeEvent::NativePopupMenuClosed(_) => {
                unreachable!("handled before window routing")
            }
        };
        let Some(target) = target else {
            return;
        };
        if !self.activate_window(target) {
            return;
        }
        (|| match event {
            RuntimeEvent::ExternalCommandsReady => unreachable!("handled before window routing"),
            RuntimeEvent::InvalidateWindow(_) | RuntimeEvent::InvalidateElement(_, _) => {
                unreachable!("handled before window routing")
            }
            RuntimeEvent::ImageLoaded(_, completion) => {
                let Some(state) = &mut self.window else {
                    return;
                };
                if state.image_assets.complete(completion) {
                    state.view_dirty = true;
                    if state.scheduler.invalidate() {
                        state.window.request_redraw();
                    }
                }
            }
            RuntimeEvent::BackgroundCompleted(completion) => {
                let mut context = self.event_context();
                if let Some(state) = &mut self.window {
                    (completion.callback)(state.view.as_any_mut(), &mut context);
                }
                self.apply_event_context(event_loop, context, false, true);
            }
            RuntimeEvent::ForegroundTasksReady => unreachable!("handled before window routing"),
            RuntimeEvent::MenuWillOpen | RuntimeEvent::MenuAction(_) => {
                unreachable!("handled before window routing")
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::DockMenuAction(_) => unreachable!("handled before window routing"),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            RuntimeEvent::NativePopupMenuAction(popup_id, action_id) => {
                let popup = self.pending_native_popup_menus.remove(&popup_id);
                if let Some(responder) = popup.as_ref().and_then(|popup| popup.responder.clone()) {
                    responder.complete(Ok(()));
                }
                let item = popup
                    .and_then(|popup| popup.actions.into_iter().nth(action_id))
                    .filter(|item| !item.disabled)
                    .map(|item| (item.action, item.os_action));
                if let Some((action, os_action)) = item {
                    let handled = if let Some(action) = action {
                        let Some(handled) = self.invoke_action(event_loop, &action) else {
                            return;
                        };
                        handled
                    } else {
                        false
                    };
                    if !handled && let Some(os_action) = os_action {
                        self.invoke_os_action(event_loop, os_action);
                    }
                    #[cfg(target_os = "macos")]
                    self.sync_native_menu_state();
                }
            }
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            RuntimeEvent::NativePopupMenuClosed(_) => {
                unreachable!("handled before window routing")
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::ExternalDragBoundary(_, point) => {
                let _ = self.promote_external_drag_at_boundary(event_loop, point);
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::ExternalDragEnded(_, operation) => {
                let source = self
                    .window
                    .as_mut()
                    .and_then(|state| state.outbound_external_drag.take().map(|drag| drag.source));
                if let Some(source) = source {
                    self.dispatch(
                        event_loop,
                        Event::ExternalDragEnded(ExternalDragEndEvent { source, operation }),
                        false,
                    );
                }
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::NativeDropChanged(_) => {
                let pending = self
                    .window
                    .as_ref()
                    .and_then(|state| state.native_drop_host.take_pending());
                if let Some(pending) = pending {
                    self.handle_native_drop_pending(event_loop, pending);
                }
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::ApplicationActivated | RuntimeEvent::ApplicationDeactivated => {
                unreachable!("handled before routing")
            }
            #[cfg(target_os = "macos")]
            RuntimeEvent::PopoverPointerDismissRequested(_) => {
                unreachable!("handled before routing")
            }
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            RuntimeEvent::PlatformDialogClosed(_, _) => unreachable!("handled before routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::PlatformDialogCancelled(_, _) => {
                unreachable!("handled before routing")
            }
            RuntimeEvent::SystemNotificationResponse(_) => {
                unreachable!("handled before window routing")
            }
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            RuntimeEvent::GlobalShortcut(_) => unreachable!("handled before window routing"),
            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd"
            ))]
            RuntimeEvent::SecondInstance(_) => unreachable!("handled before window routing"),
            RuntimeEvent::SystemPreferencesChanged(_) => {
                unreachable!("handled before window routing")
            }
            RuntimeEvent::Power(_) => unreachable!("handled before window routing"),
            RuntimeEvent::Tray(_) => unreachable!("handled before window routing"),
            RuntimeEvent::OpenUrls(_) => unreachable!("handled before window routing"),
            #[cfg(target_os = "macos")]
            RuntimeEvent::Reopen { .. }
            | RuntimeEvent::QuitRequested
            | RuntimeEvent::SystemWake
            | RuntimeEvent::DisplaysChanged
            | RuntimeEvent::KeyboardLayoutChanged
            | RuntimeEvent::NativePanel(_)
            | RuntimeEvent::SystemNotificationPermissionStatus(_)
            | RuntimeEvent::SystemNotificationAuthorization { .. } => {
                unreachable!("handled before window routing")
            }
            RuntimeEvent::Accessibility(event) => match event.window_event {
                AccessibilityWindowEvent::InitialTreeRequested => {
                    let window_title = self.config.title.as_str();
                    let window = self.window.as_mut().expect("window checked above");
                    window.accessibility_updates.activate();
                    let RuntimeWindow {
                        accessibility, ui, ..
                    } = window;
                    accessibility.update_if_active(|| ui.accessibility_update(window_title));
                }
                AccessibilityWindowEvent::ActionRequested(ActionRequest {
                    action,
                    target_node,
                    data,
                    ..
                }) => {
                    let target = self
                        .window
                        .as_ref()
                        .and_then(|window| window.ui.accessibility_element(target_node));
                    let Some(target) = target else {
                        return;
                    };
                    let previous_focus =
                        self.window.as_ref().and_then(|window| window.ui.focused());
                    match action {
                        AccessibilityAction::Focus => {
                            if let Some(window) = &mut self.window {
                                window.ui.focus(target);
                            }
                        }
                        AccessibilityAction::Blur => {
                            if let Some(window) = &mut self.window
                                && window.ui.focused() == Some(target)
                            {
                                window.ui.blur();
                            }
                        }
                        AccessibilityAction::Click => {
                            if let Some(window) = &mut self.window {
                                window.ui.focus(target);
                            }
                            self.announce_focus_change(event_loop, previous_focus);
                            self.invoke_click(event_loop, target);
                            return;
                        }
                        AccessibilityAction::SetValue => {
                            let Some(ActionData::Value(value)) = data else {
                                return;
                            };
                            let result = self
                                .window
                                .as_mut()
                                .map(|window| window.ui.input_set_value(target, &value))
                                .unwrap_or_default();
                            self.apply_input_result(event_loop, result, true);
                            return;
                        }
                        AccessibilityAction::SetTextSelection => {
                            let Some(ActionData::SetTextSelection(selection)) = data else {
                                return;
                            };
                            let result = self
                                .window
                                .as_mut()
                                .map(|window| {
                                    window
                                        .ui
                                        .set_accessibility_text_selection(target, &selection)
                                })
                                .unwrap_or_default();
                            self.apply_input_result(event_loop, result, false);
                            return;
                        }
                        _ => return,
                    }
                    self.announce_focus_change(event_loop, previous_focus);
                }
                AccessibilityWindowEvent::AccessibilityDeactivated => {
                    if let Some(window) = &mut self.window {
                        window.accessibility_updates.deactivate();
                    }
                }
            },
        })();
        self.deactivate_window();
        if released_image_capacity {
            self.resume_deferred_image_loads();
        }
        self.process_window_commands(event_loop);
    }
}
