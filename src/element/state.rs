use super::*;

impl Element {
    /// Controls logical-pixel rounding for the entire tree when set on its root.
    /// Disable to retain fractional layout coordinates through painting and hit testing.
    pub fn layout_rounding(mut self, enabled: bool) -> Self {
        self.layout_rounding = enabled;
        self
    }

    pub fn hover(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.hover = style(ElementStateStyle::default());
        self
    }

    pub fn active(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.active = style(ElementStateStyle::default());
        self
    }

    /// Smooth supported paint-only style changes under one retained element identity.
    ///
    /// A [`Duration`](web_time::Duration) converts to an all-property [`Transition`]. Layout
    /// values use [`crate::AnimationExt`] instead because they require a declaration rebuild and
    /// Taffy layout.
    pub fn transition(mut self, transition: impl Into<Transition>) -> Self {
        self.transition = Some(transition.into());
        self
    }

    /// Paint-only styling while this element owns keyboard focus and that focus is visible.
    ///
    /// Like CSS `:focus-visible`, focus a pointer press lands paints none of this, focus a key
    /// lands paints all of it, and a programmatic focus keeps the visibility that was current.
    /// Text inputs and text areas are exempt and paint it whenever focused, as a native text field
    /// always shows its ring.
    pub fn focus(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.focus = style(ElementStateStyle::default());
        self
    }

    /// Paint-only styling while this element is the source of an active internal drag.
    pub fn dragging(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.dragging = style(ElementStateStyle::default());
        self
    }

    /// Paint-only styling while a compatible typed payload is over this drop target.
    pub fn drag_over(mut self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.drag_over = style(ElementStateStyle::default());
        self
    }

    /// Make this element the group whose hover and presses its descendants' [`Self::group_hover`]
    /// and [`Self::group_active`] styles follow, like Tailwind's `group`.
    ///
    /// The group is hovered whenever the pointer rests anywhere inside its bounds — over its own
    /// padding or over any descendant, interactive or not — unless a surface above it consumes the
    /// pointer, as a dismissible overlay or a captured pointer listener does, and active while a
    /// press on it or on any clickable descendant is held. Every descendant resolves against its
    /// nearest group, so groups nest.
    pub fn group(mut self) -> Self {
        self.group = true;
        self
    }

    /// Make this element a group its descendants can follow by name, like Tailwind's
    /// `group/name`, so a member can follow it past a nearer group.
    ///
    /// A named group is still the nearest group for members that name none. Names are bounded by
    /// [`MAX_HOVER_GROUP_NAME_BYTES`](crate::MAX_HOVER_GROUP_NAME_BYTES).
    pub fn group_named(mut self, name: impl Into<Arc<str>>) -> Self {
        self.group = true;
        self.group_name = Some(bounded_hover_group_name(name.into()));
        self
    }

    /// Paint-only styling while the nearest [`Self::group`] ancestor is hovered.
    ///
    /// This is Tailwind's `group-hover`: a row reveals its actions, a card lifts its icon. Group
    /// styles layer beneath the element's own hover, active, focus, validation, and drag states
    /// and above its base style, so an action button the pointer reaches keeps every group value
    /// its own `hover` does not override. Declared more than once — for different groups, or with
    /// [`Self::group_active`] — every entry whose group is in its state paints, later declarations
    /// winning where they overlap, as matching CSS rules of equal specificity do; one element
    /// follows at most [`MAX_GROUP_STYLES_PER_ELEMENT`](crate::MAX_GROUP_STYLES_PER_ELEMENT)
    /// group states. A cursor declared here is ignored, because the pointer is over some other
    /// element.
    pub fn group_hover(self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.group_style(GroupState::Hover, None, style)
    }

    /// Paint-only styling while the nearest ancestor [`Self::group_named`] `name` is hovered, like
    /// Tailwind's `group-hover/name`.
    ///
    /// The member skips every nearer group on the way to that name and paints nothing when no
    /// ancestor carries it.
    pub fn group_hover_named(
        self,
        name: impl Into<Arc<str>>,
        style: impl FnOnce(ElementStateStyle) -> ElementStateStyle,
    ) -> Self {
        let name = bounded_hover_group_name(name.into());
        self.group_style(GroupState::Hover, Some(name), style)
    }

