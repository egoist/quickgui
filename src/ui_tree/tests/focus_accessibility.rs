use super::*;

#[test]
fn tab_order_prioritizes_positive_indices_and_skips_disabled_controls() {
    let mut root = div()
        .child(button().id(10_u64).clickable().tab_index(2))
        .child(button().id(20_u64).clickable())
        .child(button().id(30_u64).clickable().tab_index(1))
        .child(button().id(40_u64).clickable().disabled(true));
    assign_runtime_ids(&mut root);

    let mut tree = UiTree::new();
    tree.root = Some(root);
    tree.rebuild_focus_index();
    assert_eq!(
        tree.focus_order,
        vec![ElementId::new(30), ElementId::new(10), ElementId::new(20)]
    );
    assert!(tree.focus_next(false));
    assert_eq!(tree.focused(), Some(ElementId::new(30)));
    assert!(tree.focus_next(true));
    assert_eq!(tree.focused(), Some(ElementId::new(20)));
    assert!(!tree.focusable_ids.contains(&ElementId::new(40)));
}

#[test]
fn topmost_focus_trap_contains_focus_and_falls_back_to_the_remaining_layer() {
    let mut root = div()
        .child(button().id(1_u64))
        .child(
            div()
                .id(10_u64)
                .overlay()
                .z_index(1)
                .focus_trap()
                .child(button().id(11_u64)),
        )
        .child(
            div()
                .id(20_u64)
                .overlay()
                .z_index(2)
                .focus_trap()
                .children([button().id(21_u64), button().id(22_u64)]),
        );
    assign_runtime_ids(&mut root);

    let mut tree = UiTree::new();
    tree.root = Some(root);
    tree.rebuild_focus_index();
    assert_eq!(tree.active_focus_trap, Some(ElementId::new(20)));
    assert_eq!(
        tree.focus_order,
        vec![ElementId::new(21), ElementId::new(22)]
    );
    assert_eq!(tree.focused(), Some(ElementId::new(21)));
    assert!(!tree.focus(ElementId::new(1)));
    assert!(!tree.focus(ElementId::new(11)));
    assert!(tree.focus_next(false));
    assert_eq!(tree.focused(), Some(ElementId::new(22)));
    assert!(tree.focus_next(false));
    assert_eq!(tree.focused(), Some(ElementId::new(21)));

    let mut root = div().child(button().id(1_u64)).child(
        div()
            .id(10_u64)
            .overlay()
            .z_index(1)
            .focus_trap()
            .child(button().id(11_u64)),
    );
    assign_runtime_ids(&mut root);
    tree.root = Some(root);
    tree.rebuild_focus_index();
    assert_eq!(tree.active_focus_trap, Some(ElementId::new(10)));
    assert_eq!(tree.focus_order, vec![ElementId::new(11)]);
    assert_eq!(tree.focused(), Some(ElementId::new(11)));
}

#[test]
fn focus_trap_prefers_autofocus_and_restores_the_previously_focused_control() {
    fn declaration(open: bool) -> Element {
        let mut root = div()
            .size(320.0, 200.0)
            .child(button().id("outside-control"));
        if open {
            root = root.child(
                div()
                    .id("modal-root")
                    .overlay()
                    .inset_0()
                    .focus_trap()
                    .restore_previous_focus()
                    .children([
                        button().id("modal-close"),
                        crate::text_area("").id("modal-prompt").auto_focus(),
                    ]),
            );
        }
        root
    }

    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(
        declaration(false),
        Size::new(320.0, 200.0),
        1.0,
        &mut renderer,
    )
    .unwrap();
    assert!(tree.focus("outside-control".into()));

    tree.set_root(
        declaration(true),
        Size::new(320.0, 200.0),
        1.0,
        &mut renderer,
    )
    .unwrap();
    assert_eq!(tree.focused(), Some("modal-prompt".into()));

    assert!(tree.focus("modal-close".into()));
    tree.set_root(
        declaration(true),
        Size::new(320.0, 200.0),
        1.0,
        &mut renderer,
    )
    .unwrap();
    assert_eq!(tree.focused(), Some("modal-close".into()));

    tree.set_root(
        declaration(false),
        Size::new(320.0, 200.0),
        1.0,
        &mut renderer,
    )
    .unwrap();
    assert_eq!(tree.focused(), Some("outside-control".into()));
}

#[test]
fn focused_path_retains_scopes_and_contexts_without_making_them_tab_stops() {
    let mut root = div()
        .focus_scope(FocusHandle::new(10_u64))
        .key_context("Workspace")
        .child(
            div()
                .focus_scope(FocusHandle::new(20_u64))
                .key_context("Pane active=true")
                .child(
                    div()
                        .track_focus(FocusHandle::new(30_u64))
                        .key_context("Editor mode=insert"),
                ),
        );
    assign_runtime_ids(&mut root);

    let mut tree = UiTree::new();
    tree.root = Some(root);
    tree.rebuild_focus_index();
    tree.rebuild_dispatch_index();
    assert!(tree.focus(ElementId::new(30)));
    assert_eq!(
        tree.focus_path(),
        vec![ElementId::new(10), ElementId::new(20), ElementId::new(30)]
    );
    assert!(tree.focus_path().contains(&ElementId::new(10)));
    assert!(!tree.focusable_ids.contains(&ElementId::new(10)));
    let contexts = tree.key_context_stack();
    assert_eq!(contexts.len(), 3);
    assert_eq!(contexts[1].value("active"), Some("true"));
    assert_eq!(contexts[2].value("mode"), Some("insert"));
}

#[test]
fn controls_derive_accessible_names_from_text_children() {
    let control = button().child("Save changes");
    assert_eq!(
        accessibility_label(&control).as_deref(),
        Some("Save changes")
    );
}

#[test]
fn controls_derive_accessible_names_from_styled_text_children() {
    let control = button().child(
        crate::styled_text("Save changes")
            .with_highlights([(0..4, crate::HighlightStyle::default().font_bold())]),
    );
    assert_eq!(
        accessibility_label(&control).as_deref(),
        Some("Save changes")
    );
}

