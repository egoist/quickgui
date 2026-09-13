use super::*;

impl Element {
    /// Give an otherwise non-focusable label native click-to-activate behavior.
    ///
    /// This stays crate-private because arbitrary activation chains need cycle and ownership
    /// policy. The unstyled field layer uses one derived label-to-control edge.
    pub(crate) fn activate_target_on_click(mut self, target: impl Into<ElementId>) -> Self {
        self.activation_target = Some(target.into());
        self.clickable = true;
        self.set_implicit_cursor(CursorStyle::Arrow);
        self
    }

    /// Attach a listener registered by [`crate::ViewContext::listener`].
    pub fn on_click<V>(mut self, listener: crate::ClickListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.clickable = true;
        self.set_implicit_cursor(CursorStyle::PointingHand);
        self.focusable = true;
        if self.accessibility.role == AccessibilityRole::GenericContainer {
            self.accessibility.role = AccessibilityRole::Button;
        }
        self
    }

    /// Capture pointer motion from press through release, including outside this element.
    ///
    /// Attach a listener registered by [`crate::ViewContext::pointer_listener`]. The element also
    /// occludes click and hover hit testing behind its bounds while allowing wheel scrolling.
    pub fn on_pointer<V>(mut self, listener: crate::PointerListener<V>) -> Self {
        assert!(
            !self.drag_source,
            "one element cannot own both a captured pointer listener and a typed drag source"
        );
        self.bind_listener_id(listener.id());
        self.pointer_listener = true;
        self
    }