    /// Paint-only styling while a press inside the nearest [`Self::group`] ancestor is held, like
    /// Tailwind's `group-active`.
    ///
    /// Declare it after [`Self::group_hover`] so the press wins over the hover beneath it, as the
    /// CSS `:hover`-then-`:active` rule order does.
    pub fn group_active(self, style: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.group_style(GroupState::Active, None, style)
    }

    /// Paint-only styling while a press inside the nearest ancestor [`Self::group_named`] `name`
    /// is held, like Tailwind's `group-active/name`.
    pub fn group_active_named(
        self,
        name: impl Into<Arc<str>>,
        style: impl FnOnce(ElementStateStyle) -> ElementStateStyle,
    ) -> Self {
        let name = bounded_hover_group_name(name.into());
        self.group_style(GroupState::Active, Some(name), style)
    }

    /// Paint-only styling while this element or any descendant owns keyboard focus, like CSS
    /// `:focus-within`.
    ///
    /// Unlike [`Self::focus`], this follows focus itself rather than focus visibility, so a field
    /// container highlights whenever its input is focused, however that focus arrived. A cursor
    /// declared here is ignored.
    pub fn focus_within(
        mut self,
        style: impl FnOnce(ElementStateStyle) -> ElementStateStyle,
    ) -> Self {
        self.focus_within = style(ElementStateStyle::default());
        self
    }

    #[track_caller]
    fn group_style(
        mut self,
        state: GroupState,
        target: Option<Arc<str>>,
        style: impl FnOnce(ElementStateStyle) -> ElementStateStyle,
    ) -> Self {
        assert!(
            self.group_styles.len() < MAX_GROUP_STYLES_PER_ELEMENT,
            "one element follows at most {MAX_GROUP_STYLES_PER_ELEMENT} group states"
        );
        self.group_styles.push(GroupStateStyle {
            state,
            target,
            style: style(ElementStateStyle::default()),
        });
        self
    }

    pub fn accessibility_role(mut self, role: AccessibilityRole) -> Self {
        self.accessibility.role = role;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.accessibility.label = Some(label.into());
        self
    }

    pub fn accessibility_value(mut self, value: impl Into<Arc<str>>) -> Self {
        self.accessibility.value = Some(value.into());
        self
    }

    /// Set placeholder text for a [`text_input`] element.
    pub fn placeholder(mut self, placeholder: impl Into<Arc<str>>) -> Self {
        if let ElementKind::TextInput(input) = &mut self.kind {
            input.placeholder = placeholder.into();
        }
        self
    }

    /// Mask the visible value and expose native secure-text-field semantics.
    ///
    /// Password inputs retain their real controlled value for editing and submit listeners, but
    /// paint one bullet per Unicode grapheme and do not expose selections to clipboard actions.
    /// Calling this with `false` restores an ordinary single-line text input, which supports
    /// web-style reveal buttons without replacing the retained input state.
    pub fn password(mut self, password: bool) -> Self {
        let ElementKind::TextInput(input) = &mut self.kind else {
            panic!("password can only be applied to a text input");
        };
        assert!(
            !password || !input.multiline,
            "a text area cannot be a password input"
        );
        input.password = password;
        self.accessibility.role = if password {
            AccessibilityRole::PasswordInput
        } else {
            AccessibilityRole::TextInput
        };
        self
    }

    /// Limit user edits to at most this many Unicode grapheme clusters.
    ///
    /// Pasted and committed IME text is truncated at a grapheme boundary before the input filter
    /// runs. Controlled values supplied by the application remain authoritative and are not
    /// rewritten during rendering.
    pub fn max_length(mut self, length: usize) -> Self {
        let ElementKind::TextInput(input) = &mut self.kind else {
            panic!("max_length can only be applied to a text input or text area");
        };
        input.constraints.max_length = Some(length);
        self
    }

    /// Accept or reject a proposed complete value before retained text and history are mutated.
    ///
    /// The callback runs only for edit attempts—not during paint, layout, pointer movement, or
    /// controlled-value synchronization. Returning `false` rejects typing, paste, IME commit,
    /// accessibility value changes, and undo/redo consistently.
    pub fn input_filter(mut self, filter: impl Fn(&str) -> bool + 'static) -> Self {
        let ElementKind::TextInput(input) = &mut self.kind else {
            panic!("input_filter can only be applied to a text input or text area");
        };
        assert!(
            input.constraints.filter.is_none(),
            "input_filter was registered more than once on one element"
        );
        input.constraints.filter = Some(Arc::new(filter));
        self
    }

