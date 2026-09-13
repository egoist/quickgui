use super::*;

impl TestAppContext {
    pub(super) fn invoke_action(
        &mut self,
        window: WindowHandle,
        action: &AnyAction,
    ) -> Result<bool, TestAppError> {
        let path = self.window(window)?.ui.focus_path();
        let mut dispatch = std::mem::take(&mut self.window_mut(window)?.action_dispatch_scratch);
        self.window(window)?
            .ui
            .collect_action_dispatch(&path, action.type_id(), &mut dispatch);
        for binding in dispatch.iter().copied() {
            let listener = self.window(window)?.listeners.action_listener(binding.key);
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context(Some(window));
            listener(
                self.window_mut(window)?.view.as_any_mut(),
                action.as_any(),
                &mut cx,
            );
            let propagate = cx.propagate_action;
            let stopped = cx.stop_event_propagation;
            self.apply_context(Some(window), cx)?;
            let consumed = match binding.phase {
                crate::DispatchPhase::Capture => stopped,
                crate::DispatchPhase::Bubble => !propagate,
            };
            if consumed {
                dispatch.clear();
                if let Ok(state) = self.window_mut(window) {
                    state.action_dispatch_scratch = dispatch;
                }
                return Ok(true);
            }
        }
        dispatch.clear();
        if let Ok(state) = self.window_mut(window) {
            state.action_dispatch_scratch = dispatch;
        }
        self.invoke_application_action(Some(window), action)
    }

    pub(super) fn invoke_application_action(
        &mut self,
        window: Option<WindowHandle>,
        action: &AnyAction,
    ) -> Result<bool, TestAppError> {
        if !self
            .application_callbacks
            .actions
            .contains_key(&action.type_id())
        {
            return Ok(false);
        }
        let mut cx = self.event_context(window);
        let Some(consumed) = self.application_callbacks.dispatch_action(action, &mut cx) else {
            return Ok(false);
        };
        self.apply_context(window, cx)?;
        Ok(consumed)
    }

    pub(super) fn invoke_key_event(
        &mut self,
        window: WindowHandle,
        event: KeyListenerEvent,
    ) -> Result<bool, TestAppError> {
        let path = self.window(window)?.ui.focus_path();
        let mut dispatch = std::mem::take(&mut self.window_mut(window)?.key_dispatch_scratch);
        self.window(window)?
            .ui
            .collect_key_dispatch(&path, event.kind(), &mut dispatch);
        let mut default_prevented = false;
        for binding in dispatch.iter().copied() {
            let listener = self.window(window)?.listeners.key_listener(binding.key);
            let Some(listener) = listener else {
                continue;
            };
            let mut cx = self.event_context(Some(window));
            listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
            let stopped = cx.stop_event_propagation;
            default_prevented |= cx.prevent_default;
            self.apply_context(Some(window), cx)?;
            if stopped {
                break;
            }
        }
        dispatch.clear();
        if let Ok(state) = self.window_mut(window) {
            state.key_dispatch_scratch = dispatch;
        }
        Ok(default_prevented)
    }

    pub(super) fn invoke_form_submission(
        &mut self,
        window: WindowHandle,
        form: ElementId,
        trigger: Option<ElementId>,
    ) -> Result<(), TestAppError> {
        if self.form_submission_depth >= MAX_NESTED_FORM_SUBMISSIONS {
            return Ok(());
        }
        self.form_submission_depth += 1;
        let result = (|| {
            let previous_focus = self.window(window)?.ui.focused();
            let attempt = self
                .window_mut(window)?
                .ui
                .attempt_form_submission(form, trigger);
            if let Some(attempt) = attempt {
                self.window_mut(window)?.dirty = true;
                self.focus_changed(window, previous_focus)?;
                let mut cx = self.event_context(Some(window));
                match attempt {
                    FormAttempt::Valid(event) => {
                        let listener = self
                            .window(window)?
                            .listeners
                            .form_submits
                            .get(&form)
                            .cloned();
                        if let Some(listener) = listener {
                            listener(self.window_mut(window)?.view.as_any_mut(), &event, &mut cx);
                        }
                    }
                    FormAttempt::Invalid(report) => {
                        let listener = self
                            .window(window)?
                            .listeners
                            .form_invalids
                            .get(&form)
                            .cloned();
                        if let Some(listener) = listener {
                            listener(self.window_mut(window)?.view.as_any_mut(), &report, &mut cx);
                        }
                    }
                }
                self.apply_context(Some(window), cx)?;
            }
            Ok(())
        })();
        self.form_submission_depth -= 1;
        result
    }