#[test]
fn accessibility_hidden_suppresses_only_the_native_semantic_subtree() {
    let input_id = ElementId::new(31);
    let visual_list_id = ElementId::new(32);
    let visual_option_id = ElementId::new(33);
    let proxy_list_id = ElementId::new(34);
    let proxy_option_id = ElementId::new(35);
    let dangling_control_id = ElementId::new(36);
    let root = div().children([
        text_input("ap")
            .id(input_id)
            .accessibility_role(AccessibilityRole::EditableComboBox)
            .accessibility_controls(proxy_list_id)
            .accessibility_active_descendant(proxy_option_id),
        div()
            .id(visual_list_id)
            .size(120.0, 48.0)
            .accessibility_hidden(true)
            .child(
                button()
                    .id(visual_option_id)
                    .size(120.0, 24.0)
                    .clickable()
                    .child("Visual Apple"),
            ),
        div()
            .id(proxy_list_id)
            .size(0.0, 0.0)
            .accessibility_role(AccessibilityRole::ListBox)
            .child(
                div()
                    .id(proxy_option_id)
                    .accessibility_role(AccessibilityRole::ListBoxOption)
                    .child("Apple"),
            ),
        button()
            .id(dangling_control_id)
            .accessibility_controls(visual_option_id)
            .child("Hidden relation target"),
    ]);
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(320.0, 200.0), 1.0, &mut renderer)
        .unwrap();
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();

    assert!(tree.clickable_ids.contains(&visual_option_id));
    assert!(tree.element_bounds(visual_option_id).is_some());

    let update = tree.accessibility_update("Accessibility portal");
    let node = |id| {
        update
            .nodes
            .iter()
            .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
    };
    assert!(node(visual_list_id).is_none());
    assert!(node(visual_option_id).is_none());
    assert_eq!(
        tree.accessibility_element(accessibility_id(visual_option_id)),
        None
    );
    assert_eq!(
        node(input_id).expect("owner input").controls(),
        &[accessibility_id(proxy_list_id)]
    );
    assert_eq!(
        node(input_id).expect("owner input").active_descendant(),
        Some(accessibility_id(proxy_option_id))
    );
    assert!(node(proxy_list_id).is_some());
    assert!(node(proxy_option_id).is_some());
    assert!(
        node(dangling_control_id)
            .expect("visible control")
            .controls()
            .is_empty()
    );
}

#[test]
fn accessibility_hidden_text_does_not_leak_into_a_parent_name() {
    let control = button()
        .child("Visible label")
        .child(div().accessibility_hidden(true).child("Private duplicate"));
    assert_eq!(
        accessibility_label(&control).as_deref(),
        Some("Visible label")
    );
}

#[test]
fn selection_controls_project_exact_roles_toggle_states_and_actions() {
    let checkbox_id = ElementId::new(41);
    let radio_id = ElementId::new(42);
    let switch_id = ElementId::new(43);
    let disabled_id = ElementId::new(44);
    let checkbox = crate::Checkbox::new(ToggleState::Mixed);
    let radio = crate::Radio::new(true);
    let switch = crate::Switch::new(false);
    let root = div().children([
        checkbox
            .root_with(
                div()
                    .child(checkbox.indicator_with(div().child("decorative mixed mark")))
                    .child("Partial selection"),
            )
            .id(checkbox_id)
            .accessibility_description("Some files are selected"),
        radio
            .root_with(
                div()
                    .child(radio.indicator_with(div().child("decorative radio dot")))
                    .child("Selected radio"),
            )
            .id(radio_id),
        switch
            .root_with(
                div()
                    .child(switch.thumb_with(div().child("decorative switch thumb")))
                    .child("Inactive switch"),
            )
            .id(switch_id),
        crate::checkbox(false)
            .id(disabled_id)
            .disabled(true)
            .child("Unavailable"),
    ]);
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(320.0, 200.0), 1.0, &mut renderer)
        .unwrap();
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let update = tree.accessibility_update("Selection controls");
    let node = |id| {
        update
            .nodes
            .iter()
            .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
            .expect("selection accessibility node")
    };

    assert_eq!(node(checkbox_id).role(), Role::CheckBox);
    assert_eq!(
        node(checkbox_id).toggled(),
        Some(AccessibilityToggled::Mixed)
    );
    assert_eq!(node(checkbox_id).label(), Some("Partial selection"));
    assert_eq!(
        node(checkbox_id).description(),
        Some("Some files are selected")
    );
    assert!(node(checkbox_id).supports_action(Action::Click));
    assert_eq!(node(radio_id).role(), Role::RadioButton);
    assert_eq!(node(radio_id).toggled(), Some(AccessibilityToggled::True));
    assert_eq!(node(radio_id).label(), Some("Selected radio"));
    assert_eq!(node(switch_id).role(), Role::Switch);
    assert_eq!(node(switch_id).toggled(), Some(AccessibilityToggled::False));
    assert_eq!(node(switch_id).label(), Some("Inactive switch"));
    assert!(node(disabled_id).is_disabled());
    assert!(!node(disabled_id).supports_action(Action::Click));
    assert!(!node(disabled_id).supports_action(Action::Focus));
}

#[test]
fn separators_and_mounted_group_labels_project_exact_native_semantics() {
    let group_id = ElementId::new(45);
    let label_id = ElementId::new(46);
    let separator_id = ElementId::new(47);
    let self_label_id = ElementId::new(48);
    let root = div().children([
        div()
            .id(group_id)
            .accessibility_role(AccessibilityRole::Group)
            .accessibility_labelled_by(label_id)
            .children([
                div()
                    .id(label_id)
                    .accessibility_role(AccessibilityRole::Label)
                    .accessibility_label("File"),
                div().child("Open"),
            ]),
        div()
            .id(separator_id)
            .accessibility_role(AccessibilityRole::Separator),
        div()
            .id(self_label_id)
            .accessibility_role(AccessibilityRole::Group)
            .accessibility_labelled_by(self_label_id),
    ]);
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(320.0, 200.0), 1.0, &mut renderer)
        .unwrap();
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let update = tree.accessibility_update("Menu structure");
    let node = |id| {
        update
            .nodes
            .iter()
            .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
            .expect("menu structure accessibility node")
    };

    assert_eq!(node(label_id).role(), Role::Label);
    assert_eq!(node(group_id).labelled_by(), &[accessibility_id(label_id)]);
    assert_eq!(node(separator_id).role(), Role::Splitter);
    assert!(!node(separator_id).supports_action(Action::Click));
    assert!(!node(separator_id).supports_action(Action::Focus));
    assert!(node(self_label_id).labelled_by().is_empty());
}