    /// Replace every per-input text checking override at once.
    ///
    /// Unset fields inherit [`crate::default_text_checking`].
    pub fn text_checking(mut self, overrides: TextCheckingOverrides) -> Self {
        self.text_checking_mut("text_checking").text_checking = overrides;
        self
    }

    /// Underline unknown words after this input's edits settle.
    ///
    /// Checking runs once, 300 ms after the last accepted edit, over at most
    /// [`crate::MAX_SPELLCHECK_BYTES`] around the caret. A settled input holds no timer.
    pub fn spellcheck(mut self, enabled: bool) -> Self {
        self.text_checking_mut("spellcheck")
            .text_checking
            .spellcheck = Some(enabled);
        self
    }

    /// Ask the checker for grammar problems during the same settled check.
    pub fn grammar_check(mut self, enabled: bool) -> Self {
        self.text_checking_mut("grammar_check")
            .text_checking
            .grammar_check = Some(enabled);
        self
    }

    /// Replace a completed word with the checker's correction at a word boundary.
    ///
    /// The correction and the boundary character are applied as one undoable edit, and the applied
    /// change stays inspectable so an application can offer "Change back".
    pub fn autocorrect(mut self, enabled: bool) -> Self {
        self.text_checking_mut("autocorrect")
            .text_checking
            .autocorrect = Some(enabled);
        self
    }

    /// Convert straight quotes to typographic quotes at insertion time.
    pub fn smart_quotes(mut self, enabled: bool) -> Self {
        self.text_checking_mut("smart_quotes")
            .text_checking
            .smart_quotes = Some(enabled);
        self
    }

    /// Convert `--` to an em dash at insertion time.
    pub fn smart_dashes(mut self, enabled: bool) -> Self {
        self.text_checking_mut("smart_dashes")
            .text_checking
            .smart_dashes = Some(enabled);
        self
    }

    /// Apply the checker's replacement dictionary at insertion time.
    pub fn text_replacement(mut self, enabled: bool) -> Self {
        self.text_checking_mut("text_replacement")
            .text_checking
            .text_replacement = Some(enabled);
        self
    }

    /// Show the dictionary popover when a Force Touch trackpad force-clicks a word.
    pub fn lookup_on_force_click(mut self, enabled: bool) -> Self {
        self.text_checking_mut("lookup_on_force_click")
            .text_checking
            .lookup_on_force_click = Some(enabled);
        self
    }

    #[track_caller]
    fn text_checking_mut(&mut self, method: &str) -> &mut InputConstraints {
        let ElementKind::TextInput(input) = &mut self.kind else {
            panic!("{method} can only be applied to a text input or text area");
        };
        &mut input.constraints
    }

    /// Expose that a form control requires a value before submission.
    ///
    /// This projects the native accessibility state only. The application remains responsible for
    /// deriving [`Self::invalid`] and its validation message from the controlled value.
    pub fn required(mut self, required: bool) -> Self {
        self.accessibility.required = required;
        self
    }

    /// Keep this element in the keyboard Tab sequence even while it is disabled.
    ///
    /// This is Base UI's `focusableWhenDisabled`, and it exists for composite widgets — a toolbar
    /// above all — where skipping an unavailable item hides it from keyboard users entirely. The
    /// element still reports as disabled, still refuses pointer focus, and still does not activate;
    /// only keyboard reachability changes.
    pub fn focusable_when_disabled(mut self) -> Self {
        self.focusable = true;
        self.focusable_when_disabled = true;
        self
    }

    /// Expose that a control shows a value the user may read and copy but not change.
    ///
    /// This is the web's `readonly`, not `disabled`: a read-only control stays focusable, stays in
    /// the Tab sequence, and keeps its value in the accessible name, while a disabled one leaves
    /// the sequence entirely. Components that own their own editing behavior — the number field,
    /// select, combobox, checkbox, radio, and switch — refuse changes on their own state as well as
    /// projecting this.
    pub fn accessibility_read_only(mut self, read_only: bool) -> Self {
        self.accessibility.read_only = read_only;
        self
    }

