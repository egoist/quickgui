use super::*;

impl Runtime {
    pub(super) fn invoke_dismiss(&mut self, event_loop: &ActiveEventLoop, request: DismissRequest) {
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.dismisses.get(&request.id).cloned());
        let mut cx = self.event_context();
        if let Some(focus) = request.restore_focus {
            cx.focus = Some(Some(focus));
        }
        if let Some(listener) = listener
            && let Some(window) = &mut self.window
        {
            listener(window.view.as_any_mut(), &mut cx);
        }
        if !self.apply_event_context(event_loop, cx, false, true) {
            return;
        }
        self.dispatch(event_loop, Event::Dismiss(request.id), false);
    }

    pub(super) fn invoke_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: ElementId,
        value: &str,
    ) -> bool {
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.inputs.get(&id).cloned());
        if let Some(listener) = listener {
            let mut cx = self.event_context();
            if let Some(window) = &mut self.window {
                listener(window.view.as_any_mut(), value, &mut cx);
            }
            return self.apply_event_context(event_loop, cx, false, true);
        }
        true
    }

    pub(super) fn invoke_form_submission(
        &mut self,
        event_loop: &ActiveEventLoop,
        form: ElementId,
        trigger: Option<ElementId>,
    ) -> bool {
        if self.form_submission_depth >= MAX_NESTED_FORM_SUBMISSIONS {
            tracing::warn!(?form, "nested form-submission limit reached");
            return true;
        }
        self.form_submission_depth += 1;
        let result = self.invoke_form_submission_inner(event_loop, form, trigger);
        self.form_submission_depth -= 1;
        result
    }

    pub(super) fn invoke_form_submission_inner(
        &mut self,
        event_loop: &ActiveEventLoop,
        form: ElementId,
        trigger: Option<ElementId>,
    ) -> bool {
        let previous_focus = self.window.as_ref().and_then(|window| window.ui.focused());
        let attempt = self
            .window
            .as_mut()
            .and_then(|window| window.ui.attempt_form_submission(form, trigger));
        let Some(attempt) = attempt else {
            return true;
        };

        // Every attempt changes the accessibility tree: invalid attempts replace the assertive
        // live node, while valid attempts remove the preceding report. This is one action-driven
        // frame and never creates a continuous animation or polling loop.
        if let Some(window) = &mut self.window
            && window.scheduler.invalidate()
        {
            window.window.request_redraw();
        }
        self.announce_focus_change(event_loop, previous_focus);

        let mut cx = self.event_context();
        match attempt {
            FormAttempt::Valid(event) => {
                let listener = self
                    .window
                    .as_ref()
                    .and_then(|window| window.listeners.form_submits.get(&form).cloned());
                if let (Some(window), Some(listener)) = (&mut self.window, listener) {
                    listener(window.view.as_any_mut(), &event, &mut cx);
                }
            }
            FormAttempt::Invalid(report) => {
                let listener = self
                    .window
                    .as_ref()
                    .and_then(|window| window.listeners.form_invalids.get(&form).cloned());
                if let (Some(window), Some(listener)) = (&mut self.window, listener) {
                    listener(window.view.as_any_mut(), &report, &mut cx);
                }
            }
        }
        self.apply_event_context(event_loop, cx, false, true)
    }

    /// Returns whether the focused input owns a submit listener, including when invalid.
    pub(super) fn submit_focused_input(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let Some((id, value, invalid, form)) = self.window.as_ref().and_then(|window| {
            let id = window.ui.focused_text_input()?;
            let value = window.ui.focused_text_input_value()?;
            Some((
                id,
                value,
                window.ui.focused_text_input_is_invalid(),
                window.ui.form_for_control(id),
            ))
        }) else {
            return false;
        };
        if let Some(form) = form {
            self.invoke_form_submission(event_loop, form, Some(id));
            return true;
        }
        let listener = self
            .window
            .as_ref()
            .and_then(|window| window.listeners.submits.get(&id).cloned());
        let Some(listener) = listener else {
            return false;
        };
        if invalid {
            return true;
        }

        let mut cx = self.event_context();
        if let Some(window) = &mut self.window {
            listener(window.view.as_any_mut(), &value, &mut cx);
        }
        self.apply_event_context(event_loop, cx, false, true);
        true
    }

    pub(super) fn apply_input_result(
        &mut self,
        event_loop: &ActiveEventLoop,
        result: InputResult,
        notify_listener: bool,
    ) -> bool {
        if result.repaint
            && let Some(state) = &mut self.window
            && state.scheduler.invalidate()
        {
            state.window.request_redraw();
        }
        if notify_listener && let Some(change) = result.change {
            return self.invoke_input(event_loop, change.id, &change.value);
        }
        true
    }

    pub(super) fn write_clipboard_text(&self, text: &str) -> bool {
        ClipboardItem::new_string(text)
            .and_then(|item| self.clipboard.write(ClipboardTarget::General, item))
            .is_ok()
    }

    pub(super) fn read_clipboard_text(&self) -> Option<String> {
        self.clipboard
            .read(ClipboardTarget::General)
            .ok()
            .flatten()
            .and_then(|item| item.text())
    }

    pub(super) fn handle_static_text_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: &Key,
        modifiers: Modifiers,
        repeat: bool,
    ) -> bool {
        if self
            .window
            .as_ref()
            .and_then(|window| window.ui.focused_text_input())
            .is_some()
        {
            return false;
        }
        if primary_modifier(modifiers)
            && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("c"))
        {
            return self.invoke_os_action(event_loop, OsAction::Copy);
        }
        if primary_modifier(modifiers)
            && matches!(key, Key::Character(value) if value.eq_ignore_ascii_case("a"))
        {
            return self.invoke_os_action(event_loop, OsAction::SelectAll);
        }
        if !repeat && matches!(key, Key::Escape) {
            let repaint = self
                .window
                .as_mut()
                .is_some_and(|window| window.ui.clear_static_text_selection());
            if repaint
                && let Some(window) = &mut self.window
                && window.scheduler.invalidate()
            {
                window.window.request_redraw();
            }
            return repaint;
        }
        false
    }

    pub(super) fn handle_text_input_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: &Key,
        modifiers: Modifiers,
        repeat: bool,
    ) -> bool {
        let focused = self
            .window
            .as_ref()
            .and_then(|window| window.ui.focused_text_input());
        if focused.is_none() {
            return false;
        }

        let extend = modifiers.contains(Modifiers::SHIFT);
        let primary = primary_modifier(modifiers);
        let word = word_modifier(modifiers) && (!cfg!(target_os = "macos") || !primary);
        let multiline = self
            .window
            .as_ref()
            .is_some_and(|window| window.ui.focused_text_input_is_multiline());
        if matches!(key, Key::Enter)
            && (!multiline
                || (!extend
                    && self
                        .window
                        .as_ref()
                        .is_some_and(|window| window.ui.focused_text_input_submits_on_enter())))
            && !repeat
            && self.submit_focused_input(event_loop)
        {
            return true;
        }
        let result = match key {
            Key::ArrowLeft if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_line_start(extend)),
            Key::ArrowRight if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_line_end(extend)),
            Key::ArrowLeft if word => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_word_left(extend)),
            Key::ArrowRight if word => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_word_right(extend)),
            Key::ArrowLeft => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_left(extend)),
            Key::ArrowRight => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_right(extend)),
            Key::ArrowUp if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_home(extend)),
            Key::ArrowDown if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_end(extend)),
            Key::ArrowUp if multiline => self.window.as_mut().map(|window| {
                let RuntimeWindow { ui, renderer, .. } = window;
                ui.input_move_vertical(-1, extend, renderer)
            }),
            Key::ArrowDown if multiline => self.window.as_mut().map(|window| {
                let RuntimeWindow { ui, renderer, .. } = window;
                ui.input_move_vertical(1, extend, renderer)
            }),
            Key::PageUp if multiline => self.window.as_mut().map(|window| {
                let RuntimeWindow { ui, renderer, .. } = window;
                ui.input_move_page(-1, extend, renderer)
            }),
            Key::PageDown if multiline => self.window.as_mut().map(|window| {
                let RuntimeWindow { ui, renderer, .. } = window;
                ui.input_move_page(1, extend, renderer)
            }),
            Key::Home if !cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_home(extend)),
            Key::End if !cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_end(extend)),
            Key::Home => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_line_start(extend)),
            Key::End => self
                .window
                .as_mut()
                .map(|window| window.ui.input_move_line_end(extend)),
            Key::Backspace if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_delete_to_line_start()),
            Key::Delete if cfg!(target_os = "macos") && primary => self
                .window
                .as_mut()
                .map(|window| window.ui.input_delete_to_line_end()),
            Key::Backspace if word => self
                .window
                .as_mut()
                .map(|window| window.ui.input_delete_word_backward()),
            Key::Delete if word => self
                .window
                .as_mut()
                .map(|window| window.ui.input_delete_word_forward()),
            Key::Backspace => self
                .window
                .as_mut()
                .map(|window| window.ui.input_backspace()),
            Key::Delete => self.window.as_mut().map(|window| window.ui.input_delete()),
            Key::Enter if multiline => self
                .window
                .as_mut()
                .map(|window| window.ui.input_insert_newline()),
            Key::Character(value)
                if cfg!(target_os = "macos")
                    && modifiers.contains(Modifiers::CONTROL)
                    && !primary
                    && value.eq_ignore_ascii_case("a") =>
            {
                self.window
                    .as_mut()
                    .map(|window| window.ui.input_move_line_start(extend))
            }
            Key::Character(value)
                if cfg!(target_os = "macos")
                    && modifiers.contains(Modifiers::CONTROL)
                    && !primary
                    && value.eq_ignore_ascii_case("e") =>
            {
                self.window
                    .as_mut()
                    .map(|window| window.ui.input_move_line_end(extend))
            }
            Key::Character(value)
                if primary
                    && modifiers.contains(Modifiers::SHIFT)
                    && value.eq_ignore_ascii_case("z") =>
            {
                self.window.as_mut().map(|window| window.ui.input_redo())
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("z") => {
                self.window.as_mut().map(|window| window.ui.input_undo())
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("y") => {
                self.window.as_mut().map(|window| window.ui.input_redo())
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("a") => self
                .window
                .as_mut()
                .map(|window| window.ui.input_select_all()),
            Key::Character(value) if primary && value.eq_ignore_ascii_case("c") => {
                let selected = self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.selected_input_text());
                if let Some(selected) = selected {
                    let _ = self.write_clipboard_text(selected.as_ref());
                }
                return true;
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("x") => {
                let selected = self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.selected_input_text());
                let copied =
                    selected.is_some_and(|selected| self.write_clipboard_text(selected.as_ref()));
                if !copied {
                    return true;
                }
                self.window
                    .as_mut()
                    .map(|window| window.ui.input_backspace())
            }
            Key::Character(value) if primary && value.eq_ignore_ascii_case("v") => {
                let pasted = self.read_clipboard_text();
                pasted.and_then(|value| {
                    self.window
                        .as_mut()
                        .map(|window| window.ui.input_replace(&value))
                })
            }
            _ => return false,
        };

        if let Some(result) = result {
            self.apply_input_result(event_loop, result, true);
        }
        true
    }

    pub(super) fn dispatch_binding_actions(
        &mut self,
        event_loop: &ActiveEventLoop,
        bindings: &[KeyBinding],
    ) -> Option<bool> {
        for binding in bindings {
            match self.invoke_action(event_loop, binding.action()) {
                None => return None,
                Some(true) => return Some(true),
                Some(false) => {}
            }
        }
        Some(false)
    }

    pub(super) fn handle_pressed_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: PendingKey,
    ) -> bool {
        let focused = self.window.as_ref().and_then(|window| window.ui.focused());
        let mut prefix = self
            .pending_input
            .take()
            .filter(|pending| pending.focus == focused)
            .map(|pending| pending.keys)
            .unwrap_or_default();
        let mut input = prefix.iter().map(PendingKey::keystroke).collect::<Vec<_>>();
        input.push(key.keystroke());
        let contexts = self
            .window
            .as_ref()
            .map(|window| window.ui.key_context_stack())
            .unwrap_or_default();
        let matched = self.keymap.bindings_for_input(&input, &contexts);

        if matched.pending {
            prefix.push(key);
            self.pending_input = Some(PendingInput {
                keys: prefix,
                focus: focused,
                deadline: Instant::now() + self.config.key_sequence_timeout,
            });
            return true;
        }

        if !matched.bindings.is_empty() {
            match self.dispatch_binding_actions(event_loop, &matched.bindings) {
                None => return false,
                Some(true) => return true,
                Some(false) => return self.handle_pressed_key_fallback(event_loop, key),
            }
        }

        if prefix.is_empty() {
            return self.handle_pressed_key_fallback(event_loop, key);
        }
        if !self.replay_pending_keys(event_loop, prefix) {
            return false;
        }
        self.handle_pressed_key(event_loop, key)
    }

    /// Replay timed-out or mismatched prefixes without recursively re-entering key matching.
    pub(super) fn replay_pending_keys(
        &mut self,
        event_loop: &ActiveEventLoop,
        mut keys: Vec<PendingKey>,
    ) -> bool {
        while !keys.is_empty() {
            let contexts = self
                .window
                .as_ref()
                .map(|window| window.ui.key_context_stack())
                .unwrap_or_default();
            let strokes = keys.iter().map(PendingKey::keystroke).collect::<Vec<_>>();
            let exact_prefix = (1..=strokes.len()).rev().find_map(|length| {
                let matched = self
                    .keymap
                    .bindings_for_input(&strokes[..length], &contexts);
                (!matched.bindings.is_empty()).then_some((length, matched.bindings))
            });

            if let Some((length, bindings)) = exact_prefix {
                let replay_key = keys[length - 1].clone();
                keys.drain(..length);
                match self.dispatch_binding_actions(event_loop, &bindings) {
                    None => return false,
                    Some(true) => {}
                    Some(false) => {
                        if !self.handle_pressed_key_fallback(event_loop, replay_key) {
                            return false;
                        }
                    }
                }
            } else {
                let replay_key = keys.remove(0);
                if !self.handle_pressed_key_fallback(event_loop, replay_key) {
                    return false;
                }
            }
        }
        true
    }

    pub(super) fn flush_pending_input(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let Some(pending) = self.pending_input.take() else {
            return true;
        };
        let focused = self.window.as_ref().and_then(|window| window.ui.focused());
        if pending.focus != focused {
            return true;
        }
        // The replay delivers presses the user made, so it is keyboard input even though a timer
        // rather than a native key event started it.
        let scope = self.begin_input_dispatch(InputModality::Keyboard);
        let continued = self.replay_pending_keys(event_loop, pending.keys);
        self.end_input_dispatch(scope);
        continued
    }

    pub(super) fn handle_pressed_key_fallback(
        &mut self,
        event_loop: &ActiveEventLoop,
        key_event: PendingKey,
    ) -> bool {
        let PendingKey {
            stroke,
            repeat,
            text,
        } = key_event;
        let Keystroke {
            key,
            modifiers,
            key_char,
        } = stroke;
        let default_prevented = match self.invoke_key_event(
            event_loop,
            KeyListenerEvent::Down(KeyDownEvent {
                key: key.clone(),
                key_char: key_char.clone(),
                text: text.clone(),
                modifiers,
                repeat,
            }),
        ) {
            Some(default_prevented) => default_prevented,
            None => return false,
        };

        if !default_prevented {
            if !repeat
                && matches!(&key, Key::Escape)
                && let Some(request) = self
                    .window
                    .as_ref()
                    .and_then(|window| window.ui.dismiss_topmost())
            {
                self.invoke_dismiss(event_loop, request);
                return true;
            }
            #[cfg(target_os = "macos")]
            if is_default_close_shortcut(&key, modifiers, repeat) {
                if let Some(window) = self.window.as_ref()
                    && let Err(error) = perform_window_close(&window.window)
                {
                    tracing::warn!(%error, "could not perform the default macOS close command");
                }
                return true;
            }

            let mut handled_by_input = self
                .handle_static_text_key(event_loop, &key, modifiers, repeat)
                || self.handle_text_input_key(event_loop, &key, modifiers, repeat);
            if !handled_by_input
                && !modifiers.intersects(Modifiers::CONTROL | Modifiers::SUPER)
                && let Some(text) = text.as_deref().filter(|text| {
                    !text.is_empty() && text.chars().all(|character| !character.is_control())
                })
                && self
                    .window
                    .as_ref()
                    .is_some_and(|window| window.ui.focused_text_input().is_some())
            {
                let result = self
                    .window
                    .as_mut()
                    .map(|window| window.ui.input_replace(text))
                    .unwrap_or_default();
                let changed = result.change.is_some();
                if !self.apply_input_result(event_loop, result, true)
                    || (changed
                        && !self.dispatch(event_loop, Event::TextInput(text.to_owned()), false))
                {
                    return false;
                }
                handled_by_input = true;
            }

            match &key {
                _ if handled_by_input => {}
                Key::Tab => {
                    let previous_focus =
                        self.window.as_ref().and_then(|window| window.ui.focused());
                    if let Some(window) = &mut self.window
                        && window.ui.focus_next(modifiers.contains(Modifiers::SHIFT))
                        && window.scheduler.invalidate()
                    {
                        // The only Tab stop keeps focus but gains its ring, which is paint-only.
                        window.window.request_redraw();
                    }
                    self.announce_focus_change(event_loop, previous_focus);
                }
                Key::ArrowLeft if modifiers.is_empty() => {
                    if !self.navigate_adjacent_tab(event_loop, false, true) {
                        self.activate_adjacent_radio(event_loop, true);
                    }
                }
                Key::ArrowRight if modifiers.is_empty() => {
                    if !self.navigate_adjacent_tab(event_loop, false, false) {
                        self.activate_adjacent_radio(event_loop, false);
                    }
                }
                Key::ArrowUp if modifiers.is_empty() => {
                    if !self.navigate_adjacent_tab(event_loop, true, true) {
                        self.activate_adjacent_radio(event_loop, true);
                    }
                }
                Key::ArrowDown if modifiers.is_empty() => {
                    if !self.navigate_adjacent_tab(event_loop, true, false) {
                        self.activate_adjacent_radio(event_loop, false);
                    }
                }
                Key::Home if modifiers.is_empty() => {
                    self.navigate_edge_tab(event_loop, false);
                }
                Key::End if modifiers.is_empty() => {
                    self.navigate_edge_tab(event_loop, true);
                }
                Key::Enter | Key::Space if !repeat => {
                    let target = self
                        .window
                        .as_ref()
                        .and_then(|window| window.ui.activate_focused());
                    if let Some(id) = target {
                        self.invoke_click(event_loop, id);
                    }
                }
                _ => {}
            }
        }

        self.dispatch(
            event_loop,
            Event::KeyDown {
                key,
                key_char,
                modifiers,
                repeat,
            },
            false,
        )
    }

    pub(super) fn activate_adjacent_radio(&mut self, event_loop: &ActiveEventLoop, reverse: bool) {
        let previous_focus = self.window.as_ref().and_then(|window| window.ui.focused());
        let target = self
            .window
            .as_ref()
            .and_then(|window| window.ui.adjacent_radio(reverse));
        let Some(target) = target else {
            return;
        };
        self.focus_element(target);
        self.announce_focus_change(event_loop, previous_focus);
        self.invoke_click(event_loop, target);
    }

    pub(super) fn navigate_adjacent_tab(
        &mut self,
        event_loop: &ActiveEventLoop,
        vertical_axis: bool,
        reverse: bool,
    ) -> bool {
        let target = self
            .window
            .as_ref()
            .and_then(|window| window.ui.adjacent_tab(vertical_axis, reverse));
        self.apply_tab_navigation(event_loop, target)
    }

    pub(super) fn navigate_edge_tab(&mut self, event_loop: &ActiveEventLoop, last: bool) -> bool {
        let target = self
            .window
            .as_ref()
            .and_then(|window| window.ui.edge_tab(last));
        self.apply_tab_navigation(event_loop, target)
    }

    pub(super) fn apply_tab_navigation(
        &mut self,
        event_loop: &ActiveEventLoop,
        target: Option<TabNavigationTarget>,
    ) -> bool {
        let Some(target) = target else {
            return false;
        };
        let previous_focus = self.window.as_ref().and_then(|window| window.ui.focused());
        self.focus_element(target.id);
        self.announce_focus_change(event_loop, previous_focus);
        if target.activate {
            self.invoke_click(event_loop, target.id);
        }
        true
    }

    /// Drop the framework's focused element while a hosted AppKit view owns keyboard focus.
    ///
    /// A native view that becomes first responder, by click, Tab, or its own focus state, takes
    /// every key event from the Winit view, so the element the framework still counted as focused
    /// would keep painting its ring and caret without ever receiving input. Blur it the way a
    /// press on empty space does and announce the change, so the view's listeners run; the next
    /// framework focus change makes the Winit view first responder again.
    #[cfg(target_os = "macos")]
    pub(super) fn release_focus_to_native_view(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = &mut self.window else {
            return;
        };
        let previous = state.ui.focused();
        if previous.is_none()
            || !state
                .native_host
                .as_ref()
                .is_some_and(MacNativeHost::hosted_view_owns_focus)
        {
            return;
        }
        state.ui.blur();
        self.announce_focus_change(event_loop, previous);
    }

    pub(super) fn announce_focus_change(
        &mut self,
        event_loop: &ActiveEventLoop,
        previous: Option<ElementId>,
    ) {
        let focused = self.window.as_ref().and_then(|state| state.ui.focused());
        if previous == focused {
            return;
        }
        self.pending_input = None;
        if let Some(state) = &mut self.window {
            // Focus is part of ViewContext, so a logical focus change must rebuild the declared
            // view rather than only repainting the retained tree. Components such as Terminal
            // derive their focused cursor presentation from this state.
            state.view_dirty = true;
            #[cfg(target_os = "macos")]
            if focused.is_some()
                && let Some(host) = &state.native_host
            {
                host.focus_framework();
            }
            if state.scheduler.invalidate() {
                state.window.request_redraw();
            }
        }
        let mut cx = self.event_context();
        if let Some(window) = &mut self.window {
            window.view.event(&Event::FocusChanged(focused), &mut cx);
        }
        self.apply_event_context(event_loop, cx, false, false);
        #[cfg(target_os = "macos")]
        self.sync_native_menu_state();
    }
}