#[test]
fn popover_trigger_projects_expansion_popover_kind_and_mounted_control_relation() {
    let trigger_id = ElementId::new(51);
    let surface_id = ElementId::new(52);
    let popover =
        crate::Popover::new(trigger_id, surface_id, true).kind(crate::PopoverKind::Dialog);
    let root = div()
        .child(popover.trigger().child("Open details"))
        .child(
            popover.positioner_with(div().child(popover.popup_with(div()).children([
                popover.title_with(text("Details")),
                popover.description_with(text("More information about this item.")),
            ]))),
        );
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(420.0, 260.0), 1.0, &mut renderer)
        .unwrap();
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let update = tree.accessibility_update("Popover");
    let node = |id| {
        update
            .nodes
            .iter()
            .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
            .expect("popover accessibility node")
    };

    let trigger = node(trigger_id);
    assert_eq!(trigger.role(), Role::Button);
    assert_eq!(trigger.is_expanded(), Some(true));
    assert_eq!(trigger.has_popup(), Some(AccessibilityHasPopover::Dialog));
    assert_eq!(trigger.controls(), &[accessibility_id(surface_id)]);
    let popover_node = node(surface_id);
    assert_eq!(popover_node.role(), Role::Dialog);
    assert!(popover_node.supports_action(Action::Focus));
    assert_eq!(
        popover_node.labelled_by(),
        &[accessibility_id(popover.title_id())]
    );
    assert_eq!(
        popover_node.described_by(),
        &[accessibility_id(popover.description_id())]
    );
}

#[test]
fn alert_dialog_projects_modal_role_and_mounted_title_description_relations() {
    let dialog = crate::Dialog::alert("delete-dialog", true);
    let root = dialog.root_with(div()).children([
        dialog.backdrop_with(div()),
        dialog.popup_with(div()).children([
            dialog.title_with(text("Delete file?")),
            dialog.description_with(text("This cannot be undone.")),
        ]),
    ]);
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(420.0, 260.0), 1.0, &mut renderer)
        .unwrap();
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let update = tree.accessibility_update("Alert dialog");
    let node = |id| {
        update
            .nodes
            .iter()
            .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
            .expect("dialog accessibility node")
    };

    let popover = node(dialog.popover_id());
    assert_eq!(popover.role(), Role::AlertDialog);
    assert!(popover.is_modal());
    assert_eq!(
        popover.labelled_by(),
        &[accessibility_id(dialog.title_id())]
    );
    assert_eq!(
        popover.described_by(),
        &[accessibility_id(dialog.description_id())]
    );
    assert_eq!(node(dialog.title_id()).label(), Some("Delete file?"));
    assert_eq!(
        node(dialog.description_id()).label(),
        Some("This cannot be undone.")
    );
}

#[test]
fn comboboxes_project_editability_completion_options_and_active_descendants() {
    let editable_id = ElementId::new(81);
    let listbox_id = ElementId::new(82);
    let option_id = ElementId::new(83);
    let select_id = ElementId::new(84);
    let root = div().children([
        text_input("ap")
            .track_focus(FocusHandle::new(editable_id))
            .accessibility_role(AccessibilityRole::EditableComboBox)
            .accessibility_label("Fruit")
            .accessibility_expanded(true)
            .accessibility_has_popover(AccessibilityPopover::ListBox)
            .accessibility_auto_complete(AccessibilityAutoComplete::List)
            .accessibility_controls(listbox_id)
            .accessibility_active_descendant(option_id),
        div()
            .id(listbox_id)
            .accessibility_role(AccessibilityRole::ListBox)
            .accessibility_size_of_set(2)
            .child(
                button()
                    .id(option_id)
                    .tab_index(-1)
                    .accessibility_role(AccessibilityRole::ListBoxOption)
                    .accessibility_position_in_set(0)
                    .selected(true)
                    .child("Apple"),
            ),
        button()
            .id(select_id)
            .accessibility_role(AccessibilityRole::ComboBox)
            .accessibility_label("Letter")
            .accessibility_value("Gamma")
            .accessibility_expanded(false)
            .accessibility_has_popover(AccessibilityPopover::ListBox)
            .child("Gamma"),
    ]);
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(640.0, 320.0), 1.0, &mut renderer)
        .unwrap();
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let update = tree.accessibility_update("Comboboxes");
    let node = |id| {
        update
            .nodes
            .iter()
            .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
            .expect("combobox accessibility node")
    };

    let editable = node(editable_id);
    assert_eq!(editable.role(), Role::EditableComboBox);
    assert_eq!(
        editable.auto_complete(),
        Some(NativeAccessibilityAutoComplete::List)
    );
    assert_eq!(editable.is_expanded(), Some(true));
    assert_eq!(editable.has_popup(), Some(AccessibilityHasPopover::Listbox));
    assert_eq!(editable.controls(), &[accessibility_id(listbox_id)]);
    assert_eq!(
        editable.active_descendant(),
        Some(accessibility_id(option_id))
    );
    assert_eq!(node(listbox_id).role(), Role::ListBox);
    assert_eq!(node(listbox_id).size_of_set(), Some(2));
    assert_eq!(node(option_id).role(), Role::ListBoxOption);
    assert_eq!(node(option_id).position_in_set(), Some(0));
    assert_eq!(node(option_id).is_selected(), Some(true));
    assert_eq!(node(select_id).role(), Role::ComboBox);
    assert_eq!(node(select_id).value(), Some("Gamma"));
    assert_eq!(node(select_id).is_expanded(), Some(false));
}