    /// Expose web-style invalid state to paint and the native accessibility tree.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.accessibility.invalid = invalid;
        self
    }

    /// Describe why an invalid control cannot currently be submitted.
    ///
    /// Call [`Self::invalid`] separately so clearing or replacing a message never changes validity
    /// accidentally.
    pub fn validation_message(mut self, message: impl Into<Arc<str>>) -> Self {
        let (message, truncated) = bounded_validation_message(message.into());
        self.accessibility.validation_message = message;
        self.accessibility.validation_message_truncated = truncated;
        self
    }

    pub(crate) fn validation_message_retained(
        mut self,
        message: Arc<str>,
        truncated: bool,
    ) -> Self {
        debug_assert!(message.len() <= MAX_VALIDATION_MESSAGE_BYTES);
        self.accessibility.validation_message = (!message.is_empty()).then_some(message);
        self.accessibility.validation_message_truncated = truncated;
        self
    }

    /// Set a native accessibility description independently of visible text.
    pub fn accessibility_description(mut self, description: impl Into<Arc<str>>) -> Self {
        let description = description.into();
        self.accessibility.description = (!description.is_empty()).then_some(description);
        self
    }

    /// Paint-only styling while this element is marked invalid.
    pub fn invalid_style(
        mut self,
        style: impl FnOnce(ElementStateStyle) -> ElementStateStyle,
    ) -> Self {
        self.invalid_style = style(ElementStateStyle::default());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.accessibility.disabled = disabled;
        self
    }

    /// Paint-only styling while this element is disabled.
    ///
    /// Disabled elements are already removed from pointer, keyboard, and accessibility actions;
    /// this method adds an optional visual treatment without changing layout.
    pub fn disabled_style(
        mut self,
        style: impl FnOnce(ElementStateStyle) -> ElementStateStyle,
    ) -> Self {
        self.disabled_style = style(ElementStateStyle::default());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.accessibility.selected = selected;
        self
    }

    /// Paint-only styling while this element is marked [`selected`](Self::selected).
    ///
    /// The selection itself stays wherever the application or a collection keeps it; this only
    /// declares how the element paints while it is the chosen one. Like a native list row, a
    /// selected element keeps its selected paint while hovered or pressed: this state sits above
    /// the pointer states and beneath `disabled_style`.
    pub fn selected_style(
        mut self,
        style: impl FnOnce(ElementStateStyle) -> ElementStateStyle,
    ) -> Self {
        self.selected_style = style(ElementStateStyle::default());
        self
    }

    /// Hide this element and its complete subtree from the native accessibility tree.
    ///
    /// Painting, layout, pointer input, and keyboard behavior are unchanged. This is useful for
    /// a visual surface whose semantics are projected into another native window, such as a
    /// never-key autocomplete panel controlled by an owner-window text input.
    pub fn accessibility_hidden(mut self, hidden: bool) -> Self {
        self.accessibility.hidden = hidden;
        self
    }

    /// Expose the layout and keyboard axis of a composite or range control.
    pub fn accessibility_orientation(mut self, orientation: AccessibilityOrientation) -> Self {
        self.accessibility.orientation = Some(orientation);
        self
    }

    /// Expose a numeric value with its bounds and step for a range-valued role.
    ///
    /// An empty range clears the projection. Non-finite components are dropped by
    /// [`AccessibilityValueRange`] before they reach the native tree.
    pub fn accessibility_value_range(mut self, range: AccessibilityValueRange) -> Self {
        self.accessibility.value_range = (!range.is_empty()).then(|| Box::new(range));
        self
    }

    /// Announce this element's mounted text as a live region.
    ///
    /// The projection carries no timer, observer, or idle scheduler source; a rebuilt region is
    /// announced exactly once by the platform adapter.
    pub fn accessibility_live(mut self, live: AccessibilityLive) -> Self {
        self.accessibility.live = Some(live);
        self
    }

    /// Expose whether a disclosure, popover, or similar controlled surface is expanded.
    pub fn accessibility_expanded(mut self, expanded: bool) -> Self {
        self.accessibility.expanded = Some(expanded);
        self
    }

    /// Relate this control to one stable element that it controls.
    ///
    /// The relationship is projected only when the target is present in the mounted
    /// accessibility tree. This avoids dangling native node references for closed controlled
    /// popovers whose content is intentionally unmounted.
    pub fn accessibility_controls(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility.relations.set_controls(target.into());
        self
    }

    /// Identify the mounted active option, row, cell, or tree item for a composite control.
    ///
    /// Keep DOM-style keyboard focus on the composite root and update this relationship as its
    /// controlled selection moves. The native relationship is omitted while the target is not
    /// mounted, which is important for virtualized collections.
    pub fn accessibility_active_descendant(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility
            .relations
            .set_active_descendant(target.into());
        self
    }

    /// Use one mounted element as this element's accessible label.
    ///
    /// The relationship is projected only while the target is present and is omitted for a
    /// self-reference. Prefer this to copying visible group, field, or section text into a second
    /// accessibility-only string.
    pub fn accessibility_labelled_by(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility.relations.set_labelled_by(target.into());
        self
    }

    /// Use one mounted element as this element's accessible description.
    ///
    /// Dangling and self-referential relationships are omitted from the native tree, matching
    /// [`Self::accessibility_labelled_by`].
    pub fn accessibility_described_by(mut self, target: impl Into<ElementId>) -> Self {
        self.accessibility.relations.set_described_by(target.into());
        self
    }

    pub(crate) fn accessibility_described_by_pair(
        mut self,
        first: impl Into<ElementId>,
        second: impl Into<ElementId>,
    ) -> Self {
        self.accessibility
            .relations
            .set_described_by_pair(first.into(), second.into());
        self
    }

    /// Describe the kind of popover opened by this control.
    pub fn accessibility_has_popover(mut self, popover: AccessibilityPopover) -> Self {
        self.accessibility.has_popover = Some(popover);
        self
    }

    /// Describe how an editable control presents completion suggestions.
    pub fn accessibility_auto_complete(mut self, behavior: AccessibilityAutoComplete) -> Self {
        self.accessibility.auto_complete = Some(behavior);
        self
    }

    /// Mark a dialog or alert-dialog as explicitly modal for assistive technology.
    pub fn accessibility_modal(mut self, modal: bool) -> Self {
        self.accessibility.modal = modal;
        self
    }

    /// Expose the complete logical row count for a table or grid, including unmounted rows.
    pub fn accessibility_row_count(mut self, count: usize) -> Self {
        self.accessibility.collection.row_count = accessibility_collection_storage(count);
        self
    }

    /// Expose the complete logical column count for a table or grid.
    pub fn accessibility_column_count(mut self, count: usize) -> Self {
        self.accessibility.collection.column_count = accessibility_collection_storage(count);
        self
    }

    /// Set the zero-based logical row index for a row or cell.
    pub fn accessibility_row_index(mut self, index: usize) -> Self {
        self.accessibility.collection.row_index = accessibility_collection_storage(index);
        self
    }

    /// Set the zero-based logical column index for a header or cell.
    pub fn accessibility_column_index(mut self, index: usize) -> Self {
        self.accessibility.collection.column_index = accessibility_collection_storage(index);
        self
    }

    /// Set the zero-based nesting level for a hierarchical item.
    pub fn accessibility_level(mut self, level: usize) -> Self {
        self.accessibility.collection.level = accessibility_collection_storage(level);
        self
    }

    /// Expose the total logical sibling count for a virtualized collection item.
    pub fn accessibility_size_of_set(mut self, size: usize) -> Self {
        self.accessibility.collection.size_of_set = accessibility_collection_storage(size);
        self
    }

    /// Set the zero-based logical position among an item's siblings.
    pub fn accessibility_position_in_set(mut self, position: usize) -> Self {
        self.accessibility.collection.position_in_set = accessibility_collection_storage(position);
        self
    }

    /// Expose the active ordering of a sortable table or grid column.
    pub fn accessibility_sort_direction(mut self, direction: AccessibilitySortDirection) -> Self {
        self.accessibility.collection.sort_direction = Some(direction);
        self
    }

    /// Expose whether a collection accepts more than one selected descendant.
    pub fn accessibility_multiselectable(mut self, multiselectable: bool) -> Self {
        self.accessibility.multiselectable = multiselectable;
        self
    }

    /// Expose a controlled checked, unchecked, or mixed state to assistive technology.
    pub fn toggle_state(mut self, state: impl Into<ToggleState>) -> Self {
        self.accessibility.toggled = Some(state.into());
        self
    }

    /// Web-style boolean shorthand for [`Self::toggle_state`].
    pub fn checked(self, checked: bool) -> Self {
        self.toggle_state(checked)
    }

    /// Set or clear the web-style indeterminate checkbox state.
    ///
    /// Clearing indeterminate preserves an existing on/off state and maps a previously mixed
    /// state to off.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.accessibility.toggled = Some(if indeterminate {
            ToggleState::Mixed
        } else {
            match self.accessibility.toggled {
                Some(ToggleState::On) => ToggleState::On,
                _ => ToggleState::Off,
            }
        });
        self
    }

    /// Include this element in the window's focus path and Tab traversal.
    pub fn focusable(mut self) -> Self {
        self.focusable = true;
        self
    }

    /// Control whether a pointer press moves keyboard focus to this focusable element.
    ///
    /// Disabling pointer focus does not remove the element from Tab traversal or accessibility
    /// focus. This is useful for native-style sidebars and toolbars whose controls should activate
    /// without taking keyboard ownership from an editor or terminal.
    pub fn focus_on_pointer(mut self, focus: bool) -> Self {
        self.focus_on_pointer = focus;
        self
    }

    /// Expand this element's pointer hit region without changing its layout or paint bounds.
    ///
    /// The expanded region remains clipped by the element's parent. This is useful for thin native
    /// affordances such as split-view dividers and resize handles that need a forgiving target
    /// without a visible gutter.
    pub fn hit_slop(mut self, insets: Insets) -> Self {
        self.hit_slop = Insets {
            top: finite_nonnegative(insets.top),
            right: finite_nonnegative(insets.right),
            bottom: finite_nonnegative(insets.bottom),
            left: finite_nonnegative(insets.left),
        };
        self
    }

    /// Assign a stable focus identity to this element.
    pub fn track_focus(mut self, handle: FocusHandle) -> Self {
        self.bind_listener_id(handle.id());
        self.focusable = true;
        self
    }

    /// Give a non-focusable ancestor a stable identity for scoped action dispatch.
    ///
    /// Descendant focus is still tracked through this element, but the scope itself is not added to
    /// Tab traversal.
    pub fn focus_scope(mut self, handle: FocusHandle) -> Self {
        self.bind_listener_id(handle.id());
        self
    }

    /// Contain keyboard focus within this subtree while it is mounted and topmost.
    ///
    /// Nested traps are resolved by overlay plane, `z_index`, and declaration order. Only the
    /// topmost trap participates in Tab traversal or programmatic framework focus; mounting one
    /// moves focus to its first enabled Tab stop (or the trap root when it is focusable), and
    /// removing a focused child keeps focus inside the remaining trap. The marker retains no
    /// observer, timer, task, or idle scheduler source.
    pub fn focus_trap(mut self) -> Self {
        self.focus_trap = true;
        self
    }

    /// Restore the element that owned focus before this surface mounted when it unmounts.
    ///
    /// The retained UI tree captures the target once per stable surface identity, so rebuilds do
    /// not replace it with a descendant focus target. Nested surfaces restore in stack order.
    pub fn restore_previous_focus(mut self) -> Self {
        self.restore_previous_focus = true;
        self
    }

    /// Attach contextual keymap properties to this node in the focused ancestor path.
    pub fn key_context(mut self, context: impl Into<KeyContext>) -> Self {
        self.key_context = Some(context.into());
        self
    }

    /// Set keyboard traversal order. Negative values remove the element from Tab traversal.
    pub fn tab_index(mut self, index: i16) -> Self {
        self.tab_index = index;
        self
    }

    /// Prefer this element for initial window focus or when a newly mounted focus trap takes focus.
    pub fn auto_focus(mut self) -> Self {
        self.auto_focus = true;
        self.focusable = true;
        self
    }

    /// Include this node in click hit testing. Clicks arrive as [`crate::Event::Click`].
    pub fn clickable(mut self) -> Self {
        self.clickable = true;
        self.set_implicit_cursor(CursorStyle::PointingHand);
        self.focusable = true;
        if self.accessibility.role == AccessibilityRole::GenericContainer {
            self.accessibility.role = AccessibilityRole::Button;
        }
        self
    }
}

#[track_caller]
fn bounded_hover_group_name(name: Arc<str>) -> Arc<str> {
    assert!(
        !name.is_empty() && name.len() <= MAX_HOVER_GROUP_NAME_BYTES,
        "a group name is one to {MAX_HOVER_GROUP_NAME_BYTES} bytes"
    );
    name
}