    pub(super) fn apply_input_result(
        &mut self,
        window: WindowHandle,
        result: InputResult,
    ) -> Result<(), TestAppError> {
        let Some(change) = result.change else {
            return Ok(());
        };
        let listener = self
            .window(window)?
            .listeners
            .inputs
            .get(&change.id)
            .cloned();
        if let Some(listener) = listener {
            let mut cx = self.event_context(Some(window));
            listener(
                self.window_mut(window)?.view.as_any_mut(),
                &change.value,
                &mut cx,
            );
            self.apply_context(Some(window), cx)?;
        }
        Ok(())
    }

    pub(super) fn dispatch_bindings(
        &mut self,
        window: WindowHandle,
        bindings: &[KeyBinding],
    ) -> Result<bool, TestAppError> {
        for binding in bindings {
            if self.invoke_action(window, binding.action())? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn handle_keystroke_fallback(
        &mut self,
        window: WindowHandle,
        stroke: Keystroke,
    ) -> Result<(), TestAppError> {
        let modifiers = stroke.modifiers;
        let default_prevented = self.invoke_key_event(
            window,
            KeyListenerEvent::Down(KeyDownEvent {
                key: stroke.key.clone(),
                key_char: stroke.key_char.clone(),
                text: match &stroke.key {
                    Key::Character(value) => Some(value.clone()),
                    Key::Space => Some(" ".to_owned()),
                    _ => None,
                },
                modifiers,
                repeat: false,
            }),
        )?;
        if !default_prevented {
            let input_focused = self.window(window)?.ui.focused_text_input().is_some();
            let mut input_result = None;
            if input_focused {
                let extend = modifiers.contains(Modifiers::SHIFT);
                let primary = primary_modifier(modifiers);
                input_result = match &stroke.key {
                    Key::Character(value)
                        if primary
                            && modifiers.contains(Modifiers::SHIFT)
                            && value.eq_ignore_ascii_case("z") =>
                    {
                        Some(self.window_mut(window)?.ui.input_redo())
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("z") => {
                        Some(self.window_mut(window)?.ui.input_undo())
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("y") => {
                        Some(self.window_mut(window)?.ui.input_redo())
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("a") => {
                        Some(self.window_mut(window)?.ui.input_select_all())
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("c") => {
                        let selected = self.window(window)?.ui.selected_input_text();
                        if let Some(selected) = selected
                            && let Ok(item) = ClipboardItem::new_string(selected)
                        {
                            let _ = self.clipboard.write(ClipboardTarget::General, item);
                        }
                        None
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("x") => {
                        let selected = self.window(window)?.ui.selected_input_text();
                        let copied = selected.is_some_and(|selected| {
                            ClipboardItem::new_string(selected)
                                .and_then(|item| {
                                    self.clipboard.write(ClipboardTarget::General, item)
                                })
                                .is_ok()
                        });
                        if copied {
                            Some(self.window_mut(window)?.ui.input_backspace())
                        } else {
                            None
                        }
                    }
                    Key::Character(value) if primary && value.eq_ignore_ascii_case("v") => {
                        let pasted = self
                            .clipboard
                            .read(ClipboardTarget::General)
                            .ok()
                            .flatten()
                            .and_then(|item| item.text());
                        if let Some(text) = pasted {
                            Some(self.window_mut(window)?.ui.input_replace(&text))
                        } else {
                            None
                        }
                    }
                    _ if !modifiers.intersects(Modifiers::CONTROL | Modifiers::SUPER)
                        && matches!(stroke.key_char.as_ref(), Some(Key::Character(_))) =>
                    {
                        let Some(Key::Character(value)) = stroke.key_char.as_ref() else {
                            unreachable!();
                        };
                        Some(self.window_mut(window)?.ui.input_replace(value))
                    }
                    Key::Character(value)
                        if !modifiers.intersects(Modifiers::CONTROL | Modifiers::SUPER) =>
                    {
                        Some(self.window_mut(window)?.ui.input_replace(value))
                    }
                    Key::Space if !modifiers.intersects(Modifiers::CONTROL | Modifiers::SUPER) => {
                        Some(self.window_mut(window)?.ui.input_replace(" "))
                    }
                    Key::Backspace => Some(self.window_mut(window)?.ui.input_backspace()),
                    Key::Delete => Some(self.window_mut(window)?.ui.input_delete()),
                    Key::ArrowLeft => Some(self.window_mut(window)?.ui.input_move_left(extend)),
                    Key::ArrowRight => Some(self.window_mut(window)?.ui.input_move_right(extend)),
                    Key::Home => Some(self.window_mut(window)?.ui.input_move_home(extend)),
                    Key::End => Some(self.window_mut(window)?.ui.input_move_end(extend)),
                    Key::Enter
                        if self.window(window)?.ui.focused_text_input_is_multiline()
                            && (extend
                                || !self
                                    .window(window)?
                                    .ui
                                    .focused_text_input_submits_on_enter()) =>
                    {
                        Some(self.window_mut(window)?.ui.input_insert_newline())
                    }
                    _ => None,
                };
            }
            let submitted = matches!(&stroke.key, Key::Enter)
                && input_focused
                && (!self.window(window)?.ui.focused_text_input_is_multiline()
                    || (!modifiers.contains(Modifiers::SHIFT)
                        && self
                            .window(window)?
                            .ui
                            .focused_text_input_submits_on_enter()))
                && self.submit_focused_input(window)?;
            if !submitted {
                if let Some(result) = input_result {
                    self.apply_input_result(window, result)?;
                } else {
                    match &stroke.key {
                        Key::Tab => {
                            let previous = self.window(window)?.ui.focused();
                            self.window_mut(window)?
                                .ui
                                .focus_next(modifiers.contains(Modifiers::SHIFT));
                            self.focus_changed(window, previous)?;
                        }
                        Key::ArrowLeft if modifiers.is_empty() => {
                            if !self.navigate_adjacent_tab(window, false, true)?
                                && let Some(target) = self.window(window)?.ui.adjacent_radio(true)
                            {
                                self.click(window, target)?;
                            }
                        }
                        Key::ArrowRight if modifiers.is_empty() => {
                            if !self.navigate_adjacent_tab(window, false, false)?
                                && let Some(target) = self.window(window)?.ui.adjacent_radio(false)
                            {
                                self.click(window, target)?;
                            }
                        }
                        Key::ArrowUp if modifiers.is_empty() => {
                            if !self.navigate_adjacent_tab(window, true, true)?
                                && let Some(target) = self.window(window)?.ui.adjacent_radio(true)
                            {
                                self.click(window, target)?;
                            }
                        }
                        Key::ArrowDown if modifiers.is_empty() => {
                            if !self.navigate_adjacent_tab(window, true, false)?
                                && let Some(target) = self.window(window)?.ui.adjacent_radio(false)
                            {
                                self.click(window, target)?;
                            }
                        }
                        Key::Home if modifiers.is_empty() => {
                            self.navigate_edge_tab(window, false)?;
                        }
                        Key::End if modifiers.is_empty() => {
                            self.navigate_edge_tab(window, true)?;
                        }
                        Key::Escape => {
                            // Production always has a painted dismissal stack before native key
                            // input can arrive. Semantic tests normally skip paint, so materialize
                            // the same retained geometry lazily before querying the top surface.
                            self.prepare_retained_geometry(window)?;
                            if let Some(request) = self.window(window)?.ui.dismiss_topmost() {
                                self.dismiss(window, request)?;
                            }
                        }
                        Key::Enter | Key::Space => {
                            if let Some(element) = self.window(window)?.ui.activate_focused() {
                                self.click(window, element)?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        self.queue_dispatch(TestDispatch::Event(
            window,
            Event::KeyDown {
                key: stroke.key,
                key_char: stroke.key_char,
                modifiers,
                repeat: false,
            },
        ))?;
        Ok(())
    }

    pub(super) fn dismiss(
        &mut self,
        window: WindowHandle,
        request: DismissRequest,
    ) -> Result<(), TestAppError> {
        let listener = self
            .window(window)?
            .listeners
            .dismisses
            .get(&request.id)
            .cloned();
        let mut cx = self.event_context(Some(window));
        if let Some(focus) = request.restore_focus {
            cx.focus = Some(Some(focus));
        }
        if let Some(listener) = listener {
            listener(self.window_mut(window)?.view.as_any_mut(), &mut cx);
        }
        self.apply_context(Some(window), cx)?;
        self.queue_dispatch(TestDispatch::Event(window, Event::Dismiss(request.id)))
    }

    pub(super) fn submit_focused_input(
        &mut self,
        window: WindowHandle,
    ) -> Result<bool, TestAppError> {
        let Some(id) = self.window(window)?.ui.focused_text_input() else {
            return Ok(false);
        };
        let value = self
            .window(window)?
            .ui
            .focused_text_input_value()
            .unwrap_or_else(|| Arc::from(""));
        let invalid = self.window(window)?.ui.focused_text_input_is_invalid();
        if let Some(form) = self.window(window)?.ui.form_for_control(id) {
            self.invoke_form_submission(window, form, Some(id))?;
            return Ok(true);
        }
        let listener = self.window(window)?.listeners.submits.get(&id).cloned();
        let Some(listener) = listener else {
            return Ok(false);
        };
        if !invalid {
            let mut cx = self.event_context(Some(window));
            listener(self.window_mut(window)?.view.as_any_mut(), &value, &mut cx);
            self.apply_context(Some(window), cx)?;
        }
        Ok(true)
    }

    pub(super) fn focus_changed(
        &mut self,
        window: WindowHandle,
        previous: Option<ElementId>,
    ) -> Result<(), TestAppError> {
        let focused = self.window(window)?.ui.focused();
        if focused != previous {
            self.window_mut(window)?.dirty = true;
            self.queue_immediate([TestDispatch::Event(window, Event::FocusChanged(focused))])?;
        }
        Ok(())
    }

    pub(super) fn navigate_adjacent_tab(
        &mut self,
        window: WindowHandle,
        vertical_axis: bool,
        reverse: bool,
    ) -> Result<bool, TestAppError> {
        let target = self.window(window)?.ui.adjacent_tab(vertical_axis, reverse);
        self.apply_tab_navigation(window, target)
    }

    pub(super) fn navigate_edge_tab(
        &mut self,
        window: WindowHandle,
        last: bool,
    ) -> Result<bool, TestAppError> {
        let target = self.window(window)?.ui.edge_tab(last);
        self.apply_tab_navigation(window, target)
    }

    pub(super) fn apply_tab_navigation(
        &mut self,
        window: WindowHandle,
        target: Option<TabNavigationTarget>,
    ) -> Result<bool, TestAppError> {
        let Some(target) = target else {
            return Ok(false);
        };
        let previous = self.window(window)?.ui.focused();
        self.window_mut(window)?.ui.focus(target.id);
        self.focus_changed(window, previous)?;
        self.run_until_idle()?;
        if target.activate {
            self.click(window, target.id)?;
        }
        Ok(true)
    }

    pub(super) fn set_active_window(
        &mut self,
        window: Option<WindowHandle>,
    ) -> Result<(), TestAppError> {
        if let Some(window) = window {
            self.window(window)?;
            self.focus_history.retain(|candidate| *candidate != window);
            self.focus_history.push(window);
        }
        self.active_window = window;
        self.refresh_window_focus_states()
    }

    pub(super) fn refresh_window_focus_states(&mut self) -> Result<(), TestAppError> {
        for (handle, state) in &mut self.windows {
            let focused = Some(*handle) == self.active_window;
            if state.state.focused != focused {
                state.state.focused = focused;
                if state.listeners.observes_window_state {
                    state.dirty = true;
                }
            }
        }
        Ok(())
    }
}