#[test]
fn virtual_collections_project_active_items_counts_indices_levels_and_sorting() {
    let grid_id = ElementId::new(61);
    let header_id = ElementId::new(62);
    let row_id = ElementId::new(63);
    let cell_id = ElementId::new(64);
    let tree_id = ElementId::new(71);
    let tree_item_id = ElementId::new(72);
    let root = div().children([
        div()
            .id(grid_id)
            .track_focus(FocusHandle::new(grid_id))
            .accessibility_role(AccessibilityRole::Grid)
            .accessibility_row_count(100_001)
            .accessibility_column_count(3)
            .accessibility_active_descendant(cell_id)
            .child(
                div()
                    .id(header_id)
                    .accessibility_role(AccessibilityRole::ColumnHeader)
                    .accessibility_column_index(1)
                    .accessibility_sort_direction(AccessibilitySortDirection::Ascending)
                    .child("Status"),
            )
            .child(
                div()
                    .id(row_id)
                    .accessibility_role(AccessibilityRole::Row)
                    .accessibility_row_index(50_001)
                    .selected(true)
                    .child(
                        div()
                            .id(cell_id)
                            .accessibility_role(AccessibilityRole::GridCell)
                            .accessibility_row_index(50_001)
                            .accessibility_column_index(1)
                            .selected(true)
                            .child("Running"),
                    ),
            ),
        div()
            .id(tree_id)
            .track_focus(FocusHandle::new(tree_id))
            .accessibility_role(AccessibilityRole::Tree)
            .accessibility_size_of_set(2)
            .accessibility_active_descendant(tree_item_id)
            .child(
                div()
                    .id(tree_item_id)
                    .accessibility_role(AccessibilityRole::TreeItem)
                    .accessibility_level(2)
                    .accessibility_position_in_set(3)
                    .accessibility_size_of_set(5)
                    .accessibility_expanded(true)
                    .selected(true)
                    .child("Sources"),
            ),
    ]);
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(640.0, 320.0), 1.0, &mut renderer)
        .unwrap();
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let update = tree.accessibility_update("Collections");
    let node = |id| {
        update
            .nodes
            .iter()
            .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
            .expect("collection accessibility node")
    };

    assert_eq!(node(grid_id).role(), Role::Grid);
    assert_eq!(node(grid_id).row_count(), Some(100_001));
    assert_eq!(node(grid_id).column_count(), Some(3));
    assert_eq!(
        node(grid_id).active_descendant(),
        Some(accessibility_id(cell_id))
    );
    assert_eq!(node(header_id).role(), Role::ColumnHeader);
    assert_eq!(node(header_id).column_index(), Some(1));
    assert_eq!(
        node(header_id).sort_direction(),
        Some(NativeAccessibilitySortDirection::Ascending)
    );
    assert_eq!(node(row_id).role(), Role::Row);
    assert_eq!(node(row_id).row_index(), Some(50_001));
    assert_eq!(node(cell_id).role(), Role::GridCell);
    assert_eq!(node(cell_id).column_index(), Some(1));
    assert_eq!(node(tree_id).role(), Role::Tree);
    assert_eq!(node(tree_id).size_of_set(), Some(2));
    assert_eq!(
        node(tree_id).active_descendant(),
        Some(accessibility_id(tree_item_id))
    );
    assert_eq!(node(tree_item_id).role(), Role::TreeItem);
    assert_eq!(node(tree_item_id).level(), Some(2));
    assert_eq!(node(tree_item_id).position_in_set(), Some(3));
    assert_eq!(node(tree_item_id).size_of_set(), Some(5));
    assert_eq!(node(tree_item_id).is_expanded(), Some(true));
}

#[test]
fn ime_notifications_compare_commits_against_the_preedit_backup() {
    let id = ElementId::new(44);
    let mut tree = UiTree::new();
    tree.focused = Some(id);
    tree.text_inputs.insert(
        id,
        TextInputState::with_constraints(
            "hello ",
            false,
            crate::element::InputConstraints::default(),
        ),
    );

    let preedit = tree.input_preedit("你", Some((3, 3)));
    assert!(preedit.repaint);
    assert!(preedit.change.is_none());

    let commit = tree.input_replace("你");
    assert!(commit.repaint);
    assert_eq!(
        commit.change.as_ref().map(|change| change.value.as_ref()),
        Some("hello 你")
    );
}

#[test]
fn invalid_controls_expose_native_state_and_validation_description() {
    let id = ElementId::new(45);
    let mut root = crate::text_input("")
        .id(id)
        .required(true)
        .invalid(true)
        .validation_message("A value is required");
    assign_runtime_ids(&mut root);

    let mut tree = UiTree::new();
    tree.viewport = Size::new(320.0, 100.0);
    tree.element_bounds
        .insert(id, Rect::new(10.0, 10.0, 240.0, 40.0));
    tree.text_inputs.insert(
        id,
        TextInputState::with_constraints("", false, crate::element::InputConstraints::default()),
    );
    tree.root = Some(root);
    tree.rebuild_focus_index();
    tree.rebuild_dispatch_index();
    assert!(tree.focus(id));
    assert!(tree.focused_text_input_is_invalid());

    let update = tree.accessibility_update("Input test");
    let node = update
        .nodes
        .iter()
        .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
        .expect("input accessibility node");
    assert_eq!(node.invalid(), Some(AccessibilityInvalid::True));
    assert!(node.is_required());
    assert_eq!(node.description(), Some("A value is required"));
}

#[test]
fn password_inputs_mask_scene_and_accessibility_values_and_disable_copy() {
    let id = ElementId::new(46);
    let secret = "sk-é👨‍👩‍👧‍👦";
    let masked = PASSWORD_MASK.repeat(secret.graphemes(true).count());
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(
        crate::text_input(secret).id(id).password(true),
        Size::new(320.0, 100.0),
        1.0,
        &mut renderer,
    )
    .unwrap();
    assert!(tree.focus(id));
    assert!(tree.input_select_all().repaint);

    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let input_text = scene
        .text_runs()
        .iter()
        .find(|run| run.id == TextId::new(id.value()))
        .expect("password text run");
    assert_eq!(input_text.content.as_ref(), masked);
    assert!(!input_text.content.contains(secret));
    assert!(tree.selected_input_text().is_none());

    let update = tree.accessibility_update("Password test");
    let node = update
        .nodes
        .iter()
        .find_map(|(node_id, node)| (*node_id == accessibility_id(id)).then_some(node))
        .expect("password accessibility node");
    assert_eq!(node.role(), Role::PasswordInput);
    assert_eq!(node.value(), Some(masked.as_str()));

    let display = PasswordDisplay::new(secret);
    for boundary in secret
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .chain([secret.len()])
    {
        assert_eq!(
            display.source_index(display.display_index(boundary)),
            boundary
        );
    }
}

#[test]
fn invalid_form_reports_in_document_order_focuses_and_announces_once() {
    let form_id = ElementId::new(100);
    let first_id = ElementId::new(101);
    let disabled_id = ElementId::new(102);
    let nested_form_id = ElementId::new(103);
    let nested_id = ElementId::new(104);
    let mut root = form().id(form_id).children([
        crate::text_input("")
            .id(first_id)
            .invalid(true)
            .validation_message("Name is required"),
        crate::text_input("")
            .id(disabled_id)
            .invalid(true)
            .validation_message("Disabled does not participate")
            .disabled(true),
        form().id(nested_form_id).child(
            crate::text_input("")
                .id(nested_id)
                .invalid(true)
                .validation_message("Nested form owns this field"),
        ),
    ]);
    assign_runtime_ids(&mut root);

    let mut tree = UiTree::new();
    tree.root = Some(root);
    for id in [first_id, disabled_id, nested_id] {
        tree.text_inputs.insert(
            id,
            TextInputState::with_constraints("", false, InputConstraints::default()),
        );
    }
    tree.rebuild_focus_index();
    tree.rebuild_dispatch_index();

    let Some(FormAttempt::Invalid(report)) = tree.attempt_form_submission(form_id, Some(first_id))
    else {
        panic!("outer form should be invalid");
    };
    assert_eq!(report.form(), form_id);
    assert_eq!(report.trigger(), Some(first_id));
    assert_eq!(report.issues().len(), 1);
    assert_eq!(report.issues()[0].id(), first_id);
    assert_eq!(report.issues()[0].message(), Some("Name is required"));
    assert_eq!(tree.focused(), Some(first_id));

    let first_alert = tree
        .validation_announcement
        .as_ref()
        .expect("validation announcement")
        .node;
    let update = tree.accessibility_update("Form test");
    let alert = update
        .nodes
        .iter()
        .find_map(|(id, node)| (*id == first_alert).then_some(node))
        .expect("assertive validation alert");
    assert_eq!(alert.role(), Role::Alert);
    assert_eq!(alert.live(), Some(Live::Assertive));
    assert_eq!(alert.value(), Some("Name is required"));

    let Some(FormAttempt::Invalid(_)) = tree.attempt_form_submission(form_id, Some(first_id))
    else {
        panic!("repeated attempt remains invalid");
    };
    assert_ne!(
        tree.validation_announcement.as_ref().unwrap().node,
        first_alert,
        "a repeated identical message must still create an accessibility change"
    );
}