    /// Handle a matching desktop mouse-button press during the bubble phase.
    pub fn on_mouse_down<V>(
        self,
        button: crate::MouseButton,
        listener: crate::MouseDownListener<V>,
    ) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Down,
                phase: DispatchPhase::Bubble,
                button: Some(button),
                outside: false,
            },
        )
    }

    /// Handle a desktop mouse-button press for any button during the bubble phase.
    pub fn on_any_mouse_down<V>(self, listener: crate::MouseDownListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Down,
                phase: DispatchPhase::Bubble,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle any desktop mouse-button press during capture, from the root toward the target.
    pub fn capture_any_mouse_down<V>(self, listener: crate::MouseDownListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Down,
                phase: DispatchPhase::Capture,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle a desktop mouse-button press outside this element during the capture stage.
    pub fn on_mouse_down_out<V>(self, listener: crate::MouseDownListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Down,
                phase: DispatchPhase::Capture,
                button: None,
                outside: true,
            },
        )
    }

    /// Handle a matching desktop mouse-button release during the bubble phase.
    pub fn on_mouse_up<V>(
        self,
        button: crate::MouseButton,
        listener: crate::MouseUpListener<V>,
    ) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Up,
                phase: DispatchPhase::Bubble,
                button: Some(button),
                outside: false,
            },
        )
    }

    /// Handle a desktop mouse-button release for any button during the bubble phase.
    pub fn on_any_mouse_up<V>(self, listener: crate::MouseUpListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Up,
                phase: DispatchPhase::Bubble,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle any desktop mouse-button release during capture, from the root toward the target.
    pub fn capture_any_mouse_up<V>(self, listener: crate::MouseUpListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Up,
                phase: DispatchPhase::Capture,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle a matching button release outside this element during the capture stage.
    pub fn on_mouse_up_out<V>(
        self,
        button: crate::MouseButton,
        listener: crate::MouseUpListener<V>,
    ) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Up,
                phase: DispatchPhase::Capture,
                button: Some(button),
                outside: true,
            },
        )
    }

    /// Handle pointer motion over this element during the bubble phase.
    pub fn on_mouse_move<V>(self, listener: crate::MouseMoveListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Move,
                phase: DispatchPhase::Bubble,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle the pointer leaving the native window while this element is hovered.
    pub fn on_mouse_exit<V>(self, listener: crate::MouseExitListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Exit,
                phase: DispatchPhase::Bubble,
                button: None,
                outside: false,
            },
        )
    }

    /// Observe web-style hover entry and exit, including layout changes beneath a still pointer.
    pub fn on_hover<V>(self, listener: crate::HoverListener<V>) -> Self {
        self.bind_mouse_listener(
            listener.id(),
            MouseListenerBinding {
                key: listener.key(),
                kind: MouseListenerKind::Hover,
                phase: DispatchPhase::Bubble,
                button: None,
                outside: false,
            },
        )
    }

    /// Handle native scroll-wheel input while the pointer is over this element or a descendant.
    ///
    /// Events bubble from the nearest listening element through listening ancestors. Call
    /// [`crate::EventContext::prevent_default`] when the gesture drives zoom, pan, or another
    /// custom interaction instead of the retained scroll container below it.
    pub fn on_scroll_wheel<V>(mut self, listener: crate::ScrollWheelListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.scroll_wheel_listener = true;
        self
    }

    /// Capture raw touch contacts that start over this element or its descendants.
    ///
    /// Each contact keeps its own [`crate::TouchId`] and continues reaching this listening path
    /// until ended or cancelled, even after moving outside the element.
    pub fn on_touch<V>(mut self, listener: crate::TouchListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.touch_listener = true;
        self
    }

    /// Open application-defined context UI from a secondary click on this element.
    pub fn on_context_menu<V>(mut self, listener: crate::ContextMenuListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.context_menu_listener = true;
        self
    }

    /// Handle Force Touch pressure while the logical pointer is over this element.
    pub fn on_mouse_pressure<V>(mut self, listener: crate::MousePressureListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.mouse_pressure_listener = true;
        self
    }

    /// Handle native pinch-to-zoom input while the logical pointer is over this element.
    pub fn on_pinch<V>(mut self, listener: crate::PinchListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.pinch_listener = true;
        self
    }

    /// Handle native two-finger rotation while the logical pointer is over this element.
    pub fn on_rotation<V>(mut self, listener: crate::RotationListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.rotation_listener = true;
        self
    }

    /// Handle a native smart-magnify request, normally a two-finger double tap on macOS.
    pub fn on_smart_magnify<V>(mut self, listener: crate::SmartMagnifyListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.smart_magnify_listener = true;
        self
    }

    /// Start a typed drag after primary-button motion crosses the native-style threshold.
    ///
    /// The source owns the gesture, so do not attach [`Self::on_pointer`] to the same element.
    pub fn on_drag<V, T>(mut self, listener: crate::DragListener<V, T>) -> Self {
        assert!(
            !self.pointer_listener,
            "one element cannot own both a captured pointer listener and a typed drag source"
        );
        self.bind_listener_id(listener.id());
        self.drag_source = true;
        self
    }

    /// Accept a compatible typed payload when it is released over this element.
    ///
    /// Multiple payload types may be registered for the same element by using the same stable id.
    pub fn on_drop<V, T>(mut self, listener: crate::DropListener<V, T>) -> Self {
        self.bind_listener_id(listener.id());
        self.drop_target = true;
        self
    }

    /// Restrict whether a payload of type `T` may be dropped on this element.
    ///
    /// The same predicate controls both `drag_over` paint state and final delivery. Omitting it
    /// accepts every payload matching an attached [`crate::DropListener`].
    pub fn can_drop<T: 'static>(mut self, predicate: impl Fn(&T) -> bool + 'static) -> Self {
        let type_id = TypeId::of::<T>();
        assert!(
            self.drop_predicates
                .iter()
                .all(|existing| existing.type_id != type_id),
            "can_drop was registered more than once for the same payload type"
        );
        self.drop_predicates.push(DropPredicate {
            type_id,
            callback: Arc::new(move |value| {
                predicate(
                    value
                        .downcast_ref::<T>()
                        .expect("can_drop received the wrong payload type"),
                )
            }),
        });
        self
    }

    /// Attach a controlled-value listener registered by [`crate::ViewContext::input_listener`].
    pub fn on_input<V>(mut self, listener: crate::InputListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.focusable = true;
        self.set_implicit_cursor(CursorStyle::IBeam);
        self.accessibility.role = match &self.kind {
            ElementKind::TextInput(TextInputElement {
                multiline: true, ..
            }) => AccessibilityRole::MultilineTextInput,
            ElementKind::TextInput(TextInputElement { password: true, .. }) => {
                AccessibilityRole::PasswordInput
            }
            _ => AccessibilityRole::TextInput,
        };
        self
    }

    /// Submit a valid input when Return is pressed without key repeat.
    /// Multiline inputs must explicitly enable `submit_on_enter`.
    pub fn on_submit<V>(mut self, listener: crate::SubmitListener<V>) -> Self {
        assert!(
            matches!(
                &self.kind,
                ElementKind::TextInput(input) if !input.multiline || input.submit_on_enter
            ),
            "multiline on_submit requires submit_on_enter"
        );
        self.bind_listener_id(listener.id());
        self.focusable = true;
        self.set_implicit_cursor(CursorStyle::IBeam);
        self.accessibility.role = if matches!(
            &self.kind,
            ElementKind::TextInput(TextInputElement {
                multiline: true,
                ..
            })
        ) {
            AccessibilityRole::MultilineTextInput
        } else if matches!(
            &self.kind,
            ElementKind::TextInput(TextInputElement { password: true, .. })
        ) {
            AccessibilityRole::PasswordInput
        } else {
            AccessibilityRole::TextInput
        };
        self
    }

    /// Attach a valid-form callback registered by [`crate::ViewContext::form_submit_listener`].
    pub fn on_form_submit<V>(mut self, listener: crate::FormSubmitListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.form = true;
        self.accessibility.role = AccessibilityRole::Form;
        self
    }

    /// Attach a validation callback registered by
    /// [`crate::ViewContext::form_invalid_listener`].
    pub fn on_form_invalid<V>(mut self, listener: crate::FormInvalidListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        self.form = true;
        self.accessibility.role = AccessibilityRole::Form;
        self
    }

    /// Make this control validate and submit its nearest ancestor form when activated.
    pub fn form_submitter(mut self) -> Self {
        self.form_submitter = true;
        self.clickable = true;
        self.set_implicit_cursor(CursorStyle::PointingHand);
        self.focusable = true;
        if self.accessibility.role == AccessibilityRole::GenericContainer {
            self.accessibility.role = AccessibilityRole::Button;
        }
        self
    }

    /// Handle focused key presses during the bubble phase, from focus toward the root.
    pub fn on_key_down<V>(self, listener: crate::KeyDownListener<V>) -> Self {
        self.bind_key_listener(
            listener.id(),
            KeyListenerBinding {
                key: listener.key(),
                kind: KeyListenerKind::Down,
                phase: DispatchPhase::Bubble,
            },
        )
    }

    /// Handle focused key presses during capture, from the root toward focus.
    pub fn capture_key_down<V>(self, listener: crate::KeyDownListener<V>) -> Self {
        self.bind_key_listener(
            listener.id(),
            KeyListenerBinding {
                key: listener.key(),
                kind: KeyListenerKind::Down,
                phase: DispatchPhase::Capture,
            },
        )
    }

    /// Handle focused key releases during the bubble phase, from focus toward the root.
    pub fn on_key_up<V>(self, listener: crate::KeyUpListener<V>) -> Self {
        self.bind_key_listener(
            listener.id(),
            KeyListenerBinding {
                key: listener.key(),
                kind: KeyListenerKind::Up,
                phase: DispatchPhase::Bubble,
            },
        )
    }

    /// Handle focused key releases during capture, from the root toward focus.
    pub fn capture_key_up<V>(self, listener: crate::KeyUpListener<V>) -> Self {
        self.bind_key_listener(
            listener.id(),
            KeyListenerBinding {
                key: listener.key(),
                kind: KeyListenerKind::Up,
                phase: DispatchPhase::Capture,
            },
        )
    }

    /// Attach a typed action handler during the bubble phase.
    ///
    /// Bubble action handlers run from focus toward the root and consume by default. Call
    /// [`crate::EventContext::propagate`] to continue to the next handler.
    pub fn on_action<V, A>(self, listener: crate::ActionListener<V, A>) -> Self {
        self.bind_action_listener(
            listener.id(),
            ActionListenerBinding {
                key: listener.key(),
                action_type: listener.action_type(),
                phase: DispatchPhase::Bubble,
            },
        )
    }

    /// Attach a typed action handler during capture, from the root toward focus.
    ///
    /// Capture actions propagate by default. Call [`crate::EventContext::stop_propagation`] to
    /// consume the action before it reaches deeper capture listeners or the bubble phase.
    pub fn capture_action<V, A>(self, listener: crate::ActionListener<V, A>) -> Self {
        self.bind_action_listener(
            listener.id(),
            ActionListenerBinding {
                key: listener.key(),
                action_type: listener.action_type(),
                phase: DispatchPhase::Capture,
            },
        )
    }

    /// Choose whether this surface occludes pointer hits behind its bounds.
    pub fn pointer_blocking(mut self, block: bool) -> Self {
        self.blocks_pointer = block;
        self
    }

    /// Dismiss this surface on Escape or a pointer press outside its bounds.
    ///
    /// A dismissible element always emits [`crate::Event::Dismiss`]. Attach a typed callback with
    /// [`Self::on_dismiss`] when handling the event in [`crate::View::event`] is inconvenient.
    pub fn dismissible(mut self) -> Self {
        self.dismiss_policy = DismissPolicy::BOTH;
        self.blocks_pointer = true;
        self
    }

    /// Dismiss this surface on Escape without enabling outside-pointer dismissal.
    pub fn dismiss_on_escape(mut self) -> Self {
        self.dismiss_policy = self.dismiss_policy.with_escape();
        self.blocks_pointer = true;
        self
    }

    /// Dismiss this surface on a pointer press outside its bounds without consuming Escape.
    pub fn dismiss_on_pointer_outside(mut self) -> Self {
        self.dismiss_policy = self.dismiss_policy.with_pointer_outside();
        self.blocks_pointer = true;
        self
    }

    /// Attach a typed callback to a dismissible surface.
    pub fn on_dismiss<V>(mut self, listener: crate::DismissListener<V>) -> Self {
        self.bind_listener_id(listener.id());
        if self.dismiss_policy.is_empty() {
            self.dismiss_policy = DismissPolicy::BOTH;
        }
        self.blocks_pointer = true;
        self
    }

    /// Restore focus to this handle when a dismissible surface closes or leaves the retained tree.
    pub fn restore_focus_to(mut self, handle: FocusHandle) -> Self {
        self.restore_focus = Some(handle);
        self
    }

    /// Set the native cursor shown while the pointer is over this element.
    pub fn cursor(mut self, cursor: CursorStyle) -> Self {
        self.cursor_style = Some(cursor);
        self.cursor_style_explicit = true;
        self
    }

    /// Use the platform's default arrow cursor (`default`).
    pub fn cursor_default(self) -> Self {
        self.cursor(CursorStyle::Arrow)
    }

    /// Use a pointing hand cursor (`pointer`).
    pub fn cursor_pointer(self) -> Self {
        self.cursor(CursorStyle::PointingHand)
    }

    /// Use a text-selection cursor (`text`).
    pub fn cursor_text(self) -> Self {
        self.cursor(CursorStyle::IBeam)
    }

    /// Use a closed hand cursor (`move`).
    pub fn cursor_move(self) -> Self {
        self.cursor(CursorStyle::ClosedHand)
    }

    /// Indicate that the operation is unavailable (`not-allowed`).
    pub fn cursor_not_allowed(self) -> Self {
        self.cursor(CursorStyle::OperationNotAllowed)
    }

    /// Indicate that a context menu is available (`context-menu`).
    pub fn cursor_context_menu(self) -> Self {
        self.cursor(CursorStyle::ContextualMenu)
    }

    /// Use a crosshair cursor (`crosshair`).
    pub fn cursor_crosshair(self) -> Self {
        self.cursor(CursorStyle::Crosshair)
    }

    /// Use the vertical text-selection cursor (`vertical-text`).
    pub fn cursor_vertical_text(self) -> Self {
        self.cursor(CursorStyle::IBeamCursorForVerticalLayout)
    }

    /// Indicate that a drag will create an alias (`alias`).
    pub fn cursor_alias(self) -> Self {
        self.cursor(CursorStyle::DragLink)
    }

    /// Indicate that a drag will copy its payload (`copy`).
    pub fn cursor_copy(self) -> Self {
        self.cursor(CursorStyle::DragCopy)
    }

    /// Indicate that a payload cannot be dropped here (`no-drop`).
    pub fn cursor_no_drop(self) -> Self {
        self.cursor(CursorStyle::OperationNotAllowed)
    }

    /// Use an open hand cursor (`grab`).
    pub fn cursor_grab(self) -> Self {
        self.cursor(CursorStyle::OpenHand)
    }

    /// Use a closed hand cursor (`grabbing`).
    pub fn cursor_grabbing(self) -> Self {
        self.cursor(CursorStyle::ClosedHand)
    }

    /// Use a horizontal resize cursor (`ew-resize`).
    pub fn cursor_ew_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeLeftRight)
    }

    /// Use a vertical resize cursor (`ns-resize`).
    pub fn cursor_ns_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeUpDown)
    }

    /// Use a north-east/south-west resize cursor (`nesw-resize`).
    pub fn cursor_nesw_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeUpRightDownLeft)
    }

    /// Use a north-west/south-east resize cursor (`nwse-resize`).
    pub fn cursor_nwse_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeUpLeftDownRight)
    }

    /// Use a column resize cursor (`col-resize`).
    pub fn cursor_col_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeColumn)
    }

    /// Use a row resize cursor (`row-resize`).
    pub fn cursor_row_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeRow)
    }

    /// Use a north-edge resize cursor (`n-resize`).
    pub fn cursor_n_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeUp)
    }

    /// Use an east-edge resize cursor (`e-resize`).
    pub fn cursor_e_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeRight)
    }

    /// Use a south-edge resize cursor (`s-resize`).
    pub fn cursor_s_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeDown)
    }

    /// Use a west-edge resize cursor (`w-resize`).
    pub fn cursor_w_resize(self) -> Self {
        self.cursor(CursorStyle::ResizeLeft)
    }

    /// Explicitly allow browser-style pointer selection in this text subtree.
    ///
    /// Ordinary immutable text already uses `auto`, which is selectable outside controls. This
    /// override is useful for text nested in a custom clickable or draggable surface.
    pub fn user_select_text(mut self) -> Self {
        self.user_select = UserSelect::Text;
        self
    }

    /// Disable browser-style pointer selection for this element and its descendants.
    pub fn user_select_none(mut self) -> Self {
        self.user_select = UserSelect::None;
        self
    }

    /// Ergonomic alias for [`Self::user_select_text`].
    pub fn selectable(self) -> Self {
        self.user_select_text()
    }

    pub(crate) fn has_stateful_paint(&self) -> bool {
        self.hover.has_paint_overrides()
            || self.active.has_paint_overrides()
            || self.focus.has_paint_overrides()
            || self.disabled_style.has_paint_overrides()
            || self.invalid_style.has_paint_overrides()
            || self.selected_style.has_paint_overrides()
            || self.dragging.has_paint_overrides()
            || self.drag_over.has_paint_overrides()
    }

    pub(crate) fn has_stateful_cursor(&self) -> bool {
        self.hover.cursor_style.is_some()
            || self.active.cursor_style.is_some()
            || self.focus.cursor_style.is_some()
            || self.disabled_style.cursor_style.is_some()
            || self.invalid_style.cursor_style.is_some()
            || self.selected_style.cursor_style.is_some()
            || self.dragging.cursor_style.is_some()
            || self.drag_over.cursor_style.is_some()
    }

    pub(super) fn set_implicit_cursor(&mut self, cursor: CursorStyle) {
        if !self.cursor_style_explicit {
            self.cursor_style = Some(cursor);
        }
    }

    pub(super) fn bind_listener_id(&mut self, id: ElementId) {
        if let Some(existing) = self.explicit_id {
            assert_eq!(
                existing, id,
                "one element cannot attach listeners with different stable ids"
            );
        }
        self.explicit_id = Some(id);
    }

    fn bind_mouse_listener(mut self, id: ElementId, binding: MouseListenerBinding) -> Self {
        self.bind_listener_id(id);
        let listeners = self
            .mouse_listeners
            .get_or_insert_with(|| Box::new(Vec::with_capacity(2)));
        assert!(
            listeners.len() < MAX_MOUSE_LISTENERS_PER_ELEMENT,
            "one element cannot attach more than {MAX_MOUSE_LISTENERS_PER_ELEMENT} desktop mouse listeners"
        );
        listeners.push(binding);
        self
    }

    fn bind_key_listener(mut self, id: ElementId, binding: KeyListenerBinding) -> Self {
        self.bind_listener_id(id);
        let listeners = self
            .key_listeners
            .get_or_insert_with(|| Box::new(Vec::with_capacity(2)));
        assert!(
            listeners.len() < MAX_KEY_LISTENERS_PER_ELEMENT,
            "one element cannot attach more than {MAX_KEY_LISTENERS_PER_ELEMENT} focused key listeners"
        );
        listeners.push(binding);
        self
    }

    fn bind_action_listener(mut self, id: ElementId, binding: ActionListenerBinding) -> Self {
        self.bind_listener_id(id);
        let listeners = self
            .action_listeners
            .get_or_insert_with(|| Box::new(Vec::with_capacity(2)));
        assert!(
            listeners.len() < MAX_ACTION_LISTENERS_PER_ELEMENT,
            "one element cannot attach more than {MAX_ACTION_LISTENERS_PER_ELEMENT} typed action listeners"
        );
        listeners.push(binding);
        self
    }
}