#[test]
fn valid_form_shares_bounded_controlled_values_and_skips_nested_forms() {
    let form_id = ElementId::new(110);
    let name_id = ElementId::new(111);
    let notes_id = ElementId::new(112);
    let disabled_id = ElementId::new(113);
    let nested_form_id = ElementId::new(114);
    let nested_id = ElementId::new(115);
    let mut root = form().id(form_id).children([
        crate::text_input("Ada").id(name_id),
        crate::text_area("Notes").id(notes_id),
        crate::text_input("Ignored").id(disabled_id).disabled(true),
        form()
            .id(nested_form_id)
            .child(crate::text_input("Nested").id(nested_id)),
    ]);
    assign_runtime_ids(&mut root);

    let mut tree = UiTree::new();
    tree.root = Some(root);
    for (id, value, multiline) in [
        (name_id, "Ada", false),
        (notes_id, "Notes", true),
        (disabled_id, "Ignored", false),
        (nested_id, "Nested", false),
    ] {
        tree.text_inputs.insert(
            id,
            TextInputState::with_constraints(value, multiline, InputConstraints::default()),
        );
    }
    tree.rebuild_focus_index();
    tree.rebuild_dispatch_index();

    let Some(FormAttempt::Valid(event)) = tree.attempt_form_submission(form_id, Some(name_id))
    else {
        panic!("outer form should submit");
    };
    assert_eq!(event.form(), form_id);
    assert_eq!(event.trigger(), Some(name_id));
    assert_eq!(event.fields().len(), 2);
    assert_eq!(event.value(name_id), Some("Ada"));
    assert_eq!(event.value(notes_id), Some("Notes"));
    assert_eq!(event.value(disabled_id), None);
    assert_eq!(event.value(nested_id), None);
    assert!(!event.is_truncated());
}

#[test]
fn no_wrap_flex_text_skips_intrinsic_shaping_during_layout() {
    let layout = TaffyStyle {
        flex_basis: Dimension::length(0.0),
        min_size: TaffySize {
            width: Dimension::length(0.0),
            height: Dimension::auto(),
        },
        size: TaffySize {
            width: Dimension::auto(),
            height: Dimension::length(20.0),
        },
        ..TaffyStyle::default()
    };
    let known = TaffySize {
        width: None,
        height: None,
    };
    let max_content = TaffySize {
        width: AvailableSpace::MaxContent,
        height: AvailableSpace::MaxContent,
    };
    let text = TextStyle::new(14.0, Color::WHITE).wrap(TextWrap::None);

    assert_eq!(
        fixed_text_layout_size(known, max_content, &layout, &text),
        Some(TaffySize {
            width: 0.0,
            height: 20.0,
        })
    );
    assert_eq!(
        fixed_text_layout_size(
            known,
            TaffySize {
                width: AvailableSpace::Definite(640.0),
                height: AvailableSpace::MaxContent,
            },
            &layout,
            &text,
        ),
        Some(TaffySize {
            width: 640.0,
            height: 20.0,
        })
    );
}

#[test]
fn wrapped_flex_text_keeps_intrinsic_measurement() {
    let layout = TaffyStyle {
        flex_basis: Dimension::length(0.0),
        min_size: TaffySize {
            width: Dimension::length(0.0),
            height: Dimension::auto(),
        },
        size: TaffySize {
            width: Dimension::auto(),
            height: Dimension::length(20.0),
        },
        ..TaffyStyle::default()
    };
    let text = TextStyle::new(14.0, Color::WHITE).wrap(TextWrap::Word);

    assert_eq!(
        fixed_text_layout_size(
            TaffySize {
                width: None,
                height: None,
            },
            TaffySize {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
            &layout,
            &text,
        ),
        None
    );
}

#[test]
fn editable_text_ignores_inherited_display_only_overflow() {
    let root = div()
        .line_clamp(2)
        .text_ellipsis_middle()
        .child(text_input("complete editable value"));
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(320.0, 180.0), 1.0, &mut renderer)
        .unwrap();

    let input = &tree.root.as_ref().unwrap().children[0];
    assert_eq!(input.resolved_typography.wrap, TextWrap::None);
    assert!(input.resolved_typography.text_overflow.is_none());
    assert!(input.resolved_typography.line_clamp.is_none());
}

#[test]
fn anchored_placement_keeps_the_preferred_side_when_it_fits() {
    let placed = place_anchored(
        Rect::new(50.0, 50.0, 30.0, 20.0),
        Size::new(80.0, 40.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        AnchorPlacement::BottomStart,
        8.0,
        8.0,
    );
    assert_eq!(placed, Rect::new(50.0, 78.0, 80.0, 40.0));
}

#[test]
fn anchored_placement_flips_before_it_shifts() {
    let placed = place_anchored(
        Rect::new(70.0, 150.0, 30.0, 20.0),
        Size::new(80.0, 48.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        AnchorPlacement::BottomStart,
        8.0,
        8.0,
    );
    assert_eq!(placed, Rect::new(70.0, 94.0, 80.0, 48.0));
}

#[test]
fn anchored_placement_tries_opposite_alignment_near_an_edge() {
    let placed = place_anchored(
        Rect::new(210.0, 40.0, 20.0, 24.0),
        Size::new(96.0, 40.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        AnchorPlacement::BottomStart,
        8.0,
        8.0,
    );
    assert_eq!(placed, Rect::new(134.0, 72.0, 96.0, 40.0));
}

#[test]
fn oversized_anchored_surfaces_pin_to_the_viewport_margin() {
    let placed = place_anchored(
        Rect::new(80.0, 70.0, 20.0, 20.0),
        Size::new(300.0, 220.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        AnchorPlacement::Right,
        8.0,
        8.0,
    );
    assert_eq!(placed, Rect::new(8.0, 8.0, 300.0, 220.0));
}

#[test]
fn anchored_placement_reports_the_side_alignment_and_room_it_actually_used() {
    let fits = resolve_anchored(
        Rect::new(50.0, 50.0, 30.0, 20.0),
        Size::new(80.0, 40.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        AnchorGeometry {
            placement: AnchorPlacement::BottomStart,
            gap: 8.0,
            align_offset: 0.0,
            margin: 8.0,
            sticky: true,
        },
    );
    assert_eq!(fits.placement, AnchorPlacement::BottomStart);
    assert_eq!(fits.bounds, Rect::new(50.0, 78.0, 80.0, 40.0));
    assert_eq!(fits.anchor, Rect::new(50.0, 50.0, 30.0, 20.0));
    // 172 (inner bottom) - 70 (anchor bottom) - 8 (gap) on the primary axis, inner width across.
    assert_eq!(fits.available, Size::new(224.0, 94.0));
    assert!(!fits.anchor_hidden);

    // A flip reports the side it landed on, not the side that was asked for.
    let flipped = resolve_anchored(
        Rect::new(70.0, 150.0, 30.0, 20.0),
        Size::new(80.0, 48.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        AnchorGeometry {
            placement: AnchorPlacement::BottomStart,
            gap: 8.0,
            align_offset: 0.0,
            margin: 8.0,
            sticky: true,
        },
    );
    assert_eq!(flipped.placement, AnchorPlacement::TopStart);
    assert_eq!(flipped.bounds, Rect::new(70.0, 94.0, 80.0, 48.0));
    assert_eq!(flipped.available, Size::new(224.0, 134.0));

    // An alternate alignment is reported the same way.
    let realigned = resolve_anchored(
        Rect::new(210.0, 40.0, 20.0, 24.0),
        Size::new(96.0, 40.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        AnchorGeometry {
            placement: AnchorPlacement::BottomStart,
            gap: 8.0,
            align_offset: 0.0,
            margin: 8.0,
            sticky: true,
        },
    );
    assert_eq!(realigned.placement, AnchorPlacement::BottomEnd);

    // An anchor that scrolled out of the window is reported as hidden.
    let hidden = resolve_anchored(
        Rect::new(50.0, 400.0, 30.0, 20.0),
        Size::new(80.0, 40.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        AnchorGeometry {
            placement: AnchorPlacement::BottomStart,
            gap: 8.0,
            align_offset: 0.0,
            margin: 8.0,
            sticky: true,
        },
    );
    assert!(hidden.anchor_hidden);
}

#[test]
fn cross_axis_offsets_shift_before_clamping_and_sticky_placement_can_be_disabled() {
    let geometry = |align_offset: f32, sticky: bool| AnchorGeometry {
        placement: AnchorPlacement::BottomStart,
        gap: 8.0,
        align_offset,
        margin: 8.0,
        sticky,
    };
    let shifted = resolve_anchored(
        Rect::new(50.0, 50.0, 30.0, 20.0),
        Size::new(80.0, 40.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        geometry(12.0, true),
    );
    assert_eq!(shifted.bounds, Rect::new(62.0, 78.0, 80.0, 40.0));

    // A sticky surface is clamped back inside the collision viewport.
    let clamped = resolve_anchored(
        Rect::new(50.0, 50.0, 30.0, 20.0),
        Size::new(80.0, 40.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        geometry(400.0, true),
    );
    assert_eq!(clamped.bounds.x, 152.0);

    // A non-sticky surface stays locked to its anchor and leaves the viewport with it.
    let loose = resolve_anchored(
        Rect::new(50.0, 50.0, 30.0, 20.0),
        Size::new(80.0, 40.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        geometry(400.0, false),
    );
    assert_eq!(loose.bounds.x, 450.0);
    assert_eq!(loose.placement, AnchorPlacement::BottomStart);

    // A non-finite offset is ignored rather than poisoning the placement.
    let broken = resolve_anchored(
        Rect::new(50.0, 50.0, 30.0, 20.0),
        Size::new(80.0, 40.0),
        Rect::new(0.0, 0.0, 240.0, 180.0),
        geometry(f32::NAN, true),
    );
    assert_eq!(broken.bounds, Rect::new(50.0, 78.0, 80.0, 40.0));
}

#[test]
fn nested_anchor_uses_the_painted_position_inside_an_anchored_parent() {
    let root = div().size(500.0, 400.0).relative().children([
        div()
            .id("parent-anchor")
            .absolute()
            .left(100.0)
            .top(80.0)
            .size(40.0, 20.0),
        div()
            .id("parent-positioner")
            .anchor_to("parent-anchor", AnchorPlacement::BottomStart)
            .child(
                div()
                    .id("parent-popup")
                    .size(200.0, 200.0)
                    .flex_col()
                    .children([
                        div().h(120.0).flex_none(),
                        div().id("submenu-anchor").w_full().h(30.0).flex_none(),
                        div()
                            .id("submenu-positioner")
                            .anchor_to("submenu-anchor", AnchorPlacement::RightStart)
                            .child(div().id("submenu-popup").size(100.0, 80.0).clickable()),
                    ]),
            ),
    ]);
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(500.0, 400.0), 1.0, &mut renderer)
        .unwrap();
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();

    let anchor = tree.element_bounds("submenu-anchor".into()).unwrap();
    let popup = tree.element_bounds("submenu-positioner".into()).unwrap();
    assert_eq!(anchor, Rect::new(100.0, 228.0, 200.0, 30.0));
    assert_eq!(popup, Rect::new(308.0, 228.0, 100.0, 80.0));

    // The layout-only hover refresh follows the same resolved nesting, so its hit regions do not
    // jump back to the submenu's pre-anchor layout position.
    let point = Point::new(320.0, 240.0);
    tree.refresh_hover_after_layout(Some(point)).unwrap();
    tree.pointer_button(Some(point), true, false, Instant::now(), &mut renderer);
    let released = tree.pointer_button(Some(point), false, false, Instant::now(), &mut renderer);
    assert_eq!(released.clicked, Some("submenu-popup".into()));
}

#[test]
fn scrollbars_reveal_on_scroll_then_hide_with_one_deadline() {
    let mut tree = UiTree::new();
    let id = ElementId::new(7);
    let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
    tree.scroll_regions.push(ScrollRegion {
        rtl: false,
        id,
        bounds,
        scrollbar_bounds: bounds,
        clip: bounds,
        max_offset: Vector::new(0.0, 900.0),
        virtual_scroll: false,
        vertical_scrollbar: true,
        order: PaintOrder {
            layer: PaintLayerKey::default(),
            source: 0,
        },
        scrollbar_order: PaintOrder {
            layer: PaintLayerKey::default(),
            source: 1,
        },
    });

    let now = Instant::now();
    assert!(
        tree.scroll_at(Some(Point::new(50.0, 50.0)), Vector::new(0.0, -120.0), now,)
            .changed
    );
    assert_eq!(tree.scroll_offsets[&id].y, 120.0);
    let deadline = tree
        .next_scrollbar_deadline()
        .expect("scroll reveals thumb");
    assert_eq!(deadline.duration_since(now), SCROLLBAR_AUTO_HIDE_DELAY);
    assert!(!tree.advance_scrollbars(deadline - Duration::from_millis(1)));
    assert!(tree.advance_scrollbars(deadline));
    assert!(!tree.advance_scrollbars(deadline));
    assert!(tree.next_scrollbar_deadline().is_none());
}

#[test]
fn scroll_end_following_pauses_when_the_user_scrolls_away() {
    let id = ElementId::new(91);
    let mut states = HashMap::new();
    let mut offset = Vector::ZERO;

    apply_scroll_end_revision(id, Some(1), 100.0, &mut offset, &mut states);
    assert_eq!(offset.y, 100.0);
    apply_scroll_end_revision(id, Some(2), 160.0, &mut offset, &mut states);
    assert_eq!(offset.y, 160.0);

    offset.y = 40.0;
    apply_scroll_end_revision(id, Some(3), 220.0, &mut offset, &mut states);
    assert_eq!(offset.y, 40.0);

    offset.y = 220.0;
    apply_scroll_end_revision(id, Some(4), 260.0, &mut offset, &mut states);
    assert_eq!(offset.y, 260.0);
    apply_scroll_end_revision(id, None, 260.0, &mut offset, &mut states);
    assert!(!states.contains_key(&id));
}

#[test]
fn tooltips_use_one_exact_show_deadline_and_hide_without_a_loop() {
    let mut tree = UiTree::new();
    let id = ElementId::new(88);
    let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
    tree.tooltips.insert(
        id,
        Tooltip::text("Delayed").delay(Duration::from_millis(500)),
    );
    tree.hit_regions.push(HitRegion {
        transform: None,
        id,
        bounds,
        clip: bounds,
        clickable: false,
        pointer_listener: false,
        drag_source: false,
        drop_target: false,
        focusable: false,
        cursor_style: None,
        cursor_states: CursorStateStyles::default(),
        stateful: false,
        blocks_pointer: false,
        app_region: None,
        order: PaintOrder {
            layer: PaintLayerKey::default(),
            source: 0,
        },
    });

    let now = Instant::now();
    assert!(!tree.update_tooltip_hover(Some(Point::new(10.0, 10.0)), now));
    let deadline = tree.next_tooltip_deadline().expect("show deadline");
    assert_eq!(deadline, now + Duration::from_millis(500));
    assert!(!tree.advance_tooltips(deadline - Duration::from_millis(1)));
    assert!(tree.advance_tooltips(deadline));
    assert_eq!(tree.visible_tooltip, Some(id));
    assert!(tree.next_tooltip_deadline().is_none());
    assert!(!tree.update_tooltip_hover(Some(Point::new(20.0, 20.0)), deadline));
    assert!(tree.update_tooltip_hover(None, deadline));
    assert!(tree.visible_tooltip.is_none());
    assert!(tree.next_tooltip_deadline().is_none());
}

#[test]
fn keyboard_focus_schedules_tooltips_but_pointer_focus_does_not_reopen_them() {
    let mut tree = UiTree::new();
    let id = ElementId::new(89);
    tree.tooltips.insert(id, Tooltip::text("Keyboard help"));
    tree.focusable_ids.insert(id);

    assert!(tree.focus(id));
    assert!(tree.next_tooltip_deadline().is_some());
    assert!(!tree.clear_tooltip());
    tree.focused = None;

    assert!(tree.focus_from_pointer(id));
    assert!(tree.next_tooltip_deadline().is_none());
}

#[test]
fn selectable_text_focuses_its_control_ancestor_without_losing_selection() {
    let terminal_id = ElementId::new(93);
    let text_id = ElementId::new(94);
    let root = div()
        .size(240.0, 40.0)
        .track_focus(FocusHandle::new(terminal_id))
        .child(text("shell output").id(text_id).selectable());
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(240.0, 40.0), 1.0, &mut renderer)
        .unwrap();
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let bounds = tree
        .element_bounds(text_id)
        .expect("selectable text bounds");

    let result = tree.pointer_button(
        Some(Point::new(bounds.x + 2.0, bounds.y + 2.0)),
        true,
        false,
        Instant::now(),
        &mut renderer,
    );

    assert!(result.repaint);
    assert_eq!(tree.focused(), Some(terminal_id));
    assert!(tree.static_text_selection.is_some());
}

#[test]
fn pointer_focus_can_be_disabled_without_disabling_button_activation_or_keyboard_focus() {
    let terminal_id = ElementId::new(95);
    let sidebar_button_id = ElementId::new(96);
    let root = div()
        .size(200.0, 40.0)
        .child(
            button()
                .id(terminal_id)
                .absolute()
                .top(0.0)
                .left(0.0)
                .size(100.0, 40.0)
                .clickable(),
        )
        .child(
            button()
                .id(sidebar_button_id)
                .absolute()
                .top(0.0)
                .left(100.0)
                .size(100.0, 40.0)
                .focus_on_pointer(false)
                .clickable(),
        );
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(200.0, 40.0), 1.0, &mut renderer)
        .unwrap();
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    assert!(tree.focus(terminal_id));

    let point = Point::new(150.0, 20.0);
    tree.pointer_button(Some(point), true, false, Instant::now(), &mut renderer);
    assert_eq!(tree.focused(), Some(terminal_id));
    let release = tree.pointer_button(Some(point), false, false, Instant::now(), &mut renderer);
    assert_eq!(release.clicked, Some(sidebar_button_id));

    assert!(tree.focus(sidebar_button_id));
    assert_eq!(tree.focused(), Some(sidebar_button_id));
}

#[test]
fn pointer_focus_hides_focus_styles_until_keyboard_input_reveals_them() {
    let focus_color = Color::rgb8(0, 0, 255);
    let root = div().size(200.0, 40.0).flex_row().children([
        button()
            .id("first")
            .size(80.0, 32.0)
            .flex_none()
            .clickable()
            .focus(|style| style.bg(focus_color)),
        button()
            .id("second")
            .size(80.0, 32.0)
            .flex_none()
            .clickable()
            .focus(|style| style.bg(focus_color)),
    ]);
    let first = ElementId::named("first");
    let second = ElementId::named("second");
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    let mut scene = Scene::new();
    tree.set_root(root, Size::new(200.0, 40.0), 1.0, &mut renderer)
        .unwrap();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let focus_painted = |scene: &Scene| {
        scene
            .edge_quads()
            .iter()
            .any(|quad| quad.fill == focus_color)
    };
    assert!(!focus_painted(&scene));
    let now = Instant::now();

    // A click focuses the button but paints no focus styles: the pointer shows what was pressed.
    let first_point = Point::new(40.0, 16.0);
    tree.pointer_button(Some(first_point), true, false, now, &mut renderer);
    tree.pointer_button(Some(first_point), false, false, now, &mut renderer);
    assert_eq!(tree.focused(), Some(first));
    assert!(!tree.focus_visible());
    assert_eq!(tree.styled_focus(), None);
    scene.clear(Color::TRANSPARENT);
    tree.paint(&mut scene, &mut renderer).unwrap();
    assert!(!focus_painted(&scene));

    // A programmatic focus outside any input keeps the hidden answer.
    assert!(tree.focus(second));
    assert!(!tree.focus_visible());

    // Tab is keyboard input, so the focus it lands paints its styles.
    assert!(tree.focus_next(false));
    assert_eq!(tree.focused(), Some(first));
    assert!(tree.focus_visible());
    assert_eq!(tree.styled_focus(), Some(first));
    scene.clear(Color::TRANSPARENT);
    tree.paint(&mut scene, &mut renderer).unwrap();
    assert!(focus_painted(&scene));

    // A programmatic focus now keeps the visible answer instead.
    assert!(tree.focus(second));
    assert!(tree.focus_visible());

    // Clicking the keyboard-focused button keeps focus there but hides the styles again.
    let second_point = Point::new(120.0, 16.0);
    tree.pointer_button(Some(second_point), true, false, now, &mut renderer);
    tree.pointer_button(Some(second_point), false, false, now, &mut renderer);
    assert_eq!(tree.focused(), Some(second));
    assert!(!tree.focus_visible());
    scene.clear(Color::TRANSPARENT);
    tree.paint(&mut scene, &mut renderer).unwrap();
    assert!(!focus_painted(&scene));

    // A navigation key that moves nothing still reveals the focus already there, exactly once.
    assert!(tree.reveal_focus());
    assert!(tree.focus_visible());
    assert!(!tree.reveal_focus());
    scene.clear(Color::TRANSPARENT);
    tree.paint(&mut scene, &mut renderer).unwrap();
    assert!(focus_painted(&scene));
}

#[test]
fn input_dispatch_scopes_decide_the_visibility_of_focus_moved_inside_them() {
    let mut root = div()
        .child(button().id(10_u64).clickable())
        .child(button().id(20_u64).clickable());
    assign_runtime_ids(&mut root);
    let mut tree = UiTree::new();
    tree.root = Some(root);
    tree.rebuild_focus_index();
    let first = ElementId::new(10);
    let second = ElementId::new(20);
    assert!(tree.focus_visible());

    // A focus moved while a pointer event is dispatched — a listener's `cx.focus` — hides.
    let pointer = tree.begin_input_dispatch(InputModality::Pointer);
    assert!(tree.focus(first));
    assert!(!tree.focus_visible());
    tree.end_input_dispatch(pointer);

    // Outside any dispatch the current answer is inherited.
    assert!(tree.focus(second));
    assert!(!tree.focus_visible());

    // A focus moved while a key event is dispatched shows, and a click nested inside that key
    // event — Enter activating a button — is still keyboard input.
    let key = tree.begin_input_dispatch(InputModality::Keyboard);
    assert!(tree.focus(first));
    assert!(tree.focus_visible());
    let nested = tree.begin_input_dispatch(InputModality::Pointer);
    assert!(tree.focus(second));
    assert!(tree.focus_visible());
    tree.end_input_dispatch(nested);
    assert!(tree.focus(first));
    assert!(tree.focus_visible());
    tree.end_input_dispatch(key);

    // Re-focusing the visible element from a pointer changes only the paint, which is reported
    // as a change so the window repaints; repeating it changes nothing.
    let pointer = tree.begin_input_dispatch(InputModality::Pointer);
    assert!(tree.focus(first));
    assert!(!tree.focus_visible());
    assert!(!tree.focus(first));
    tree.end_input_dispatch(pointer);

    // A deferred request carries the device that made it past the end of its dispatch, so the
    // focus it lands after the rebuild paints as if the target had already been mounted.
    let key = tree.begin_input_dispatch(InputModality::Keyboard);
    let request = tree.pending_focus(second);
    tree.end_input_dispatch(key);
    assert_eq!(request.modality, Some(InputModality::Keyboard));
    assert!(tree.focus_as(request.element, request.modality));
    assert_eq!(tree.focused(), Some(second));
    assert!(tree.focus_visible());
    assert_eq!(tree.pending_focus(first).modality, None);

    // Dropping focus inside a pointer dispatch leaves a hidden answer for the next programmatic
    // focus; inside a key dispatch a visible one.
    let pointer = tree.begin_input_dispatch(InputModality::Pointer);
    assert!(tree.blur());
    tree.end_input_dispatch(pointer);
    assert!(tree.focus(first));
    assert!(!tree.focus_visible());
    let key = tree.begin_input_dispatch(InputModality::Keyboard);
    assert!(tree.blur());
    tree.end_input_dispatch(key);
    assert!(tree.focus(first));
    assert!(tree.focus_visible());
}

#[test]
fn a_text_input_paints_its_focus_styles_however_it_was_focused() {
    let focus_color = Color::rgb8(0, 128, 0);
    let root = div().size(200.0, 40.0).child(
        text_input("")
            .id("field")
            .size(160.0, 32.0)
            .focus(|style| style.bg(focus_color)),
    );
    let field = ElementId::named("field");
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    let mut scene = Scene::new();
    tree.set_root(root, Size::new(200.0, 40.0), 1.0, &mut renderer)
        .unwrap();
    tree.paint(&mut scene, &mut renderer).unwrap();

    let point = Point::new(20.0, 16.0);
    let now = Instant::now();
    tree.pointer_button(Some(point), true, false, now, &mut renderer);
    tree.pointer_button(Some(point), false, false, now, &mut renderer);
    assert_eq!(tree.focused(), Some(field));
    // The press hid focus, yet a text field is styled like a native one that always shows its
    // ring: the styled focus is the field even though nothing else would paint focus styles now.
    assert!(!tree.focus_visible());
    assert_eq!(tree.styled_focus(), Some(field));
    scene.clear(Color::TRANSPARENT);
    tree.paint(&mut scene, &mut renderer).unwrap();
    assert!(
        scene
            .edge_quads()
            .iter()
            .any(|quad| quad.fill == focus_color)
    );
}
