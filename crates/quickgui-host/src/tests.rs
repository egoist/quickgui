use super::*;
use quickgui::{
    ElementStateStyle, MAX_GROUP_STYLES_PER_ELEMENT, MAX_HOVER_GROUP_NAME_BYTES, Transform2D,
};

mod incremental;

#[test]
fn go_mutation_fixture_uses_native_text_and_color_encoding() {
    let bytes = include_bytes!("../../../go/testdata/text.qgmb");
    let mut tree = NativeTree::default();
    apply_mutations(&mut tree, decode_batch(bytes).unwrap()).unwrap();
    let node = tree.nodes.get(&1).unwrap();
    assert_eq!(node.text.as_ref(), "Hello 世界");
    assert_eq!(
        node.color(property::COLOR),
        Some(Color::rgba8(0x12, 0x34, 0x56, 0x78))
    );
}

#[test]
fn hosted_void_mutations_are_fire_and_forget_host_commands() {
    let host = HostCoordinator::new();
    host.enqueue(HostCommand::Mutation {
        app: 7,
        command: system::SystemCommand::WindowAction {
            window: 11,
            action: system::WindowAction::SetMacOsVibrancy(Some(MacOsVibrancy::Sidebar)),
        },
    })
    .expect("a window mutation fits the host queue");
    host.enqueue(HostCommand::Mutation {
        app: 7,
        command: system::SystemCommand::SetDockBadge(Some("3".to_owned())),
    })
    .expect("an application mutation fits the host queue");

    let mut commands = host.take_commands().expect("the host queue is readable");
    match commands
        .pop_front()
        .expect("the window mutation was queued")
    {
        HostCommand::Mutation {
            app,
            command:
                system::SystemCommand::WindowAction {
                    window,
                    action: system::WindowAction::SetMacOsVibrancy(vibrancy),
                },
        } => {
            assert_eq!(app, 7);
            assert_eq!(window, 11);
            assert_eq!(vibrancy, Some(MacOsVibrancy::Sidebar));
        }
        _ => panic!("the mutation must not be wrapped in a synchronous system command"),
    }
    match commands
        .pop_front()
        .expect("the application mutation was queued")
    {
        HostCommand::Mutation {
            app,
            command: system::SystemCommand::SetDockBadge(value),
        } => {
            assert_eq!(app, 7);
            assert_eq!(value.as_deref(), Some("3"));
        }
        _ => panic!("the mutation must not carry a synchronous reply"),
    }
    assert!(commands.is_empty());
}

#[test]
fn hosted_window_creation_uses_a_preallocated_handle_without_a_reply() {
    let host = HostCoordinator::new();
    let window = host
        .allocate_window()
        .expect("the worker can allocate a hosted window handle");
    host.enqueue(HostCommand::CreateWindow {
        app: 7,
        window,
        options: NativeWindowOptions::default(),
        initial_batch: Vec::new(),
    })
    .expect("window creation fits the host queue");

    let mut commands = host.take_commands().expect("the host queue is readable");
    match commands.pop_front().expect("window creation was queued") {
        HostCommand::CreateWindow {
            app,
            window: queued_window,
            initial_batch,
            ..
        } => {
            assert_eq!(app, 7);
            assert_eq!(queued_window, window);
            assert!(initial_batch.is_empty());
        }
        _ => panic!("hosted creation must use a no-reply command"),
    }
    assert!(commands.is_empty());
}

#[test]
fn app_configuration_updates_preserve_embedded_identity_and_paths() {
    let (info, paths, quit_mode) = native_app_configuration(NativeAppOptions {
        name: Some("QuickGUI Test".to_owned()),
        version: Some("1.2.3".to_owned()),
        identifier: Some("dev.quickgui.binding-test".to_owned()),
        ..NativeAppOptions::default()
    })
    .expect("valid initial application configuration");
    assert_eq!(quit_mode, QuitMode::Default);
    let initial_paths = paths.clone().expect("identity resolves application paths");

    let (info, paths, quit_mode) = update_native_app_configuration(
        info,
        paths,
        quit_mode,
        NativeAppOptions {
            quit_mode: Some("explicit".to_owned()),
            ..NativeAppOptions::default()
        },
    )
    .expect("quit-only configuration update");

    assert_eq!(
        info.as_ref().map(AppInfo::identifier),
        Some("dev.quickgui.binding-test")
    );
    assert_eq!(paths.as_ref(), Some(&initial_paths));
    assert_eq!(quit_mode, QuitMode::Explicit);

    let overridden = PathBuf::from("/tmp/quickgui-binding-test-config");
    let (info, paths, quit_mode) = update_native_app_configuration(
        info,
        paths,
        quit_mode,
        NativeAppOptions {
            config_dir: Some(overridden.to_string_lossy().into_owned()),
            ..NativeAppOptions::default()
        },
    )
    .expect("path-only configuration update");

    assert_eq!(
        info.as_ref().map(AppInfo::identifier),
        Some("dev.quickgui.binding-test")
    );
    assert_eq!(
        paths.as_ref().and_then(AppPaths::config_dir),
        Some(overridden.as_path())
    );
    assert_eq!(quit_mode, QuitMode::Explicit);
}

#[test]
fn window_configuration_maps_every_electron_compatible_macos_vibrancy_type() {
    let materials = [
        ("appearance-based", MacOsVibrancy::AppearanceBased),
        ("titlebar", MacOsVibrancy::Titlebar),
        ("selection", MacOsVibrancy::Selection),
        ("menu", MacOsVibrancy::Menu),
        ("popover", MacOsVibrancy::Popover),
        ("sidebar", MacOsVibrancy::Sidebar),
        ("header", MacOsVibrancy::Header),
        ("sheet", MacOsVibrancy::Sheet),
        ("window", MacOsVibrancy::Window),
        ("hud", MacOsVibrancy::Hud),
        ("fullscreen-ui", MacOsVibrancy::FullscreenUi),
        ("tooltip", MacOsVibrancy::Tooltip),
        ("content", MacOsVibrancy::Content),
        ("under-window", MacOsVibrancy::UnderWindow),
        ("under-page", MacOsVibrancy::UnderPage),
    ];

    for (name, expected) in materials {
        let config = window_config(&NativeWindowOptions {
            vibrancy: Some(name.to_owned()),
            visual_effect_state: Some("inactive".to_owned()),
            ..NativeWindowOptions::default()
        })
        .expect("Electron-compatible vibrancy material maps into the Rust core");
        assert_eq!(config.macos_vibrancy, Some(expected));
        assert_eq!(
            config.macos_visual_effect_state,
            MacOsVisualEffectState::Inactive
        );
    }

    assert!(
        window_config(&NativeWindowOptions {
            vibrancy: Some("glass".to_owned()),
            ..NativeWindowOptions::default()
        })
        .unwrap_err()
        .contains("unknown macOS vibrancy type")
    );
}

struct BatchWriter {
    bytes: Vec<u8>,
    count: u32,
}

impl BatchWriter {
    fn new() -> Self {
        let mut bytes = PROTOCOL_MAGIC.to_vec();
        bytes.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        Self { bytes, count: 0 }
    }

    fn op(&mut self, opcode: u8) {
        self.count += 1;
        self.bytes.push(opcode);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn string(&mut self, value: &str) {
        self.u32(value.len() as u32);
        self.bytes.extend_from_slice(value.as_bytes());
    }

    fn finish(mut self) -> Vec<u8> {
        self.bytes[6..10].copy_from_slice(&self.count.to_le_bytes());
        self.bytes
    }
}

#[test]
fn binary_batch_builds_and_removes_a_tree_atomically() {
    let mut writer = BatchWriter::new();
    writer.op(1);
    writer.u32(1);
    writer.bytes.push(1);
    writer.op(2);
    writer.u32(2);
    writer.string("Hello");
    writer.op(6);
    writer.u32(1);
    writer.u32(2);
    writer.u32(NO_ANCHOR);
    writer.op(6);
    writer.u32(ROOT_NODE);
    writer.u32(1);
    writer.u32(NO_ANCHOR);

    let mut tree = NativeTree::default();
    let revision = apply_mutations(&mut tree, decode_batch(&writer.finish()).unwrap()).unwrap();
    assert_eq!(revision, 1);
    assert_eq!(tree.nodes[&ROOT_NODE].children, [1]);
    assert_eq!(tree.nodes[&1].children, [2]);
    assert_eq!(tree.nodes[&2].text.as_ref(), "Hello");

    let mut remove = BatchWriter::new();
    remove.op(7);
    remove.u32(ROOT_NODE);
    remove.u32(1);
    apply_mutations(&mut tree, decode_batch(&remove.finish()).unwrap()).unwrap();
    assert_eq!(tree.nodes.len(), 1);
    assert!(tree.nodes[&ROOT_NODE].children.is_empty());
}

#[test]
fn initial_window_batch_is_committed_before_the_core_view_opens() {
    let mut writer = BatchWriter::new();
    writer.op(2);
    writer.u32(1);
    writer.string("Ready");
    writer.op(6);
    writer.u32(ROOT_NODE);
    writer.u32(1);
    writer.u32(NO_ANCHOR);

    let tree = native_tree_from_initial_batch(&writer.finish()).unwrap();
    assert_eq!(tree.revision, 1);
    assert_eq!(tree.nodes[&ROOT_NODE].children, [1]);
    assert_eq!(tree.nodes[&1].text.as_ref(), "Ready");
}

#[test]
fn failed_cycle_does_not_mutate_the_committed_tree() {
    let mut tree = NativeTree::default();
    let mut setup = BatchWriter::new();
    for id in [1, 2] {
        setup.op(1);
        setup.u32(id);
        setup.bytes.push(1);
    }
    setup.op(6);
    setup.u32(ROOT_NODE);
    setup.u32(1);
    setup.u32(NO_ANCHOR);
    setup.op(6);
    setup.u32(1);
    setup.u32(2);
    setup.u32(NO_ANCHOR);
    apply_mutations(&mut tree, decode_batch(&setup.finish()).unwrap()).unwrap();
    let before = tree.clone();

    let mut cycle = BatchWriter::new();
    cycle.op(6);
    cycle.u32(2);
    cycle.u32(1);
    cycle.u32(NO_ANCHOR);
    assert!(apply_mutations(&mut tree, decode_batch(&cycle.finish()).unwrap()).is_err());
    assert_eq!(tree.revision, before.revision);
    assert_eq!(
        tree.nodes[&ROOT_NODE].children,
        before.nodes[&ROOT_NODE].children
    );
    assert_eq!(tree.nodes[&1].children, before.nodes[&1].children);
}

#[test]
fn malformed_batch_is_rejected_before_tree_mutation() {
    let error = decode_batch(b"not a batch").unwrap_err();
    assert!(error.to_string().contains("magic"));
}

#[test]
fn equivalent_mutation_batches_do_not_advance_the_render_revision() {
    let mut tree = NativeTree::default();
    assert_eq!(apply_mutations(&mut tree, vec![]).unwrap(), 0);
    let mut create = TreeTransaction::new(&tree);
    create.create(1, NodeTag::Text, Arc::from("Hello")).unwrap();
    create
        .set_property(1, property::COLOR, Some(PropertyValue::Color(0xff0000ff)))
        .unwrap();
    create.insert(ROOT_NODE, 1, None).unwrap();
    let overlay = create.finish().unwrap();
    assert_eq!(commit_overlay(&mut tree, overlay), 1);

    for mutations in [
        vec![],
        vec![Mutation::ReplaceText {
            id: 1,
            text: Arc::from("Hello"),
        }],
        vec![Mutation::SetProperty {
            id: 1,
            key: property::COLOR,
            value: Some(PropertyValue::Color(0xff0000ff)),
        }],
        vec![Mutation::SetProperty {
            id: 1,
            key: property::WIDTH,
            value: None,
        }],
        vec![Mutation::Insert {
            parent: ROOT_NODE,
            child: 1,
            before: None,
        }],
        vec![
            Mutation::ReplaceText {
                id: 1,
                text: Arc::from("Temporary"),
            },
            Mutation::ReplaceText {
                id: 1,
                text: Arc::from("Hello"),
            },
        ],
    ] {
        assert_eq!(apply_mutations(&mut tree, mutations).unwrap(), 1);
    }
    assert_eq!(
        apply_mutations(
            &mut tree,
            vec![Mutation::ReplaceText {
                id: 1,
                text: Arc::from("Changed")
            },]
        )
        .unwrap(),
        2
    );
    assert_eq!(tree.nodes[&1].text.as_ref(), "Changed");
}

fn retained_update_tree() -> NativeTree {
    let mut tree = NativeTree::default();
    let mut transaction = TreeTransaction::new(&tree);
    transaction
        .create(1, NodeTag::Button, Arc::from(""))
        .unwrap();
    transaction
        .set_property(1, property::CLICK_LISTENER, Some(PropertyValue::Bool(true)))
        .unwrap();
    transaction
        .create(2, NodeTag::Text, Arc::from("Before"))
        .unwrap();
    transaction.insert(1, 2, None).unwrap();
    transaction.insert(ROOT_NODE, 1, None).unwrap();
    let overlay = transaction.finish().unwrap();
    commit_overlay(&mut tree, overlay);
    tree
}

#[test]
fn signal_text_and_color_batches_skip_native_view_rebuild_and_keep_click_delivery() {
    let events = Rc::new(RefCell::new(VecDeque::new()));
    let native_view = component_part_view(7, retained_update_tree(), events.clone());
    let tree = native_view.tree.clone();
    let (mut cx, view) = quickgui::TestAppContext::new(native_view).unwrap();
    let window = view.window_handle();
    cx.focus(window, ElementId::new(1)).unwrap();
    events.borrow_mut().clear();
    let renders = cx.render_count(window).unwrap();
    let mutations = vec![
        Mutation::ReplaceText {
            id: 2,
            text: Arc::from("After"),
        },
        Mutation::SetProperty {
            id: 1,
            key: property::COLOR,
            value: Some(PropertyValue::Color(0xff0000ff)),
        },
    ];
    let updates = retained_element_updates(&tree.borrow(), &mutations).unwrap();
    apply_mutations(&mut tree.borrow_mut(), mutations).unwrap();
    assert!(cx.update_elements(window, &updates).unwrap());
    assert_eq!(cx.render_count(window).unwrap(), renders);
    let accessibility = cx.accessibility_update(window).unwrap();
    assert_eq!(
        accessibility
            .nodes
            .iter()
            .find(|(id, _)| id.0 == 1)
            .unwrap()
            .1
            .label(),
        Some("After")
    );
    cx.click(window, ElementId::new(1)).unwrap();
    assert_eq!(cx.render_count(window).unwrap(), renders);
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| event.kind == "click" && event.window == 7 && event.target == 1)
    );
}

#[test]
fn component_and_structural_batches_use_the_declaration_path() {
    let mut tree = retained_update_tree();
    let replace = || Mutation::ReplaceText {
        id: 2,
        text: Arc::from("After"),
    };
    assert!(retained_element_updates(&tree, &[replace()]).is_some());
    for mutation in [
        Mutation::SetProperty {
            id: 1,
            key: property::WIDTH,
            value: Some(PropertyValue::Number(100.0)),
        },
        Mutation::SetProperty {
            id: 1,
            key: property::COLOR,
            value: None,
        },
        Mutation::SetProperty {
            id: 1,
            key: property::CLICK_LISTENER,
            value: Some(PropertyValue::Bool(false)),
        },
        Mutation::Remove {
            parent: 1,
            child: 2,
        },
    ] {
        assert!(retained_element_updates(&tree, &[replace(), mutation]).is_none());
    }
    tree.nodes.get_mut(&1).unwrap().set_property(
        property::PART,
        Some(PropertyValue::String(Arc::from("menu-item"))),
    );
    assert!(retained_element_updates(&tree, &[replace()]).is_none());
    tree.nodes
        .get_mut(&1)
        .unwrap()
        .set_property(property::PART, None);
    tree.nodes.get_mut(&1).unwrap().tag = NodeTag::SwiftUiButton;
    assert!(retained_element_updates(&tree, &[replace()]).is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn swift_ui_host_children_skip_region_sentinels() {
    let mut tree = NativeTree::default();
    let mut button = NativeNode::new(NodeTag::SwiftUiButton);
    button.set_property(
        property::VALUE,
        Some(PropertyValue::String(Arc::from("Generate"))),
    );
    tree.nodes.insert(1, button);
    // `Show` inside a host: the content sits before the region's sentinel among the host's children.
    tree.nodes.insert(2, NativeNode::new(NodeTag::Sentinel));
    tree.nodes.insert(3, NativeNode::new(NodeTag::View));
    let embedded = std::collections::HashMap::new();

    let elements = swift_ui_children(&[1, 2], &tree, &embedded).unwrap();
    assert_eq!(elements.len(), 1);
    let error = swift_ui_children(&[1, 3], &tree, &embedded).unwrap_err();
    assert!(
        error.contains("node 3 is not a SwiftUI component"),
        "{error}"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn swift_ui_button_modifiers_decode_in_declared_order() {
    let mut button = NativeNode::new(NodeTag::SwiftUiButton);
    button.set_property(
        property::SWIFT_UI_MODIFIERS,
        Some(PropertyValue::String(Arc::from(
            r##"[{"$type":"buttonStyle","style":"glass"},{"$type":"controlSize","size":"large"},{"$type":"buttonBorderShape","shape":"roundedRectangle","cornerRadius":14},{"$type":"labelStyle","style":"iconOnly"},{"$type":"tint","color":"#3366ffff"},{"$type":"disabled","disabled":false}]"##,
        ))),
    );

    assert_eq!(
        swift_ui_modifiers(&button).unwrap(),
        vec![
            SwiftUiModifier::ButtonStyle(SwiftUiButtonStyle::Glass),
            SwiftUiModifier::ControlSize(SwiftUiControlSize::Large),
            SwiftUiModifier::ButtonBorderShape {
                shape: SwiftUiButtonBorderShape::RoundedRectangle,
                corner_radius: Some(14.0),
            },
            SwiftUiModifier::LabelStyle(SwiftUiLabelStyle::IconOnly),
            SwiftUiModifier::Tint(Arc::from("#3366ffff")),
            SwiftUiModifier::Disabled(false),
        ]
    );
}

#[cfg(target_os = "macos")]
#[test]
fn swift_ui_button_rejects_unknown_modifiers() {
    let mut button = NativeNode::new(NodeTag::SwiftUiButton);
    button.set_property(
        property::SWIFT_UI_MODIFIERS,
        Some(PropertyValue::String(Arc::from(r#"[{"$type":"unknown"}]"#))),
    );

    assert!(
        swift_ui_modifiers(&button)
            .unwrap_err()
            .contains("unsupported SwiftUI modifier")
    );
}

#[cfg(target_os = "macos")]
#[test]
fn swift_ui_form_controls_decode_from_the_native_tree() {
    let mut tree = NativeTree::default();
    let label_id = 90;
    let mut label = NativeNode::new(NodeTag::Text);
    label.text = Arc::from("Volume");
    tree.nodes.insert(label_id, label);

    let mut slider = NativeNode::new(NodeTag::SwiftUiSlider);
    slider.children.push(label_id);
    slider.set_property(property::VALUE, Some(PropertyValue::Number(0.4)));
    slider.set_property(property::MINIMUM, Some(PropertyValue::Number(0.0)));
    slider.set_property(property::MAXIMUM, Some(PropertyValue::Number(1.0)));
    slider.set_property(property::STEP, Some(PropertyValue::Number(0.1)));
    slider.set_property(property::INPUT_LISTENER, Some(PropertyValue::Bool(true)));
    let slider = swift_ui_slider(1, &slider, &tree).unwrap();
    assert_eq!(slider.value, f64::from(0.4_f32));
    assert_eq!(slider.minimum, 0.0);
    assert_eq!(slider.maximum, 1.0);
    assert_eq!(slider.step, Some(0.1_f32.into()));
    assert_eq!(slider.label.as_deref(), Some("Volume"));
    assert!(slider.has_value_change);

    let mut toggle = NativeNode::new(NodeTag::SwiftUiToggle);
    toggle.set_property(property::CHECKED, Some(PropertyValue::Bool(true)));
    let toggle = swift_ui_toggle(2, &toggle, &tree).unwrap();
    assert!(toggle.is_on);

    let mut progress = NativeNode::new(NodeTag::SwiftUiProgressView);
    progress.set_property(property::VALUE, Some(PropertyValue::Number(3.0)));
    progress.set_property(property::MAXIMUM, Some(PropertyValue::Number(10.0)));
    progress.set_property(
        property::VALUE_TEXT,
        Some(PropertyValue::String(Arc::from("3 of 10"))),
    );
    let progress = swift_ui_progress_view(3, &progress, &tree).unwrap();
    assert_eq!(progress.value, Some(3.0));
    assert_eq!(progress.total, 10.0);
    assert_eq!(progress.current_value_label.as_deref(), Some("3 of 10"));

    let mut stepper = NativeNode::new(NodeTag::SwiftUiStepper);
    stepper.set_property(property::VALUE, Some(PropertyValue::Number(2.0)));
    stepper.set_property(property::STEP, Some(PropertyValue::Number(0.5)));
    let stepper = swift_ui_stepper(4, &stepper, &tree).unwrap();
    assert_eq!(stepper.value, 2.0);
    assert_eq!(stepper.step, 0.5);

    let mut field = NativeNode::new(NodeTag::SwiftUiTextField);
    field.set_property(
        property::VALUE,
        Some(PropertyValue::String(Arc::from("Ada"))),
    );
    field.set_property(
        property::PLACEHOLDER,
        Some(PropertyValue::String(Arc::from("Name"))),
    );
    field.set_property(property::PASSWORD, Some(PropertyValue::Bool(true)));
    field.set_property(property::SUBMIT_LISTENER, Some(PropertyValue::Bool(true)));
    let field = swift_ui_text_field(5, &field).unwrap();
    assert_eq!(&*field.text, "Ada");
    assert_eq!(field.placeholder.as_deref(), Some("Name"));
    assert!(field.secure);
    assert!(field.has_submit);

    let mut picker = NativeNode::new(NodeTag::SwiftUiPicker);
    picker.children.push(label_id);
    picker.set_property(
        property::VALUE,
        Some(PropertyValue::String(Arc::from("grid"))),
    );
    picker.set_property(
        property::ITEMS,
        Some(PropertyValue::String(Arc::from(
            r#"[{"value":"list","label":"List","systemImage":"list.bullet"},{"value":"grid","label":"Grid","disabled":true}]"#,
        ))),
    );
    picker.set_property(
        property::SWIFT_UI_PICKER_STYLE,
        Some(PropertyValue::String(Arc::from("segmented"))),
    );
    let picker = swift_ui_picker(6, &picker, &tree).unwrap();
    assert_eq!(&*picker.selection, "grid");
    assert_eq!(picker.style, SwiftUiPickerStyle::Segmented);
    assert_eq!(picker.options.len(), 2);
    assert_eq!(
        picker.options[0].system_image.as_deref(),
        Some("list.bullet")
    );
    assert!(picker.options[1].disabled);

    let mut tabs = NativeNode::new(NodeTag::SwiftUiPicker);
    tabs.set_property(
        property::SWIFT_UI_PICKER_STYLE,
        Some(PropertyValue::String(Arc::from("segmented"))),
    );
    tabs.set_property(
        property::ROLE,
        Some(PropertyValue::String(Arc::from("tabs"))),
    );
    let tabs = swift_ui_picker(16, &tabs, &tree).unwrap();
    assert_eq!(tabs.style, SwiftUiPickerStyle::Tabs);

    let mut date = NativeNode::new(NodeTag::SwiftUiDatePicker);
    date.set_property(
        property::CIVIL_VALUE,
        Some(PropertyValue::String(Arc::from("1788525000.25"))),
    );
    date.set_property(
        property::CIVIL_MINIMUM,
        Some(PropertyValue::String(Arc::from("1767225600"))),
    );
    date.set_property(
        property::SWIFT_UI_DATE_PICKER_COMPONENTS,
        Some(PropertyValue::String(Arc::from("date"))),
    );
    date.set_property(
        property::SWIFT_UI_DATE_PICKER_STYLE,
        Some(PropertyValue::String(Arc::from("field"))),
    );
    let date = swift_ui_date_picker(7, &date, &tree).unwrap();
    assert_eq!(date.value, 1_788_525_000.25);
    assert_eq!(date.minimum, Some(1_767_225_600.0));
    assert_eq!(date.components, SwiftUiDatePickerComponents::Date);
    assert_eq!(date.style, SwiftUiDatePickerStyle::Field);

    let mut color = NativeNode::new(NodeTag::SwiftUiColorPicker);
    color.set_property(
        property::VALUE,
        Some(PropertyValue::String(Arc::from("#3366ffff"))),
    );
    color.set_property(
        property::SWIFT_UI_COLOR_SUPPORTS_OPACITY,
        Some(PropertyValue::Bool(false)),
    );
    let color = swift_ui_color_picker(8, &color, &tree).unwrap();
    assert_eq!(&*color.selection, "#3366ffff");
    assert!(!color.supports_opacity);

    let mut gauge = NativeNode::new(NodeTag::SwiftUiGauge);
    gauge.set_property(property::VALUE, Some(PropertyValue::Number(0.72)));
    gauge.set_property(
        property::SWIFT_UI_GAUGE_STYLE,
        Some(PropertyValue::String(Arc::from("accessoryLinearCapacity"))),
    );
    gauge.set_property(
        property::SWIFT_UI_GAUGE_MINIMUM_VALUE_LABEL,
        Some(PropertyValue::String(Arc::from("0%"))),
    );
    let gauge = swift_ui_gauge(9, &gauge, &tree).unwrap();
    assert_eq!(gauge.value, f64::from(0.72_f32));
    assert_eq!(gauge.style, SwiftUiGaugeStyle::AccessoryLinearCapacity);
    assert_eq!(gauge.minimum_value_label.as_deref(), Some("0%"));
}

#[test]
fn queued_input_and_submit_survive_until_javascript_commits_the_controlled_value() {
    let input_id = 7;
    let mut tree = NativeTree::default();
    let mut input = NativeNode::new(NodeTag::Input);
    input.parent = Some(ROOT_NODE);
    input.set_property(property::INPUT_LISTENER, Some(PropertyValue::Bool(true)));
    input.set_property(property::SUBMIT_LISTENER, Some(PropertyValue::Bool(true)));
    tree.nodes.insert(input_id, input);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(input_id);

    let events = Rc::new(RefCell::new(VecDeque::new()));
    let view = NativeView {
        window: 3,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::clone(&events),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        images: Rc::new(RefCell::new(HashMap::new())),
        background_images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
        components: NativeComponentStates::default(),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    cx.focus(window, ElementId::new(input_id as u64)).unwrap();
    cx.simulate_input(window, "hello").unwrap();

    assert_eq!(
        cx.focused_input_value(window).unwrap().as_deref(),
        Some("hello")
    );
    let event = events.borrow_mut().pop_front().unwrap();
    assert_eq!(event.kind, "input");
    assert_eq!(event.window, 3);
    assert_eq!(event.target, input_id);
    assert_eq!(event.value.as_deref(), Some("hello"));

    cx.simulate_keystrokes(window, "enter").unwrap();

    assert_eq!(
        cx.focused(window).unwrap(),
        Some(ElementId::new(input_id as u64))
    );
    assert_eq!(
        cx.focused_input_value(window).unwrap().as_deref(),
        Some("hello")
    );
    let event = events.borrow_mut().pop_front().unwrap();
    assert_eq!(event.kind, "submit");
    assert_eq!(event.window, 3);
    assert_eq!(event.target, input_id);
    assert_eq!(event.value.as_deref(), Some("hello"));
}

#[test]
fn flex_without_direction_uses_css_row_default() {
    let implicit_row_id = 10;
    let row_first_id = 11;
    let row_second_id = 12;
    let explicit_column_id = 20;
    let column_first_id = 21;
    let column_second_id = 22;
    let mut tree = NativeTree::default();

    let mut implicit_row = NativeNode::new(NodeTag::Button);
    implicit_row.parent = Some(ROOT_NODE);
    implicit_row.children.extend([row_first_id, row_second_id]);
    implicit_row.set_property(
        property::DISPLAY,
        Some(PropertyValue::String(Arc::from("flex"))),
    );
    implicit_row.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
    implicit_row.set_property(property::HEIGHT, Some(PropertyValue::Number(60.0)));
    tree.nodes.insert(implicit_row_id, implicit_row);

    let mut explicit_column = NativeNode::new(NodeTag::Button);
    explicit_column.parent = Some(ROOT_NODE);
    explicit_column
        .children
        .extend([column_first_id, column_second_id]);
    explicit_column.set_property(
        property::DISPLAY,
        Some(PropertyValue::String(Arc::from("flex"))),
    );
    explicit_column.set_property(
        property::FLEX_DIRECTION,
        Some(PropertyValue::String(Arc::from("column"))),
    );
    explicit_column.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
    explicit_column.set_property(property::HEIGHT, Some(PropertyValue::Number(60.0)));
    tree.nodes.insert(explicit_column_id, explicit_column);

    for (id, parent, value) in [
        (row_first_id, implicit_row_id, "Count:"),
        (row_second_id, implicit_row_id, "0"),
        (column_first_id, explicit_column_id, "Count:"),
        (column_second_id, explicit_column_id, "0"),
    ] {
        let mut text = NativeNode::new(NodeTag::Text);
        text.parent = Some(parent);
        text.text = Arc::from(value);
        tree.nodes.insert(id, text);
    }
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .extend([implicit_row_id, explicit_column_id]);

    let view = NativeView {
        window: 4,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::new(RefCell::new(VecDeque::new())),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        images: Rc::new(RefCell::new(HashMap::new())),
        background_images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
        components: NativeComponentStates::default(),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    let row_first = cx
        .element_bounds(window, ElementId::new(row_first_id as u64))
        .unwrap();
    let row_second = cx
        .element_bounds(window, ElementId::new(row_second_id as u64))
        .unwrap();
    assert_eq!(row_second.y, row_first.y);
    assert!(row_second.x >= row_first.right());

    let column_first = cx
        .element_bounds(window, ElementId::new(column_first_id as u64))
        .unwrap();
    let column_second = cx
        .element_bounds(window, ElementId::new(column_second_id as u64))
        .unwrap();
    assert_eq!(column_second.x, column_first.x);
    assert!(column_second.y >= column_first.bottom());
}

#[test]
fn retained_popover_uses_core_placement_dismissal_and_focus_restoration() {
    let trigger_id = 20;
    let popover_id = 21;
    let mut tree = NativeTree::default();

    let mut trigger = NativeNode::new(NodeTag::Button);
    trigger.parent = Some(ROOT_NODE);
    trigger.set_property(property::WIDTH, Some(PropertyValue::Number(120.0)));
    trigger.set_property(property::HEIGHT, Some(PropertyValue::Number(40.0)));
    tree.nodes.insert(trigger_id, trigger);

    let mut popover = NativeNode::new(NodeTag::View);
    popover.parent = Some(ROOT_NODE);
    popover.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
    popover.set_property(property::HEIGHT, Some(PropertyValue::Number(100.0)));
    popover.set_property(
        property::ANCHOR_TARGET,
        Some(PropertyValue::String(Arc::from(trigger_id.to_string()))),
    );
    popover.set_property(
        property::ANCHOR_PLACEMENT,
        Some(PropertyValue::String(Arc::from("bottom-start"))),
    );
    popover.set_property(property::ANCHOR_GAP, Some(PropertyValue::Number(6.0)));
    popover.set_property(property::VIEWPORT_MARGIN, Some(PropertyValue::Number(12.0)));
    popover.set_property(property::DISMISS_LISTENER, Some(PropertyValue::Bool(true)));
    tree.nodes.insert(popover_id, popover);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .extend([trigger_id, popover_id]);

    let events = Rc::new(RefCell::new(VecDeque::new()));
    let view = NativeView {
        window: 5,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::clone(&events),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        images: Rc::new(RefCell::new(HashMap::new())),
        background_images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
        components: NativeComponentStates::default(),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    let trigger_element = ElementId::new(trigger_id as u64);
    let popover_element = ElementId::new(popover_id as u64);

    let trigger_bounds = cx.element_bounds(window, trigger_element).unwrap();
    let popover_bounds = cx.element_bounds(window, popover_element).unwrap();
    assert_eq!(popover_bounds.x, 12.0);
    assert_eq!(popover_bounds.y, trigger_bounds.bottom() + 6.0);
    assert_eq!(popover_bounds.width, 200.0);
    assert_eq!(popover_bounds.height, 100.0);

    cx.focus(window, popover_element).unwrap();
    cx.simulate_keystrokes(window, "escape").unwrap();

    assert_eq!(cx.focused(window).unwrap(), Some(trigger_element));
    let event = events.borrow_mut().pop_front().unwrap();
    assert_eq!(event.kind, "dismiss");
    assert_eq!(event.window, 5);
    assert_eq!(event.target, popover_id);

    cx.focus(window, popover_element).unwrap();
    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .retain(|child| *child != popover_id);
        tree.nodes.remove(&popover_id);
        cx.invalidate();
    })
    .unwrap();

    assert!(!cx.contains_element(window, popover_element).unwrap());
    assert_eq!(cx.focused(window).unwrap(), Some(trigger_element));
}

#[test]
fn unanchored_overlay_traps_autofocus_dismisses_and_restores_previous_focus() {
    let outside_id = 40;
    let overlay_id = 41;
    let surface_id = 42;
    let prompt_id = 43;
    let mut tree = NativeTree::default();

    let mut outside = NativeNode::new(NodeTag::Button);
    outside.parent = Some(ROOT_NODE);
    tree.nodes.insert(outside_id, outside);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(outside_id);

    let events = Rc::new(RefCell::new(VecDeque::new()));
    let view = NativeView {
        window: 6,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::clone(&events),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        images: Rc::new(RefCell::new(HashMap::new())),
        background_images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
        components: NativeComponentStates::default(),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    let outside_element = ElementId::new(outside_id as u64);
    let prompt_element = ElementId::new(prompt_id as u64);

    cx.focus(window, outside_element).unwrap();
    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();

        let mut overlay = NativeNode::new(NodeTag::View);
        overlay.parent = Some(ROOT_NODE);
        overlay.children.push(surface_id);
        overlay.set_property(property::OVERLAY, Some(PropertyValue::Bool(true)));
        overlay.set_property(property::FOCUS_TRAP, Some(PropertyValue::Bool(true)));
        overlay.set_property(
            property::RESTORE_PREVIOUS_FOCUS,
            Some(PropertyValue::Bool(true)),
        );

        let mut surface = NativeNode::new(NodeTag::View);
        surface.parent = Some(overlay_id);
        surface.children.push(prompt_id);
        surface.set_property(property::DISMISS_ON_ESCAPE, Some(PropertyValue::Bool(true)));
        surface.set_property(
            property::DISMISS_ON_POINTER_OUTSIDE,
            Some(PropertyValue::Bool(true)),
        );
        surface.set_property(property::DISMISS_LISTENER, Some(PropertyValue::Bool(true)));
        surface.set_property(
            property::ACCESSIBILITY_MODAL,
            Some(PropertyValue::Bool(true)),
        );

        let mut prompt = NativeNode::new(NodeTag::Input);
        prompt.parent = Some(surface_id);
        prompt.set_property(property::MULTILINE, Some(PropertyValue::Bool(true)));
        prompt.set_property(property::AUTO_FOCUS, Some(PropertyValue::Bool(true)));

        tree.nodes.insert(overlay_id, overlay);
        tree.nodes.insert(surface_id, surface);
        tree.nodes.insert(prompt_id, prompt);
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .push(overlay_id);
        cx.invalidate();
    })
    .unwrap();

    assert_eq!(cx.focused(window).unwrap(), Some(prompt_element));
    cx.simulate_keystrokes(window, "escape").unwrap();
    let event = events.borrow_mut().pop_front().unwrap();
    assert_eq!(event.kind, "dismiss");
    assert_eq!(event.target, surface_id);

    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();
        tree.nodes
            .get_mut(&ROOT_NODE)
            .unwrap()
            .children
            .retain(|child| *child != overlay_id);
        tree.nodes.remove(&prompt_id);
        tree.nodes.remove(&surface_id);
        tree.nodes.remove(&overlay_id);
        cx.invalidate();
    })
    .unwrap();

    assert_eq!(cx.focused(window).unwrap(), Some(outside_element));
}

#[test]
fn native_svg_is_parsed_once_until_its_source_changes() {
    let svg_id = 30;
    let first_source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><path d="M2 2h20v20H2z"/></svg>"#;
    let second_source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="10"/></svg>"#;
    let mut tree = NativeTree::default();
    let mut icon = NativeNode::new(NodeTag::Svg);
    icon.parent = Some(ROOT_NODE);
    icon.set_property(
        property::VALUE,
        Some(PropertyValue::String(Arc::from(first_source))),
    );
    icon.set_property(property::WIDTH, Some(PropertyValue::Number(16.0)));
    icon.set_property(property::HEIGHT, Some(PropertyValue::Number(16.0)));
    tree.nodes.insert(svg_id, icon);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(svg_id);

    let svgs = Rc::new(RefCell::new(HashMap::new()));
    let view = NativeView {
        window: 7,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::new(RefCell::new(VecDeque::new())),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::clone(&svgs),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        images: Rc::new(RefCell::new(HashMap::new())),
        background_images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
        components: NativeComponentStates::default(),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    let bounds = cx
        .element_bounds(window, ElementId::new(svg_id as u64))
        .unwrap();
    assert_eq!((bounds.width, bounds.height), (16.0, 16.0));

    let first = svgs.borrow()[&svg_id].parsed.as_ref().unwrap().clone();
    cx.update(view, |_view, cx| cx.invalidate()).unwrap();
    let unchanged = svgs.borrow()[&svg_id].parsed.as_ref().unwrap().clone();
    assert_eq!(first, unchanged);

    cx.update(view, |view, cx| {
        view.tree
            .borrow_mut()
            .nodes
            .get_mut(&svg_id)
            .unwrap()
            .set_property(
                property::VALUE,
                Some(PropertyValue::String(Arc::from(second_source))),
            );
        cx.invalidate();
    })
    .unwrap();
    let changed = svgs.borrow()[&svg_id].parsed.as_ref().unwrap().clone();
    assert_ne!(first, changed);
}

#[test]
fn native_virtual_list_mounts_only_the_initial_window_and_overscan() {
    let list_id = 40;
    let first_item_id = 1_000;
    let item_count = 100;
    let mut tree = NativeTree::default();
    let mut list = NativeNode::new(NodeTag::VirtualList);
    list.parent = Some(ROOT_NODE);
    list.set_property(
        property::ESTIMATED_ITEM_HEIGHT,
        Some(PropertyValue::Number(24.0)),
    );
    list.set_property(property::OVERSCAN, Some(PropertyValue::Number(2.0)));
    for index in 0..item_count {
        let id = first_item_id + index;
        let mut item = NativeNode::new(NodeTag::Text);
        item.parent = Some(list_id);
        item.text = Arc::from(format!("Item {index}"));
        tree.nodes.insert(id, item);
        list.children.push(id);
    }
    tree.nodes.insert(list_id, list);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(list_id);

    let lists = Rc::new(RefCell::new(HashMap::new()));
    let view = NativeView {
        window: 4,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::new(RefCell::new(VecDeque::new())),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::clone(&lists),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        images: Rc::new(RefCell::new(HashMap::new())),
        background_images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
        components: NativeComponentStates::default(),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    assert!(
        cx.contains_element(window, ElementId::new(first_item_id as u64))
            .unwrap()
    );
    assert!(
        !cx.contains_element(
            window,
            ElementId::new((first_item_id + item_count - 1) as u64),
        )
        .unwrap()
    );
    let mounted = lists.borrow()[&list_id].list.visible_rows().len();
    assert!(mounted < item_count as usize);
}

#[test]
fn native_virtual_list_applies_pixel_overscan_and_known_heights() {
    let oversized: Arc<str> = Arc::from("[2000000]");
    assert_eq!(
        decode_list_item_heights(Some(&oversized), 1),
        Some(vec![quickgui::MAX_LIST_ITEM_HEIGHT])
    );

    let mut node = NativeNode::new(NodeTag::VirtualList);
    node.children = (1..=1_000).collect();
    node.set_property(
        property::ESTIMATED_ITEM_HEIGHT,
        Some(PropertyValue::Number(10.0)),
    );
    node.set_property(property::OVERSCAN, Some(PropertyValue::Number(0.0)));
    node.set_property(property::OVERSCAN_PIXELS, Some(PropertyValue::Number(20.0)));
    let mut heights = vec![10.0_f32; node.children.len()];
    heights[1] = 100.0;
    node.set_property(
        property::ITEM_HEIGHTS,
        Some(PropertyValue::String(Arc::from(
            serde_json::to_string(&heights).unwrap(),
        ))),
    );

    let mut retained = NativeListState::new(&node);
    retained.list.set_viewport_size(400.0, 40.0);
    retained.list.scroll_to_pixels(110.0);
    assert_eq!(retained.list.visible_rows().range, 1..8);
    assert_eq!(retained.list.stats().measured_items, 1);

    let anchor = retained.list.logical_scroll_top();
    heights[0] = 20.0;
    node.set_property(
        property::ITEM_HEIGHTS,
        Some(PropertyValue::String(Arc::from(
            serde_json::to_string(&heights).unwrap(),
        ))),
    );
    retained.sync(&node);
    assert_eq!(retained.list.logical_scroll_top(), anchor);
    assert_eq!(retained.list.scroll_offset(), 120.0);

    node.set_property(property::ITEM_HEIGHTS, None);
    retained.sync(&node);
    assert_eq!(retained.list.stats().measured_items, 0);
}

#[cfg(unix)]
#[test]
fn native_terminal_runs_a_real_pty_and_rerenders_ghostty_output() {
    // Production loads this descriptor from its separate image through purego. The native
    // integration test links the same backend as a dev dependency and exercises the C ABI.
    unsafe {
        quickgui::extensions::register_extension(
            quickgui_terminal::quickgui_extension_v1().cast(),
            b"terminal",
        )
    }
    .unwrap();
    let terminal_id = 50;
    let mut tree = NativeTree::default();
    let mut terminal = NativeNode::new(NodeTag::Terminal);
    terminal.parent = Some(ROOT_NODE);
    terminal.set_property(
        property::TERMINAL_PROGRAM,
        Some(PropertyValue::String(Arc::from("/bin/sh"))),
    );
    terminal.set_property(
        property::TERMINAL_ARGUMENTS,
        Some(PropertyValue::String(Arc::from(
            serde_json::json!(["-c", "printf 'quickgui-pty-ok\\n'"]).to_string(),
        ))),
    );
    terminal.set_property(
        property::TERMINAL_STATUS_LISTENER,
        Some(PropertyValue::Bool(true)),
    );
    terminal.set_property(
        property::POSITION,
        Some(PropertyValue::String(Arc::from("absolute"))),
    );
    terminal.set_property(property::TOP, Some(PropertyValue::Number(8.0)));
    terminal.set_property(property::RIGHT, Some(PropertyValue::Number(9.0)));
    terminal.set_property(property::BOTTOM, Some(PropertyValue::Number(8.0)));
    terminal.set_property(property::LEFT, Some(PropertyValue::Number(9.0)));
    terminal.set_property(property::PADDING_TOP, Some(PropertyValue::Number(8.0)));
    terminal.set_property(property::PADDING_RIGHT, Some(PropertyValue::Number(9.0)));
    terminal.set_property(property::PADDING_BOTTOM, Some(PropertyValue::Number(8.0)));
    terminal.set_property(property::PADDING_LEFT, Some(PropertyValue::Number(9.0)));
    tree.nodes.insert(terminal_id, terminal);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(terminal_id);

    let terminals = Rc::new(RefCell::new(HashMap::new()));
    let events = Rc::new(RefCell::new(VecDeque::new()));
    let view = NativeView {
        window: 6,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events: Rc::clone(&events),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::clone(&terminals),
        images: Rc::new(RefCell::new(HashMap::new())),
        background_images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
        components: NativeComponentStates::default(),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    for _ in 0..200 {
        let finished_with_output = terminals
            .borrow()
            .get(&terminal_id)
            .and_then(|state| state.terminal.as_ref())
            .is_some_and(|terminal| {
                let snapshot = terminal.snapshot();
                matches!(snapshot.status, TerminalStatus::Exited { .. })
                    && snapshot.content.contains("quickgui-pty-ok")
            });
        if finished_with_output {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    cx.update(view, |_view, cx| cx.invalidate()).unwrap();

    let snapshot = terminals.borrow()[&terminal_id]
        .terminal
        .as_ref()
        .unwrap()
        .snapshot();
    assert!(
        snapshot.content.contains("quickgui-pty-ok"),
        "snapshot: {snapshot:#?}"
    );
    assert!(matches!(snapshot.status, TerminalStatus::Exited { .. }));
    let root_bounds = cx
        .element_bounds(window, ElementId::new(crate::ROOT_ELEMENT_ID))
        .unwrap();
    let terminal_bounds = cx
        .element_bounds(window, ElementId::new(terminal_id as u64))
        .unwrap();
    assert_eq!(terminal_bounds.x, root_bounds.x + 9.0);
    assert_eq!(terminal_bounds.y, root_bounds.y + 8.0);
    assert_eq!(terminal_bounds.right(), root_bounds.right() - 9.0);
    assert_eq!(terminal_bounds.bottom(), root_bounds.bottom() - 8.0);
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| event.kind == "terminal" && event.target == terminal_id)
    );
}

#[test]
fn close_interception_is_declared_ahead_of_the_native_decision() {
    crate::runtime::reset_interception_state();
    assert!(!crate::runtime::intercepts_close(4));

    let action = crate::system::parse_window_action("set-close-interception", Some("true".into()))
        .expect("the hosted window action parses");
    assert!(matches!(
        action,
        system::WindowAction::SetCloseInterception(true)
    ));

    crate::runtime::set_close_interception(4, true);
    assert!(crate::runtime::intercepts_close(4));
    assert!(!crate::runtime::intercepts_close(5));

    // Withdrawing the last listener lets native closes proceed again.
    crate::runtime::set_close_interception(4, false);
    assert!(!crate::runtime::intercepts_close(4));
    crate::runtime::reset_interception_state();
}

#[test]
fn hosted_close_interception_travels_as_a_fire_and_forget_mutation() {
    let host = HostCoordinator::new();
    host.enqueue(HostCommand::Mutation {
        app: 3,
        command: system::SystemCommand::WindowAction {
            window: 9,
            action: system::WindowAction::SetCloseInterception(true),
        },
    })
    .expect("a close-interception declaration fits the host queue");

    let mut commands = host.take_commands().expect("the host queue is readable");
    match commands.pop_front().expect("the declaration was queued") {
        HostCommand::Mutation {
            app,
            command:
                system::SystemCommand::WindowAction {
                    window,
                    action: system::WindowAction::SetCloseInterception(intercepting),
                },
        } => {
            assert_eq!(app, 3);
            assert_eq!(window, 9);
            assert!(intercepting);
        }
        _ => panic!("close interception must never wait on a synchronous reply"),
    }
    assert!(commands.is_empty());
}

#[test]
fn quit_interception_is_declared_before_the_before_quit_phase() {
    crate::runtime::reset_interception_state();
    assert!(!crate::runtime::intercepts_quit());
    crate::runtime::set_quit_interception(true);
    assert!(crate::runtime::intercepts_quit());
    crate::runtime::reset_interception_state();
    assert!(!crate::runtime::intercepts_quit());
}

#[test]
fn quit_reasons_reach_javascript_with_stable_names() {
    for (reason, name) in [
        (quickgui::QuitReason::Explicit, "explicit"),
        (quickgui::QuitReason::Relaunch, "relaunch"),
        (quickgui::QuitReason::LastWindowClosed, "last-window-closed"),
        (quickgui::QuitReason::OperatingSystem, "operating-system"),
    ] {
        assert_eq!(crate::runtime::quit_reason_name(reason), name);
    }
}

#[test]
fn window_tab_and_character_palette_actions_parse_from_the_hosted_boundary() {
    for (action, expected) in [
        (
            "show-character-palette",
            system::WindowAction::ShowCharacterPalette,
        ),
        ("select-next-tab", system::WindowAction::SelectNextTab),
        (
            "select-previous-tab",
            system::WindowAction::SelectPreviousTab,
        ),
        ("merge-all-windows", system::WindowAction::MergeAllWindows),
        (
            "move-tab-to-new-window",
            system::WindowAction::MoveTabToNewWindow,
        ),
        ("toggle-tab-bar", system::WindowAction::ToggleTabBar),
        (
            "toggle-tab-overview",
            system::WindowAction::ToggleTabOverview,
        ),
    ] {
        let parsed = crate::system::parse_window_action(action, None)
            .unwrap_or_else(|error| panic!("{action} parses: {error}"));
        assert_eq!(
            std::mem::discriminant(&parsed),
            std::mem::discriminant(&expected),
            "action {action}"
        );
    }

    assert!(matches!(
        crate::system::parse_window_action("select-tab", Some("3".into())).unwrap(),
        system::WindowAction::SelectTab(3)
    ));
    assert!(crate::system::parse_window_action("select-tab", Some("-1".into())).is_err());
    assert!(crate::system::parse_window_action("select-tab", None).is_err());

    assert!(matches!(
        crate::system::parse_window_action("set-tabbing-identifier", Some("docs".into())).unwrap(),
        system::WindowAction::SetTabbingIdentifier(Some(identifier)) if identifier == "docs"
    ));
    assert!(matches!(
        crate::system::parse_window_action("set-tabbing-identifier", Some(String::new())).unwrap(),
        system::WindowAction::SetTabbingIdentifier(None)
    ));
}

#[test]
fn a_declared_close_interception_holds_the_window_and_reports_it_to_javascript() {
    crate::runtime::reset_interception_state();
    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = NativeView {
        window: 21,
        handles: None,
        tree: Rc::new(RefCell::new(NativeTree::default())),
        events: Rc::clone(&events),
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        images: Rc::new(RefCell::new(HashMap::new())),
        background_images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
        components: NativeComponentStates::default(),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    };
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    // While interception is declared the native close is held and reported to JavaScript.
    crate::runtime::set_close_interception(21, true);
    assert!(!cx.simulate_close_requested(window).unwrap());
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| event.kind == "close-requested" && event.window == 21)
            .count(),
        1
    );

    // Withdrawing the last listener lets the next native close proceed with no extra event.
    crate::runtime::set_close_interception(21, false);
    assert!(cx.simulate_close_requested(window).unwrap());
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| event.kind == "close-requested")
            .count(),
        1
    );
    crate::runtime::reset_interception_state();
}

/// Build one component-part node with the string, boolean, and numeric properties it declares.
fn component_part_node(
    tag: NodeTag,
    parent: u32,
    part: &str,
    strings: &[(u16, &str)],
    flags: &[(u16, bool)],
) -> NativeNode {
    let mut node = NativeNode::new(tag);
    node.parent = Some(parent);
    node.set_property(property::PART, Some(PropertyValue::String(Arc::from(part))));
    node.set_property(property::WIDTH, Some(PropertyValue::Number(120.0)));
    node.set_property(property::HEIGHT, Some(PropertyValue::Number(32.0)));
    for (key, value) in strings {
        node.set_property(*key, Some(PropertyValue::String(Arc::from(*value))));
    }
    for (key, value) in flags {
        node.set_property(*key, Some(PropertyValue::Bool(*value)));
    }
    node
}

fn component_part_view(window: u32, tree: NativeTree, events: EventQueue) -> NativeView {
    NativeView {
        window,
        handles: None,
        tree: Rc::new(RefCell::new(tree)),
        events,
        markdown: Rc::new(RefCell::new(HashMap::new())),
        svgs: Rc::new(RefCell::new(HashMap::new())),
        lists: Rc::new(RefCell::new(HashMap::new())),
        terminals: Rc::new(RefCell::new(HashMap::new())),
        images: Rc::new(RefCell::new(HashMap::new())),
        background_images: Rc::new(RefCell::new(HashMap::new())),
        shaders: Rc::new(RefCell::new(HashMap::new())),
        menus: Rc::new(RefCell::new(HashMap::new())),
        context_menu: ContextMenuState::new(),
        context_menu_owner: None,
        focused_node: None,
        components: NativeComponentStates::default(),
        #[cfg(target_os = "macos")]
        swift_ui_hosts: Rc::new(RefCell::new(HashMap::new())),
        embedded_views: Rc::new(RefCell::new(HashMap::new())),
    }
}

#[test]
fn component_part_properties_decode_core_identities_and_mount_policy() {
    let tabs = Tabs::new("tabset", "one");

    let inactive_panel = component_part_node(
        NodeTag::View,
        0,
        "tab-panel",
        &[
            (property::SCOPE, "tabset"),
            (property::ACTIVE_VALUE, "one"),
            (property::PART_VALUE, "two"),
        ],
        &[],
    );
    assert_eq!(
        native_part_element_id(5, &inactive_panel),
        Some(tabs.tab("two").panel_id())
    );
    assert!(apply_part(div(), 5, &inactive_panel).is_none());

    let mut retained_panel = inactive_panel.clone();
    retained_panel.set_property(property::KEEP_MOUNTED, Some(PropertyValue::Bool(true)));
    assert_eq!(
        native_part_element_id(5, &retained_panel),
        Some(tabs.tab("two").panel_id())
    );
    assert!(apply_part(div(), 5, &retained_panel).is_some());

    let active_panel = component_part_node(
        NodeTag::View,
        0,
        "tab-panel",
        &[
            (property::SCOPE, "tabset"),
            (property::ACTIVE_VALUE, "one"),
            (property::PART_VALUE, "one"),
        ],
        &[],
    );
    assert_eq!(
        native_part_element_id(6, &active_panel),
        Some(tabs.tab("one").panel_id())
    );
    assert!(apply_part(div(), 6, &active_panel).is_some());

    let closed_panel = component_part_node(
        NodeTag::View,
        0,
        "collapsible-panel",
        &[(property::SCOPE, "disclosure")],
        &[(property::OPEN, false)],
    );
    assert_eq!(
        native_part_element_id(7, &closed_panel),
        Some(Collapsible::new("disclosure", false).panel_id())
    );
    assert!(apply_part(div(), 7, &closed_panel).is_none());

    let field_error = component_part_node(
        NodeTag::View,
        0,
        "field-error",
        &[(property::SCOPE, "email")],
        &[(property::INVALID, true)],
    );
    assert_eq!(
        native_part_element_id(8, &field_error),
        Some(Field::new("email").error_id())
    );
    assert!(apply_part(div(), 8, &field_error).is_some());

    // An unknown part name keeps the ordinary node identity and decorates nothing.
    let unknown = component_part_node(NodeTag::View, 0, "not-a-part", &[], &[]);
    assert_eq!(native_part_element_id(9, &unknown), None);
    assert!(apply_part(div(), 9, &unknown).is_some());

    // Scopes and item values are bounded; an oversized key falls back to the node identity.
    let oversized = component_part_node(
        NodeTag::View,
        0,
        "fieldset",
        &[(property::SCOPE, &"s".repeat(MAX_COMPONENT_VALUE_BYTES + 1))],
        &[],
    );
    assert_eq!(
        native_part_element_id(10, &oversized),
        Some(ElementId::new(10))
    );

    // A tab without a bounded value cannot resolve a core identity and mounts undecorated.
    let valueless_tab =
        component_part_node(NodeTag::View, 0, "tab", &[(property::SCOPE, "tabset")], &[]);
    assert_eq!(native_part_element_id(11, &valueless_tab), None);
    assert!(apply_part(div(), 11, &valueless_tab).is_some());
}

#[test]
fn native_tabs_parts_mount_core_panels_and_roving_arrow_navigation() {
    let root_id = 100;
    let list_id = 101;
    let first_tab_id = 102;
    let second_tab_id = 103;
    let first_panel_id = 110;
    let second_panel_id = 111;
    let scope = [(property::SCOPE, "tabset"), (property::ACTIVE_VALUE, "one")];
    let mut tree = NativeTree::default();

    let mut root = component_part_node(NodeTag::View, ROOT_NODE, "tabs", &scope, &[]);
    root.children
        .extend([list_id, first_panel_id, second_panel_id]);
    tree.nodes.insert(root_id, root);

    let mut list = component_part_node(NodeTag::View, root_id, "tabs-list", &scope, &[]);
    list.children.extend([first_tab_id, second_tab_id]);
    tree.nodes.insert(list_id, list);

    for (id, value) in [(first_tab_id, "one"), (second_tab_id, "two")] {
        let mut strings = scope.to_vec();
        strings.push((property::PART_VALUE, value));
        tree.nodes.insert(
            id,
            component_part_node(NodeTag::Button, list_id, "tab", &strings, &[]),
        );
    }
    for (id, value) in [(first_panel_id, "one"), (second_panel_id, "two")] {
        let mut strings = scope.to_vec();
        strings.push((property::PART_VALUE, value));
        tree.nodes.insert(
            id,
            component_part_node(NodeTag::View, root_id, "tab-panel", &strings, &[]),
        );
    }
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(root_id);

    let view = component_part_view(6, tree, Rc::new(RefCell::new(VecDeque::new())));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    let tabs = Tabs::new("tabset", "one");
    let first = tabs.tab("one");
    let second = tabs.tab("two");
    assert!(cx.contains_element(window, first.tab_id()).unwrap());
    assert!(cx.contains_element(window, second.tab_id()).unwrap());
    assert!(cx.contains_element(window, first.panel_id()).unwrap());
    assert!(!cx.contains_element(window, second.panel_id()).unwrap());
    // The list identity is the core's derived one, not either node id.
    assert!(cx.contains_element(window, tabs.list_id()).unwrap());
    assert!(
        !cx.contains_element(window, ElementId::new(list_id as u64))
            .unwrap()
    );

    cx.focus(window, first.tab_id()).unwrap();
    cx.simulate_keystrokes(window, "right").unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(second.tab_id()));
    cx.simulate_keystrokes(window, "right").unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(first.tab_id()));
}

#[test]
fn selection_and_field_parts_adopt_core_focus_activation_and_labelling() {
    let checkbox_id = 120;
    let plain_id = 121;
    let field_root_id = 130;
    let label_id = 131;
    let control_id = 132;
    let mut tree = NativeTree::default();

    tree.nodes.insert(
        checkbox_id,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "checkbox",
            &[],
            &[(property::CHECKED, true)],
        ),
    );
    let mut plain = NativeNode::new(NodeTag::View);
    plain.parent = Some(ROOT_NODE);
    plain.set_property(property::WIDTH, Some(PropertyValue::Number(120.0)));
    plain.set_property(property::HEIGHT, Some(PropertyValue::Number(32.0)));
    tree.nodes.insert(plain_id, plain);

    let field = [(property::SCOPE, "email")];
    let mut field_root = component_part_node(NodeTag::View, ROOT_NODE, "field", &field, &[]);
    field_root.children.extend([label_id, control_id]);
    tree.nodes.insert(field_root_id, field_root);
    tree.nodes.insert(
        label_id,
        component_part_node(NodeTag::View, field_root_id, "field-label", &field, &[]),
    );
    tree.nodes.insert(
        control_id,
        component_part_node(NodeTag::Input, field_root_id, "field-control", &field, &[]),
    );
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .extend([checkbox_id, plain_id, field_root_id]);

    let view = component_part_view(7, tree, Rc::new(RefCell::new(VecDeque::new())));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    // The core checkbox part supplies focus and click behavior the plain view does not have.
    let checkbox_element = ElementId::new(checkbox_id as u64);
    cx.focus(window, checkbox_element).unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(checkbox_element));
    cx.click(window, checkbox_element).unwrap();
    assert!(cx.focus(window, ElementId::new(plain_id as u64)).is_err());

    // The field label activates the control it names through the core relationship.
    let field = Field::new("email");
    assert!(cx.contains_element(window, field.root_id()).unwrap());
    cx.click(window, field.label_id()).unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(field.control_id()));
}

#[test]
fn in_window_dialog_parts_mount_only_while_open_and_dismiss_through_the_core() {
    let trigger_id = 140;
    let portal_id = 141;
    let backdrop_id = 142;
    let popup_id = 143;
    let title_id = 144;
    let close_id = 145;
    let dialog = [(property::SCOPE, "confirm")];
    let mut tree = NativeTree::default();

    tree.nodes.insert(
        trigger_id,
        component_part_node(
            NodeTag::Button,
            ROOT_NODE,
            "dialog-trigger",
            &dialog,
            &[(property::OPEN, true)],
        ),
    );
    let mut portal = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "dialog",
        &dialog,
        &[(property::OPEN, true)],
    );
    portal.children.extend([backdrop_id, popup_id]);
    tree.nodes.insert(portal_id, portal);
    tree.nodes.insert(
        backdrop_id,
        component_part_node(
            NodeTag::View,
            portal_id,
            "dialog-backdrop",
            &dialog,
            &[(property::OPEN, true)],
        ),
    );
    let mut popup = component_part_node(
        NodeTag::View,
        portal_id,
        "dialog-popup",
        &dialog,
        &[
            (property::OPEN, true),
            (property::DISMISS_ON_ESCAPE, true),
            (property::DISMISS_LISTENER, true),
        ],
    );
    popup.children.extend([title_id, close_id]);
    tree.nodes.insert(popup_id, popup);
    tree.nodes.insert(
        title_id,
        component_part_node(
            NodeTag::View,
            popup_id,
            "dialog-title",
            &dialog,
            &[(property::OPEN, true)],
        ),
    );
    tree.nodes.insert(
        close_id,
        component_part_node(
            NodeTag::Button,
            popup_id,
            "dialog-close",
            &dialog,
            &[(property::OPEN, true)],
        ),
    );
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .extend([trigger_id, portal_id]);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(8, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    let core = quickgui::Dialog::new("confirm", true);
    assert!(cx.contains_element(window, core.root_id()).unwrap());
    assert!(cx.contains_element(window, core.popover_id()).unwrap());
    assert!(cx.contains_element(window, core.title_id()).unwrap());
    assert!(cx.contains_element(window, core.close_id()).unwrap());

    // Escape reaches the core surface and is reported back as one asynchronous dismiss event.
    cx.focus(window, core.popover_id()).unwrap();
    cx.simulate_keystrokes(window, "escape").unwrap();
    let event = events.borrow_mut().pop_front().unwrap();
    assert_eq!(event.kind, "dismiss");
    assert_eq!(event.target, popup_id);

    // A closed dialog contributes no portal, backdrop, popup, or title at all.
    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();
        for id in [
            trigger_id,
            portal_id,
            backdrop_id,
            popup_id,
            title_id,
            close_id,
        ] {
            tree.nodes
                .get_mut(&id)
                .unwrap()
                .set_property(property::OPEN, Some(PropertyValue::Bool(false)));
        }
        cx.invalidate();
    })
    .unwrap();

    assert!(!cx.contains_element(window, core.root_id()).unwrap());
    assert!(!cx.contains_element(window, core.popover_id()).unwrap());
    assert!(!cx.contains_element(window, core.backdrop_id()).unwrap());
    assert!(!cx.contains_element(window, core.title_id()).unwrap());
    assert!(
        cx.contains_element(window, ElementId::new(trigger_id as u64))
            .unwrap()
    );
}

#[test]
fn window_stacking_and_input_actions_parse_into_bounded_core_commands() {
    let action =
        crate::system::parse_window_action("move-top", None).expect("move-top needs no payload");
    assert!(matches!(action, system::WindowAction::MoveTop));

    let action = crate::system::parse_window_action("move-above", Some("12".into()))
        .expect("move-above names another hosted window");
    assert!(matches!(action, system::WindowAction::MoveAbove(12)));
    assert!(crate::system::parse_window_action("move-above", Some("not-a-window".into())).is_err());
    assert!(crate::system::parse_window_action("move-above", None).is_err());

    let action = crate::system::parse_window_action(
        "set-ignore-mouse-events",
        Some("{\"ignore\":true,\"forward\":true}".into()),
    )
    .expect("the ignore-mouse-events payload parses");
    assert!(matches!(
        action,
        system::WindowAction::SetIgnoreMouseEvents(true, true)
    ));

    let action = crate::system::parse_window_action("set-enabled", Some("false".into()))
        .expect("set-enabled takes a boolean");
    assert!(matches!(
        action,
        system::WindowAction::SetWindowEnabled(false)
    ));

    let action =
        crate::system::parse_window_action("set-window-button-visibility", Some("true".into()))
            .expect("button visibility takes a boolean");
    assert!(matches!(
        action,
        system::WindowAction::SetWindowButtonVisibility(true)
    ));
}

#[test]
fn always_on_top_maps_electron_level_names_onto_the_core_levels() {
    let action = crate::system::parse_window_action(
        "set-always-on-top",
        Some("{\"flag\":true,\"level\":\"screenSaver\"}".into()),
    )
    .expect("the always-on-top payload parses");
    match action {
        system::WindowAction::SetAlwaysOnTop(flag, level) => {
            assert!(flag);
            assert_eq!(level, Some(quickgui::WindowLevel::ScreenSaver));
        }
        _ => panic!("set-always-on-top must produce a stacking command"),
    }

    let action =
        crate::system::parse_window_action("set-always-on-top", Some("{\"flag\":false}".into()))
            .expect("turning always-on-top off needs no level");
    assert!(matches!(
        action,
        system::WindowAction::SetAlwaysOnTop(false, None)
    ));

    assert!(
        crate::system::parse_window_action(
            "set-always-on-top",
            Some("{\"flag\":true,\"level\":\"nope\"}".into())
        )
        .is_err()
    );

    for (name, level) in [
        ("normal", quickgui::WindowLevel::Normal),
        ("floating", quickgui::WindowLevel::Floating),
        ("modalPanel", quickgui::WindowLevel::ModalPanel),
        ("mainMenu", quickgui::WindowLevel::MainMenu),
        ("status", quickgui::WindowLevel::Status),
        ("popUpMenu", quickgui::WindowLevel::PopUpMenu),
        ("screen-saver", quickgui::WindowLevel::ScreenSaver),
        ("always-on-bottom", quickgui::WindowLevel::AlwaysOnBottom),
    ] {
        assert_eq!(
            crate::system::parse_window_level(name).expect("a documented level name parses"),
            level
        );
    }
}

#[test]
fn resize_and_move_policies_are_declared_ahead_and_bounded() {
    crate::runtime::reset_interception_state();
    let action = crate::system::parse_window_action(
        "set-resize-policy",
        Some("{\"aspectRatio\":2,\"minimum\":{\"width\":100,\"height\":50}}".into()),
    )
    .expect("the resize policy action parses");
    let policy = match action {
        system::WindowAction::SetResizePolicy(Some(json)) => {
            crate::system::parse_resize_policy(&json).expect("a bounded policy is accepted")
        }
        _ => panic!("set-resize-policy must carry its declaration"),
    };
    assert_eq!(policy.aspect_ratio, Some(2.0));
    assert_eq!(policy.minimum, Some((100.0, 50.0)));

    // The aspect ratio is applied after the grid step and before the bounds.
    let constrained = policy
        .constrain(quickgui::Size::new(400.0, 999.0))
        .expect("the proposal is narrowed");
    assert_eq!(constrained, quickgui::Size::new(400.0, 200.0));
    assert_eq!(policy.constrain(quickgui::Size::new(400.0, 200.0)), None);

    let snapped = crate::system::parse_resize_policy("{\"snap\":{\"width\":25,\"height\":25}}")
        .expect("a grid step is accepted")
        .constrain(quickgui::Size::new(413.0, 187.0))
        .expect("an off-grid proposal is snapped");
    assert_eq!(snapped, quickgui::Size::new(425.0, 175.0));

    assert!(crate::system::parse_resize_policy("{\"aspectRatio\":0}").is_err());
    assert!(crate::system::parse_resize_policy("{\"aspectRatio\":100000}").is_err());
    assert!(
        crate::system::parse_resize_policy(
            "{\"minimum\":{\"width\":900,\"height\":10},\"maximum\":{\"width\":100,\"height\":100}}"
        )
        .is_err()
    );
    assert!(crate::system::parse_resize_policy("{\"snap\":{\"width\":-1,\"height\":4}}").is_err());

    let move_policy =
        crate::system::parse_move_policy("{\"keepOnScreen\":true}").expect("a move policy parses");
    assert!(move_policy.keep_on_screen);
    assert!(
        !crate::system::parse_move_policy("{}")
            .expect("an empty move policy is accepted")
            .keep_on_screen
    );
    assert!(crate::system::parse_move_policy("not json").is_err());
}

#[test]
fn declared_window_policies_stay_bounded_and_are_released_with_their_window() {
    crate::runtime::reset_interception_state();
    let policy = crate::runtime::WindowResizePolicy {
        aspect_ratio: Some(1.5),
        ..Default::default()
    };
    for window in 0..(MAX_WINDOWS as u32 + 8) {
        crate::runtime::set_resize_policy(window, Some(policy));
        crate::runtime::set_move_policy(
            window,
            Some(crate::runtime::WindowMovePolicy {
                keep_on_screen: true,
            }),
        );
    }
    assert!(crate::runtime::retained_window_policy_counts() <= (MAX_WINDOWS, MAX_WINDOWS));

    crate::runtime::forget_window_policies(0);
    // An empty declaration withdraws the policy instead of retaining a no-op entry.
    crate::runtime::set_resize_policy(1, Some(crate::runtime::WindowResizePolicy::default()));
    crate::runtime::set_move_policy(1, None);
    crate::runtime::reset_interception_state();
    assert_eq!(crate::runtime::retained_window_policy_counts(), (0, 0));
}

#[test]
fn keep_on_screen_clamps_only_positions_outside_the_work_area() {
    let id = quickgui::DisplayId::new(1);
    let displays = quickgui::Displays::new(
        vec![
            quickgui::Display::new(
                id,
                "Primary",
                quickgui::Rect::new(0.0, 0.0, 1600.0, 1000.0),
                quickgui::Rect::new(0.0, 25.0, 1600.0, 975.0),
                2.0,
            )
            .expect("a valid display snapshot"),
        ],
        Some(id),
    )
    .expect("a single-display snapshot is valid");
    assert_eq!(
        crate::runtime::keep_position_on_screen(displays.all(), Point::new(100.0, 200.0)),
        None
    );
    assert_eq!(
        crate::runtime::keep_position_on_screen(displays.all(), Point::new(-40.0, 0.0)),
        Some(Point::new(0.0, 25.0))
    );
    assert_eq!(
        crate::runtime::keep_position_on_screen(&[], Point::new(1.0, 1.0)),
        None
    );
}

const DECLARED_MENU: &str = r#"{
  "width": 200,
  "itemHeight": 30,
  "items": [
    { "type": "group", "label": "File" },
    { "id": "open", "label": "Open…", "shortcut": "⌘O" },
    { "type": "separator" },
    { "type": "checkbox", "id": "sidebar", "label": "Show sidebar", "checked": true },
    { "type": "radio", "id": "small", "group": "size", "label": "Small", "checked": true },
    { "id": "recent", "label": "Recent", "items": [{ "id": "one", "label": "One" }] },
    { "id": "quit", "label": "Quit", "disabled": true },
    { "label": "no identifier" }
  ]
}"#;

#[test]
fn declared_menu_entries_adopt_core_kinds_bounds_and_shortcuts() {
    let declaration =
        NativeMenuDeclaration::parse(DECLARED_MENU).expect("the declaration is bounded JSON");
    let style = declaration.style();
    assert_eq!(style.width, 200.0);
    assert_eq!(style.item_height, 30.0);
    // Unspecified measurements keep the core's own defaults instead of a JavaScript guess.
    assert_eq!(
        style.separator_height,
        NativeMenuStyle::default().separator_height
    );

    let items = menu_items(7, declaration.entries(), 0);
    // The entry without a stable identifier cannot form a core command and is skipped.
    assert_eq!(items.len(), 7);
    let kinds = items.iter().map(PopoverMenuItem::kind).collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            PopoverMenuItemKind::GroupLabel,
            PopoverMenuItemKind::Action,
            PopoverMenuItemKind::Separator,
            PopoverMenuItemKind::Checkbox,
            PopoverMenuItemKind::Radio,
            PopoverMenuItemKind::Submenu,
            PopoverMenuItemKind::Action,
        ]
    );
    assert_eq!(
        items[1].shortcut_text().map(|value| &**value),
        Some("\u{2318}O")
    );
    assert_eq!(items[3].checked(), Some(true));
    assert_eq!(items[4].checked(), Some(true));
    assert!(items[6].is_disabled());
    assert_eq!(
        items[5]
            .submenu_menu()
            .expect("a declared submenu carries a validated core model")
            .items()
            .len(),
        1
    );

    // The whole declaration validates through the core before any of it is retained.
    let menu = PopoverMenu::new(items).expect("the declared tree passes core validation");
    assert_eq!(menu.items().len(), 7);
}

#[test]
fn malformed_and_oversized_menu_declarations_are_rejected_without_panicking() {
    assert!(NativeMenuDeclaration::parse("not json").is_none());
    assert!(NativeMenuDeclaration::parse(&"a".repeat(MAX_MENU_JSON_BYTES + 1)).is_none());

    // A bounded declaration with an oversized identifier keeps the rest of the menu.
    let oversized = format!(
        r#"{{"items":[{{"id":"{}","label":"Too long"}},{{"id":"ok","label":"Fine"}}]}}"#,
        "x".repeat(MAX_MENU_ITEM_ID_BYTES + 1)
    );
    let declaration = NativeMenuDeclaration::parse(&oversized).expect("the JSON itself is bounded");
    let items = menu_items(3, declaration.entries(), 0);
    assert_eq!(items.len(), 1);
    assert_eq!(&**items[0].label(), "Fine");

    // An empty declaration still forms a valid, inert model.
    let empty = NativeMenuDeclaration::parse("{}").expect("an empty object is a valid declaration");
    assert!(menu_items(3, empty.entries(), 0).is_empty());
}

fn menu_tree(popup_id: u32, trigger_id: u32, open: bool) -> NativeTree {
    let mut tree = NativeTree::default();
    let mut trigger = component_part_node(
        NodeTag::Button,
        ROOT_NODE,
        POPOVER_MENU_TRIGGER_PART,
        &[(property::CONTROLS, &popup_id.to_string())],
        &[(property::OPEN, open)],
    );
    trigger.children.clear();
    tree.nodes.insert(trigger_id, trigger);

    let popup = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        POPOVER_MENU_POPUP_PART,
        &[
            (property::MENU, DECLARED_MENU),
            (property::ANCHOR_TARGET, &trigger_id.to_string()),
        ],
        &[
            (property::SELECT_LISTENER, true),
            (property::DISMISS_LISTENER, true),
        ],
    );
    tree.nodes.insert(popup_id, popup);

    let root = tree.nodes.get_mut(&ROOT_NODE).unwrap();
    root.children.push(trigger_id);
    if open {
        root.children.push(popup_id);
    }
    tree
}

#[test]
fn declared_popover_menu_rows_navigate_and_activate_through_the_core_model() {
    let trigger_id = 300;
    let popup_id = 301;
    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(9, menu_tree(popup_id, trigger_id, true), Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        quickgui::Application::new().bind_keys(quickgui::popover_menu_key_bindings()),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();
    let surface = ElementId::new(popup_id as u64);

    let declaration = NativeMenuDeclaration::parse(DECLARED_MENU).unwrap();
    let menu = PopoverMenu::new(menu_items(popup_id, declaration.entries(), 0)).unwrap();
    // Rows mount with the core's derived identities, not the declaring node identity.
    let open_row = menu
        .item_element_id(surface, 1)
        .expect("an interactive row has a derived identity");
    assert!(cx.contains_element(window, open_row).unwrap());
    assert!(
        cx.contains_element(window, menu.item_element_id(surface, 0).unwrap())
            .unwrap()
    );

    cx.focus(window, surface).unwrap();
    // The first enabled entry is highlighted by the core, so one Down lands on the checkbox.
    cx.simulate_keystrokes(window, "down").unwrap();
    cx.simulate_keystrokes(window, "enter").unwrap();

    let selected = events
        .borrow()
        .iter()
        .find(|event| event.kind == "menuselect")
        .cloned()
        .expect("activating a row queues one selection event");
    assert_eq!(selected.target, popup_id);
    assert_eq!(selected.window, 9);
    let payload = selected.value.expect("a selection carries its declared id");
    assert!(
        payload.contains("\"id\":\"sidebar\""),
        "payload was {payload}"
    );
    // A checkbox reports the value the core computed, never a JavaScript-side guess.
    assert!(
        payload.contains("\"checked\":false"),
        "payload was {payload}"
    );
}

#[test]
fn application_shell_services_split_awaited_results_from_fire_and_forget_mutations() {
    assert!(matches!(
        crate::system::parse_app_service_action("set-activation-policy", Some("accessory".into()))
            .expect("an activation policy parses"),
        system::AppServiceAction::SetActivationPolicy(quickgui::ActivationPolicy::Accessory)
    ));
    assert!(
        crate::system::parse_app_service_action("set-activation-policy", Some("nope".into()))
            .is_err()
    );
    assert!(matches!(
        crate::system::parse_app_service_action("request-dock-attention", Some("critical".into()))
            .expect("a dock attention request parses"),
        system::AppServiceAction::RequestDockAttention(quickgui::DockAttention::Critical)
    ));
    assert!(matches!(
        crate::system::parse_app_service_action("move-to-applications-folder", None)
            .expect("the applications-folder move parses"),
        system::AppServiceAction::MoveToApplicationsFolder
    ));
    assert!(crate::system::parse_app_service_action("beep", None).is_err());

    assert!(matches!(
        crate::system::parse_app_mutation_action("activate", Some("true".into()))
            .expect("activation parses"),
        system::AppMutationAction::Activate(true)
    ));
    assert!(matches!(
        crate::system::parse_app_mutation_action("beep", None).expect("beep parses"),
        system::AppMutationAction::Beep
    ));
    assert!(matches!(
        crate::system::parse_app_mutation_action("cancel-dock-attention", Some("42".into()))
            .expect("a cancellation names its request"),
        system::AppMutationAction::CancelDockAttention(42)
    ));
    assert!(crate::system::parse_app_mutation_action("cancel-dock-attention", None).is_err());
    assert!(
        crate::system::parse_app_mutation_action("set-dock-visible", Some("true".into())).is_err()
    );
}

#[test]
fn application_dictionary_words_are_bounded_before_they_reach_the_provider() {
    assert!(matches!(
        crate::system::parse_app_mutation_action("learn-word", Some("quickgui".into()))
            .expect("a bounded word is accepted"),
        system::AppMutationAction::LearnWord(word) if word == "quickgui"
    ));
    assert!(matches!(
        crate::system::parse_app_mutation_action("ignore-word", Some("quickgui".into()))
            .expect("a bounded word is accepted"),
        system::AppMutationAction::IgnoreWord(word) if word == "quickgui"
    ));
    assert!(crate::system::parse_app_mutation_action("learn-word", None).is_err());
    assert!(crate::system::parse_app_mutation_action("learn-word", Some(String::new())).is_err());
    assert!(
        crate::system::parse_app_mutation_action("ignore-word", Some("a".repeat(257))).is_err()
    );
    assert!(
        crate::system::parse_app_mutation_action("ignore-word", Some("a\0b".to_owned())).is_err()
    );
}

#[test]
fn a_stale_dock_attention_identifier_cancels_nothing() {
    crate::system::reset_dock_attention_requests();
    assert!(crate::system::take_dock_attention_request(7).is_none());
}

#[test]
fn restore_state_round_trips_through_the_hosted_boundary() {
    let native = system::NativeWindowRestoreState {
        x: 120.0,
        y: 64.0,
        width: 900.0,
        height: 600.0,
        maximized: false,
        fullscreen: true,
        display_id: Some("3".to_owned()),
        display_uuid: Some("00112233-4455-6677-8899-aabbccddeeff".to_owned()),
        scale_factor: 2.0,
    };
    let core = crate::system::parse_window_restore_state(&native)
        .expect("a well-formed restore state is accepted");
    assert_eq!(core.width, 900.0);
    assert!(core.fullscreen);
    assert_eq!(core.display_id, Some(3));
    assert_eq!(
        core.display_uuid,
        Some([
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff
        ])
    );

    let round_tripped = system::NativeWindowRestoreState::from(core);
    assert_eq!(round_tripped.display_uuid, native.display_uuid);
    assert_eq!(round_tripped.display_id, native.display_id);

    let invalid = system::NativeWindowRestoreState {
        display_uuid: Some("not-a-uuid".to_owned()),
        ..native.clone()
    };
    assert!(crate::system::parse_window_restore_state(&invalid).is_err());
    let invalid = system::NativeWindowRestoreState {
        x: f64::NAN,
        ..native
    };
    assert!(crate::system::parse_window_restore_state(&invalid).is_err());
}

#[test]
fn a_popup_menu_reuses_the_application_menu_grammar_and_requires_one_root() {
    let menu = crate::system::menu::popup_menu(
        "[{\"label\":\"Context\",\"items\":[{\"type\":\"action\",\"id\":5,\"label\":\"Copy\"}]}]",
    )
    .expect("one root menu is accepted");
    assert_eq!(menu.name.as_ref(), "Context");
    assert_eq!(menu.items.len(), 1);

    assert!(crate::system::menu::popup_menu("[]").is_err());
    assert!(
        crate::system::menu::popup_menu(
            "[{\"label\":\"A\",\"items\":[]},{\"label\":\"B\",\"items\":[]}]"
        )
        .is_err()
    );
}

#[test]
fn per_window_menus_travel_as_a_fire_and_forget_declaration() {
    let action = crate::system::parse_window_action(
        "set-menu",
        Some("[{\"label\":\"File\",\"items\":[]}]".into()),
    )
    .expect("a per-window menu parses");
    match action {
        system::WindowAction::SetMenu(Some(json)) => {
            assert!(crate::system::menu::application_menus(&json).is_ok());
        }
        _ => panic!("set-menu must carry its declaration"),
    }
    assert!(matches!(
        crate::system::parse_window_action("set-menu", None)
            .expect("clearing inherits the app menu"),
        system::WindowAction::SetMenu(None)
    ));
}

#[test]
fn hosted_popup_menus_resolve_through_an_asynchronous_request_id() {
    let host = HostCoordinator::new();
    host.enqueue(HostCommand::Request {
        app: 5,
        request: 1,
        command: system::SystemCommand::WindowPopupMenu {
            request: 9,
            window: 2,
            menu: "[{\"label\":\"Context\",\"items\":[]}]".to_owned(),
            position: Some(Point::new(10.0, 20.0)),
        },
    })
    .expect("a popup-menu request fits the host queue");

    let mut commands = host.take_commands().expect("the host queue is readable");
    match commands.pop_front().expect("the request was queued") {
        HostCommand::Request {
            command:
                system::SystemCommand::WindowPopupMenu {
                    request,
                    window,
                    position,
                    ..
                },
            ..
        } => {
            assert_eq!(request, 9);
            assert_eq!(window, 2);
            assert_eq!(position, Some(Point::new(10.0, 20.0)));
        }
        _ => panic!("a popup menu must travel as a system command"),
    }
    assert!(commands.is_empty());
}

#[test]
fn queued_events_drain_ahead_of_polled_request_completions() {
    // The core dispatches a chosen popup item and completes the popup in the same turn, and
    // JavaScript releases the item callbacks on the `popup-menu` completion, so the drained
    // `menu-action` has to come first or the click is lost.
    let mut runtime = NativeRuntime::new(NativeAppOptions::default()).expect("a bare runtime");
    runtime
        .pending_app_services
        .push(system::PendingAppService::completed(9, "popup-menu"));
    enqueue_event(
        &runtime.events,
        QueuedEvent {
            kind: "menu-action",
            window: 1,
            target: 42,
            value: None,
        },
    );

    let drained = runtime
        .drain_events()
        .into_iter()
        .map(|event| (event.kind, event.target))
        .collect::<Vec<_>>();
    assert_eq!(
        drained,
        vec![("menu-action".to_owned(), 42), ("popup-menu".to_owned(), 9)]
    );
    assert!(runtime.pending_app_services.is_empty());
    assert!(runtime.events.borrow().is_empty());
}

#[test]
fn a_declared_context_menu_target_opens_the_core_cursor_point_surface() {
    let target_id = 320;
    let mut tree = NativeTree::default();
    tree.nodes.insert(
        target_id,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            CONTEXT_MENU_TRIGGER_PART,
            &[(property::MENU, DECLARED_MENU)],
            &[(property::SELECT_LISTENER, true)],
        ),
    );
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(target_id);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(11, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        quickgui::Application::new().bind_keys(quickgui::popover_menu_key_bindings()),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();
    let target = ElementId::new(target_id as u64);
    assert!(cx.contains_element(window, target).unwrap());
    assert_eq!(cx.windows().len(), 1);

    cx.simulate_context_menu(
        window,
        target,
        quickgui::Point::new(40.0, 24.0),
        quickgui::Modifiers::empty(),
    )
    .unwrap();
    // The core owns the separate cursor-point surface; JavaScript never creates or measures it.
    assert_eq!(
        cx.windows().len(),
        2,
        "a declared context menu opens exactly one core-owned surface"
    );
}

#[test]
fn declared_css_grid_tracks_and_placement_lay_out_through_taffy() {
    let grid_id = 400;
    let first_id = 401;
    let second_id = 402;
    let mut tree = NativeTree::default();

    let mut grid = NativeNode::new(NodeTag::View);
    grid.parent = Some(ROOT_NODE);
    grid.set_property(
        property::DISPLAY,
        Some(PropertyValue::String(Arc::from("grid"))),
    );
    grid.set_property(
        property::GRID_TEMPLATE_COLUMNS,
        Some(PropertyValue::String(Arc::from("100px 1fr"))),
    );
    grid.set_property(
        property::GRID_TEMPLATE_ROWS,
        Some(PropertyValue::String(Arc::from("repeat(1, 40px)"))),
    );
    grid.set_property(property::WIDTH, Some(PropertyValue::Number(300.0)));
    grid.set_property(property::HEIGHT, Some(PropertyValue::Number(40.0)));
    grid.children.extend([first_id, second_id]);
    tree.nodes.insert(grid_id, grid);

    for id in [first_id, second_id] {
        let mut cell = NativeNode::new(NodeTag::View);
        cell.parent = Some(grid_id);
        tree.nodes.insert(id, cell);
    }
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(grid_id);

    let view = component_part_view(12, tree, Rc::new(RefCell::new(VecDeque::new())));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();

    let first = cx
        .element_bounds(window, ElementId::new(first_id as u64))
        .unwrap();
    let second = cx
        .element_bounds(window, ElementId::new(second_id as u64))
        .unwrap();
    assert_eq!(first.width, 100.0);
    assert_eq!(second.x - first.x, 100.0);
    assert_eq!(second.width, 200.0);
    assert_eq!(first.height, 40.0);
}

#[test]
fn declared_transitions_map_css_names_onto_core_paint_flags_and_easing() {
    let mut node = NativeNode::new(NodeTag::View);
    // The shorthand-only declaration keeps the previous colors-only behavior.
    node.set_property(property::TRANSITION, Some(PropertyValue::Number(120.0)));
    let transition = native_transition(&node).expect("a duration declares a transition");
    assert!((transition.duration.as_secs_f32() - 0.120).abs() < 0.001);
    assert_eq!(
        transition.properties,
        quickgui::TransitionProperties::COLORS
    );

    node.set_property(
        property::TRANSITION_PROPERTIES,
        Some(PropertyValue::String(Arc::from(
            "opacity,box-shadow,border-radius,transform",
        ))),
    );
    node.set_property(
        property::TRANSITION_DURATION,
        Some(PropertyValue::Number(240.0)),
    );
    node.set_property(
        property::TRANSITION_EASING,
        Some(PropertyValue::String(Arc::from("linear"))),
    );
    node.set_property(
        property::TRANSITION_MAX_FPS,
        Some(PropertyValue::Number(30.0)),
    );
    let transition = native_transition(&node).expect("an explicit duration declares a transition");
    assert!((transition.duration.as_secs_f32() - 0.240).abs() < 0.001);
    assert_eq!(
        transition.properties,
        quickgui::TransitionProperties::OPACITY
            | quickgui::TransitionProperties::BOX_SHADOW
            | quickgui::TransitionProperties::BORDER_RADIUS
            | quickgui::TransitionProperties::TRANSFORM
    );
    assert_eq!(transition.max_fps, Some(30.0));
    // Easing comes from the core's own curves, so linear is the identity.
    assert_eq!((transition.easing)(0.25), 0.25);

    // An unknown property name falls back to the core's color set instead of an empty transition.
    node.set_property(
        property::TRANSITION_PROPERTIES,
        Some(PropertyValue::String(Arc::from("left,width"))),
    );
    assert_eq!(
        native_transition(&node).unwrap().properties,
        quickgui::TransitionProperties::COLORS
    );

    let mut plain = NativeNode::new(NodeTag::View);
    plain.set_property(property::COLOR, Some(PropertyValue::Color(0xff00_00ff)));
    assert!(native_transition(&plain).is_none());
}

#[test]
fn range_and_feedback_parts_adopt_core_value_ranges() {
    let mut progress = component_part_node(NodeTag::View, ROOT_NODE, "progress", &[], &[]);
    progress.set_property(property::VALUE, Some(PropertyValue::Number(3.0)));
    progress.set_property(property::MAXIMUM, Some(PropertyValue::Number(12.0)));
    progress.set_property(
        property::VALUE_TEXT,
        Some(PropertyValue::String(Arc::from("3 of 12 files"))),
    );
    let declared = native_progress(41, &progress);
    assert_eq!(declared.current_value(), Some(3.0));
    assert_eq!(declared.maximum(), 12.0);
    assert_eq!(declared.completion(), Some(0.25));
    assert!(apply_part(div(), 41, &progress).is_some());

    progress.set_property(property::INDETERMINATE, Some(PropertyValue::Bool(true)));
    assert!(native_progress(41, &progress).is_indeterminate());

    let mut meter = component_part_node(NodeTag::View, ROOT_NODE, "meter", &[], &[]);
    meter.set_property(property::VALUE, Some(PropertyValue::Number(20.0)));
    meter.set_property(property::MINIMUM, Some(PropertyValue::Number(0.0)));
    meter.set_property(property::MAXIMUM, Some(PropertyValue::Number(100.0)));
    meter.set_property(property::LOW, Some(PropertyValue::Number(25.0)));
    meter.set_property(property::HIGH, Some(PropertyValue::Number(75.0)));
    let declared = native_meter(42, &meter);
    assert_eq!(declared.completion(), 0.2);
    assert!(declared.is_low());
    assert!(!declared.is_high());
    assert!(apply_part(div(), 42, &meter).is_some());

    let pressed = component_part_node(
        NodeTag::Button,
        ROOT_NODE,
        "toggle",
        &[],
        &[(property::PRESSED, true)],
    );
    assert!(apply_part(div(), 43, &pressed).is_some());
    assert!(
        apply_part(
            div(),
            44,
            &component_part_node(NodeTag::View, ROOT_NODE, "toggle-indicator", &[], &[])
        )
        .is_some()
    );
}

/// A one-pixel opaque red PNG.
const RED_PIXEL_PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

#[test]
fn declared_image_sources_decode_once_and_degrade_without_panicking() {
    let mut node = NativeNode::new(NodeTag::Image);
    node.set_property(
        property::VALUE,
        Some(PropertyValue::String(Arc::from(RED_PIXEL_PNG))),
    );
    node.set_property(
        property::OBJECT_FIT,
        Some(PropertyValue::String(Arc::from("cover"))),
    );
    let mut state = NativeImageState::new(Arc::from(RED_PIXEL_PNG));
    assert!(state.parsed.is_ok(), "a base64 data URL decodes once");
    // Re-declaring the same source keeps the decoded image.
    let before = state.source.clone();
    state.sync(RED_PIXEL_PNG);
    assert!(Arc::ptr_eq(&before, &state.source));

    state.sync("/tmp/quickgui-does-not-exist.png");
    // A path stays a lazy core resource; the core's worker pool owns the decode.
    assert!(state.parsed.is_ok());

    let broken = NativeImageState::new(Arc::from("data:image/png;base64,not-base64!!"));
    assert!(broken.parsed.is_err());
    assert!(NativeImageState::new(Arc::from("")).parsed.is_err());
}

#[test]
fn declared_shaders_validate_wgsl_and_bound_their_parameter_vectors() {
    let valid = "fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> {\n\
         return vec4<f32>(input.uv, 0.0, 1.0);\n\
     }";
    let state = NativeShaderState::new(Arc::from(valid));
    assert!(state.parsed.is_ok(), "{:?}", state.parsed.as_ref().err());
    assert!(
        NativeShaderState::new(Arc::from("not wgsl"))
            .parsed
            .is_err()
    );
    assert!(NativeShaderState::new(Arc::from("")).parsed.is_err());

    let mut node = NativeNode::new(NodeTag::Shader);
    node.set_property(
        property::SHADER_PARAMETERS,
        Some(PropertyValue::String(Arc::from("[0.5,0.25,0,1]"))),
    );
    let parameters = native_shader_parameters(&node);
    assert_eq!(parameters.vectors()[0], [0.5, 0.25, 0.0, 1.0]);

    // Extra declared floats are ignored instead of growing the core's fixed uniform.
    let many = (0..64).map(|_| "1").collect::<Vec<_>>().join(",");
    node.set_property(
        property::SHADER_PARAMETERS,
        Some(PropertyValue::String(Arc::from(format!("[{many}]")))),
    );
    let parameters = native_shader_parameters(&node);
    assert_eq!(parameters.vectors().len(), 4);

    node.set_property(
        property::SHADER_PARAMETERS,
        Some(PropertyValue::String(Arc::from("not json"))),
    );
    assert_eq!(native_shader_parameters(&node).vectors()[0], [0.0; 4]);
}

fn input_node(parent: u32, flags: &[(u16, bool)], strings: &[(u16, &str)]) -> NativeNode {
    let mut node = NativeNode::new(NodeTag::View);
    node.parent = Some(parent);
    node.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
    node.set_property(property::HEIGHT, Some(PropertyValue::Number(80.0)));
    node.set_property(property::TAB_INDEX, Some(PropertyValue::Number(0.0)));
    for (key, value) in flags {
        node.set_property(*key, Some(PropertyValue::Bool(*value)));
    }
    for (key, value) in strings {
        node.set_property(*key, Some(PropertyValue::String(Arc::from(*value))));
    }
    node
}

fn queued(events: &EventQueue, kind: &str) -> Option<QueuedEvent> {
    events
        .borrow()
        .iter()
        .find(|event| event.kind == kind)
        .cloned()
}

#[test]
fn declared_input_listeners_report_bounded_core_payloads() {
    let target_id = 500;
    let mut tree = NativeTree::default();
    tree.nodes.insert(
        target_id,
        input_node(
            ROOT_NODE,
            &[
                (property::KEY_DOWN_LISTENER, true),
                (property::KEY_UP_LISTENER, true),
                (property::MOUSE_DOWN_LISTENER, true),
                (property::MOUSE_UP_LISTENER, true),
                (property::MOUSE_MOVE_LISTENER, true),
                (property::DOUBLE_CLICK_LISTENER, true),
                (property::SCROLL_LISTENER, true),
                (property::CONTEXT_MENU_LISTENER, true),
                (property::PINCH_LISTENER, true),
                (property::ROTATION_LISTENER, true),
                (property::SMART_MAGNIFY_LISTENER, true),
                (property::PRESSURE_LISTENER, true),
                (property::FOCUS_LISTENER, true),
            ],
            &[],
        ),
    );
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(target_id);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(21, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    let target = ElementId::new(target_id as u64);

    cx.focus(window, target).unwrap();
    let focus = queued(&events, "focus").expect("focus is reported once");
    assert_eq!(focus.target, target_id);
    assert_eq!(focus.window, 21);
    let renders = cx.render_count(window).unwrap();

    cx.simulate_keystrokes(window, "cmd-shift-k").unwrap();
    let key = queued(&events, "keydown").expect("a declared key listener reports the press");
    let payload = key.value.expect("a key press carries its payload");
    assert!(payload.contains("\"key\":\"k\""), "payload was {payload}");
    assert!(payload.contains("\"meta\":true"), "payload was {payload}");
    assert!(payload.contains("\"shift\":true"), "payload was {payload}");
    assert!(
        payload.contains("\"repeat\":false"),
        "payload was {payload}"
    );

    cx.simulate_key_up(
        window,
        quickgui::Keystroke::parse("k").expect("a bounded keystroke"),
    )
    .unwrap();
    assert!(queued(&events, "keyup").is_some());

    cx.simulate_mouse_down(
        window,
        target,
        quickgui::MouseDownEvent {
            button: quickgui::MouseButton::Left,
            position: quickgui::Point::new(12.0, 8.0),
            modifiers: quickgui::Modifiers::empty(),
            click_count: 2,
            first_mouse: false,
        },
    )
    .unwrap();
    let down = queued(&events, "mousedown").expect("a mouse press is reported");
    let payload = down.value.expect("a mouse press carries its payload");
    assert!(
        payload.contains("\"button\":\"left\""),
        "payload was {payload}"
    );
    assert!(
        payload.contains("\"clickCount\":2"),
        "payload was {payload}"
    );
    // The exact native multi-click count comes from the core, not a JavaScript timer.
    assert!(queued(&events, "dblclick").is_some());

    cx.simulate_mouse_up(
        window,
        target,
        quickgui::MouseUpEvent {
            button: quickgui::MouseButton::Left,
            position: quickgui::Point::new(12.0, 8.0),
            modifiers: quickgui::Modifiers::empty(),
            click_count: 2,
        },
    )
    .unwrap();
    assert!(queued(&events, "mouseup").is_some());

    cx.simulate_mouse_move(
        window,
        target,
        quickgui::MouseMoveEvent {
            position: quickgui::Point::new(20.0, 10.0),
            pressed_button: None,
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    assert!(queued(&events, "mousemove").is_some());

    cx.simulate_scroll_wheel(
        window,
        target,
        quickgui::ScrollWheelEvent {
            position: quickgui::Point::new(20.0, 10.0),
            delta: quickgui::ScrollDelta::Pixels(quickgui::Vector::new(0.0, -24.0)),
            phase: quickgui::GesturePhase::Moved,
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    let wheel = queued(&events, "wheel").expect("a wheel event is reported");
    let payload = wheel.value.expect("a wheel event carries its payload");
    assert!(
        payload.contains("\"deltaY\":-24.0"),
        "payload was {payload}"
    );
    assert!(
        payload.contains("\"precise\":true"),
        "payload was {payload}"
    );

    cx.simulate_context_menu(
        window,
        target,
        quickgui::Point::new(30.0, 12.0),
        quickgui::Modifiers::CONTROL,
    )
    .unwrap();
    let menu = queued(&events, "contextmenu").expect("a secondary click is reported");
    assert!(
        menu.value
            .expect("a context-menu event carries its payload")
            .contains("\"control\":true")
    );

    cx.simulate_pinch(
        window,
        target,
        quickgui::PinchEvent {
            position: quickgui::Point::new(4.0, 4.0),
            delta: 0.1,
            phase: quickgui::GesturePhase::Started,
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    let pinch = queued(&events, "pinch").expect("a pinch gesture is reported");
    assert!(
        pinch
            .value
            .expect("a pinch carries its payload")
            .contains("\"phase\":\"started\"")
    );

    cx.simulate_rotation(
        window,
        target,
        quickgui::RotationEvent {
            position: quickgui::Point::new(4.0, 4.0),
            delta: -12.0,
            phase: quickgui::GesturePhase::Moved,
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    assert!(queued(&events, "rotate").is_some());

    cx.simulate_smart_magnify(
        window,
        target,
        quickgui::SmartMagnifyEvent {
            position: quickgui::Point::new(4.0, 4.0),
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    assert!(queued(&events, "smartmagnify").is_some());

    cx.simulate_mouse_pressure(
        window,
        target,
        quickgui::MousePressureEvent {
            position: quickgui::Point::new(4.0, 4.0),
            pressure: 0.5,
            stage: quickgui::PressureStage::Force,
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    let pressure = queued(&events, "pressure").expect("a pressure stage is reported");
    assert!(
        pressure
            .value
            .expect("a pressure event carries its payload")
            .contains("\"stage\":\"force\"")
    );
    assert_eq!(
        cx.render_count(window).unwrap(),
        renders,
        "forwarding input without a mutation must not rebuild the view"
    );
}

#[test]
fn a_declared_keymap_parses_accelerators_through_the_core_and_dispatches_ids() {
    let declaration =
        r#"{"CmdOrCtrl+S":"save","CmdOrCtrl+Shift+P":"palette","Nonsense++":"skipped","Alt+X":""}"#;
    let keymap = NativeKeymap::parse(declaration).expect("valid accelerators form a keymap");
    // The unparsable accelerator and the empty identifier are skipped, not fatal.
    assert_eq!(keymap.len(), 2);
    assert!(NativeKeymap::parse("not json").is_none());
    assert!(NativeKeymap::parse(&"x".repeat(MAX_KEYMAP_JSON_BYTES + 1)).is_none());

    let target_id = 520;
    let mut tree = NativeTree::default();
    tree.nodes.insert(
        target_id,
        input_node(
            ROOT_NODE,
            &[(property::ACTION_LISTENER, true)],
            &[(property::KEYMAP, declaration)],
        ),
    );
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(target_id);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(22, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    let target = ElementId::new(target_id as u64);

    cx.focus(window, target).unwrap();
    cx.simulate_keystrokes(window, "cmd-s").unwrap();
    let action = queued(&events, "action").expect("a declared accelerator dispatches its id");
    assert_eq!(action.target, target_id);
    assert_eq!(action.value.as_deref(), Some("save"));

    // An unbound keystroke without a declared key listener queues nothing.
    events.borrow_mut().clear();
    cx.simulate_keystrokes(window, "cmd-q").unwrap();
    assert!(queued(&events, "action").is_none());
    assert!(queued(&events, "keydown").is_none());
}

#[test]
fn declared_drag_sources_and_drop_targets_route_typed_core_payloads() {
    let source_id = 540;
    let target_id = 541;
    let mut tree = NativeTree::default();
    tree.nodes.insert(
        source_id,
        input_node(
            ROOT_NODE,
            &[(property::DRAG_LISTENER, true)],
            &[(
                property::DRAGGABLE,
                r#"{"id":"row-7","text":"quickgui","files":[{"path":"/tmp/a.txt"}]}"#,
            )],
        ),
    );
    tree.nodes.insert(
        target_id,
        input_node(
            ROOT_NODE,
            &[(property::DROP_LISTENER, true)],
            &[(property::DROP_KINDS, r#"["local","files"]"#)],
        ),
    );
    let root = tree.nodes.get_mut(&ROOT_NODE).unwrap();
    root.children.push(source_id);
    root.children.push(target_id);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(23, tree, Rc::clone(&events));
    let (cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    // Both the declared drag source and both declared drop kinds mount on the core element tree.
    assert!(
        cx.contains_element(window, ElementId::new(source_id as u64))
            .unwrap()
    );
    assert!(
        cx.contains_element(window, ElementId::new(target_id as u64))
            .unwrap()
    );
    assert!(events.borrow().is_empty());
}

#[test]
fn a_drag_source_that_also_captures_the_pointer_is_ignored_instead_of_panicking() {
    let node_id = 560;
    let mut tree = NativeTree::default();
    let mut conflicting = input_node(
        ROOT_NODE,
        &[
            (property::POINTER_LISTENER, true),
            (property::DRAG_LISTENER, true),
            (property::DROP_LISTENER, true),
        ],
        &[
            (property::DRAGGABLE, r#"{"id":"row-1"}"#),
            (property::DROP_KINDS, r#"["local"]"#),
        ],
    );
    conflicting.parent = Some(ROOT_NODE);
    tree.nodes.insert(node_id, conflicting);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(node_id);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(24, tree, Rc::clone(&events));
    // The core rejects one element owning both, so the drag source is dropped and the element and
    // its declared drop target still mount.
    let (cx, view) = quickgui::TestAppContext::new(view).unwrap();
    assert!(
        cx.contains_element(view.window_handle(), ElementId::new(node_id as u64))
            .unwrap()
    );
}

// ---------------------------------------------------------------------------
// Declared range, ordering, and roving-focus components
//
// Two instances of one component are declared in the same hosted window on purpose: the core now
// reaches each one through a per-instance `StateAccessor`, and these tests fail if a listener
// resolves the wrong instance.
// ---------------------------------------------------------------------------

fn component_application() -> quickgui::Application {
    quickgui::Application::new()
        .bind_keys(quickgui::slider_key_bindings())
        .bind_keys(quickgui::splitter_key_bindings())
        .bind_keys(quickgui::toolbar_key_bindings())
        .bind_keys(quickgui::toggle_group_key_bindings())
}

fn insert_component_node(tree: &mut NativeTree, id: u32, parent: u32, node: NativeNode) {
    tree.nodes.insert(id, node);
    tree.nodes
        .get_mut(&parent)
        .expect("a declared component parent exists")
        .children
        .push(id);
}

/// Take every queued change for one declared instance and return the last one.
///
/// The core reports each frame it moved a value, so a two-key gesture leaves two events; the last
/// one is the value the hosted runtime will commit. Events for other instances stay queued.
fn component_change(events: &EventQueue, target: u32) -> serde_json::Value {
    let mut queue = events.borrow_mut();
    let mut latest = serde_json::Value::Null;
    let mut remaining = VecDeque::with_capacity(queue.len());
    while let Some(event) = queue.pop_front() {
        if event.kind == "componentchange" && event.target == target {
            latest = serde_json::from_str(event.value.as_deref().unwrap_or("null"))
                .expect("a component change carries bounded JSON");
        } else {
            remaining.push_back(event);
        }
    }
    *queue = remaining;
    latest
}

#[test]
fn declared_sliders_keep_independent_values_and_report_them_asynchronously() {
    let first_id = 400;
    let second_id = 401;
    let mut tree = NativeTree::default();
    for (id, scope, values) in [(first_id, "left", "[10]"), (second_id, "right", "[60]")] {
        let mut node = component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "slider",
            &[
                (property::SCOPE, scope),
                (property::VALUES, values),
                (property::ACCESSIBILITY_LABEL, scope),
            ],
            &[(property::COMPONENT_CHANGE_LISTENER, true)],
        );
        node.set_property(property::MINIMUM, Some(PropertyValue::Number(0.0)));
        node.set_property(property::MAXIMUM, Some(PropertyValue::Number(100.0)));
        node.set_property(property::STEP, Some(PropertyValue::Number(5.0)));
        insert_component_node(&mut tree, id, ROOT_NODE, node);
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(11, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        component_application(),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();
    let left = ElementId::named("left");
    let right = ElementId::named("right");

    cx.focus(window, left).unwrap();
    cx.simulate_keystrokes(window, "right right").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change(&events, first_id),
        serde_json::json!({ "values": [20.0], "dragging": false, "committed": false, "displayValue": null })
    );

    // The second slider never moved, so it reports nothing at all.
    assert_eq!(
        component_change(&events, second_id),
        serde_json::Value::Null
    );

    cx.focus(window, right).unwrap();
    cx.simulate_keystrokes(window, "home").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change(&events, second_id),
        serde_json::json!({ "values": [0.0], "dragging": false, "committed": false, "displayValue": null })
    );

    // The core keeps the value it decided until JavaScript commits the matching declaration, and
    // a settled window schedules no extra frame for either instance.
    let renders = cx.render_count(window).unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(cx.render_count(window).unwrap(), renders);
    assert_eq!(
        cx.read(view, |view| view
            .components
            .sliders
            .values()
            .map(|slider| slider.state.value())
            .sum::<f64>())
            .unwrap(),
        20.0
    );
}

#[test]
fn declared_range_slider_thumbs_move_independently_through_the_core() {
    let root_id = 410;
    let lower_id = 411;
    let upper_id = 412;
    let mut tree = NativeTree::default();
    let mut root = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "slider",
        &[(property::SCOPE, "price"), (property::VALUES, "[20,80]")],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    root.set_property(property::MINIMUM, Some(PropertyValue::Number(0.0)));
    root.set_property(property::MAXIMUM, Some(PropertyValue::Number(100.0)));
    root.set_property(property::STEP, Some(PropertyValue::Number(10.0)));
    insert_component_node(&mut tree, root_id, ROOT_NODE, root);
    for (id, index) in [(lower_id, 0.0), (upper_id, 1.0)] {
        let mut thumb = component_part_node(
            NodeTag::View,
            root_id,
            "slider-thumb",
            &[(property::SCOPE, "price")],
            &[],
        );
        thumb.set_property(property::ITEM_INDEX, Some(PropertyValue::Number(index)));
        insert_component_node(&mut tree, id, root_id, thumb);
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(12, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        component_application(),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();
    let slider = Slider::new(
        ElementId::named("price"),
        &SliderState::range(0.0, 100.0, &[20.0, 80.0]),
    );

    cx.focus(window, slider.thumb_id(1)).unwrap();
    cx.simulate_keystrokes(window, "right").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "values": [20.0, 90.0], "dragging": false, "committed": false, "displayValue": null })
    );

    cx.focus(window, slider.thumb_id(0)).unwrap();
    cx.simulate_keystrokes(window, "left").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "values": [10.0, 90.0], "dragging": false, "committed": false, "displayValue": null })
    );
}

#[test]
fn declared_splitter_handles_conserve_sizes_and_report_them() {
    let root_id = 420;
    let handle_id = 421;
    let mut tree = NativeTree::default();
    let mut root = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "splitter",
        &[
            (property::SCOPE, "panes"),
            (property::VALUES, "[200,200]"),
            (property::ITEMS, r#"[{"min":80},{"min":80}]"#),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    root.set_property(property::STEP, Some(PropertyValue::Number(16.0)));
    insert_component_node(&mut tree, root_id, ROOT_NODE, root);
    let mut handle = component_part_node(
        NodeTag::View,
        root_id,
        "splitter-handle",
        &[(property::SCOPE, "panes")],
        &[],
    );
    handle.set_property(property::ITEM_INDEX, Some(PropertyValue::Number(0.0)));
    insert_component_node(&mut tree, handle_id, root_id, handle);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(13, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        component_application(),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();
    let splitter = Splitter::new(
        ElementId::named("panes"),
        &SplitterState::new(SplitterOrientation::Horizontal, &[200.0, 200.0]),
    );

    cx.focus(window, splitter.handle_id(0)).unwrap();
    cx.simulate_keystrokes(window, "right").unwrap();
    cx.run_until_idle().unwrap();
    // The core conserves the total across the drag, so both sizes travel back together.
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "sizes": [216.0, 184.0] })
    );
}

#[test]
fn declared_toolbars_and_toggle_groups_rove_and_report_per_instance() {
    let toolbar_id = 430;
    let toolbar_item_ids = [431_u32, 432];
    let group_id = 440;
    let group_item_ids = [441_u32, 442];
    let mut tree = NativeTree::default();

    let toolbar = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "toolbar",
        &[
            (property::SCOPE, "actions"),
            (
                property::ITEMS,
                r#"[{"value":"cut"},{"value":"copy"},{"value":"cut"}]"#,
            ),
            (property::ACTIVE_VALUE, "cut"),
        ],
        &[
            (property::COMPONENT_CHANGE_LISTENER, true),
            (property::LOOP_FOCUS, true),
        ],
    );
    insert_component_node(&mut tree, toolbar_id, ROOT_NODE, toolbar);
    for (id, value) in toolbar_item_ids.iter().zip(["cut", "copy"]) {
        let item = component_part_node(
            NodeTag::Button,
            toolbar_id,
            "toolbar-item",
            &[(property::SCOPE, "actions"), (property::PART_VALUE, value)],
            &[],
        );
        insert_component_node(&mut tree, *id, toolbar_id, item);
    }

    let group = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "toggle-group",
        &[
            (property::SCOPE, "align"),
            (property::ITEMS, r#"[{"value":"left"},{"value":"right"}]"#),
            (property::VALUES, r#"["left"]"#),
            (property::VARIANT, "multiple"),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    insert_component_node(&mut tree, group_id, ROOT_NODE, group);
    for (id, value) in group_item_ids.iter().zip(["left", "right"]) {
        let item = component_part_node(
            NodeTag::Button,
            group_id,
            "toggle-group-item",
            &[(property::SCOPE, "align"), (property::PART_VALUE, value)],
            &[],
        );
        insert_component_node(&mut tree, *id, group_id, item);
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(14, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        component_application(),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();

    // The duplicate declared toolbar value is dropped rather than panicking the core.
    assert_eq!(
        cx.read(view, |view| view
            .components
            .toolbars
            .values()
            .map(|toolbar| toolbar.items.len())
            .sum::<usize>())
            .unwrap(),
        2
    );

    let toolbar_state = ToolbarState::new(ElementId::named("cut"));
    let items = [
        ToolbarItem::new(ElementId::named("cut")),
        ToolbarItem::new(ElementId::named("copy")),
    ];
    let bar = Toolbar::new(ElementId::named("actions"), &toolbar_state, &items);
    cx.focus(window, bar.item_id(ElementId::named("cut")))
        .unwrap();
    cx.simulate_keystrokes(window, "right").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change(&events, toolbar_id),
        serde_json::json!({ "active": "copy" })
    );
    // Moving the toolbar's roving stop never touches the toggle group declared beside it.
    assert_eq!(component_change(&events, group_id), serde_json::Value::Null);

    let group_state = ToggleGroupState::multiple();
    let group_items = [
        ToggleGroupItem::new(ElementId::named("left")),
        ToggleGroupItem::new(ElementId::named("right")),
    ];
    let group = ToggleGroup::new(ElementId::named("align"), &group_state, &group_items);
    cx.click(window, group.item_id(ElementId::named("right")))
        .unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change(&events, group_id),
        serde_json::json!({ "pressed": ["left", "right"] })
    );
}

#[test]
fn malformed_component_declarations_are_bounded_instead_of_panicking() {
    let slider_id = 450;
    let splitter_id = 451;
    let toolbar_id = 452;
    let mut tree = NativeTree::default();
    insert_component_node(
        &mut tree,
        slider_id,
        ROOT_NODE,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "slider",
            &[(property::SCOPE, "broken"), (property::VALUES, "{oops")],
            &[],
        ),
    );
    insert_component_node(
        &mut tree,
        splitter_id,
        ROOT_NODE,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "splitter",
            &[
                (property::SCOPE, "single"),
                (property::VALUES, "[10]"),
                (property::ITEMS, "not json"),
            ],
            &[],
        ),
    );
    insert_component_node(
        &mut tree,
        toolbar_id,
        ROOT_NODE,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "toolbar",
            &[(property::SCOPE, "empty"), (property::ITEMS, "[]")],
            &[],
        ),
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(15, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        component_application(),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    cx.run_until_idle().unwrap();

    // A malformed declaration still produces one bounded, mounted instance of each component.
    assert_eq!(
        cx.read(view, |view| (
            view.components.sliders.len(),
            view.components.splitters.len(),
            view.components.toolbars.len(),
        ))
        .unwrap(),
        (1, 1, 1)
    );
    // A splitter needs two panes, so the short declaration falls back instead of panicking.
    assert_eq!(
        cx.read(view, |view| view
            .components
            .splitters
            .values()
            .map(|splitter| splitter.state.sizes().len())
            .sum::<usize>())
            .unwrap(),
        2
    );
    assert!(events.borrow().is_empty());
}

// ---------------------------------------------------------------------------
// Declared option sources, virtual collections, and the remaining stateful fields
// ---------------------------------------------------------------------------

/// An application bound to every typed action the newly declared components adopt.
fn collection_application() -> quickgui::Application {
    component_application()
        .bind_keys(quickgui::picker_key_bindings())
        .bind_keys(quickgui::select_key_bindings())
        .bind_keys(quickgui::combobox_key_bindings())
        .bind_keys(quickgui::table_key_bindings())
        .bind_keys(quickgui::tree_key_bindings())
        .bind_keys(quickgui::date_field_key_bindings())
        .bind_keys(quickgui::time_field_key_bindings())
        .bind_keys(quickgui::calendar_key_bindings())
        .bind_keys(quickgui::menubar_key_bindings())
}

/// Take every queued event of one kind for one target and return the last payload.
fn queued_payload(events: &EventQueue, kind: &str, target: u32) -> serde_json::Value {
    let mut queue = events.borrow_mut();
    let mut latest = serde_json::Value::Null;
    let mut remaining = VecDeque::with_capacity(queue.len());
    while let Some(event) = queue.pop_front() {
        if event.kind == kind && event.target == target {
            latest = serde_json::from_str(event.value.as_deref().unwrap_or("null"))
                .expect("a declared component event carries bounded JSON");
        } else {
            remaining.push_back(event);
        }
    }
    *queue = remaining;
    latest
}

/// Merge every queued change for one target, so a multi-frame gesture reads as one payload.
fn merged_component_change(
    events: &EventQueue,
    target: u32,
) -> serde_json::Map<String, serde_json::Value> {
    let mut queue = events.borrow_mut();
    let mut merged = serde_json::Map::new();
    let mut remaining = VecDeque::with_capacity(queue.len());
    while let Some(event) = queue.pop_front() {
        if event.kind == "componentchange" && event.target == target {
            let value: serde_json::Value =
                serde_json::from_str(event.value.as_deref().unwrap_or("null"))
                    .expect("a component change carries bounded JSON");
            if let serde_json::Value::Object(object) = value {
                merged.extend(object);
            }
        } else {
            remaining.push_back(event);
        }
    }
    *queue = remaining;
    merged
}

fn mounted_component_view(
    tree: NativeTree,
    events: EventQueue,
) -> (
    quickgui::TestAppContext,
    quickgui::TestWindowHandle<NativeView>,
) {
    let view = component_part_view(90, tree, events);
    quickgui::TestAppContext::from_application(
        collection_application(),
        quickgui::WindowOptions::default(),
        view,
    )
    .expect("the hosted component view mounts")
}

#[test]
fn declared_select_options_commit_through_the_core_popover() {
    let id = 500;
    let mut tree = NativeTree::default();
    let node = component_part_node(
        NodeTag::Button,
        ROOT_NODE,
        "select",
        &[
            (property::SCOPE, "theme"),
            (
                property::OPTIONS,
                r#"[{"value":"light","label":"Light"},{"value":"dark","label":"Dark"},{"value":"light","label":"Duplicate"}]"#,
            ),
            (property::ACCESSIBILITY_LABEL, "Theme"),
        ],
        &[
            (property::COMPONENT_CHANGE_LISTENER, true),
            (property::COMMIT_LISTENER, true),
        ],
    );
    insert_component_node(&mut tree, id, ROOT_NODE, node);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let trigger = ElementId::named("theme");

    // A duplicate declared value keeps its first occurrence, so the core never sees a source it
    // would reject for a reason the caller cannot see.
    assert_eq!(
        cx.read(view, |view| view
            .components
            .selects
            .values()
            .map(|select| select.state.items().len())
            .sum::<usize>())
            .unwrap(),
        2
    );

    cx.click(window, trigger).unwrap();
    let popover = cx
        .read(view, |view| {
            view.components
                .selects
                .values()
                .next()
                .and_then(|select| select.state.popover_window())
        })
        .unwrap()
        .expect("the core opens its own native popover window");
    assert_eq!(
        cx.window_state(popover).unwrap().kind,
        quickgui::WindowKind::SystemPopover
    );

    cx.simulate_keystrokes(popover, "down enter").unwrap();
    cx.run_until_idle().unwrap();
    assert!(!cx.is_window_open(popover));
    assert_eq!(
        queued_payload(&events, "commit", id),
        serde_json::json!({ "value": "dark" })
    );
    let change = merged_component_change(&events, id);
    assert_eq!(change.get("value"), Some(&serde_json::json!("dark")));
    assert_eq!(change.get("open"), Some(&serde_json::json!(false)));
}

#[test]
fn declared_option_children_and_malformed_sources_never_panic() {
    let select_id = 510;
    let option_ids = [511_u32, 512];
    let malformed_id = 520;
    let mut tree = NativeTree::default();
    let select = component_part_node(
        NodeTag::Button,
        ROOT_NODE,
        "select",
        &[(property::SCOPE, "child-options")],
        &[],
    );
    insert_component_node(&mut tree, select_id, ROOT_NODE, select);
    for (id, value, label) in [(option_ids[0], "one", "One"), (option_ids[1], "two", "Two")] {
        let option = component_part_node(
            NodeTag::Sentinel,
            select_id,
            "option",
            &[(property::PART_VALUE, value), (property::VALUE, label)],
            &[],
        );
        insert_component_node(&mut tree, id, select_id, option);
    }
    let malformed = component_part_node(
        NodeTag::Input,
        ROOT_NODE,
        "autocomplete",
        &[
            (property::SCOPE, "broken"),
            (property::OPTIONS, "{not json"),
            (property::APPEARANCE, "{also not json"),
        ],
        &[],
    );
    insert_component_node(&mut tree, malformed_id, ROOT_NODE, malformed);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    cx.run_until_idle().unwrap();

    let counts = cx
        .read(view, |view| {
            (
                view.components
                    .selects
                    .values()
                    .map(|select| select.state.items().len())
                    .sum::<usize>(),
                view.components
                    .autocompletes
                    .values()
                    .map(|state| state.state.items().len())
                    .sum::<usize>(),
            )
        })
        .unwrap();
    assert_eq!(counts, (2, 0));
    // A declared option contributes no element of its own: every row is painted by the core in
    // its own window from the declared appearance.
    assert!(
        !cx.contains_element(
            view.window_handle(),
            ElementId::new(u64::from(option_ids[0]))
        )
        .unwrap()
    );
}

#[test]
fn declared_autocomplete_reports_the_free_form_value_the_core_retains() {
    let id = 530;
    let mut tree = NativeTree::default();
    let node = component_part_node(
        NodeTag::Input,
        ROOT_NODE,
        "autocomplete",
        &[
            (property::SCOPE, "search"),
            (
                property::OPTIONS,
                r#"[{"value":"alpha","label":"Alpha"},{"value":"beta","label":"Beta"}]"#,
            ),
            (property::ACCESSIBILITY_LABEL, "Search"),
            (property::INPUT_VALUE, "al"),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    insert_component_node(&mut tree, id, ROOT_NODE, node);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let control = ElementId::named("search");

    // The declared value seeds the retained state; the core owns every edit after that.
    assert_eq!(
        cx.read(view, |view| view
            .components
            .autocompletes
            .values()
            .next()
            .map(|state| state.state.value().to_string()))
            .unwrap(),
        Some("al".to_owned())
    );

    cx.focus(window, control).unwrap();
    cx.simulate_input(window, "p").unwrap();
    cx.run_until_idle().unwrap();
    let change = merged_component_change(&events, id);
    assert_eq!(change.get("inputValue"), Some(&serde_json::json!("alp")));
}

#[test]
fn declared_table_renders_visible_cells_and_reports_what_the_core_decides() {
    let table_id = 600;
    let header_id = 601;
    let row_id = 602;
    let cell_id = 603;
    let mut tree = NativeTree::default();
    let mut table = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "table",
        &[
            (property::SCOPE, "files"),
            (
                property::COLUMNS,
                r#"[{"id":"name","label":"Name","width":160,"sortable":true},{"id":"size","label":"Size","track":"1fr"},{"id":"name","label":"Duplicate"}]"#,
            ),
            (property::SELECTION_MODE, "multiple"),
            (property::SORT_COLUMN, "name"),
            (property::SORT_DIRECTION, "descending"),
        ],
        &[
            (property::COMPONENT_CHANGE_LISTENER, true),
            (property::COMMIT_LISTENER, true),
        ],
    );
    table.set_property(property::ROW_COUNT, Some(PropertyValue::Number(64.0)));
    table.set_property(property::ROW_HEIGHT, Some(PropertyValue::Number(24.0)));
    insert_component_node(&mut tree, table_id, ROOT_NODE, table);
    let header = component_part_node(
        NodeTag::View,
        table_id,
        "table-header",
        &[(property::PART_VALUE, "name")],
        &[],
    );
    insert_component_node(&mut tree, header_id, table_id, header);
    let mut row = component_part_node(NodeTag::View, table_id, "table-row", &[], &[]);
    row.set_property(property::ROW_INDEX, Some(PropertyValue::Number(0.0)));
    insert_component_node(&mut tree, row_id, table_id, row);
    let cell = component_part_node(
        NodeTag::View,
        row_id,
        "table-cell",
        &[
            (property::PART_VALUE, "name"),
            (property::ACCESSIBILITY_LABEL, "first cell"),
        ],
        &[],
    );
    insert_component_node(&mut tree, cell_id, row_id, cell);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let root = ElementId::named("files");
    cx.run_until_idle().unwrap();

    // The duplicate declared column identifier is dropped before the core can panic on it, and
    // the declared sort survives into the retained state.
    let (columns, sort, rows) = cx
        .read(view, |view| {
            let table = view.components.tables.values().next().unwrap();
            (
                table.columns.len(),
                table.state.sort().map(|sort| sort.direction),
                table.state.row_count(),
            )
        })
        .unwrap();
    assert_eq!(columns, 2);
    assert_eq!(sort, Some(quickgui::TableSortDirection::Descending));
    assert_eq!(rows, 64);

    // The core reports the range it is virtualizing so JavaScript can declare exactly those rows.
    let change = merged_component_change(&events, table_id);
    let visible = change
        .get("visibleRange")
        .expect("the core reports the range it mounted");
    assert_eq!(visible.get("start"), Some(&serde_json::json!(0)));

    // The declared cell is mounted by the core's own row renderer, under the core's cell identity.
    let cell_element = TableState::cell_id(root, TableCellPosition { row: 0, column: 0 });
    assert!(cx.contains_element(window, cell_element).unwrap());

    cx.focus(window, root).unwrap();
    cx.simulate_keystrokes(window, "down down").unwrap();
    cx.run_until_idle().unwrap();
    let change = merged_component_change(&events, table_id);
    assert_eq!(
        change.get("selectedRanges"),
        Some(&serde_json::json!([[2, 2]]))
    );
    assert_eq!(
        change.get("activeCell"),
        Some(&serde_json::json!({ "row": 2, "column": 0 }))
    );

    // Column resizing is the core's own typed action on the handle it hands the declared header,
    // and the width it decided travels back keyed by the caller's declared identifier.
    cx.focus(
        window,
        TableState::resize_handle_id(root, ElementId::named("name")),
    )
    .unwrap();
    cx.simulate_keystrokes(window, "right").unwrap();
    cx.run_until_idle().unwrap();
    let change = merged_component_change(&events, table_id);
    assert_eq!(
        change.get("columnWidths"),
        Some(&serde_json::json!([{
            "id": "name",
            "width": 160.0 + quickgui::TABLE_COLUMN_RESIZE_STEP,
        }]))
    );

    // Reordering is likewise the core's own typed action over the declared display order.
    cx.focus(window, root).unwrap();
    cx.simulate_keystrokes(window, "alt-right").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        merged_component_change(&events, table_id).get("columnOrder"),
        Some(&serde_json::json!(["size", "name"]))
    );
}

#[test]
fn declared_tree_expands_through_the_core_and_asks_for_lazy_children() {
    let tree_id = 620;
    let row_ids = [621_u32, 622];
    let mut tree = NativeTree::default();
    let node = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "tree",
        &[
            (property::SCOPE, "explorer"),
            (
                property::NODES,
                r#"[{"id":"src","label":"src","pending":true},{"id":"readme","label":"README"}]"#,
            ),
            (property::LOADING_LABEL, "Fetching…"),
        ],
        &[
            (property::COMPONENT_CHANGE_LISTENER, true),
            (property::COMMIT_LISTENER, true),
        ],
    );
    insert_component_node(&mut tree, tree_id, ROOT_NODE, node);
    for (id, value) in row_ids.iter().zip(["src", "readme"]) {
        let row = component_part_node(
            NodeTag::View,
            tree_id,
            "tree-row",
            &[(property::PART_VALUE, value)],
            &[],
        );
        insert_component_node(&mut tree, *id, tree_id, row);
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let root = ElementId::named("explorer");
    cx.run_until_idle().unwrap();
    assert_eq!(
        cx.read(view, |view| view
            .components
            .trees
            .values()
            .next()
            .map(|state| state.state.node_count()))
            .unwrap(),
        Some(2)
    );

    // Expanding a pending branch is the core's decision; the binding only forwards the request.
    cx.click(
        window,
        TreeState::<Arc<str>>::disclosure_id(root, ElementId::named("src")),
    )
    .unwrap();
    cx.run_until_idle().unwrap();
    let change = merged_component_change(&events, tree_id);
    assert_eq!(change.get("loadChildren"), Some(&serde_json::json!("src")));
    assert_eq!(change.get("expanded"), Some(&serde_json::json!(["src"])));

    // Supplying the children is a declaration, spliced atomically by the core.
    cx.update(view, |view, _cx| {
        view.tree
            .borrow_mut()
            .nodes
            .get_mut(&tree_id)
            .unwrap()
            .set_property(
                property::SET_CHILDREN,
                Some(PropertyValue::String(Arc::from(
                    r#"{"id":"src","children":[{"id":"main","label":"main.rs"}]}"#,
                ))),
            );
    })
    .unwrap();
    cx.update(view, |_view, cx| cx.invalidate()).unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        cx.read(view, |view| view
            .components
            .trees
            .values()
            .next()
            .map(|state| (state.state.node_count(), state.names.len())))
            .unwrap(),
        Some((3, 3))
    );
}

#[test]
fn declared_number_field_parses_and_steps_through_the_core() {
    let field_id = 640;
    let input_id = 641;
    let increment_id = 642;
    let mut tree = NativeTree::default();
    let mut field = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "number-field",
        &[(property::SCOPE, "quantity"), (property::VALUES, "[2]")],
        &[
            (property::COMPONENT_CHANGE_LISTENER, true),
            (property::COMMIT_LISTENER, true),
        ],
    );
    field.set_property(property::MINIMUM, Some(PropertyValue::Number(0.0)));
    field.set_property(property::MAXIMUM, Some(PropertyValue::Number(1000.0)));
    field.set_property(property::STEP, Some(PropertyValue::Number(2.0)));
    insert_component_node(&mut tree, field_id, ROOT_NODE, field);
    let input = component_part_node(
        NodeTag::Input,
        field_id,
        "number-field-input",
        &[(property::SCOPE, "quantity")],
        &[(property::COMMIT_LISTENER, true)],
    );
    insert_component_node(&mut tree, input_id, field_id, input);
    let increment = component_part_node(
        NodeTag::Button,
        field_id,
        "number-field-increment",
        &[(property::SCOPE, "quantity")],
        &[],
    );
    insert_component_node(&mut tree, increment_id, field_id, increment);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let root = ElementId::named("quantity");
    let field = NumberField::new(root);

    cx.focus(window, field.input_id()).unwrap();
    cx.simulate_input(window, "4").unwrap();
    cx.run_until_idle().unwrap();
    let change = merged_component_change(&events, field_id);
    // The declared value seeded the editor, so typing appends to the core's retained text.
    assert_eq!(change.get("text"), Some(&serde_json::json!("24")));
    assert_eq!(change.get("numberValue"), Some(&serde_json::json!(24.0)));
    assert_eq!(change.get("valid"), Some(&serde_json::json!(true)));

    cx.simulate_keystrokes(window, "enter").unwrap();
    cx.run_until_idle().unwrap();
    // Commit is declared on the input part, which is the control the core submits.
    assert_eq!(
        queued_payload(&events, "commit", input_id),
        serde_json::json!({ "numberValue": 24.0 })
    );

    // Pressing a stepper arms the core's own bounded repeat and steps exactly once.
    cx.simulate_mouse_down(
        window,
        field.increment_id(),
        quickgui::MouseDownEvent {
            button: quickgui::MouseButton::Left,
            position: quickgui::Point::new(1.0, 1.0),
            modifiers: quickgui::Modifiers::empty(),
            click_count: 1,
            first_mouse: false,
        },
    )
    .unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        cx.read(view, |view| view
            .components
            .number_fields
            .values()
            .next()
            .map(|field| field.state.value()))
            .unwrap(),
        Some(Some(26.0))
    );
    cx.simulate_mouse_up(
        window,
        field.increment_id(),
        quickgui::MouseUpEvent {
            button: quickgui::MouseButton::Left,
            position: quickgui::Point::new(1.0, 1.0),
            modifiers: quickgui::Modifiers::empty(),
            click_count: 1,
        },
    )
    .unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        cx.read(view, |view| view
            .components
            .number_fields
            .values()
            .next()
            .and_then(|field| field.state.repeat_deadline()))
            .unwrap(),
        None
    );
}

#[test]
fn declared_toasts_push_and_dismiss_through_the_core_queue() {
    let viewport_id = 660;
    let toast_id = 661;
    let close_id = 662;
    let mut tree = NativeTree::default();
    let viewport = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "toast-viewport",
        &[
            (property::SCOPE, "toasts"),
            (
                property::TOASTS,
                r#"[{"id":"saved","title":"Saved","kind":"success"},{"id":"","title":"Dropped"}]"#,
            ),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    insert_component_node(&mut tree, viewport_id, ROOT_NODE, viewport);
    let toast = component_part_node(
        NodeTag::View,
        viewport_id,
        "toast",
        &[(property::SCOPE, "toasts"), (property::PART_VALUE, "saved")],
        &[],
    );
    insert_component_node(&mut tree, toast_id, viewport_id, toast);
    let close = component_part_node(
        NodeTag::Button,
        toast_id,
        "toast-close",
        &[(property::SCOPE, "toasts"), (property::PART_VALUE, "saved")],
        &[],
    );
    insert_component_node(&mut tree, close_id, toast_id, close);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    cx.run_until_idle().unwrap();

    // An identifier-less declared toast is dropped instead of queued.
    assert_eq!(
        cx.read(view, |view| view
            .components
            .toasts
            .values()
            .next()
            .map(|state| state.manager.len()))
            .unwrap(),
        Some(1)
    );

    let close_element = cx
        .read(view, |view| {
            let state = view.components.toasts.values().next().unwrap();
            let entry = state.entry("saved").unwrap();
            ToastViewport::new(ElementId::named("toasts"))
                .toast(entry)
                .close_id()
        })
        .unwrap();
    cx.click(window, close_element).unwrap();
    cx.run_until_idle().unwrap();
    let change = merged_component_change(&events, viewport_id);
    assert_eq!(change.get("dismissed"), Some(&serde_json::json!(["saved"])));
    assert_eq!(
        cx.read(view, |view| view
            .components
            .toasts
            .values()
            .next()
            .map(|state| state.manager.len()))
            .unwrap(),
        Some(0)
    );
}

#[test]
fn declared_date_and_time_segments_step_through_the_core() {
    let date_id = 680;
    let year_id = 681;
    let time_id = 690;
    let hour_id = 691;
    let mut tree = NativeTree::default();
    let date = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "date-field",
        &[
            (property::SCOPE, "due"),
            (property::CIVIL_VALUE, "2026-09-03"),
            (property::CIVIL_MINIMUM, "2000-01-01"),
            (property::SEGMENT_ORDER, "mdy"),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    insert_component_node(&mut tree, date_id, ROOT_NODE, date);
    let year = component_part_node(
        NodeTag::View,
        date_id,
        "date-field-segment",
        &[(property::SCOPE, "due"), (property::SEGMENT, "year")],
        &[],
    );
    insert_component_node(&mut tree, year_id, date_id, year);
    let time = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "time-field",
        &[
            (property::SCOPE, "alarm"),
            (property::CIVIL_VALUE, "07:30:00"),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    insert_component_node(&mut tree, time_id, ROOT_NODE, time);
    let hour = component_part_node(
        NodeTag::View,
        time_id,
        "time-field-segment",
        &[(property::SCOPE, "alarm"), (property::SEGMENT, "hour")],
        &[],
    );
    insert_component_node(&mut tree, hour_id, time_id, hour);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();

    assert_eq!(
        cx.read(view, |view| view
            .components
            .date_fields
            .values()
            .next()
            .map(|field| field.state.segment_order()))
            .unwrap(),
        Some(quickgui::DateFieldOrder::MonthDayYear)
    );

    cx.focus(
        window,
        DateField::new(ElementId::named("due")).segment_id(quickgui::DateSegment::Year),
    )
    .unwrap();
    cx.simulate_keystrokes(window, "up").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        merged_component_change(&events, date_id).get("value"),
        Some(&serde_json::json!("2027-09-03"))
    );

    cx.focus(
        window,
        TimeField::new(ElementId::named("alarm")).segment_id(quickgui::TimeSegment::Hour),
    )
    .unwrap();
    cx.simulate_keystrokes(window, "down").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        merged_component_change(&events, time_id).get("value"),
        Some(&serde_json::json!("06:30:00"))
    );
}

#[test]
fn declared_calendar_and_menubar_move_focus_through_the_core() {
    let calendar_id = 700;
    let day_id = 701;
    let menubar_id = 710;
    let menu_ids = [711_u32, 712];
    let mut tree = NativeTree::default();
    let calendar = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "calendar",
        &[
            (property::SCOPE, "month"),
            (property::CIVIL_VALUE, "2026-09-03"),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    insert_component_node(&mut tree, calendar_id, ROOT_NODE, calendar);
    let day = component_part_node(
        NodeTag::View,
        calendar_id,
        "calendar-day",
        &[
            (property::SCOPE, "month"),
            (property::CIVIL_VALUE, "2026-09-03"),
        ],
        &[],
    );
    insert_component_node(&mut tree, day_id, calendar_id, day);

    let mut menubar = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "menubar",
        &[(property::SCOPE, "bar")],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    menubar.set_property(property::MENU_COUNT, Some(PropertyValue::Number(2.0)));
    insert_component_node(&mut tree, menubar_id, ROOT_NODE, menubar);
    for (index, id) in menu_ids.iter().enumerate() {
        let mut item = component_part_node(
            NodeTag::Button,
            menubar_id,
            "menubar-item",
            &[(property::SCOPE, "bar")],
            &[],
        );
        item.set_property(
            property::ITEM_INDEX,
            Some(PropertyValue::Number(index as f32)),
        );
        insert_component_node(&mut tree, *id, menubar_id, item);
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();

    let calendar = Calendar::new(ElementId::named("month"));
    let day = parse_civil_date("2026-09-03").unwrap();
    cx.focus(window, calendar.day_id(day)).unwrap();
    cx.simulate_keystrokes(window, "right").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        merged_component_change(&events, calendar_id).get("focused"),
        Some(&serde_json::json!("2026-09-04"))
    );

    let bar = Menubar::new(ElementId::named("bar"));
    cx.focus(window, bar.item_id(0)).unwrap();
    cx.simulate_keystrokes(window, "right").unwrap();
    cx.run_until_idle().unwrap();
    let change = merged_component_change(&events, menubar_id);
    assert_eq!(change.get("focusedIndex"), Some(&serde_json::json!(1)));

    cx.simulate_keystrokes(window, "down").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        merged_component_change(&events, menubar_id).get("openIndex"),
        Some(&serde_json::json!(1))
    );
}

#[test]
fn clicking_a_scoped_calendar_day_reports_the_selected_date() {
    let mut tree = NativeTree::default();
    let root = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "calendar",
        &[
            (property::SCOPE, "calendar-click"),
            (property::CIVIL_VALUE, "2026-09-03"),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    insert_component_node(&mut tree, 700, ROOT_NODE, root);
    let mut day = component_part_node(
        NodeTag::View,
        700,
        "calendar-day",
        &[
            (property::SCOPE, "calendar-click"),
            (property::CIVIL_VALUE, "2026-09-04"),
        ],
        &[],
    );
    day.set_property(property::WIDTH, Some(PropertyValue::Number(40.0)));
    day.set_property(property::HEIGHT, Some(PropertyValue::Number(40.0)));
    insert_component_node(&mut tree, 701, 700, day);
    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let calendar = Calendar::new(ElementId::named("calendar-click"));
    cx.click(
        view.window_handle(),
        calendar.day_id(parse_civil_date("2026-09-04").unwrap()),
    )
    .unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        merged_component_change(&events, 700).get("value"),
        Some(&serde_json::json!("2026-09-04"))
    );
}

// -------------------------------------------------------------------------------------------
// Extended text, box, and layout styling
//
// Every declaration below is parsed into the Rust core's own bounded value types. A malformed
// declaration must produce `None` and leave the element untouched rather than panicking.
// -------------------------------------------------------------------------------------------

#[test]
fn declared_css_colors_parse_the_same_grammar_the_renderer_packs() {
    assert_eq!(parse_css_color("#abc"), Some(Color::rgb8(0xaa, 0xbb, 0xcc)));
    assert_eq!(
        parse_css_color("#11223380"),
        Some(Color::rgba8(0x11, 0x22, 0x33, 0x80))
    );
    assert_eq!(
        parse_css_color("rgb(12, 34, 56)"),
        Some(Color::rgb8(12, 34, 56))
    );
    assert_eq!(
        parse_css_color("rgba(255, 0, 0, 0.5)"),
        Some(Color::rgba8(255, 0, 0, 128))
    );
    assert_eq!(parse_css_color("transparent"), Some(Color::TRANSPARENT));
    assert_eq!(parse_css_color("black"), Some(Color::rgb8(0, 0, 0)));

    assert_eq!(parse_css_color("chartreuse"), None);
    assert_eq!(parse_css_color("#12345"), None);
    assert_eq!(parse_css_color("rgb(1, 2)"), None);
    assert_eq!(parse_css_color(""), None);
    assert_eq!(parse_css_color(&"#".repeat(4096)), None);
}

#[test]
fn declared_linear_gradients_adopt_css_angles_directions_and_stop_positions() {
    let gradient =
        parse_gradient("linear-gradient(to bottom right, #ff0000, #00ff00 40%, #0000ff)")
            .expect("a CSS linear gradient");
    match gradient.kind() {
        quickgui::GradientKind::Linear { angle } => assert_eq!(angle.degrees(), 135.0),
        other => panic!("expected a linear gradient, got {other:?}"),
    }
    let stops = gradient.stops().as_slice();
    assert_eq!(stops.len(), 3);
    assert_eq!(stops[0].color, Color::rgb8(255, 0, 0));
    assert_eq!(stops[1].position, 0.4);
    assert_eq!(stops[2].color, Color::rgb8(0, 0, 255));

    let angled = parse_gradient("linear-gradient(45deg, black, white)").expect("an angle");
    match angled.kind() {
        quickgui::GradientKind::Linear { angle } => assert_eq!(angle.degrees(), 45.0),
        other => panic!("expected a linear gradient, got {other:?}"),
    }
    // Without an explicit angle CSS falls to `to bottom`, and unpositioned stops space evenly.
    let defaulted = parse_gradient("linear-gradient(black, white)").expect("a default angle");
    match defaulted.kind() {
        quickgui::GradientKind::Linear { angle } => assert_eq!(angle.degrees(), 180.0),
        other => panic!("expected a linear gradient, got {other:?}"),
    }
    assert_eq!(defaulted.stops().as_slice()[1].position, 1.0);

    let interpolated =
        parse_gradient("linear-gradient(in oklab, black, white)").expect("an interpolation space");
    assert_eq!(
        interpolated.interpolation(),
        quickgui::GradientColorSpace::Oklab
    );
}

#[test]
fn declared_radial_and_conic_gradients_reach_the_core_shape_extent_and_center() {
    let radial =
        parse_gradient("radial-gradient(circle closest-side at 30% 70%, #ffffff, rgba(0,0,0,0))")
            .expect("a CSS radial gradient");
    match radial.kind() {
        quickgui::GradientKind::Radial {
            shape,
            extent,
            center,
        } => {
            assert_eq!(shape, quickgui::RadialGradientShape::Circle);
            assert_eq!(extent, quickgui::RadialGradientExtent::ClosestSide);
            assert_eq!((center.x, center.y), (0.3, 0.7));
        }
        other => panic!("expected a radial gradient, got {other:?}"),
    }

    let conic = parse_gradient("conic-gradient(from 90deg at left top, #ff0000, #0000ff)")
        .expect("a CSS conic gradient");
    match conic.kind() {
        quickgui::GradientKind::Conic { from_angle, center } => {
            assert_eq!(from_angle.degrees(), 90.0);
            assert_eq!((center.x, center.y), (0.0, 0.0));
        }
        other => panic!("expected a conic gradient, got {other:?}"),
    }
    // `red` and `blue` are outside the bounded color grammar, so the whole declaration is refused.
    assert!(parse_gradient("conic-gradient(from 90deg, red, blue)").is_none());
}

#[test]
fn declared_gradient_objects_and_malformed_declarations_stay_bounded() {
    let gradient = parse_gradient(
        r##"{"type":"radial","shape":"circle","extent":"farthest-side","center":{"x":0.25,"y":0.5},"interpolation":"srgb","stops":[{"color":"#000000","position":0},{"color":"#ffffff","position":1}]}"##,
    )
    .expect("the declared object form");
    assert_eq!(gradient.interpolation(), quickgui::GradientColorSpace::Srgb);
    assert_eq!(gradient.stops().len(), 2);

    // The core retains at most eight stops; a longer declaration keeps the first eight.
    let many = (0..12).map(|_| "\"#102030\"").collect::<Vec<_>>().join(",");
    let bounded = parse_gradient(&format!(r#"{{"type":"linear","stops":[{many}]}}"#))
        .expect("a bounded declaration");
    assert_eq!(bounded.stops().len(), quickgui::MAX_GRADIENT_STOPS);

    assert!(parse_gradient("linear-gradient(").is_none());
    assert!(parse_gradient("linear-gradient()").is_none());
    assert!(parse_gradient("repeating-linear-gradient(#000000, #ffffff)").is_none());
    assert!(parse_gradient(r##"{"type":"spiral","stops":["#000000"]}"##).is_none());
    assert!(parse_gradient(&"linear-gradient(#000000, #ffffff) ".repeat(400)).is_none());
}

#[test]
fn declared_filter_chains_map_onto_the_core_filter_variants() {
    let filters = parse_filters(
        "brightness(1.2) contrast(80%) saturate(2) grayscale(0.5) invert(1) sepia(.25) hue-rotate(90deg) opacity(0.5)",
    )
    .expect("a CSS filter list");
    assert_eq!(
        filters,
        vec![
            quickgui::Filter::Brightness(1.2),
            quickgui::Filter::Contrast(0.8),
            quickgui::Filter::Saturate(2.0),
            quickgui::Filter::Grayscale(0.5),
            quickgui::Filter::Invert(1.0),
            quickgui::Filter::Sepia(0.25),
            quickgui::Filter::HueRotate(90.0),
            quickgui::Filter::Opacity(0.5),
        ]
    );

    let blurred = parse_filters("blur(4px) drop-shadow(0 2px 6px #00000080)")
        .expect("subtree filters promote a compositing group");
    assert_eq!(blurred[0], quickgui::Filter::Blur(4.0));
    match blurred[1] {
        quickgui::Filter::DropShadow(shadow) => {
            assert_eq!(shadow.offset, quickgui::Vector::new(0.0, 2.0));
            assert_eq!(shadow.color, Color::rgba8(0, 0, 0, 0x80));
        }
        other => panic!("expected a drop shadow, got {other:?}"),
    }

    // A chain longer than the core retains stops at the bound rather than reaching the core.
    let long = parse_filters(&"invert(1) ".repeat(20)).expect("a bounded chain");
    assert_eq!(long.len(), quickgui::MAX_FILTERS_PER_ELEMENT);

    assert!(parse_filters("none").is_none());
    assert!(parse_filters("wobble(2)").is_none());
    assert!(parse_filters("blur(4px) wobble(2)").is_none());
}

#[test]
fn declared_transforms_compose_in_css_order_and_accept_the_matrix_form() {
    // CSS applies the rightmost function to the point first: scale, then translate.
    let transform = parse_transform("translate(10px, 0) scale(2)").expect("a CSS transform list");
    assert_eq!(
        transform.apply(quickgui::Point::new(1.0, 0.0)),
        quickgui::Point::new(12.0, 0.0)
    );

    let rotated = parse_transform("rotate(0.5turn)").expect("a turn angle");
    assert!((rotated.a + 1.0).abs() < 1.0e-5);

    let matrix = parse_transform(r#"{"a":1,"b":0,"c":0,"d":1,"tx":4,"ty":8}"#)
        .expect("the declared matrix form");
    assert_eq!(matrix, quickgui::Transform2D::translate(4.0, 8.0));

    assert!(parse_transform("none").is_none());
    assert!(parse_transform("perspective(200px)").is_none());
    assert!(parse_transform("translate()").is_none());
    assert_eq!(parse_transform_origin("left top"), Some((0.0, 0.0)));
    assert_eq!(parse_transform_origin("25% 75%"), Some((0.25, 0.75)));
    assert_eq!(parse_transform_origin("center"), Some((0.5, 0.5)));
    assert_eq!(parse_transform_origin("top left center"), None);
}

#[test]
fn declared_text_shadows_fall_back_to_the_elements_own_text_color() {
    let current = Color::rgb8(0x11, 0x22, 0x33);
    let shadow = parse_text_shadow("0 2px 4px #00000080", current).expect("a CSS text shadow");
    assert_eq!(
        (shadow.offset_x, shadow.offset_y, shadow.blur),
        (0.0, 2.0, 4.0)
    );
    assert_eq!(shadow.color, Color::rgba8(0, 0, 0, 0x80));

    let inherited = parse_text_shadow("1px 1px", current).expect("an offset-only shadow");
    assert_eq!(inherited.color, current);
    assert_eq!(inherited.blur, 0.0);

    let declared = parse_text_shadow(
        r##"{"offsetX":2,"offsetY":3,"blur":5,"color":"#ffffff"}"##,
        current,
    )
    .expect("the declared object form");
    assert_eq!(declared.color, Color::rgb8(255, 255, 255));

    assert!(parse_text_shadow("none", current).is_none());
    assert!(parse_text_shadow("4px", current).is_none());
    assert!(parse_text_shadow("1 2 3 4 #000000", current).is_none());
}

#[test]
fn declared_corner_radii_outlines_and_backgrounds_adopt_the_css_shorthands() {
    assert_eq!(
        parse_corner_radii("8px 12px 0 4px"),
        Some(quickgui::Corners::new(8.0, 12.0, 0.0, 4.0))
    );
    assert_eq!(
        parse_corner_radii("8px 12px"),
        Some(quickgui::Corners::new(8.0, 12.0, 8.0, 12.0))
    );
    assert_eq!(
        parse_corner_radii("8 12 4"),
        Some(quickgui::Corners::new(8.0, 12.0, 4.0, 12.0))
    );
    assert_eq!(parse_corner_radii("1 2 3 4 5"), None);

    let outline = parse_outline("2px dashed #11223344", 3.0).expect("an outline shorthand");
    assert_eq!(outline.width, 2.0);
    assert_eq!(outline.offset, 3.0);
    assert_eq!(outline.style, quickgui::BorderStyle::Dashed);
    assert_eq!(outline.color, Color::rgba8(0x11, 0x22, 0x33, 0x44));
    assert!(parse_outline("none", 0.0).is_none());
    assert!(parse_outline("solid #000000", 0.0).is_none());

    assert_eq!(
        parse_background_size("cover"),
        Some(quickgui::BackgroundSize::Cover)
    );
    assert_eq!(
        parse_background_size("64px 32px"),
        Some(quickgui::BackgroundSize::Fixed(64.0, 32.0))
    );
    assert_eq!(parse_background_size("stretch"), None);
    assert_eq!(
        parse_background_repeat("repeat-x"),
        Some(quickgui::BackgroundRepeat::RepeatX)
    );
    assert_eq!(parse_background_repeat("space"), None);
    let position = parse_background_position("right bottom").expect("a background position");
    assert_eq!((position.x, position.y), (1.0, 1.0));
    assert_eq!(parse_background_position("nowhere"), None);
    assert_eq!(
        parse_blend_mode("hard-light"),
        Some(quickgui::BlendMode::HardLight)
    );
    assert_eq!(parse_blend_mode("luminosity"), None);
    assert_eq!(
        parse_border_style("dotted"),
        Some(quickgui::BorderStyle::Dotted)
    );
    assert_eq!(parse_border_style("groove"), None);
}

#[test]
fn nested_state_styles_collect_everything_the_core_state_style_can_swap() {
    let mut node = NativeNode::new(NodeTag::View);
    node.set_property(property::OUTLINE_OFFSET, Some(PropertyValue::Number(2.0)));
    node.set_property(property::COLOR, Some(PropertyValue::Color(0xff10_2030)));
    node.set_property(
        property::HOVER_STYLE,
        Some(PropertyValue::String(Arc::from(concat!(
            r#"{"backgroundColor":4281545523,"background":"linear-gradient(90deg, #000000, #ffffff)","#,
            r#""color":4294967295,"borderColor":4278190335,"borderWidth":2,"borderRadius":8,"#,
            r#""outline":"3px dashed #ffffff","#,
            r#""boxShadow":[{"offsetX":0,"offsetY":4,"blurRadius":12,"spreadRadius":-2,"color":null,"inset":false}],"#,
            r#""opacity":0.5,"cursor":"grab","transform":"scale(1.05)","transformOrigin":"left top","#,
            r#""padding":12}"#
        )))),
    );

    let style = native_state_style(&node, &HOVER_STYLE_CODES).expect("a declared hover style");
    let expected = ElementStateStyle::default()
        .bg(unpack_color(4281545523))
        .bg_gradient(parse_gradient("linear-gradient(90deg, #000000, #ffffff)").unwrap())
        .text_color(unpack_color(4294967295))
        .border_color(unpack_color(4278190335))
        .border_width(2.0)
        .rounded(8.0)
        .outline_offset(3.0, Color::WHITE, 2.0)
        .outline_dashed()
        .shadows([BoxShadow::new(0.0, 4.0, unpack_color(0xff10_2030))
            .blur_radius(12.0)
            .spread_radius(-2.0)])
        .opacity(0.5)
        .cursor(CursorStyle::OpenHand)
        .transform(Transform2D::scale(1.05, 1.05))
        .transform_origin(0.0, 0.0);
    assert_eq!(style, expected);

    // A state that declares nothing must not register a core state style at all.
    assert!(native_state_style(&NativeNode::new(NodeTag::View), &FOCUS_STYLE_CODES).is_none());
}

#[test]
fn groups_travel_as_a_bare_marker_or_a_bounded_name() {
    let mut node = NativeNode::new(NodeTag::View);
    assert_eq!(native_group(&node), None);
    node.set_property(property::HOVER_GROUP, Some(PropertyValue::Bool(true)));
    assert_eq!(native_group(&node), Some(None));
    node.set_property(
        property::HOVER_GROUP,
        Some(PropertyValue::String(Arc::from("sidebar"))),
    );
    assert_eq!(native_group(&node), Some(Some(Arc::<str>::from("sidebar"))));
    node.set_property(
        property::HOVER_GROUP,
        Some(PropertyValue::String(Arc::from(
            "x".repeat(MAX_HOVER_GROUP_NAME_BYTES + 1),
        ))),
    );
    assert_eq!(native_group(&node), None);
    node.set_property(property::HOVER_GROUP, Some(PropertyValue::Bool(false)));
    assert_eq!(native_group(&node), None);
}

#[test]
fn group_states_collect_one_entry_per_group_in_declaration_order() {
    let mut node = NativeNode::new(NodeTag::View);
    assert!(native_group_styles(&node, property::GROUP_HOVER_STYLE).is_empty());

    // One entry is a plain object; it may name the group it follows.
    node.set_property(
        property::GROUP_HOVER_STYLE,
        Some(PropertyValue::String(Arc::from(
            r#"{"group":"sidebar","opacity":1}"#,
        ))),
    );
    assert_eq!(
        native_group_styles(&node, property::GROUP_HOVER_STYLE),
        vec![(
            Some(Arc::<str>::from("sidebar")),
            ElementStateStyle::default().opacity(1.0)
        )]
    );

    // Several entries are a list; an entry naming a group the core would refuse and an entry
    // declaring nothing are dropped, and the rest keep their order.
    node.set_property(
        property::GROUP_ACTIVE_STYLE,
        Some(PropertyValue::String(Arc::from(format!(
            r#"[{{"opacity":0.8}},{{"group":"list","borderRadius":4}},{{"group":"{}","opacity":0.5}},{{"group":"x"}}]"#,
            "x".repeat(MAX_HOVER_GROUP_NAME_BYTES + 1)
        )))),
    );
    assert_eq!(
        native_group_styles(&node, property::GROUP_ACTIVE_STYLE),
        vec![
            (None, ElementStateStyle::default().opacity(0.8)),
            (
                Some(Arc::<str>::from("list")),
                ElementStateStyle::default().rounded(4.0)
            ),
        ]
    );

    // Entries past the core's bound are dropped in source order rather than refused.
    let entries = (0..=MAX_GROUP_STYLES_PER_ELEMENT)
        .map(|index| format!(r#"{{"borderRadius":{index}}}"#))
        .collect::<Vec<_>>()
        .join(",");
    node.set_property(
        property::GROUP_HOVER_STYLE,
        Some(PropertyValue::String(Arc::from(format!("[{entries}]")))),
    );
    let styles = native_group_styles(&node, property::GROUP_HOVER_STYLE);
    assert_eq!(styles.len(), MAX_GROUP_STYLES_PER_ELEMENT);
    assert_eq!(styles[1].1, ElementStateStyle::default().rounded(1.0));

    // `focusWithin` is an ordinary single state.
    node.set_property(
        property::FOCUS_WITHIN_STYLE,
        Some(PropertyValue::String(Arc::from(
            r#"{"outline":"2px solid #ffffff"}"#,
        ))),
    );
    assert_eq!(
        native_state_style(&node, &FOCUS_WITHIN_STYLE_CODES),
        Some(ElementStateStyle::default().outline_offset(2.0, Color::WHITE, 0.0))
    );

    // `selected` is an ordinary single state as well, following the element's selected flag.
    node.set_property(
        property::SELECTED_STYLE,
        Some(PropertyValue::String(Arc::from(
            r#"{"backgroundColor":4278190335,"color":4294967295}"#,
        ))),
    );
    assert_eq!(
        native_state_style(&node, &SELECTED_STYLE_CODES),
        Some(
            ElementStateStyle::default()
                .bg(unpack_color(4278190335))
                .text_color(unpack_color(4294967295))
        )
    );
}

#[test]
fn nested_state_styles_honour_none_keywords_bounds_and_the_flat_legacy_names() {
    let mut node = NativeNode::new(NodeTag::View);
    node.set_property(
        property::DRAG_OVER_STYLE,
        Some(PropertyValue::String(Arc::from(
            r#"{"outline":"none","boxShadow":[]}"#,
        ))),
    );
    assert_eq!(
        native_state_style(&node, &DRAG_OVER_STYLE_CODES),
        Some(ElementStateStyle::default().outline_none().shadow_none())
    );

    // Malformed JSON and an oversize declaration register nothing rather than panicking.
    node.set_property(
        property::ACTIVE_STYLE,
        Some(PropertyValue::String(Arc::from("{not json"))),
    );
    assert!(native_state_style(&node, &ACTIVE_STYLE_CODES).is_none());
    node.set_property(
        property::ACTIVE_STYLE,
        Some(PropertyValue::String(Arc::from(format!(
            r#"{{"opacity":0.5,"cursor":"{}"}}"#,
            "x".repeat(MAX_STATE_STYLE_JSON_BYTES)
        )))),
    );
    assert!(native_state_style(&node, &ACTIVE_STYLE_CODES).is_none());

    // Too many shadows drop the list, never the rest of the state.
    let shadows = (0..=MAX_BOX_SHADOWS_PER_ELEMENT)
        .map(|index| {
            format!(
                r#"{{"offsetX":0,"offsetY":{index},"blurRadius":0,"spreadRadius":0,"color":null,"inset":false}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    node.set_property(
        property::DRAGGING_STYLE,
        Some(PropertyValue::String(Arc::from(format!(
            r#"{{"boxShadow":[{shadows}],"opacity":0.5}}"#
        )))),
    );
    assert_eq!(
        native_state_style(&node, &DRAGGING_STYLE_CODES),
        Some(ElementStateStyle::default().opacity(0.5))
    );

    // The flat legacy hover declarations overlay the nested one, so a partial migration paints
    // exactly what both declared.
    node.set_property(
        property::HOVER_STYLE,
        Some(PropertyValue::String(Arc::from(
            r#"{"backgroundColor":1,"opacity":0.25}"#,
        ))),
    );
    node.set_property(
        property::HOVER_BACKGROUND_COLOR,
        Some(PropertyValue::Color(0xff33_2211)),
    );
    node.set_property(
        property::HOVER_TRANSFORM,
        Some(PropertyValue::String(Arc::from("scale(1.05)"))),
    );
    assert_eq!(
        native_state_style(&node, &HOVER_STYLE_CODES),
        Some(
            ElementStateStyle::default()
                .bg(unpack_color(0xff33_2211))
                .opacity(0.25)
                .transform(Transform2D::scale(1.05, 1.05))
        )
    );
}

#[test]
fn malformed_style_declarations_leave_the_element_untouched_instead_of_panicking() {
    let mut node = NativeNode::new(NodeTag::View);
    for code in [
        property::TEXT_SHADOW,
        property::TEXT_TRANSFORM,
        property::WORD_BREAK,
        property::OVERFLOW_WRAP,
        property::HYPHENS,
        property::TEXT_DIRECTION,
        property::TEXT_DECORATION_LINE,
        property::TEXT_DECORATION_STYLE,
        property::DIRECTION,
        property::BACKGROUND_GRADIENT,
        property::BORDER_STYLE,
        property::BORDER_RADIUS,
        property::OUTLINE_STYLE,
        property::FILTER,
        property::BACKDROP_FILTER,
        property::TRANSFORM,
        property::TRANSFORM_ORIGIN,
        property::MIX_BLEND_MODE,
        property::SCROLL_SNAP_TYPE,
        property::SCROLL_SNAP_ALIGN,
        property::SCROLL_SNAP_STOP,
        property::BACKGROUND_SIZE,
        property::BACKGROUND_REPEAT,
        property::BACKGROUND_POSITION,
    ] {
        node.set_property(
            code,
            Some(PropertyValue::String(Arc::from("\u{1f600} not a value"))),
        );
    }
    node.set_property(
        property::LETTER_SPACING,
        Some(PropertyValue::Number(f32::MAX)),
    );
    node.set_property(
        property::TEXT_DECORATION_THICKNESS,
        Some(PropertyValue::Number(-40.0)),
    );

    // Building the element must not panic; every unusable declaration is simply not applied.
    drop(apply_properties(div(), &node));
}

#[test]
fn declared_scroll_snapping_sticky_insets_and_logical_spacing_reach_the_core_builders() {
    let mut node = NativeNode::new(NodeTag::View);
    node.set_property(
        property::SCROLL_SNAP_TYPE,
        Some(PropertyValue::String(Arc::from("x mandatory"))),
    );
    node.set_property(
        property::SCROLL_SNAP_ALIGN,
        Some(PropertyValue::String(Arc::from("center"))),
    );
    node.set_property(
        property::SCROLL_SNAP_STOP,
        Some(PropertyValue::String(Arc::from("always"))),
    );
    node.set_property(
        property::DIRECTION,
        Some(PropertyValue::String(Arc::from("rtl"))),
    );
    node.set_property(property::PADDING_START, Some(PropertyValue::Number(12.0)));
    node.set_property(property::MARGIN_END, Some(PropertyValue::Number(8.0)));
    node.set_property(
        property::BORDER_START_WIDTH,
        Some(PropertyValue::Number(3.0)),
    );

    // The core owns snapping, mirroring, and logical edge resolution; the binding only declares
    // them, so the assertion here is that a complete declaration builds exactly one element.
    drop(apply_layout_styles(div(), &node));

    let mut sticky = NativeNode::new(NodeTag::View);
    sticky.set_property(
        property::POSITION,
        Some(PropertyValue::String(Arc::from("sticky"))),
    );
    sticky.set_property(property::TOP, Some(PropertyValue::Number(0.0)));
    sticky.set_property(
        property::OVERFLOW_X,
        Some(PropertyValue::String(Arc::from("scroll"))),
    );
    drop(apply_properties(div(), &sticky));
}

// ---------------------------------------------------------------------------
// Base UI parity components
//
// Separators, avatars, checkbox groups, preview cards, scroll areas, OTP fields, drawers, and
// navigation menus follow the same contract as every other declared component: the declaration is
// the source of truth, the core owns behavior and mount policy, and everything the core decides
// leaves as one asynchronous `componentchange` event.
// ---------------------------------------------------------------------------

fn base_ui_application() -> quickgui::Application {
    collection_application()
        .bind_keys(quickgui::otp_field_key_bindings())
        .bind_keys(quickgui::navigation_menu_key_bindings())
}

fn mounted_base_ui_view(
    tree: NativeTree,
    events: EventQueue,
) -> (
    quickgui::TestAppContext,
    quickgui::TestWindowHandle<NativeView>,
) {
    let view = component_part_view(91, tree, events);
    quickgui::TestAppContext::from_application(
        base_ui_application(),
        quickgui::WindowOptions::default(),
        view,
    )
    .expect("the hosted Base UI view mounts")
}

/// Declare one avatar and its image and fallback parts under one shared scope.
fn declare_avatar(tree: &mut NativeTree, base: u32, scope: &str, source: &str) {
    insert_component_node(
        tree,
        base,
        ROOT_NODE,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "avatar",
            &[
                (property::SCOPE, scope),
                (property::ACCESSIBILITY_LABEL, "Ada Lovelace"),
            ],
            &[(property::COMPONENT_CHANGE_LISTENER, true)],
        ),
    );
    insert_component_node(
        tree,
        base + 1,
        base,
        component_part_node(
            NodeTag::Image,
            base,
            "avatar-image",
            &[(property::SCOPE, scope), (property::VALUE, source)],
            &[],
        ),
    );
    insert_component_node(
        tree,
        base + 2,
        base,
        component_part_node(
            NodeTag::View,
            base,
            "avatar-fallback",
            &[(property::SCOPE, scope)],
            &[],
        ),
    );
}

#[test]
fn declared_separators_and_avatars_adopt_core_identity_and_report_the_load_outcome() {
    let separator_id = 700;
    let loaded_base = 710;
    let broken_base = 720;
    let mut tree = NativeTree::default();
    insert_component_node(
        &mut tree,
        separator_id,
        ROOT_NODE,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "separator",
            &[(property::ORIENTATION, "vertical")],
            &[],
        ),
    );
    declare_avatar(&mut tree, loaded_base, "member", RED_PIXEL_PNG);
    declare_avatar(
        &mut tree,
        broken_base,
        "stranger",
        "data:image/png;base64,not-base64!!",
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();

    // A separator is stateless, so it keeps the ordinary node identity and only gains semantics.
    assert!(
        cx.contains_element(window, ElementId::new(separator_id as u64))
            .unwrap()
    );

    let loaded = quickgui::Avatar::new("member", "Ada Lovelace");
    assert!(cx.contains_element(window, loaded.root_id()).unwrap());
    assert!(cx.contains_element(window, loaded.image_id()).unwrap());
    // A loaded avatar mounts no fallback at all, so the name is announced exactly once.
    assert!(!cx.contains_element(window, loaded.fallback_id()).unwrap());

    let broken = quickgui::Avatar::new("stranger", "Ada Lovelace");
    assert!(!cx.contains_element(window, broken.image_id()).unwrap());
    assert!(cx.contains_element(window, broken.fallback_id()).unwrap());

    assert_eq!(
        component_change(&events, loaded_base),
        serde_json::json!({ "loadingStatus": "loaded" })
    );
    assert_eq!(
        component_change(&events, broken_base),
        serde_json::json!({ "loadingStatus": "error" })
    );
}

#[test]
fn a_declared_avatar_fallback_delay_holds_the_fallback_back_for_exactly_one_deadline() {
    let base = 730;
    let mut tree = NativeTree::default();
    declare_avatar(&mut tree, base, "member", RED_PIXEL_PNG);
    tree.nodes
        .get_mut(&base)
        .unwrap()
        .set_property(property::DELAY, Some(PropertyValue::Number(200.0)));

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let avatar = quickgui::Avatar::new("member", "Ada Lovelace");

    // While the declared delay is armed neither part is mounted, so a fast decode never flashes
    // initials on screen.
    assert!(!cx.contains_element(window, avatar.image_id()).unwrap());
    assert!(!cx.contains_element(window, avatar.fallback_id()).unwrap());
    assert_eq!(
        component_change(&events, base),
        serde_json::json!({ "loadingStatus": "loading" })
    );

    // The resolved outcome applies on the very next frame, long before the fallback deadline.
    cx.advance_time(std::time::Duration::from_millis(50))
        .unwrap();
    cx.run_until_idle().unwrap();
    assert!(cx.contains_element(window, avatar.image_id()).unwrap());
    assert_eq!(
        component_change(&events, base),
        serde_json::json!({ "loadingStatus": "loaded" })
    );
}

#[test]
fn declared_checkbox_groups_derive_the_parent_state_and_stay_independent() {
    let colors_id = 740;
    let sizes_id = 741;
    let mut tree = NativeTree::default();
    for (id, scope, items, values) in [
        (
            colors_id,
            "colors",
            r#"["red","green","blue"]"#,
            r#"["green"]"#,
        ),
        (sizes_id, "sizes", r#"["small","large"]"#, r#"[]"#),
    ] {
        insert_component_node(
            &mut tree,
            id,
            ROOT_NODE,
            component_part_node(
                NodeTag::View,
                ROOT_NODE,
                "checkbox-group",
                &[
                    (property::SCOPE, scope),
                    (property::ITEMS, items),
                    (property::VALUES, values),
                ],
                &[(property::COMPONENT_CHANGE_LISTENER, true)],
            ),
        );
    }
    for (index, value) in ["red", "green", "blue"].iter().enumerate() {
        insert_component_node(
            &mut tree,
            750 + index as u32,
            colors_id,
            component_part_node(
                NodeTag::Button,
                colors_id,
                "checkbox-group-item",
                &[(property::SCOPE, "colors"), (property::PART_VALUE, value)],
                &[],
            ),
        );
    }
    insert_component_node(
        &mut tree,
        760,
        colors_id,
        component_part_node(
            NodeTag::Button,
            colors_id,
            "checkbox-group-parent",
            &[(property::SCOPE, "colors")],
            &[],
        ),
    );
    insert_component_node(
        &mut tree,
        761,
        sizes_id,
        component_part_node(
            NodeTag::Button,
            sizes_id,
            "checkbox-group-item",
            &[(property::SCOPE, "sizes"), (property::PART_VALUE, "small")],
            &[],
        ),
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let colors_state = quickgui::CheckboxGroupState::new(["red", "green", "blue"]);
    let colors = quickgui::CheckboxGroup::new("colors", &colors_state);
    let sizes_state = quickgui::CheckboxGroupState::new(["small", "large"]);
    let sizes = quickgui::CheckboxGroup::new("sizes", &sizes_state);

    cx.click(window, colors.checkbox_id("red")).unwrap();
    // Checked values keep the declared order regardless of click order.
    assert_eq!(
        component_change(&events, colors_id),
        serde_json::json!({ "checkedValues": ["red", "green"] })
    );

    // The parent has no retained value of its own: a mixed parent completes the group.
    cx.click(window, colors.parent_id()).unwrap();
    assert_eq!(
        component_change(&events, colors_id),
        serde_json::json!({ "checkedValues": ["red", "green", "blue"] })
    );

    // The second declared instance is untouched by either click.
    assert_eq!(component_change(&events, sizes_id), serde_json::Value::Null);
    cx.click(window, sizes.checkbox_id("small")).unwrap();
    assert_eq!(
        component_change(&events, sizes_id),
        serde_json::json!({ "checkedValues": ["small"] })
    );
    assert_eq!(
        component_change(&events, colors_id),
        serde_json::Value::Null
    );
}

#[test]
fn a_declared_preview_card_opens_from_hover_and_holds_a_declared_delay_back() {
    let root_id = 770;
    let trigger_id = 771;
    let positioner_id = 772;
    let popup_id = 773;
    let slow_root_id = 774;
    let slow_trigger_id = 775;
    let slow_positioner_id = 776;
    let mut tree = NativeTree::default();
    for (root, trigger, positioner, scope, delay) in [
        (root_id, trigger_id, positioner_id, "profile", 0.0),
        (
            slow_root_id,
            slow_trigger_id,
            slow_positioner_id,
            "slow",
            600.0,
        ),
    ] {
        let mut node = component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "preview-card",
            &[(property::SCOPE, scope)],
            &[(property::COMPONENT_CHANGE_LISTENER, true)],
        );
        node.set_property(property::DELAY, Some(PropertyValue::Number(delay)));
        node.set_property(property::CLOSE_DELAY, Some(PropertyValue::Number(0.0)));
        insert_component_node(&mut tree, root, ROOT_NODE, node);
        insert_component_node(
            &mut tree,
            trigger,
            root,
            component_part_node(
                NodeTag::Button,
                root,
                "preview-card-trigger",
                &[(property::SCOPE, scope)],
                &[],
            ),
        );
        insert_component_node(
            &mut tree,
            positioner,
            root,
            component_part_node(
                NodeTag::View,
                root,
                "preview-card-positioner",
                &[(property::SCOPE, scope)],
                &[],
            ),
        );
    }
    insert_component_node(
        &mut tree,
        popup_id,
        positioner_id,
        component_part_node(
            NodeTag::View,
            positioner_id,
            "preview-card-popup",
            &[(property::SCOPE, "profile")],
            &[],
        ),
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let card = native_preview_card(ElementId::named("profile"), false);
    let slow = native_preview_card(ElementId::named("slow"), false);

    // A closed card mounts no positioner, popup, or arrow at all.
    assert!(!cx.contains_element(window, card.popup_id()).unwrap());
    let bounds = cx.element_bounds(window, card.trigger_id()).unwrap();
    let center = quickgui::Point::new(
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    );
    cx.visual(window).unwrap().move_pointer(center).unwrap();

    assert!(cx.contains_element(window, card.popup_id()).unwrap());
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "open": true })
    );

    // The declared 600 ms delay reaches the core as an armed deadline rather than an open card,
    // so resting on the second trigger changes nothing yet.
    let slow_bounds = cx.element_bounds(window, slow.trigger_id()).unwrap();
    cx.visual(window)
        .unwrap()
        .move_pointer(quickgui::Point::new(
            slow_bounds.x + slow_bounds.width / 2.0,
            slow_bounds.y + slow_bounds.height / 2.0,
        ))
        .unwrap();
    assert!(!cx.contains_element(window, slow.popup_id()).unwrap());
    assert_eq!(
        component_change(&events, slow_root_id),
        serde_json::Value::Null
    );

    // Leaving both the trigger and the popup closes the first card on its zero close delay.
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "open": false })
    );
    assert!(!cx.contains_element(window, card.popup_id()).unwrap());
}

#[test]
fn a_declared_scroll_area_owns_its_offsets_overflow_flags_and_scrollbar_mount_policy() {
    let root_id = 780;
    let viewport_id = 781;
    let content_id = 782;
    let vertical_id = 783;
    let thumb_id = 784;
    let horizontal_id = 785;
    let mut tree = NativeTree::default();
    insert_component_node(
        &mut tree,
        root_id,
        ROOT_NODE,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "scroll-area",
            &[
                (property::SCOPE, "log"),
                (property::VIEWPORT_SIZE, "[260,160]"),
                (property::CONTENT_SIZE, "[260,900]"),
            ],
            &[(property::COMPONENT_CHANGE_LISTENER, true)],
        ),
    );
    insert_component_node(
        &mut tree,
        viewport_id,
        root_id,
        component_part_node(
            NodeTag::View,
            root_id,
            "scroll-area-viewport",
            &[(property::SCOPE, "log")],
            &[],
        ),
    );
    insert_component_node(
        &mut tree,
        content_id,
        viewport_id,
        component_part_node(
            NodeTag::View,
            viewport_id,
            "scroll-area-content",
            &[(property::SCOPE, "log")],
            &[],
        ),
    );
    insert_component_node(
        &mut tree,
        vertical_id,
        root_id,
        component_part_node(
            NodeTag::View,
            root_id,
            "scroll-area-scrollbar",
            &[
                (property::SCOPE, "log"),
                (property::ORIENTATION, "vertical"),
            ],
            &[],
        ),
    );
    insert_component_node(
        &mut tree,
        thumb_id,
        vertical_id,
        component_part_node(
            NodeTag::View,
            vertical_id,
            "scroll-area-thumb",
            &[
                (property::SCOPE, "log"),
                (property::ORIENTATION, "vertical"),
            ],
            &[],
        ),
    );
    insert_component_node(
        &mut tree,
        horizontal_id,
        root_id,
        component_part_node(
            NodeTag::View,
            root_id,
            "scroll-area-scrollbar",
            &[
                (property::SCOPE, "log"),
                (property::ORIENTATION, "horizontal"),
            ],
            &[],
        ),
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let area = quickgui::ScrollArea::new("log");
    let vertical = quickgui::ScrollAreaOrientation::Vertical;
    let horizontal = quickgui::ScrollAreaOrientation::Horizontal;

    assert!(cx.contains_element(window, area.viewport_id()).unwrap());
    assert!(
        cx.contains_element(window, area.scrollbar_id(vertical))
            .unwrap()
    );
    assert!(
        cx.contains_element(window, area.thumb_id(vertical))
            .unwrap()
    );
    // The horizontal axis cannot scroll and `keepMounted` was not declared, so its scrollbar
    // contributes no element at all.
    assert!(
        !cx.contains_element(window, area.scrollbar_id(horizontal))
            .unwrap()
    );

    let overflow = component_change(&events, root_id);
    assert_eq!(overflow["hasOverflowY"], serde_json::json!(true));
    assert_eq!(overflow["hasOverflowX"], serde_json::json!(false));
    assert_eq!(overflow["overflowYStart"], serde_json::json!(false));
    assert_eq!(overflow["overflowYEnd"], serde_json::json!(true));

    cx.simulate_scroll_wheel(
        window,
        area.viewport_id(),
        quickgui::ScrollWheelEvent {
            position: quickgui::Point::new(10.0, 10.0),
            delta: quickgui::ScrollDelta::Pixels(quickgui::Vector::new(0.0, -40.0)),
            phase: quickgui::GesturePhase::Moved,
            modifiers: quickgui::Modifiers::empty(),
        },
    )
    .unwrap();
    cx.run_until_idle().unwrap();

    let scrolled = component_change(&events, root_id);
    assert_eq!(scrolled["offset"]["y"], serde_json::json!(40.0));
    assert_eq!(scrolled["scrolling"], serde_json::json!(true));
    assert_eq!(scrolled["overflowYStart"], serde_json::json!(true));
}

#[test]
fn a_declared_otp_field_fills_slots_advances_focus_and_reports_completion() {
    let root_id = 790;
    let mut tree = NativeTree::default();
    let mut root = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "otp-field",
        &[(property::SCOPE, "code")],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    root.set_property(property::LENGTH, Some(PropertyValue::Number(4.0)));
    insert_component_node(&mut tree, root_id, ROOT_NODE, root);
    for index in 0..4u32 {
        let mut slot = component_part_node(
            NodeTag::Input,
            root_id,
            "otp-field-input",
            &[(property::SCOPE, "code")],
            &[],
        );
        slot.set_property(
            property::ITEM_INDEX,
            Some(PropertyValue::Number(index as f32)),
        );
        insert_component_node(&mut tree, 791 + index, root_id, slot);
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let field = quickgui::OtpField::new("code");

    cx.focus(window, field.input_id(0)).unwrap();
    cx.simulate_input(window, "1").unwrap();
    // The core owns slot advancement, so the next accepted character lands in the next slot.
    assert_eq!(cx.focused(window).unwrap(), Some(field.input_id(1)));
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "value": "1" })
    );

    for character in ["2", "3"] {
        cx.simulate_input(window, character).unwrap();
    }
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "value": "123" })
    );

    cx.simulate_input(window, "4").unwrap();
    // Completion is an edge, so it travels with the value that completed the code.
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "value": "1234", "complete": "1234" })
    );

    // Backspace clears in place and then walks back, all inside the core.
    cx.simulate_keystrokes(window, "backspace").unwrap();
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "value": "123" })
    );
}

#[test]
fn a_declared_drawer_mounts_only_while_open_and_reports_its_snap_point() {
    let root_id = 800;
    let portal_id = 801;
    let backdrop_id = 802;
    let viewport_id = 803;
    let popup_id = 804;
    let swipe_id = 805;
    let title_id = 806;
    let mut tree = NativeTree::default();
    let mut root = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "drawer",
        &[
            (property::SCOPE, "filters"),
            (property::SWIPE_DIRECTION, "down"),
            (property::VALUES, "[0.45,1]"),
        ],
        &[
            (property::OPEN, true),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    root.set_property(property::ITEM_INDEX, Some(PropertyValue::Number(0.0)));
    insert_component_node(&mut tree, root_id, ROOT_NODE, root);
    for (id, parent, part) in [
        (portal_id, root_id, "drawer-portal"),
        (backdrop_id, portal_id, "drawer-backdrop"),
        (viewport_id, portal_id, "drawer-viewport"),
        (popup_id, viewport_id, "drawer-popup"),
        (swipe_id, popup_id, "drawer-swipe-area"),
        (title_id, popup_id, "drawer-title"),
    ] {
        insert_component_node(
            &mut tree,
            id,
            parent,
            component_part_node(
                NodeTag::View,
                parent,
                part,
                &[(property::SCOPE, "filters")],
                &[],
            ),
        );
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let drawer = quickgui::Drawer::new("filters", true);

    assert!(cx.contains_element(window, drawer.popup_id()).unwrap());
    assert!(cx.contains_element(window, drawer.swipe_area_id()).unwrap());
    assert!(cx.contains_element(window, drawer.backdrop_id()).unwrap());
    let opened = component_change(&events, root_id);
    assert_eq!(opened["open"], serde_json::json!(true));
    // The declared active snap point is the smaller of the two declared points.
    assert_eq!(opened["snapPoint"], serde_json::json!(0));
    assert_eq!(opened["swiping"], serde_json::json!(false));

    cx.simulate_keystrokes(window, "escape").unwrap();
    cx.run_until_idle().unwrap();
    let closed = component_change(&events, root_id);
    assert_eq!(closed["open"], serde_json::json!(false));
    // A closed drawer contributes no overlay, focus trap, backdrop, or swipe surface.
    assert!(!cx.contains_element(window, drawer.popup_id()).unwrap());
    assert!(!cx.contains_element(window, drawer.swipe_area_id()).unwrap());
}

#[test]
fn a_declared_navigation_menu_switches_panels_and_reports_the_activation_direction() {
    let root_id = 810;
    let list_id = 811;
    let mut tree = NativeTree::default();
    let items =
        r#"[{"value":"products"},{"value":"solutions"},{"value":"support","disabled":true}]"#;
    insert_component_node(
        &mut tree,
        root_id,
        ROOT_NODE,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "navigation-menu",
            &[(property::SCOPE, "main-nav"), (property::ITEMS, items)],
            &[(property::COMPONENT_CHANGE_LISTENER, true)],
        ),
    );
    insert_component_node(
        &mut tree,
        list_id,
        root_id,
        component_part_node(
            NodeTag::View,
            root_id,
            "navigation-menu-list",
            &[(property::SCOPE, "main-nav")],
            &[],
        ),
    );
    for (index, value) in ["products", "solutions", "support"].iter().enumerate() {
        let item_id = 820 + index as u32 * 3;
        insert_component_node(
            &mut tree,
            item_id,
            list_id,
            component_part_node(
                NodeTag::View,
                list_id,
                "navigation-menu-item",
                &[(property::SCOPE, "main-nav"), (property::PART_VALUE, value)],
                &[],
            ),
        );
        insert_component_node(
            &mut tree,
            item_id + 1,
            item_id,
            component_part_node(
                NodeTag::Button,
                item_id,
                "navigation-menu-trigger",
                &[(property::SCOPE, "main-nav"), (property::PART_VALUE, value)],
                &[],
            ),
        );
        insert_component_node(
            &mut tree,
            item_id + 2,
            item_id,
            component_part_node(
                NodeTag::View,
                item_id,
                "navigation-menu-popup",
                &[(property::SCOPE, "main-nav"), (property::PART_VALUE, value)],
                &[],
            ),
        );
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let state = quickgui::NavigationMenuState::new();
    let declared = [
        quickgui::NavigationMenuItem::new("products"),
        quickgui::NavigationMenuItem::new("solutions"),
        quickgui::NavigationMenuItem::new("support").disabled(true),
    ];
    let menu = quickgui::NavigationMenu::new("main-nav", &state, &declared);
    let solutions = menu.entry("solutions").expect("declared item");
    let products = menu.entry("products").expect("declared item");

    assert!(!cx.contains_element(window, solutions.popup_id()).unwrap());
    cx.click(window, solutions.trigger_id()).unwrap();
    cx.run_until_idle().unwrap();
    assert!(cx.contains_element(window, solutions.popup_id()).unwrap());
    let opened = component_change(&events, root_id);
    assert_eq!(opened["value"], serde_json::json!("solutions"));
    assert_eq!(opened["focused"], serde_json::json!("solutions"));

    cx.click(window, products.trigger_id()).unwrap();
    cx.run_until_idle().unwrap();
    let switched = component_change(&events, root_id);
    assert_eq!(switched["value"], serde_json::json!("products"));
    // Attention travelled from the second item to the first, so the panel slides left.
    assert_eq!(switched["activationDirection"], serde_json::json!("left"));
    assert!(cx.contains_element(window, products.popup_id()).unwrap());
    assert!(!cx.contains_element(window, solutions.popup_id()).unwrap());

    // Clicking the open trigger again closes its panel through the core's own toggle.
    cx.click(window, products.trigger_id()).unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change(&events, root_id)["value"],
        serde_json::Value::Null
    );
    assert!(!cx.contains_element(window, products.popup_id()).unwrap());
}

#[test]
fn malformed_base_ui_declarations_decline_instead_of_panicking() {
    let group_id = 840;
    let drawer_id = 841;
    let menu_id = 842;
    let area_id = 843;
    let otp_id = 844;
    let mut tree = NativeTree::default();
    insert_component_node(
        &mut tree,
        group_id,
        ROOT_NODE,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "checkbox-group",
            &[
                (property::SCOPE, "broken"),
                (property::ITEMS, "not json"),
                (property::VALUES, "{}"),
            ],
            &[],
        ),
    );
    // Duplicate and out-of-range snap points would panic inside the core's own constructor.
    insert_component_node(
        &mut tree,
        drawer_id,
        ROOT_NODE,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "drawer",
            &[
                (property::SCOPE, "broken-drawer"),
                (property::VALUES, "[0,-4,\"nope\"]"),
                (property::SWIPE_DIRECTION, "sideways"),
            ],
            &[],
        ),
    );
    // Duplicate item values would panic inside `NavigationMenu::new`.
    insert_component_node(
        &mut tree,
        menu_id,
        ROOT_NODE,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "navigation-menu",
            &[
                (property::SCOPE, "broken-nav"),
                (
                    property::ITEMS,
                    r#"[{"value":"one"},{"value":"one"},{"value":""}]"#,
                ),
            ],
            &[],
        ),
    );
    insert_component_node(
        &mut tree,
        area_id,
        ROOT_NODE,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            "scroll-area",
            &[
                (property::SCOPE, "broken-area"),
                (property::VIEWPORT_SIZE, "[\"wide\"]"),
                (property::CONTENT_SIZE, "not json"),
            ],
            &[],
        ),
    );
    // A zero length and an oversized one both clamp into the core's own bound.
    let mut otp = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "otp-field",
        &[(property::SCOPE, "broken-code")],
        &[],
    );
    otp.set_property(property::LENGTH, Some(PropertyValue::Number(9_000.0)));
    insert_component_node(&mut tree, otp_id, ROOT_NODE, otp);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();

    // Every malformed declaration mounts a plain element and declares nothing.
    for scope in [
        "broken",
        "broken-drawer",
        "broken-nav",
        "broken-area",
        "broken-code",
    ] {
        assert!(
            cx.contains_element(window, ElementId::named(scope))
                .unwrap(),
            "{scope} mounted"
        );
    }
    assert!(
        events
            .borrow()
            .iter()
            .all(|event| event.kind != "componentchange")
    );
}

// ---------------------------------------------------------------------------
// Base UI-aligned popovers, tooltips, range parts, toasts, tabs, toolbars,
// fields, selection controls, and dialogs
//
// Every test below declares the compound exactly the way the Solid renderer does and then asks
// the core what it decided. Nothing here re-derives geometry, a deadline, or a resolved side in
// the binding: the assertions read the identity the core mounted and the asynchronous payload it
// reported.
// ---------------------------------------------------------------------------

/// Declare one part node under a shared scope and attach it to its parent.
#[allow(clippy::too_many_arguments)]
fn declare_aligned_part(
    tree: &mut NativeTree,
    id: u32,
    parent: u32,
    tag: NodeTag,
    part: &str,
    scope: &str,
    numbers: &[(u16, f32)],
    strings: &[(u16, &str)],
    flags: &[(u16, bool)],
) {
    let mut owned = vec![(property::SCOPE, scope)];
    owned.extend_from_slice(strings);
    let mut node = component_part_node(tag, parent, part, &owned, flags);
    for (key, value) in numbers {
        node.set_property(*key, Some(PropertyValue::Number(*value)));
    }
    insert_component_node(tree, id, parent, node);
}

/// The centre of one mounted element, for a pointer gesture the core owns.
fn element_center(
    cx: &mut quickgui::TestAppContext,
    window: quickgui::WindowHandle,
    element: ElementId,
) -> quickgui::Point {
    let bounds = cx.element_bounds(window, element).unwrap();
    quickgui::Point::new(
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    )
}

/// One captured pointer event the core's own drag arithmetic consumes.
///
/// `TestAppContext` simulates targeted mouse events, not pointer capture, so a captured gesture is
/// delivered to the retained instance the same way the binding's own listener delivers it.
fn captured_pointer(
    phase: quickgui::PointerPhase,
    position: quickgui::Point,
    delta: quickgui::Vector,
    size: quickgui::Size,
) -> quickgui::PointerEvent {
    quickgui::PointerEvent {
        phase,
        position,
        origin: position,
        local_position: position,
        local_origin: position,
        delta,
        button: quickgui::MouseButton::Left,
        modifiers: quickgui::Modifiers::default(),
        size,
    }
}

#[test]
fn declared_popover_parts_mount_the_core_surface_and_report_the_resolved_placement() {
    let root_id = 900;
    let trigger_id = 901;
    let positioner_id = 902;
    let popup_id = 903;
    let arrow_id = 904;
    let viewport_id = 905;
    let title_id = 906;
    let close_id = 907;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "popover",
        "account",
        &[],
        &[],
        &[],
    );
    // The trigger is mounted whether the surface is open or closed, so it carries the whole
    // declaration and owns the retained instance.
    declare_aligned_part(
        &mut tree,
        trigger_id,
        root_id,
        NodeTag::Button,
        "popover-trigger",
        "account",
        &[
            (property::SIDE_OFFSET, 10.0),
            (property::ALIGN_OFFSET, 4.0),
            (property::COLLISION_PADDING, 12.0),
        ],
        &[(property::SIDE, "top"), (property::ALIGN, "end")],
        &[
            (property::OPEN, true),
            (property::MODAL, true),
            (property::STICKY, false),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    for (id, parent, tag, part) in [
        (positioner_id, root_id, NodeTag::View, "popover-positioner"),
        (popup_id, positioner_id, NodeTag::View, "popover-popup"),
        (arrow_id, popup_id, NodeTag::View, "popover-arrow"),
        (viewport_id, popup_id, NodeTag::View, "popover-viewport"),
        (title_id, popup_id, NodeTag::View, "popover-title"),
        (close_id, popup_id, NodeTag::Button, "popover-close"),
    ] {
        declare_aligned_part(&mut tree, id, parent, tag, part, "account", &[], &[], &[]);
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let root = ElementId::named("account");
    let popover = quickgui::Popover::new(popover_trigger_id(root), popover_surface_id(root), true);

    for element in [
        popover.trigger_id(),
        popover.popover_id(),
        popover.positioner_id(),
        popover.arrow_id(),
        popover.viewport_id(),
        popover.title_id(),
        popover.close_id(),
    ] {
        assert!(cx.contains_element(window, element).unwrap());
    }

    // The declared side and align are only a preference; the retained tree publishes the side the
    // surface really ended up on and the binding reports exactly that.
    // The placement is published during the paint QuickGUI was already performing, and the core
    // requests exactly one correcting frame when it changes.
    cx.element_bounds(window, popover.positioner_id()).unwrap();
    cx.run_until_idle().unwrap();
    let reported = component_change(&events, trigger_id);
    let placement = &reported["placement"];
    assert!(
        matches!(placement["side"].as_str(), Some("top" | "bottom")),
        "the core resolved a real side: {placement}"
    );
    assert!(placement["anchorWidth"].as_f64().unwrap() > 0.0);
    assert!(placement["availableHeight"].as_f64().unwrap() >= 0.0);
    assert_eq!(placement["anchorHidden"], serde_json::json!(false));

    // Closing the declaration unmounts every surface part, exactly as the core decides.
    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();
        tree.nodes
            .get_mut(&trigger_id)
            .unwrap()
            .set_property(property::OPEN, Some(PropertyValue::Bool(false)));
        cx.invalidate();
    })
    .unwrap();
    cx.run_until_idle().unwrap();
    assert!(!cx.contains_element(window, popover.popover_id()).unwrap());
    assert!(!cx.contains_element(window, popover.arrow_id()).unwrap());
    assert!(cx.contains_element(window, popover.trigger_id()).unwrap());
}

#[test]
fn a_declared_popover_trigger_opens_on_hover_through_the_core_deadline() {
    let root_id = 910;
    let trigger_id = 911;
    let positioner_id = 912;
    let popup_id = 913;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "popover",
        "hovered",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        trigger_id,
        root_id,
        NodeTag::Button,
        "popover-trigger",
        "hovered",
        &[(property::DELAY, 0.0), (property::CLOSE_DELAY, 0.0)],
        &[],
        &[
            (property::OPEN_ON_HOVER, true),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    declare_aligned_part(
        &mut tree,
        positioner_id,
        root_id,
        NodeTag::View,
        "popover-positioner",
        "hovered",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        popup_id,
        positioner_id,
        NodeTag::View,
        "popover-popup",
        "hovered",
        &[],
        &[],
        &[],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let root = ElementId::named("hovered");
    let surface = popover_surface_id(root);
    assert!(!cx.contains_element(window, surface).unwrap());

    let center = element_center(&mut cx, window, popover_trigger_id(root));
    cx.visual(window).unwrap().move_pointer(center).unwrap();
    assert!(cx.contains_element(window, surface).unwrap());
    assert_eq!(component_change(&events, trigger_id)["open"], true);
}

#[test]
fn declared_tooltip_parts_mount_only_while_the_core_holds_them_open() {
    let provider_id = 920;
    let root_id = 921;
    let trigger_id = 922;
    let positioner_id = 923;
    let popup_id = 924;
    let arrow_id = 925;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        provider_id,
        ROOT_NODE,
        NodeTag::View,
        "tooltip-provider",
        "hints",
        &[
            (property::DELAY, 0.0),
            (property::CLOSE_DELAY, 0.0),
            (property::TIMEOUT, 400.0),
        ],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        root_id,
        provider_id,
        NodeTag::View,
        "tooltip",
        "save-hint",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        trigger_id,
        root_id,
        NodeTag::Button,
        "tooltip-trigger",
        "save-hint",
        &[(property::SIDE_OFFSET, 9.0)],
        &[
            (property::PROVIDER, "hints"),
            (property::SIDE, "top"),
            (property::TRACK_CURSOR_AXIS, "x"),
        ],
        &[
            (property::OPEN, true),
            (property::HOVERABLE, true),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    for (id, parent, tag, part) in [
        (positioner_id, root_id, NodeTag::View, "tooltip-positioner"),
        (popup_id, positioner_id, NodeTag::View, "tooltip-popup"),
        (arrow_id, popup_id, NodeTag::View, "tooltip-arrow"),
    ] {
        declare_aligned_part(&mut tree, id, parent, tag, part, "save-hint", &[], &[], &[]);
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let root = ElementId::named("save-hint");
    assert!(
        cx.contains_element(window, tooltip_trigger_id(root))
            .unwrap()
    );
    assert!(cx.contains_element(window, tooltip_popup_id(root)).unwrap());
    cx.run_until_idle().unwrap();
    // A cursor-tracking trigger reports its painted bounds, so the harness paints at idle and the
    // core resolves the declared `top` against the room above a trigger sitting at the top of the
    // window: it flips to `bottom`, and that resolved side is what JavaScript reads.
    assert_eq!(
        component_change(&events, trigger_id)["placement"]["side"],
        serde_json::json!("bottom")
    );

    // A disabled tooltip cancels its pending deadline and closes, so nothing is mounted at all.
    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();
        let node = tree.nodes.get_mut(&trigger_id).unwrap();
        node.set_property(property::OPEN, Some(PropertyValue::Bool(false)));
        node.set_property(property::DISABLED, Some(PropertyValue::Bool(true)));
        cx.invalidate();
    })
    .unwrap();
    cx.run_until_idle().unwrap();
    assert!(!cx.contains_element(window, tooltip_popup_id(root)).unwrap());
    assert!(
        !cx.contains_element(window, tooltip_popup_id(root)).unwrap(),
        "a disabled tooltip cancels its deadline and mounts no popup"
    );
}

#[test]
fn a_declared_slider_reports_the_cores_commit_boundary_and_formatted_value() {
    let root_id = 930;
    let track_id = 931;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "slider",
        "volume",
        &[
            (property::MINIMUM, 0.0),
            (property::MAXIMUM, 100.0),
            (property::STEP, 10.0),
            (property::MIN_STEPS_BETWEEN_VALUES, 1.0),
        ],
        &[
            (property::VALUES, "[20]"),
            (property::FORMAT, "percent"),
            (property::THUMB_ALIGNMENT, "edge"),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    for (id, part) in [
        (track_id, "slider-track"),
        (932, "slider-indicator"),
        (933, "slider-label"),
        (934, "slider-value"),
        (935, "slider-control"),
    ] {
        let parent = if part == "slider-indicator" {
            track_id
        } else {
            root_id
        };
        declare_aligned_part(
            &mut tree,
            id,
            parent,
            NodeTag::View,
            part,
            "volume",
            &[],
            &[],
            &[],
        );
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let slider = Slider::new(
        ElementId::named("volume"),
        &SliderState::range(0.0, 100.0, &[20.0]),
    );
    for element in [
        slider.track_id(),
        slider.range_id(),
        slider.label_id(),
        slider.value_id(),
        slider.control_id(),
    ] {
        assert!(cx.contains_element(window, element).unwrap());
    }

    // A captured drag reports the core's own dragging flag while it is in flight and its commit
    // boundary on release; the binding never invents either one.
    let track = cx.element_bounds(window, slider.track_id()).unwrap();
    let size = quickgui::Size::new(track.width, track.height);
    cx.update(view, |view, cx| {
        let retained = view
            .components
            .sliders
            .get_mut(&ElementId::named("volume").as_u64())
            .expect("the declared slider is retained");
        let change = retained.state.apply_pointer_change(
            &captured_pointer(
                quickgui::PointerPhase::Down,
                quickgui::Point::new(track.x + track.width * 0.8, track.y),
                quickgui::Vector::default(),
                size,
            ),
            size,
        );
        assert!(change.changed);
        assert!(!change.committed);
        cx.invalidate();
    })
    .unwrap();
    cx.run_until_idle().unwrap();
    let dragging = component_change(&events, root_id);
    assert_eq!(dragging["dragging"], true);
    assert_eq!(dragging["committed"], false);
    assert_eq!(dragging["displayValue"], serde_json::json!("80%"));

    cx.update(view, |view, cx| {
        let retained = view
            .components
            .sliders
            .get_mut(&ElementId::named("volume").as_u64())
            .expect("the declared slider is retained");
        let change = retained.state.apply_pointer_change(
            &captured_pointer(
                quickgui::PointerPhase::Up,
                quickgui::Point::new(track.x + track.width * 0.8, track.y),
                quickgui::Vector::default(),
                size,
            ),
            size,
        );
        // The core owns the commit boundary; the binding only records that it happened.
        assert!(change.committed);
        retained.committed = true;
        cx.invalidate();
    })
    .unwrap();
    cx.run_until_idle().unwrap();
    let committed = component_change(&events, root_id);
    assert_eq!(committed["committed"], true);
    assert_eq!(committed["dragging"], false);
}

#[test]
fn slider_control_owns_the_hit_area_and_controlled_echoes_keep_the_upper_thumb() {
    let root_id = 936;
    let control_id = 937;
    let track_id = 938;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "slider",
        "price-drag",
        &[
            (property::MINIMUM, 0.0),
            (property::MAXIMUM, 100.0),
            (property::STEP, 1.0),
            (property::MIN_STEPS_BETWEEN_VALUES, 5.0),
        ],
        &[(property::VALUES, "[20,70]")],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    declare_aligned_part(
        &mut tree,
        control_id,
        root_id,
        NodeTag::View,
        "slider-control",
        "price-drag",
        &[(property::WIDTH, 200.0), (property::HEIGHT, 24.0)],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        track_id,
        control_id,
        NodeTag::View,
        "slider-track",
        "price-drag",
        &[(property::WIDTH, 200.0), (property::HEIGHT, 4.0)],
        &[],
        &[],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let slider = Slider::new(
        ElementId::named("price-drag"),
        &SliderState::range(0.0, 100.0, &[20.0, 70.0]),
    );
    let control = cx.element_bounds(window, slider.control_id()).unwrap();
    let track = cx.element_bounds(window, slider.track_id()).unwrap();
    assert_eq!(control.height, 24.0);
    assert_eq!(track.height, 4.0);

    let point = |fraction: f32| {
        quickgui::Point::new(
            control.x + control.width * fraction,
            control.y + control.height - 1.0,
        )
    };
    let pointer = |phase, position| {
        captured_pointer(
            phase,
            position,
            quickgui::Vector::default(),
            quickgui::Size::ZERO,
        )
    };
    let redeclare = |cx: &mut quickgui::TestAppContext, values: &serde_json::Value| {
        cx.update(view, |view, cx| {
            view.tree
                .borrow_mut()
                .nodes
                .get_mut(&root_id)
                .unwrap()
                .set_property(
                    property::VALUES,
                    Some(PropertyValue::String(Arc::from(
                        values.to_string().as_str(),
                    ))),
                );
            cx.invalidate();
        })
        .unwrap();
        cx.run_until_idle().unwrap();
    };

    // A press near the upper thumb lands in Control's 24px box, outside the 4px Track, and the
    // full Control width is the value coordinate space.
    cx.simulate_pointer(
        window,
        slider.control_id(),
        pointer(quickgui::PointerPhase::Down, point(0.75)),
    )
    .unwrap();
    let first = component_change(&events, root_id);
    assert_eq!(first["values"], serde_json::json!([20.0, 75.0]));
    assert_eq!(first["dragging"], true);

    // Solid immediately echoes the controlled value. That acknowledgement must not rebuild the
    // state and silently return the active thumb to index zero.
    redeclare(&mut cx, &first["values"]);
    let (active, dragging) = cx
        .read(view, |view| {
            let state = &view.components.sliders.values().next().unwrap().state;
            (state.active_thumb(), state.is_dragging())
        })
        .unwrap();
    assert_eq!(active, 1);
    assert!(dragging);

    cx.simulate_pointer(
        window,
        slider.control_id(),
        pointer(quickgui::PointerPhase::Move, point(0.85)),
    )
    .unwrap();
    let second = component_change(&events, root_id);
    assert_eq!(second["values"], serde_json::json!([20.0, 85.0]));

    cx.simulate_pointer(
        window,
        slider.control_id(),
        pointer(quickgui::PointerPhase::Move, point(0.90)),
    )
    .unwrap();
    let third = component_change(&events, root_id);
    assert_eq!(third["values"], serde_json::json!([20.0, 90.0]));
    cx.simulate_pointer(
        window,
        slider.control_id(),
        pointer(quickgui::PointerPhase::Up, point(0.90)),
    )
    .unwrap();
    let released = component_change(&events, root_id);
    assert_eq!(released["values"], serde_json::json!([20.0, 90.0]));
    assert_eq!(released["committed"], true);

    // Even after release, late controlled echoes are acknowledgements rather than new values that
    // can rewind the thumb. A genuinely new declaration still reseeds the settled slider.
    redeclare(&mut cx, &second["values"]);
    redeclare(&mut cx, &third["values"]);
    let values = cx
        .read(view, |view| {
            view.components
                .sliders
                .values()
                .next()
                .unwrap()
                .state
                .values()
                .to_vec()
        })
        .unwrap();
    assert_eq!(values, vec![20.0, 90.0]);

    redeclare(&mut cx, &serde_json::json!([10.0, 95.0]));
    let values = cx
        .read(view, |view| {
            view.components
                .sliders
                .values()
                .next()
                .unwrap()
                .state
                .values()
                .to_vec()
        })
        .unwrap();
    assert_eq!(values, vec![10.0, 95.0]);
}

#[test]
fn a_declared_number_field_scrub_area_steps_through_the_core() {
    let root_id = 940;
    let group_id = 941;
    let scrub_id = 942;
    let cursor_id = 943;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "number-field",
        "quantity",
        &[
            (property::MINIMUM, 0.0),
            (property::MAXIMUM, 100.0),
            (property::STEP, 1.0),
            (property::SMALL_STEP, 0.5),
            (property::LARGE_STEP, 5.0),
            (property::PITCH, 2.0),
        ],
        &[(property::VALUES, "[10]")],
        &[
            (property::SNAP_ON_STEP, true),
            (property::ALLOW_WHEEL_SCRUB, false),
            (property::REQUIRED, true),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    declare_aligned_part(
        &mut tree,
        group_id,
        root_id,
        NodeTag::View,
        "number-field-group",
        "quantity",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        scrub_id,
        group_id,
        NodeTag::View,
        "number-field-scrub-area",
        "quantity",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        cursor_id,
        scrub_id,
        NodeTag::View,
        "number-field-scrub-area-cursor",
        "quantity",
        &[],
        &[],
        &[],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let field = NumberField::new(ElementId::named("quantity"));
    for element in [
        field.group_id(),
        field.scrub_area_id(),
        field.scrub_area_cursor_id(),
    ] {
        assert!(cx.contains_element(window, element).unwrap());
    }

    // The core turns the captured drag into whole steps at the declared sensitivity, keeping the
    // unconverted remainder so a slow drag moves one step at a time.
    let center = element_center(&mut cx, window, field.scrub_area_id());
    let key = ElementId::named("quantity").as_u64();
    cx.update(view, |view, cx| {
        let state = &mut view
            .components
            .number_fields
            .get_mut(&key)
            .expect("the declared number field is retained")
            .state;
        assert!(state.apply_scrub(&captured_pointer(
            quickgui::PointerPhase::Down,
            center,
            quickgui::Vector::default(),
            quickgui::Size::new(40.0, 40.0),
        )));
        assert!(state.apply_scrub(&captured_pointer(
            quickgui::PointerPhase::Move,
            quickgui::Point::new(center.x + 20.0, center.y),
            quickgui::Vector::new(20.0, 0.0),
            quickgui::Size::new(40.0, 40.0),
        )));
        cx.invalidate();
    })
    .unwrap();
    cx.run_until_idle().unwrap();
    let scrubbing = component_change(&events, root_id);
    assert_eq!(scrubbing["scrubbing"], true);
    assert_eq!(scrubbing["required"], true);
    assert!(scrubbing["numberValue"].as_f64().unwrap() > 10.0);

    cx.update(view, |view, cx| {
        let state = &mut view
            .components
            .number_fields
            .get_mut(&key)
            .expect("the declared number field is retained")
            .state;
        assert!(state.end_scrub());
        cx.invalidate();
    })
    .unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(component_change(&events, root_id)["scrubbing"], false);
}

#[test]
fn a_read_only_number_field_refuses_every_change_while_staying_focusable() {
    let root_id = 950;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "number-field",
        "locked",
        &[(property::STEP, 1.0)],
        &[(property::VALUES, "[7]")],
        &[(property::READ_ONLY, true)],
    );
    declare_aligned_part(
        &mut tree,
        951,
        root_id,
        NodeTag::Input,
        "number-field-input",
        "locked",
        &[],
        &[],
        &[],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let field = NumberField::new(ElementId::named("locked"));
    // Read-only is not disabled: the input still takes focus.
    cx.focus(window, field.input_id()).unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(field.input_id()));
    // Every mutator refuses while the control is read-only, and the value is untouched.
    let refused = cx
        .update(view, |view, _cx| {
            let state = &mut view
                .components
                .number_fields
                .get_mut(&ElementId::named("locked").as_u64())
                .expect("the declared number field is retained")
                .state;
            (state.step_by(1.0), state.set_text("42"), state.value())
        })
        .unwrap();
    assert_eq!(refused, (false, false, Some(7.0)));
}

#[test]
fn a_declared_toast_provider_limits_the_stack_and_reports_each_toasts_geometry() {
    let viewport_id = 960;
    let portal_id = 961;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        viewport_id,
        ROOT_NODE,
        NodeTag::View,
        "toast-viewport",
        "notices",
        &[(property::LIMIT, 2.0), (property::PITCH, 12.0)],
        &[
            (
                property::TOASTS,
                r#"[{"id":"a","title":"Saved","type":"success"},
                    {"id":"b","title":"Uploading","type":"loading"},
                    {"id":"c","title":"Failed","type":"error"}]"#,
            ),
            (property::SWIPE_DIRECTION, "left"),
        ],
        &[
            (property::STACK_EXPANDED, true),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    declare_aligned_part(
        &mut tree,
        portal_id,
        viewport_id,
        NodeTag::View,
        "toast-portal",
        "notices",
        &[],
        &[],
        &[],
    );
    let mut next = 962;
    for declared in ["a", "b", "c"] {
        let root = next;
        declare_aligned_part(
            &mut tree,
            root,
            portal_id,
            NodeTag::View,
            "toast",
            "notices",
            &[],
            &[(property::PART_VALUE, declared)],
            &[],
        );
        declare_aligned_part(
            &mut tree,
            root + 1,
            root,
            NodeTag::View,
            "toast-content",
            "notices",
            &[],
            &[(property::PART_VALUE, declared)],
            &[],
        );
        next += 2;
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, _view) = mounted_base_ui_view(tree, Rc::clone(&events));
    cx.run_until_idle().unwrap();
    let reported = component_change(&events, viewport_id);
    let toasts = reported["toasts"]
        .as_array()
        .expect("the queue is reported");
    assert_eq!(toasts.len(), 3);
    // The newest toast is index zero, the declared limit flags the oldest without silencing it,
    // and each offset is the core's own `index * pitch`.
    assert_eq!(toasts[0]["id"], serde_json::json!("c"));
    assert_eq!(toasts[0]["type"], serde_json::json!("error"));
    assert_eq!(toasts[0]["index"], serde_json::json!(0));
    assert_eq!(toasts[0]["offset"], serde_json::json!(0.0));
    assert_eq!(toasts[1]["offset"], serde_json::json!(12.0));
    assert_eq!(toasts[1]["type"], serde_json::json!("loading"));
    assert_eq!(toasts[2]["limited"], serde_json::json!(true));
    assert_eq!(toasts[0]["expanded"], serde_json::json!(true));
}

#[test]
fn declared_tabs_report_the_cores_activation_direction_and_indicator_geometry() {
    let root_id = 970;
    let list_id = 971;
    let first_id = 972;
    let second_id = 973;
    let indicator_id = 974;
    let declare = |tree: &mut NativeTree, id, parent, tag, part, value: &str, index: f32| {
        declare_aligned_part(
            tree,
            id,
            parent,
            tag,
            part,
            "views",
            &[(property::ITEM_INDEX, index)],
            &[
                (property::ACTIVE_VALUE, "list"),
                (property::PART_VALUE, value),
                (property::ANCHOR_PLACEMENT, "bottom"),
            ],
            &[],
        );
    };
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "tabs",
        "views",
        &[],
        &[(property::ACTIVE_VALUE, "list")],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    declare(
        &mut tree,
        list_id,
        root_id,
        NodeTag::View,
        "tabs-list",
        "",
        0.0,
    );
    declare(
        &mut tree,
        first_id,
        list_id,
        NodeTag::Button,
        "tab",
        "list",
        0.0,
    );
    declare(
        &mut tree,
        second_id,
        list_id,
        NodeTag::Button,
        "tab",
        "grid",
        1.0,
    );
    declare(
        &mut tree,
        indicator_id,
        list_id,
        NodeTag::View,
        "tab-indicator",
        "list",
        0.0,
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let active = Tabs::new(ElementId::named("views"), ElementId::named("list")).tab("list");
    // The indicator publishes the active tab's laid-out box during paint.
    cx.element_bounds(window, active.indicator_id()).unwrap();
    cx.run_until_idle().unwrap();
    let first = component_change(&events, root_id);
    assert_eq!(first["activationDirection"], serde_json::json!("none"));
    let indicator = &first["indicator"];
    assert!(indicator["width"].as_f64().unwrap() > 0.0);

    // Moving the declaration forward makes the core record the direction the selection travelled.
    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();
        for id in [root_id, list_id, first_id, second_id, indicator_id] {
            tree.nodes.get_mut(&id).unwrap().set_property(
                property::ACTIVE_VALUE,
                Some(PropertyValue::String(Arc::from("grid"))),
            );
        }
        tree.nodes.get_mut(&indicator_id).unwrap().set_property(
            property::PART_VALUE,
            Some(PropertyValue::String(Arc::from("grid"))),
        );
        cx.invalidate();
    })
    .unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change(&events, root_id)["activationDirection"],
        serde_json::json!("right")
    );
}

#[test]
fn declared_toolbar_button_link_input_group_and_separator_parts_share_one_roving_stop() {
    let root_id = 980;
    let group_id = 981;
    let button_id = 982;
    let link_id = 983;
    let input_id = 984;
    let separator_id = 985;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "toolbar",
        "actions",
        &[],
        &[(
            property::ITEMS,
            r#"[{"value":"cut"},{"value":"docs"},{"value":"search"},
                {"value":"paste","disabled":true,"focusableWhenDisabled":true}]"#,
        )],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    declare_aligned_part(
        &mut tree,
        group_id,
        root_id,
        NodeTag::View,
        "toolbar-group",
        "actions",
        &[],
        &[],
        &[],
    );
    for (id, part, value) in [
        (button_id, "toolbar-button", "cut"),
        (link_id, "toolbar-link", "docs"),
        (input_id, "toolbar-input", "search"),
        (986, "toolbar-item", "paste"),
    ] {
        declare_aligned_part(
            &mut tree,
            id,
            group_id,
            if part == "toolbar-input" {
                NodeTag::Input
            } else {
                NodeTag::Button
            },
            part,
            "actions",
            &[],
            &[(property::PART_VALUE, value)],
            &[],
        );
    }
    declare_aligned_part(
        &mut tree,
        separator_id,
        root_id,
        NodeTag::View,
        "toolbar-separator",
        "actions",
        &[],
        &[],
        &[],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let toolbar_items = [
        ToolbarItem::new(ElementId::named("cut")),
        ToolbarItem::new(ElementId::named("docs")),
        ToolbarItem::new(ElementId::named("search")),
        ToolbarItem::new(ElementId::named("paste")).disabled(true),
    ];
    let toolbar = Toolbar::new(
        ElementId::named("actions"),
        &ToolbarState::empty(),
        &toolbar_items,
    );
    for value in ["cut", "docs", "search", "paste"] {
        assert!(
            cx.contains_element(window, toolbar.item_id(ElementId::named(value)))
                .unwrap(),
            "{value} mounted"
        );
    }

    // Arrow navigation skips the disabled item, so the roving stop lands on the last enabled one.
    cx.focus(window, toolbar.item_id(ElementId::named("cut")))
        .unwrap();
    cx.simulate_keystrokes(window, "right right right").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "active": "search" })
    );

    // `focusableWhenDisabled` is about the Tab sequence: the disabled command stays discoverable.
    let paste = toolbar.item_id(ElementId::named("paste"));
    cx.focus(window, paste).unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(paste));
}

#[test]
fn a_declared_field_reports_the_cores_validation_contract_and_mounts_item_and_validity() {
    let root_id = 990;
    let item_id = 991;
    let validity_id = 992;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "field",
        "email",
        &[(property::VALIDATION_DEBOUNCE_TIME, 250.0)],
        &[(property::VALIDATION_MODE, "onChange")],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    declare_aligned_part(
        &mut tree,
        item_id,
        root_id,
        NodeTag::View,
        "field-item",
        "email",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        validity_id,
        root_id,
        NodeTag::View,
        "field-validity",
        "email",
        &[],
        &[],
        &[(property::OPEN, true)],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let field = Field::new(ElementId::named("email"));
    assert!(cx.contains_element(window, field.item_id()).unwrap());
    assert!(cx.contains_element(window, field.validity_id()).unwrap());

    cx.run_until_idle().unwrap();
    let reported = component_change(&events, root_id);
    assert_eq!(reported["validation"]["change"], true);
    assert_eq!(reported["validation"]["blur"], true);
    assert_eq!(reported["validation"]["submit"], true);
    assert_eq!(reported["validationDelay"]["change"], 250.0);
    assert_eq!(reported["validationDelay"]["blur"], 0.0);
}

#[test]
fn read_only_selection_controls_and_a_parent_checkbox_derive_their_state_from_the_core() {
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        1000,
        ROOT_NODE,
        NodeTag::Button,
        "checkbox",
        "mixed",
        &[],
        &[(property::VALUES, "[true,false,true]")],
        &[(property::PARENT, true)],
    );
    declare_aligned_part(
        &mut tree,
        1001,
        ROOT_NODE,
        NodeTag::Button,
        "switch",
        "locked-switch",
        &[],
        &[],
        &[(property::CHECKED, true), (property::READ_ONLY, true)],
    );
    declare_aligned_part(
        &mut tree,
        1002,
        ROOT_NODE,
        NodeTag::Button,
        "radio",
        "locked-radio",
        &[],
        &[],
        &[(property::READ_ONLY, true)],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    for id in [1000u32, 1001, 1002] {
        assert!(
            cx.contains_element(window, ElementId::new(u64::from(id)))
                .unwrap(),
            "{id} mounted"
        );
    }
    // A read-only control refuses every change while keeping its place in the Tab sequence.
    assert_eq!(
        quickgui::Switch::new(true).read_only(true).next_checked(),
        None
    );
    assert!(
        !quickgui::Radio::new(false)
            .read_only(true)
            .accepts_selection()
    );
    assert_eq!(
        Checkbox::parent([true, false, true]).state(),
        ToggleState::Mixed
    );
}

#[test]
fn a_declared_dialog_viewport_mounts_and_the_exit_transition_reports_its_completion() {
    let root_id = 1010;
    let popup_id = 1011;
    let viewport_id = 1012;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "dialog",
        "confirm",
        &[(property::EXIT_DURATION, 40.0)],
        &[],
        &[
            (property::OPEN, true),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    declare_aligned_part(
        &mut tree,
        popup_id,
        root_id,
        NodeTag::View,
        "dialog-popup",
        "confirm",
        &[],
        &[],
        &[(property::OPEN, true)],
    );
    declare_aligned_part(
        &mut tree,
        viewport_id,
        popup_id,
        NodeTag::View,
        "dialog-viewport",
        "confirm",
        &[],
        &[],
        &[(property::OPEN, true)],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let dialog = quickgui::Dialog::new("confirm", true);
    assert!(cx.contains_element(window, dialog.viewport_id()).unwrap());
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "openChangeComplete": true })
    );

    // The core holds a closing dialog mounted for exactly the declared exit transition.
    cx.update(view, |view, cx| {
        let mut tree = view.tree.borrow_mut();
        for id in [root_id, popup_id, viewport_id] {
            tree.nodes
                .get_mut(&id)
                .unwrap()
                .set_property(property::OPEN, Some(PropertyValue::Bool(false)));
        }
        cx.invalidate();
    })
    .unwrap();
    cx.advance_frame().unwrap();
    assert!(cx.contains_element(window, dialog.root_id()).unwrap());
    cx.advance_time(Duration::from_millis(60)).unwrap();
    cx.run_until_idle().unwrap();
    assert!(!cx.contains_element(window, dialog.root_id()).unwrap());
    assert_eq!(
        component_change(&events, root_id),
        serde_json::json!({ "openChangeComplete": false })
    );
}

#[test]
fn declared_progress_and_meter_parts_report_the_cores_status_and_formatted_value() {
    let progress_id = 1020;
    let meter_id = 1030;
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        progress_id,
        ROOT_NODE,
        NodeTag::View,
        "progress",
        "upload",
        &[(property::VALUE, 3.0), (property::MAXIMUM, 4.0)],
        &[(property::FORMAT, "fraction")],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    for (offset, part) in [
        (1u32, "progress-track"),
        (2, "progress-indicator"),
        (3, "progress-label"),
        (4, "progress-value"),
    ] {
        let parent = if part == "progress-indicator" {
            progress_id + 1
        } else {
            progress_id
        };
        declare_aligned_part(
            &mut tree,
            progress_id + offset,
            parent,
            NodeTag::View,
            part,
            "upload",
            &[],
            &[],
            &[],
        );
    }
    declare_aligned_part(
        &mut tree,
        meter_id,
        ROOT_NODE,
        NodeTag::View,
        "meter",
        "storage",
        &[
            (property::VALUE, 50.0),
            (property::MINIMUM, 0.0),
            (property::MAXIMUM, 100.0),
        ],
        &[(property::FORMAT, "percent")],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    for (offset, part) in [
        (1u32, "meter-track"),
        (2, "meter-label"),
        (3, "meter-value"),
    ] {
        declare_aligned_part(
            &mut tree,
            meter_id + offset,
            meter_id,
            NodeTag::View,
            part,
            "storage",
            &[],
            &[],
            &[],
        );
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let progress = Progress::new(3.0, 4.0).id("upload");
    for element in [
        progress.track_id(),
        progress.indicator_id(),
        progress.label_id(),
        progress.value_id(),
    ] {
        assert!(cx.contains_element(window, element.unwrap()).unwrap());
    }
    cx.run_until_idle().unwrap();
    let reported = component_change(&events, progress_id);
    assert_eq!(reported["status"], serde_json::json!("progressing"));
    assert_eq!(reported["displayValue"], serde_json::json!("3 of 4"));
    assert_eq!(
        component_change(&events, meter_id)["displayValue"],
        serde_json::json!("50%")
    );
}

#[test]
fn malformed_aligned_declarations_declare_nothing_instead_of_panicking() {
    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        1100,
        ROOT_NODE,
        NodeTag::Button,
        "popover-trigger",
        "broken-popover",
        &[
            (property::SIDE_OFFSET, f32::MAX),
            (property::ALIGN_OFFSET, f32::MIN),
            (property::COLLISION_PADDING, -1.0),
        ],
        &[
            (property::SIDE, "sideways"),
            (property::ALIGN, "middle"),
            (property::ANCHOR_POINT, "not,a,point"),
            (property::ANCHOR_TARGET, "4294967295"),
        ],
        &[(property::OPEN, true)],
    );
    declare_aligned_part(
        &mut tree,
        1101,
        ROOT_NODE,
        NodeTag::Button,
        "tooltip-trigger",
        "broken-tooltip",
        &[(property::DELAY, -5.0), (property::SIDE_OFFSET, f32::MAX)],
        &[
            (property::PROVIDER, "missing"),
            (property::TRACK_CURSOR_AXIS, "diagonal"),
        ],
        &[(property::OPEN, true)],
    );
    declare_aligned_part(
        &mut tree,
        1102,
        ROOT_NODE,
        NodeTag::View,
        "toast-viewport",
        "broken-toasts",
        &[(property::LIMIT, -3.0), (property::PITCH, f32::NAN)],
        &[
            (property::TOASTS, "{not json"),
            (property::SWIPE_DIRECTION, "sideways"),
        ],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        1103,
        ROOT_NODE,
        NodeTag::Button,
        "checkbox",
        "broken-parent",
        &[],
        &[(property::VALUES, "[1,2,3]")],
        &[(property::PARENT, true)],
    );
    declare_aligned_part(
        &mut tree,
        1104,
        ROOT_NODE,
        NodeTag::View,
        "field",
        "broken-field",
        &[(property::VALIDATION_DEBOUNCE_TIME, f32::INFINITY)],
        &[(property::VALIDATION_MODE, "whenever")],
        &[],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    assert!(
        cx.contains_element(window, ElementId::named("broken-toasts"))
            .unwrap()
    );
    for scope in ["broken-popover", "broken-tooltip"] {
        let root = ElementId::named(scope);
        let trigger = if scope == "broken-popover" {
            popover_trigger_id(root)
        } else {
            tooltip_trigger_id(root)
        };
        assert!(
            cx.contains_element(window, trigger).unwrap(),
            "{scope} mounted"
        );
    }
    assert!(
        cx.contains_element(
            window,
            Field::new(ElementId::named("broken-field")).root_id()
        )
        .unwrap(),
        "a field root mounts under the identity the core derives from its control"
    );
    assert!(
        cx.contains_element(window, ElementId::new(1103)).unwrap(),
        "a checkbox mounts under its own node identity"
    );
    // A parent checkbox whose declared children are not booleans folds an empty set, which the
    // core answers with `Off` rather than a panic.
    assert_eq!(
        Checkbox::parent(declared_flags(
            &component_part_node(NodeTag::Button, ROOT_NODE, "checkbox", &[], &[]),
            property::VALUES,
        ))
        .state(),
        ToggleState::Off
    );
}

// ---------------------------------------------------------------------------
// Base UI-aligned menus, selects, and comboboxes
//
// Every test below declares the compound exactly the way the Solid renderer does and then asks
// the core what it decided: which identity a row mounted under, which row the model holds
// checked, and what the asynchronous payload carried. Nothing here re-derives a row identity, a
// toggle, or a filter in the binding.
// ---------------------------------------------------------------------------

fn menu_application() -> quickgui::Application {
    base_ui_application()
        .bind_keys(quickgui::popover_menu_key_bindings())
        .bind_keys(quickgui::popover_menu_horizontal_key_bindings())
}

fn mounted_menu_view(
    tree: NativeTree,
    events: EventQueue,
) -> (
    quickgui::TestAppContext,
    quickgui::TestWindowHandle<NativeView>,
) {
    let view = component_part_view(93, tree, events);
    quickgui::TestAppContext::from_application(
        menu_application(),
        quickgui::WindowOptions::default(),
        view,
    )
    .expect("the hosted menu view mounts")
}

/// The last queued change for one instance that carries `field`.
///
/// A menu row reports two different things on the same channel — the activation edge and the
/// roving item state — so a test that wants one of them names the field it is waiting for.
fn component_change_field(events: &EventQueue, target: u32, field: &str) -> serde_json::Value {
    let queue = events.borrow();
    let mut latest = serde_json::Value::Null;
    for event in queue.iter() {
        if event.kind != "componentchange" || event.target != target {
            continue;
        }
        let payload: serde_json::Value =
            serde_json::from_str(event.value.as_deref().unwrap_or("null"))
                .expect("a component change carries bounded JSON");
        if payload.get(field).is_some() {
            latest = payload;
        }
    }
    latest
}

/// The identity one declared row mounts under, derived exactly the way the binding derives it.
fn menu_row_id(scope: &str, items: &[PopoverMenuItem], index: usize) -> ElementId {
    let popup = menu_popup_id(ElementId::named(scope));
    PopoverMenu::new(items.to_vec())
        .expect("a declared level is valid")
        .item_element_id(popup, index)
        .expect("an interactive row has an identity")
}

#[test]
fn declared_menu_parts_mount_the_core_rows_and_report_what_it_activated() {
    let root_id = 1_400;
    let trigger_id = 1_401;
    let positioner_id = 1_402;
    let popup_id = 1_403;
    let label_id = 1_404;
    let copy_id = 1_405;
    let separator_id = 1_406;
    let wrap_id = 1_407;
    let group_id = 1_408;
    let compact_id = 1_409;
    let cozy_id = 1_410;

    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "menu",
        "edit",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        trigger_id,
        root_id,
        NodeTag::Button,
        "menu-trigger",
        "edit",
        &[],
        &[],
        &[
            (property::OPEN, false),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    declare_aligned_part(
        &mut tree,
        positioner_id,
        root_id,
        NodeTag::View,
        "menu-positioner",
        "edit",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        popup_id,
        positioner_id,
        NodeTag::View,
        "menu-popup",
        "edit",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        label_id,
        popup_id,
        NodeTag::View,
        "menu-group-label",
        "edit",
        &[],
        &[
            (property::PART_VALUE, "clipboard"),
            (property::ACCESSIBILITY_LABEL, "Clipboard"),
        ],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        copy_id,
        popup_id,
        NodeTag::View,
        "menu-item",
        "edit",
        &[],
        &[
            (property::PART_VALUE, "copy"),
            (property::ACCESSIBILITY_LABEL, "Copy"),
        ],
        &[
            (property::CLICK_LISTENER, true),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    declare_aligned_part(
        &mut tree,
        separator_id,
        popup_id,
        NodeTag::View,
        "menu-separator",
        "edit",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        wrap_id,
        popup_id,
        NodeTag::View,
        "menu-checkbox-item",
        "edit",
        &[],
        &[
            (property::PART_VALUE, "wrap"),
            (property::ACCESSIBILITY_LABEL, "Wrap lines"),
        ],
        &[
            (property::CHECKED, false),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    declare_aligned_part(
        &mut tree,
        group_id,
        popup_id,
        NodeTag::View,
        "menu-radio-group",
        "edit",
        &[],
        &[(property::PART_VALUE, "density")],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    for (id, value, label) in [
        (compact_id, "compact", "Compact"),
        (cozy_id, "cozy", "Cozy"),
    ] {
        declare_aligned_part(
            &mut tree,
            id,
            group_id,
            NodeTag::View,
            "menu-radio-item",
            "edit",
            &[],
            &[
                (property::PART_VALUE, value),
                (property::ACCESSIBILITY_LABEL, label),
            ],
            &[(property::COMPONENT_CHANGE_LISTENER, true)],
        );
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_menu_view(tree, Rc::clone(&events));
    let window = view.window_handle();

    let trigger = menu_trigger_id(ElementId::named("edit"));
    let popup = menu_popup_id(ElementId::named("edit"));
    assert!(!cx.contains_element(window, popup).unwrap());
    cx.click(window, trigger).unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change_field(&events, trigger_id, "open")["open"],
        serde_json::json!(true)
    );
    assert_eq!(cx.focused(window).unwrap(), Some(popup));

    // Opening from the trigger transfers the menu key context to the popup, so Arrow Down moves
    // the retained roving highlight rather than remaining on the trigger.
    cx.simulate_keystrokes(window, "down").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        cx.read(view, |view| view
            .components
            .menu_compound
            .menus
            .get(&ElementId::named("edit").as_u64())
            .and_then(|instance| instance.menu.active_index()))
            .unwrap(),
        Some(3)
    );

    // The declared rows become the core's own model, so each row mounts under the identity the
    // model derived and the popup points its active descendant at exactly that identity.
    let items = [
        PopoverMenuItem::group_label("Clipboard"),
        PopoverMenuItem::action(ElementId::named("copy"), "Copy", ()),
        PopoverMenuItem::separator(),
        PopoverMenuItem::checkbox(ElementId::named("wrap"), "Wrap lines", false, ()),
        PopoverMenuItem::radio(
            ElementId::named("compact"),
            "Compact",
            ElementId::named("density"),
            false,
            (),
        ),
        PopoverMenuItem::radio(
            ElementId::named("cozy"),
            "Cozy",
            ElementId::named("density"),
            false,
            (),
        ),
    ];
    let copy = menu_row_id("edit", &items, 1);
    let wrap = menu_row_id("edit", &items, 3);
    let compact = menu_row_id("edit", &items, 4);
    for element in [
        menu_trigger_id(ElementId::named("edit")),
        menu_popup_id(ElementId::named("edit")),
        copy,
        wrap,
        compact,
    ] {
        assert!(cx.contains_element(window, element).unwrap());
    }

    // A pointer highlight clears when that pointer leaves the row, so its styled state cannot
    // remain stuck after the cursor moves back to the trigger.
    let compact_center = element_center(&mut cx, window, compact);
    cx.visual(window)
        .unwrap()
        .move_pointer(compact_center)
        .unwrap();
    assert_eq!(
        cx.read(view, |view| view
            .components
            .menu_compound
            .menus
            .get(&ElementId::named("edit").as_u64())
            .and_then(|instance| instance.menu.active_index()))
            .unwrap(),
        Some(4)
    );
    let trigger_center = element_center(&mut cx, window, trigger);
    cx.visual(window)
        .unwrap()
        .move_pointer(trigger_center)
        .unwrap();
    assert_eq!(
        cx.read(view, |view| view
            .components
            .menu_compound
            .menus
            .get(&ElementId::named("edit").as_u64())
            .and_then(|instance| instance.menu.active_index()))
            .unwrap(),
        None
    );

    // A checkbox row toggles inside the core's model and stays open, exactly as Base UI does.
    cx.click(window, wrap).unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change_field(&events, wrap_id, "activated")["checked"],
        true
    );

    // A radio row's new value belongs to its group, exactly as Base UI reports `onValueChange`.
    cx.click(window, compact).unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change_field(&events, group_id, "value")["value"],
        serde_json::json!("compact")
    );

    // A command row closes the level after activating, which is the core's own default policy,
    // so every surface part unmounts and the trigger reports the value it committed.
    cx.click(window, copy).unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change_field(&events, copy_id, "activated")["activated"],
        serde_json::json!("copy")
    );
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| event.kind == "click" && event.target == copy_id)
    );
    let _ = view;
    assert!(
        !cx.contains_element(window, menu_popup_id(ElementId::named("edit")))
            .unwrap()
    );
    assert!(
        cx.contains_element(window, menu_trigger_id(ElementId::named("edit")))
            .unwrap()
    );
}

#[test]
fn a_declared_submenu_trigger_is_a_row_of_its_parent_and_the_trigger_of_its_own_level() {
    let root_id = 1_420;
    let trigger_id = 1_421;
    let positioner_id = 1_422;
    let popup_id = 1_423;
    let submenu_root_id = 1_424;
    let submenu_trigger_id = 1_425;
    let submenu_positioner_id = 1_426;
    let submenu_popup_id = 1_427;
    let nested_item_id = 1_428;
    let second_nested_item_id = 1_429;

    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "menu",
        "file",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        trigger_id,
        root_id,
        NodeTag::Button,
        "menu-trigger",
        "file",
        &[],
        &[],
        &[
            (property::OPEN, true),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    declare_aligned_part(
        &mut tree,
        positioner_id,
        root_id,
        NodeTag::View,
        "menu-positioner",
        "file",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        popup_id,
        positioner_id,
        NodeTag::View,
        "menu-popup",
        "file",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        submenu_root_id,
        popup_id,
        NodeTag::View,
        "menu-submenu-root",
        "recent",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        submenu_trigger_id,
        submenu_root_id,
        NodeTag::View,
        "menu-submenu-trigger",
        "recent",
        &[],
        &[
            (property::PART_VALUE, "recent"),
            (property::ACCESSIBILITY_LABEL, "Open recent"),
        ],
        &[
            (property::OPEN, true),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    declare_aligned_part(
        &mut tree,
        submenu_positioner_id,
        submenu_root_id,
        NodeTag::View,
        "menu-positioner",
        "recent",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        submenu_popup_id,
        submenu_positioner_id,
        NodeTag::View,
        "menu-popup",
        "recent",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        nested_item_id,
        submenu_popup_id,
        NodeTag::View,
        "menu-item",
        "recent",
        &[],
        &[
            (property::PART_VALUE, "notes"),
            (property::ACCESSIBILITY_LABEL, "notes.md"),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    declare_aligned_part(
        &mut tree,
        second_nested_item_id,
        submenu_popup_id,
        NodeTag::View,
        "menu-item",
        "recent",
        &[],
        &[
            (property::PART_VALUE, "readme"),
            (property::ACCESSIBILITY_LABEL, "README.md"),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_menu_view(tree, Rc::clone(&events));
    let window = view.window_handle();

    // The nested level anchors to the row identity its parent's model derived, so exactly one
    // element carries the submenu trigger and the parent's active descendant can reach it.
    let parent_items = [PopoverMenuItem::submenu(
        ElementId::named("recent"),
        "Open recent",
        PopoverMenu::new(Vec::new()).unwrap(),
    )];
    let row = menu_row_id("file", &parent_items, 0);
    assert!(cx.contains_element(window, row).unwrap());
    assert!(
        !cx.contains_element(window, menu_trigger_id(ElementId::named("recent")))
            .unwrap(),
        "a submenu trigger is a row of its parent, not a second standalone trigger"
    );

    // The nested level is its own compound: its popup and rows mount under its own scope.
    let nested_items = [
        PopoverMenuItem::action(ElementId::named("notes"), "notes.md", ()),
        PopoverMenuItem::action(ElementId::named("readme"), "README.md", ()),
    ];
    assert!(
        cx.contains_element(window, menu_popup_id(ElementId::named("recent")))
            .unwrap()
    );
    assert!(
        cx.contains_element(window, menu_row_id("recent", &nested_items, 0))
            .unwrap()
    );

    // Right transfers the key context into the submenu, where Down advances its own highlight.
    let parent_popup = menu_popup_id(ElementId::named("file"));
    let child_popup = menu_popup_id(ElementId::named("recent"));
    assert_eq!(cx.focused(window).unwrap(), Some(parent_popup));
    cx.simulate_keystrokes(window, "right").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(child_popup));
    let active_before_down = cx
        .read(view, |view| {
            view.components
                .menu_compound
                .menus
                .get(&ElementId::named("recent").as_u64())
                .and_then(|instance| instance.menu.active_index())
        })
        .unwrap();
    cx.simulate_keystrokes(window, "down").unwrap();
    cx.run_until_idle().unwrap();
    let active_after_down = cx
        .read(view, |view| {
            view.components
                .menu_compound
                .menus
                .get(&ElementId::named("recent").as_u64())
                .and_then(|instance| instance.menu.active_index())
        })
        .unwrap();
    assert_ne!(active_after_down, active_before_down);

    // Left closes only the child and returns to its trigger on the parent's key-context path;
    // Right can reopen it from there.
    cx.simulate_keystrokes(window, "left").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(row));
    assert!(!cx.contains_element(window, child_popup).unwrap());
    assert!(cx.contains_element(window, parent_popup).unwrap());
    cx.simulate_keystrokes(window, "right").unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(child_popup));

    // A command closes its whole open chain, not just the submenu it belongs to.
    let nested_row = menu_row_id("recent", &nested_items, 0);
    cx.click(window, nested_row).unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change_field(&events, nested_item_id, "activated")["activated"],
        serde_json::json!("notes")
    );
    assert_eq!(
        component_change_field(&events, submenu_trigger_id, "open")["open"],
        serde_json::json!(false)
    );
    assert_eq!(
        component_change_field(&events, trigger_id, "open")["open"],
        serde_json::json!(false)
    );
    assert!(
        !cx.contains_element(window, menu_popup_id(ElementId::named("recent")))
            .unwrap()
    );
    assert!(
        !cx.contains_element(window, menu_popup_id(ElementId::named("file")))
            .unwrap()
    );
}

#[test]
fn declared_menu_link_items_and_malformed_rows_decline_instead_of_panicking() {
    let root_id = 1_440;
    let trigger_id = 1_441;
    let positioner_id = 1_442;
    let popup_id = 1_443;
    let link_id = 1_444;
    let nameless_id = 1_445;
    let oversized_id = 1_446;

    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        root_id,
        ROOT_NODE,
        NodeTag::View,
        "menu",
        "help",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        trigger_id,
        root_id,
        NodeTag::Button,
        "menu-trigger",
        "help",
        &[],
        &[],
        &[(property::OPEN, true)],
    );
    declare_aligned_part(
        &mut tree,
        positioner_id,
        root_id,
        NodeTag::View,
        "menu-positioner",
        "help",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        popup_id,
        positioner_id,
        NodeTag::View,
        "menu-popup",
        "help",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        link_id,
        popup_id,
        NodeTag::View,
        "menu-link-item",
        "help",
        &[],
        &[
            (property::PART_VALUE, "docs"),
            (property::ACCESSIBILITY_LABEL, "Documentation"),
            (property::HREF, "https://example.invalid/docs"),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    // A row without a stable identity declares nothing rather than reaching a core constructor.
    declare_aligned_part(
        &mut tree,
        nameless_id,
        popup_id,
        NodeTag::View,
        "menu-item",
        "help",
        &[],
        &[(property::ACCESSIBILITY_LABEL, "Nameless")],
        &[],
    );
    // An oversized destination is refused before the core would truncate it.
    declare_aligned_part(
        &mut tree,
        oversized_id,
        popup_id,
        NodeTag::View,
        "menu-link-item",
        "help",
        &[],
        &[
            (property::PART_VALUE, "huge"),
            (property::ACCESSIBILITY_LABEL, "Huge"),
            (
                property::HREF,
                &"h".repeat(quickgui::MAX_POPOVER_MENU_LINK_BYTES + 1),
            ),
        ],
        &[],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_menu_view(tree, Rc::clone(&events));
    let window = view.window_handle();

    let items = [
        PopoverMenuItem::link(
            ElementId::named("docs"),
            "Documentation",
            "https://example.invalid/docs",
        ),
        PopoverMenuItem::link(ElementId::named("huge"), "Huge", ""),
    ];
    let link = menu_row_id("help", &items, 0);
    assert!(cx.contains_element(window, link).unwrap());
    // The nameless row contributed no model entry, so the oversized link is the second row.
    assert!(
        cx.contains_element(window, menu_row_id("help", &items, 1))
            .unwrap()
    );

    // Activation hands the destination to the core's own open-URL path, which `TestAppContext`
    // answers with `Unsupported`; the reported edge is enqueued before that request is made.
    let _ = cx.click(window, link);
    assert_eq!(
        component_change_field(&events, link_id, "href")["href"],
        serde_json::json!("https://example.invalid/docs")
    );
}

#[test]
fn a_declared_context_menu_accepts_the_same_child_item_parts() {
    let trigger_id = 1_460;
    let copy_id = 1_461;
    let paste_id = 1_462;

    let mut tree = NativeTree::default();
    declare_aligned_part(
        &mut tree,
        trigger_id,
        ROOT_NODE,
        NodeTag::View,
        "context-menu-trigger",
        "canvas",
        &[],
        &[],
        &[(property::SELECT_LISTENER, true)],
    );
    for (id, value, label) in [(copy_id, "copy", "Copy"), (paste_id, "paste", "Paste")] {
        declare_aligned_part(
            &mut tree,
            id,
            trigger_id,
            NodeTag::View,
            "menu-item",
            "canvas",
            &[],
            &[
                (property::PART_VALUE, value),
                (property::ACCESSIBILITY_LABEL, label),
            ],
            &[(property::CLICK_LISTENER, true)],
        );
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (cx, view) = mounted_menu_view(tree, Rc::clone(&events));
    let window = view.window_handle();

    // The core paints a context menu in its own cursor-point window, so the declared rows are a
    // model rather than owner-window elements: they mount nothing and the trigger stays.
    assert!(
        cx.contains_element(window, ElementId::new(u64::from(trigger_id)))
            .unwrap()
    );
    let items = [
        PopoverMenuItem::action(ElementId::named("copy"), "Copy", ()),
        PopoverMenuItem::action(ElementId::named("paste"), "Paste", ()),
    ];
    for index in 0..items.len() {
        assert!(
            !cx.contains_element(window, menu_row_id("canvas", &items, index))
                .unwrap(),
            "a context-menu row contributes a declaration, not an element"
        );
    }
}

#[test]
fn declared_select_parts_report_the_core_part_state_and_every_selected_value() {
    let select_id = 1_480;
    let label_id = 1_481;
    let value_id = 1_482;
    let icon_id = 1_483;
    let positioner_id = 1_484;
    let up_arrow_id = 1_485;
    let down_arrow_id = 1_486;
    let group_id = 1_487;
    let group_label_id = 1_488;
    let alpha_id = 1_489;
    let bravo_id = 1_490;

    let mut tree = NativeTree::default();
    let mut select = component_part_node(
        NodeTag::Button,
        ROOT_NODE,
        "select",
        &[
            (property::SCOPE, "fruit"),
            (property::ACCESSIBILITY_LABEL, "Fruit"),
            (property::VALUES, r#"["alpha","bravo"]"#),
        ],
        &[
            (property::MULTIPLE, true),
            (property::REQUIRED, true),
            (property::READ_ONLY, true),
            (property::MODAL, true),
            (property::ALIGN_ITEM_WITH_TRIGGER, true),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    select.set_property(
        property::OPTIONS,
        Some(PropertyValue::String(Arc::from(
            r#"{"alpha":"Alpha","bravo":"Bravo","charlie":"Charlie"}"#,
        ))),
    );
    insert_component_node(&mut tree, select_id, ROOT_NODE, select);
    for (id, part) in [
        (label_id, "select-label"),
        (value_id, "select-value"),
        (icon_id, "select-icon"),
    ] {
        declare_aligned_part(
            &mut tree,
            id,
            select_id,
            NodeTag::View,
            part,
            "fruit",
            &[],
            &[],
            &[],
        );
    }
    declare_aligned_part(
        &mut tree,
        positioner_id,
        select_id,
        NodeTag::View,
        "select-positioner",
        "fruit",
        &[(property::SIDE_OFFSET, 12.0)],
        &[(property::SIDE, "top")],
        &[],
    );
    for (id, part) in [
        (up_arrow_id, "select-scroll-up-arrow"),
        (down_arrow_id, "select-scroll-down-arrow"),
    ] {
        declare_aligned_part(
            &mut tree,
            id,
            positioner_id,
            NodeTag::View,
            part,
            "fruit",
            &[],
            &[],
            &[],
        );
    }
    declare_aligned_part(
        &mut tree,
        group_id,
        positioner_id,
        NodeTag::View,
        "select-group",
        "fruit",
        &[],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        group_label_id,
        group_id,
        NodeTag::View,
        "select-group-label",
        "fruit",
        &[],
        &[],
        &[],
    );
    for (id, value) in [(alpha_id, "alpha"), (bravo_id, "bravo")] {
        declare_aligned_part(
            &mut tree,
            id,
            group_id,
            NodeTag::View,
            "select-item",
            "fruit",
            &[],
            &[(property::PART_VALUE, value)],
            &[],
        );
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (cx, view) = mounted_menu_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let root = ElementId::named("fruit");

    // The owner-window parts mount under the identities the core derives from the scope alone.
    for element in [
        root,
        SelectState::<Arc<str>>::label_id(root),
        SelectState::<Arc<str>>::value_id(root),
        SelectState::<Arc<str>>::icon_id(root),
    ] {
        assert!(cx.contains_element(window, element).unwrap());
    }

    // The core paints the option list in its own native window, so every popup-side part is a
    // declaration and mounts nothing at all in the owner window.
    for element in [
        SelectState::<Arc<str>>::scroll_up_arrow_id(root),
        SelectState::<Arc<str>>::list_id(root),
        SelectState::<Arc<str>>::surface_id(root),
    ] {
        assert!(!cx.contains_element(window, element).unwrap());
    }

    // The `items` map form and the declared value set both reach the core, and the state it
    // publishes is exactly Base UI's own trigger snapshot.
    let reported = component_change(&events, select_id);
    assert_eq!(
        reported["selectedValues"],
        serde_json::json!(["alpha", "bravo"])
    );
    assert_eq!(reported["valueText"], serde_json::json!("Alpha, Bravo"));
    let state = &reported["state"];
    assert_eq!(state["required"], true);
    assert_eq!(state["readOnly"], true);
    assert_eq!(state["filled"], true);
    assert_eq!(state["placeholder"], false);
    // Seeding a declared value is not an interaction, so the core's own edges stay clean.
    assert_eq!(state["dirty"], false);
    assert_eq!(state["touched"], false);
    assert_eq!(state["popupOpen"], false);
    assert_eq!(state["popupSide"], serde_json::json!("top"));
}

#[test]
fn declared_combobox_parts_mount_chips_status_and_the_cores_empty_edge() {
    let combobox_id = 1_500;
    let label_id = 1_501;
    let group_wrapper_id = 1_502;
    let clear_id = 1_503;
    let status_id = 1_504;
    let empty_id = 1_505;
    let chips_id = 1_506;
    let chip_id = 1_507;
    let chip_remove_id = 1_508;

    let mut tree = NativeTree::default();
    let mut combobox = component_part_node(
        NodeTag::Input,
        ROOT_NODE,
        "combobox",
        &[
            (property::SCOPE, "tags"),
            (property::ACCESSIBILITY_LABEL, "Tags"),
            (property::VALUES, r#"["rust"]"#),
            (property::FILTER_MODE, "startsWith"),
        ],
        &[
            (property::MULTIPLE, true),
            (property::AUTO_HIGHLIGHT, true),
            (property::OPEN_ON_INPUT_CLICK, false),
            (property::HIGHLIGHT_ITEM_ON_HOVER, false),
            (property::LOOP_FOCUS, false),
            (property::REQUIRED, true),
            (property::COMPONENT_CHANGE_LISTENER, true),
        ],
    );
    combobox.set_property(
        property::OPTIONS,
        Some(PropertyValue::String(Arc::from(
            r#"[{"value":"rust","label":"Rust"},{"value":"zig","label":"Zig"}]"#,
        ))),
    );
    insert_component_node(&mut tree, combobox_id, ROOT_NODE, combobox);
    // A text input paints its own content, so every other part of the compound is a sibling of it
    // that repeats the scope; the Rust binding resolves the instance from that scope alone.
    for (id, part) in [
        (label_id, "combobox-label"),
        (group_wrapper_id, "combobox-input-group"),
        (clear_id, "combobox-clear"),
        (status_id, "combobox-status"),
        (empty_id, "combobox-empty"),
        (chips_id, "combobox-chips"),
    ] {
        declare_aligned_part(
            &mut tree,
            id,
            ROOT_NODE,
            NodeTag::View,
            part,
            "tags",
            &[],
            &[],
            &[],
        );
    }
    declare_aligned_part(
        &mut tree,
        chip_id,
        chips_id,
        NodeTag::View,
        "combobox-chip",
        "tags",
        &[(property::ITEM_INDEX, 0.0)],
        &[],
        &[],
    );
    declare_aligned_part(
        &mut tree,
        chip_remove_id,
        chip_id,
        NodeTag::Button,
        "combobox-chip-remove",
        "tags",
        &[(property::ITEM_INDEX, 0.0)],
        &[(property::ACCESSIBILITY_LABEL, "Remove Rust")],
        &[],
    );

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_menu_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let root = ElementId::named("tags");

    for element in [
        ComboboxState::<Arc<str>>::label_id(root),
        ComboboxState::<Arc<str>>::input_group_id(root),
        ComboboxState::<Arc<str>>::clear_id(root),
        ComboboxState::<Arc<str>>::status_id(root),
        ComboboxState::<Arc<str>>::chips_id(root),
        ComboboxState::<Arc<str>>::chip_id(root, 0),
        ComboboxState::<Arc<str>>::chip_remove_id(root, 0),
    ] {
        assert!(cx.contains_element(window, element).unwrap());
    }
    // `Combobox.Empty` mounts only while the core really matched nothing.
    assert!(
        !cx.contains_element(window, ComboboxState::<Arc<str>>::empty_id(root))
            .unwrap()
    );

    let reported = component_change(&events, combobox_id);
    assert_eq!(reported["chipValues"], serde_json::json!(["rust"]));
    assert_eq!(reported["chipLabels"], serde_json::json!(["Rust"]));
    assert_eq!(reported["state"]["required"], true);
    assert_eq!(reported["state"]["touched"], false);
    assert!(
        reported["state"]["status"]
            .as_str()
            .is_some_and(|status| !status.is_empty())
    );

    // Removing a chip is the core's decision; the binding reports the set it kept.
    cx.click(window, ComboboxState::<Arc<str>>::chip_remove_id(root, 0))
        .unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        component_change(&events, combobox_id)["chipValues"],
        serde_json::json!([])
    );
}

fn drag_captured_element(
    cx: &mut quickgui::TestAppContext,
    window: quickgui::WindowHandle,
    element: ElementId,
    from: quickgui::Point,
    to: quickgui::Point,
) {
    cx.simulate_pointer_drag(window, element, from, to).unwrap();
}

#[test]
fn a_declared_splitter_handle_follows_a_captured_pointer_drag() {
    let root_id = 440;
    let first_pane_id = 441;
    let handle_id = 442;
    let second_pane_id = 443;
    let mut tree = NativeTree::default();
    let mut root = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "splitter",
        &[
            (property::SCOPE, "shrunk"),
            (property::VALUES, "[180,180]"),
            (property::ITEMS, r#"[{"min":20},{"min":20}]"#),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    // Pane extents are framework-owned structural geometry (`flex_none`), so the root is laid
    // out at exactly the declared total plus the handle.
    root.set_property(property::WIDTH, Some(PropertyValue::Number(366.0)));
    root.set_property(property::HEIGHT, Some(PropertyValue::Number(60.0)));
    insert_component_node(&mut tree, root_id, ROOT_NODE, root);
    for (id, index) in [(first_pane_id, 0.0), (second_pane_id, 1.0)] {
        let mut pane = component_part_node(
            NodeTag::View,
            root_id,
            "splitter-pane",
            &[(property::SCOPE, "shrunk")],
            &[],
        );
        pane.set_property(property::ITEM_INDEX, Some(PropertyValue::Number(index)));
        insert_component_node(&mut tree, id, root_id, pane);
    }
    let mut handle = component_part_node(
        NodeTag::View,
        root_id,
        "splitter-handle",
        &[(property::SCOPE, "shrunk")],
        &[],
    );
    handle.set_property(property::ITEM_INDEX, Some(PropertyValue::Number(0.0)));
    handle.set_property(property::WIDTH, Some(PropertyValue::Number(6.0)));
    insert_component_node(&mut tree, handle_id, root_id, handle);
    // Declaration order is pane, handle, pane.
    let root_children = &mut tree.nodes.get_mut(&root_id).unwrap().children;
    root_children.clear();
    root_children.extend([first_pane_id, handle_id, second_pane_id]);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(13, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        component_application(),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();
    cx.run_until_idle().unwrap();

    let splitter = Splitter::new(
        ElementId::named("shrunk"),
        &SplitterState::new(SplitterOrientation::Horizontal, &[180.0, 180.0]),
    );
    let handle_bounds = cx.element_bounds(window, splitter.handle_id(0)).unwrap();
    assert!(
        (handle_bounds.x - 180.0).abs() < 0.5,
        "handle sat at {handle_bounds:?}"
    );

    // Dragging the handle 20 pixels moves it exactly 20 pixels.
    let from = quickgui::Point::new(handle_bounds.x + 3.0, handle_bounds.y + 30.0);
    let to = quickgui::Point::new(from.x + 20.0, from.y);
    drag_captured_element(&mut cx, window, splitter.handle_id(0), from, to);
    let dragged = component_change(&events, root_id);
    let sizes = dragged["sizes"].as_array().expect("sizes reported").clone();
    assert!(
        (sizes[0].as_f64().unwrap() - 200.0).abs() < 0.5,
        "sizes were {sizes:?}"
    );
    assert!(
        (sizes[1].as_f64().unwrap() - 160.0).abs() < 0.5,
        "sizes were {sizes:?}"
    );
    let handle_bounds = cx.element_bounds(window, splitter.handle_id(0)).unwrap();
    assert!(
        (handle_bounds.x - 200.0).abs() < 0.5,
        "handle moved to {handle_bounds:?}"
    );
}

#[test]
fn a_single_declared_splitter_pane_resizes_inside_an_ordinary_flex_row() {
    let mut tree = NativeTree::default();
    let mut outer = NativeNode::new(NodeTag::View);
    outer.parent = Some(ROOT_NODE);
    outer.set_property(
        property::DISPLAY,
        Some(PropertyValue::String("flex".into())),
    );
    outer.set_property(property::WIDTH, Some(PropertyValue::Number(900.0)));
    outer.set_property(property::HEIGHT, Some(PropertyValue::Number(100.0)));
    insert_component_node(&mut tree, 440, ROOT_NODE, outer);

    let mut root = component_part_node(
        NodeTag::View,
        440,
        "splitter",
        &[
            (property::SCOPE, "single-pane"),
            (property::VALUES, "[300,350]"),
            (property::ITEMS, r#"[{"min":220},{}]"#),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    root.set_property(property::WIDTH, None);
    root.set_property(property::HEIGHT, None);
    root.set_property(property::FLEX_SHRINK, Some(PropertyValue::Number(0.0)));
    insert_component_node(&mut tree, 441, 440, root);
    let mut pane = component_part_node(
        NodeTag::View,
        441,
        "splitter-pane",
        &[(property::SCOPE, "single-pane")],
        &[],
    );
    pane.set_property(property::WIDTH, None);
    pane.set_property(property::HEIGHT, None);
    pane.set_property(property::FLEX_GROW, Some(PropertyValue::Number(1.0)));
    pane.set_property(property::FLEX_BASIS, Some(PropertyValue::Number(0.0)));
    pane.set_property(
        property::BACKGROUND_COLOR,
        Some(PropertyValue::Color(0xff0000c8)),
    );
    pane.set_property(property::ITEM_INDEX, Some(PropertyValue::Number(0.0)));
    insert_component_node(&mut tree, 442, 441, pane);
    // A real sidebar/history qualifies for retained subtree paint caching.
    for id in 1000..1040 {
        let mut child = NativeNode::new(NodeTag::View);
        child.parent = Some(442);
        child.set_property(property::WIDTH, Some(PropertyValue::Number(1.0)));
        child.set_property(property::HEIGHT, Some(PropertyValue::Number(1.0)));
        insert_component_node(&mut tree, id, 442, child);
    }
    let mut handle = component_part_node(
        NodeTag::View,
        441,
        "splitter-handle",
        &[(property::SCOPE, "single-pane")],
        &[],
    );
    handle.set_property(property::WIDTH, Some(PropertyValue::Number(1.0)));
    handle.set_property(property::HEIGHT, None);
    handle.set_property(property::ITEM_INDEX, Some(PropertyValue::Number(0.0)));
    insert_component_node(&mut tree, 443, 441, handle);
    let mut content = NativeNode::new(NodeTag::View);
    content.parent = Some(440);
    content.set_property(property::FLEX_GROW, Some(PropertyValue::Number(1.0)));
    content.set_property(property::FLEX_BASIS, Some(PropertyValue::Number(0.0)));
    content.set_property(
        property::BACKGROUND_COLOR,
        Some(PropertyValue::Color(0xffc80000)),
    );
    insert_component_node(&mut tree, 444, 440, content);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(13, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        component_application(),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();
    cx.run_until_idle().unwrap();
    let state = SplitterState::new(SplitterOrientation::Horizontal, &[300.0, 350.0]);
    let splitter = Splitter::new("single-pane", &state);
    let bounds = cx.element_bounds(window, splitter.handle_id(0)).unwrap();
    assert!((bounds.x - 300.0).abs() < 0.5, "initial {bounds:?}");
    cx.focus(window, splitter.handle_id(0)).unwrap();
    cx.simulate_keystrokes(window, "right right").unwrap();
    cx.run_until_idle().unwrap();
    let keyboard_bounds = cx.element_bounds(window, splitter.handle_id(0)).unwrap();
    assert!(
        (keyboard_bounds.x - 332.0).abs() < 0.5,
        "keyboard {keyboard_bounds:?}"
    );
    #[cfg(target_os = "macos")]
    assert_eq!(
        cx.capture_screenshot(window).unwrap().pixel(640, 100),
        Some([200, 0, 0, 255])
    );
    cx.simulate_keystrokes(window, "left left").unwrap();
    cx.run_until_idle().unwrap();
    #[cfg(target_os = "macos")]
    assert_eq!(
        cx.capture_screenshot(window).unwrap().pixel(640, 100),
        Some([0, 0, 200, 255])
    );
    let from = quickgui::Point::new(bounds.x + 0.5, bounds.y + 30.0);
    let to = quickgui::Point::new(from.x + 40.0, from.y);
    drag_captured_element(&mut cx, window, splitter.handle_id(0), from, to);
    let changed = component_change(&events, 441);
    assert_eq!(changed["sizes"], serde_json::json!([340.0, 310.0]));
    let bounds = cx.element_bounds(window, splitter.handle_id(0)).unwrap();
    assert!((bounds.x - 340.0).abs() < 0.5, "resized {bounds:?}");
    #[cfg(target_os = "macos")]
    assert_eq!(
        cx.capture_screenshot(window).unwrap().pixel(640, 100),
        Some([200, 0, 0, 255])
    );
}

#[test]
fn a_declared_scroll_area_measures_its_geometry_and_the_thumb_follows_a_drag() {
    let root_id = 450;
    let viewport_id = 451;
    let content_id = 452;
    let scrollbar_id = 453;
    let thumb_id = 454;
    let mut tree = NativeTree::default();
    let mut root = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "scroll-area",
        &[(property::SCOPE, "measured")],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    root.set_property(property::WIDTH, Some(PropertyValue::Number(206.0)));
    root.set_property(property::HEIGHT, Some(PropertyValue::Number(120.0)));
    insert_component_node(&mut tree, root_id, ROOT_NODE, root);
    let mut viewport = component_part_node(
        NodeTag::View,
        root_id,
        "scroll-area-viewport",
        &[(property::SCOPE, "measured")],
        &[],
    );
    viewport.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
    viewport.set_property(property::HEIGHT, Some(PropertyValue::Number(120.0)));
    insert_component_node(&mut tree, viewport_id, root_id, viewport);
    let mut content = component_part_node(
        NodeTag::View,
        viewport_id,
        "scroll-area-content",
        &[(property::SCOPE, "measured")],
        &[],
    );
    content.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
    content.set_property(property::HEIGHT, Some(PropertyValue::Number(600.0)));
    insert_component_node(&mut tree, content_id, viewport_id, content);
    let mut scrollbar = component_part_node(
        NodeTag::View,
        root_id,
        "scroll-area-scrollbar",
        &[
            (property::SCOPE, "measured"),
            (property::ORIENTATION, "vertical"),
        ],
        &[],
    );
    scrollbar.set_property(property::WIDTH, Some(PropertyValue::Number(6.0)));
    scrollbar.set_property(property::HEIGHT, Some(PropertyValue::Number(120.0)));
    insert_component_node(&mut tree, scrollbar_id, root_id, scrollbar);
    let mut thumb = component_part_node(
        NodeTag::View,
        scrollbar_id,
        "scroll-area-thumb",
        &[
            (property::SCOPE, "measured"),
            (property::ORIENTATION, "vertical"),
        ],
        &[],
    );
    thumb.set_property(property::WIDTH, Some(PropertyValue::Number(6.0)));
    insert_component_node(&mut tree, thumb_id, scrollbar_id, thumb);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_base_ui_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    cx.run_until_idle().unwrap();
    let area = quickgui::ScrollArea::new("measured");
    let vertical = quickgui::ScrollAreaOrientation::Vertical;

    // Nothing was declared: the viewport and content extents come from the painted bounds.
    let measured = component_change(&events, root_id);
    assert_eq!(measured["hasOverflowY"], serde_json::json!(true));
    let scrollbar_bounds = cx
        .element_bounds(window, area.scrollbar_id(vertical))
        .unwrap();
    let thumb_bounds = cx.element_bounds(window, area.thumb_id(vertical)).unwrap();
    // 120 of 600 is visible, so the thumb spans a fifth of the 120-pixel track from its top.
    assert!(
        (thumb_bounds.height - 24.0).abs() < 0.5,
        "thumb was {thumb_bounds:?}"
    );
    assert!(
        (thumb_bounds.y - scrollbar_bounds.y).abs() < 0.5,
        "thumb was {thumb_bounds:?}"
    );

    // Dragging the thumb 48 pixels moves it 48 pixels and scrolls half the overflow.
    let from = quickgui::Point::new(thumb_bounds.x + 3.0, thumb_bounds.y + 12.0);
    let to = quickgui::Point::new(from.x, from.y + 48.0);
    drag_captured_element(&mut cx, window, area.thumb_id(vertical), from, to);
    let scrolled = component_change(&events, root_id);
    let offset = scrolled["offset"]["y"].as_f64().unwrap();
    assert!((offset - 240.0).abs() < 1.0, "offset was {offset}");
    let dragged = cx.element_bounds(window, area.thumb_id(vertical)).unwrap();
    assert!(
        (dragged.y - (thumb_bounds.y + 48.0)).abs() < 0.5,
        "thumb moved to {dragged:?}"
    );
}

#[test]
fn a_declared_table_cell_with_text_children_builds_a_consistent_accessibility_tree() {
    let table_id = 640;
    let row_id = 642;
    let mut tree = NativeTree::default();
    let mut table = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "table",
        &[
            (property::SCOPE, "files"),
            (
                property::COLUMNS,
                r#"[{"id":"name","label":"Name","width":160},{"id":"size","label":"Size","track":"1fr"}]"#,
            ),
        ],
        &[],
    );
    table.set_property(property::ROW_COUNT, Some(PropertyValue::Number(2.0)));
    table.set_property(property::ROW_HEIGHT, Some(PropertyValue::Number(24.0)));
    table.set_property(property::WIDTH, Some(PropertyValue::Number(400.0)));
    table.set_property(property::HEIGHT, Some(PropertyValue::Number(200.0)));
    insert_component_node(&mut tree, table_id, ROOT_NODE, table);
    let mut row = component_part_node(NodeTag::View, table_id, "table-row", &[], &[]);
    row.set_property(property::ROW_INDEX, Some(PropertyValue::Number(0.0)));
    insert_component_node(&mut tree, row_id, table_id, row);
    for (cell_id, text_id, column, value) in
        [(643, 644, "name", "notes.txt"), (645, 646, "size", "12 KB")]
    {
        let cell = component_part_node(
            NodeTag::View,
            row_id,
            "table-cell",
            &[(property::PART_VALUE, column)],
            &[],
        );
        insert_component_node(&mut tree, cell_id, row_id, cell);
        let mut text = NativeNode::new(NodeTag::Text);
        text.parent = Some(cell_id);
        text.text = Arc::from(value);
        insert_component_node(&mut tree, text_id, cell_id, text);
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    cx.run_until_idle().unwrap();

    // Every child a node lists must be a node of the same update: an assistive client aborts on
    // a dangling child, which is how a table cell with text used to take the process down.
    let update = cx.accessibility_update(window).unwrap();
    let emitted = update
        .nodes
        .iter()
        .map(|(id, _)| *id)
        .collect::<std::collections::HashSet<_>>();
    let orphans = update
        .nodes
        .iter()
        .flat_map(|(_, node)| node.children().iter().copied())
        .filter(|child| !emitted.contains(child))
        .map(|child| child.0)
        .collect::<Vec<_>>();
    assert_eq!(orphans, Vec::<u64>::new());
    // The declared text reaches the client under the cells the core mounted it in.
    let root = ElementId::named("files");
    for (text_id, column) in [(644_u64, 0), (646, 1)] {
        let cell = TableState::cell_id(root, TableCellPosition { row: 0, column });
        let listed = update
            .nodes
            .iter()
            .find(|(id, _)| id.0 == cell.as_u64())
            .map(|(_, node)| node.children().iter().map(|id| id.0).collect::<Vec<_>>());
        assert_eq!(listed, Some(vec![text_id]));
        assert!(emitted.iter().any(|id| id.0 == text_id));
    }
}

#[test]
fn a_declared_percentage_width_sizes_against_the_parent() {
    let parent_id = 660;
    let child_id = 661;
    let mut tree = NativeTree::default();
    let mut parent = NativeNode::new(NodeTag::View);
    parent.parent = Some(ROOT_NODE);
    parent.set_property(property::WIDTH, Some(PropertyValue::Number(400.0)));
    parent.set_property(property::HEIGHT, Some(PropertyValue::Number(40.0)));
    insert_component_node(&mut tree, parent_id, ROOT_NODE, parent);
    let mut child = NativeNode::new(NodeTag::View);
    child.parent = Some(parent_id);
    child.set_property(
        property::WIDTH,
        Some(PropertyValue::String(Arc::from("62%"))),
    );
    child.set_property(
        property::HEIGHT,
        Some(PropertyValue::String(Arc::from("50%"))),
    );
    insert_component_node(&mut tree, child_id, parent_id, child);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    cx.run_until_idle().unwrap();

    // A CSS percentage resolves against the parent's box, exactly as `100%` always did.
    let bounds = cx
        .element_bounds(window, ElementId::new(child_id as u64))
        .unwrap();
    assert!((bounds.width - 248.0).abs() < 0.5, "{bounds:?}");
    assert!((bounds.height - 20.0).abs() < 0.5, "{bounds:?}");
}

#[test]
fn a_dialog_declared_inside_a_panel_covers_the_window() {
    let panel_id = 670;
    let portal_id = 671;
    let backdrop_id = 672;
    let popup_id = 673;
    let dialog = [(property::SCOPE, "confirm")];
    let mut tree = NativeTree::default();
    let mut panel = NativeNode::new(NodeTag::View);
    panel.parent = Some(ROOT_NODE);
    panel.set_property(property::WIDTH, Some(PropertyValue::Number(200.0)));
    panel.set_property(property::HEIGHT, Some(PropertyValue::Number(100.0)));
    insert_component_node(&mut tree, panel_id, ROOT_NODE, panel);
    let portal = component_part_node(
        NodeTag::View,
        panel_id,
        "dialog",
        &dialog,
        &[(property::OPEN, true)],
    );
    insert_component_node(&mut tree, portal_id, panel_id, portal);
    let mut backdrop = component_part_node(
        NodeTag::View,
        portal_id,
        "dialog-backdrop",
        &dialog,
        &[(property::OPEN, true)],
    );
    backdrop.set_property(
        property::WIDTH,
        Some(PropertyValue::String(Arc::from("100%"))),
    );
    backdrop.set_property(
        property::HEIGHT,
        Some(PropertyValue::String(Arc::from("100%"))),
    );
    insert_component_node(&mut tree, backdrop_id, portal_id, backdrop);
    let mut popup = component_part_node(
        NodeTag::View,
        portal_id,
        "dialog-popup",
        &dialog,
        &[(property::OPEN, true)],
    );
    popup.set_property(property::WIDTH, Some(PropertyValue::Number(120.0)));
    popup.set_property(property::HEIGHT, Some(PropertyValue::Number(60.0)));
    insert_component_node(&mut tree, popup_id, portal_id, popup);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    cx.run_until_idle().unwrap();

    // The portal is a viewport overlay, so it mounts under the window root and its insets and
    // percentage sizes resolve against the window rather than the 200x100 panel that declared it.
    let core = quickgui::Dialog::new("confirm", true);
    let window_bounds = cx
        .element_bounds(window, ElementId::new(crate::ROOT_ELEMENT_ID))
        .unwrap();
    assert!(
        window_bounds.width > 200.0 && window_bounds.height > 100.0,
        "{window_bounds:?}"
    );
    assert_eq!(
        cx.element_bounds(window, core.root_id()).unwrap(),
        window_bounds
    );
    assert_eq!(
        cx.element_bounds(window, core.backdrop_id()).unwrap(),
        window_bounds
    );
    let popup = cx.element_bounds(window, core.popover_id()).unwrap();
    assert!(
        (popup.width - 120.0).abs() < 0.5 && (popup.height - 60.0).abs() < 0.5,
        "{popup:?}"
    );
}

#[test]
fn a_context_menu_declared_with_item_parts_and_no_items_opens_the_core_surface() {
    let target_id = 330;
    let rename_id = 331;
    let delete_id = 332;
    let mut tree = NativeTree::default();
    // A binding always declares the menu appearance, so the JSON model can arrive with no rows of
    // its own; the Base UI-shaped child parts are the rows then.
    tree.nodes.insert(
        target_id,
        component_part_node(
            NodeTag::View,
            ROOT_NODE,
            CONTEXT_MENU_TRIGGER_PART,
            &[(property::MENU, r#"{"items":[]}"#)],
            &[(property::SELECT_LISTENER, true)],
        ),
    );
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(target_id);
    for (id, value, label) in [
        (rename_id, "rename", "Rename"),
        (delete_id, "delete", "Delete"),
    ] {
        declare_aligned_part(
            &mut tree,
            id,
            target_id,
            NodeTag::View,
            "menu-item",
            "files",
            &[],
            &[
                (property::PART_VALUE, value),
                (property::ACCESSIBILITY_LABEL, label),
            ],
            &[(property::CLICK_LISTENER, true)],
        );
    }

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(12, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        quickgui::Application::new().bind_keys(quickgui::popover_menu_key_bindings()),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();
    let target = ElementId::new(target_id as u64);
    assert!(cx.contains_element(window, target).unwrap());
    cx.simulate_context_menu(
        window,
        target,
        quickgui::Point::new(40.0, 24.0),
        quickgui::Modifiers::empty(),
    )
    .unwrap();
    assert_eq!(
        cx.windows().len(),
        2,
        "the child item parts open the core's cursor-point surface"
    );
}

#[test]
fn a_date_segment_declared_without_children_shows_the_core_text() {
    let date_id = 700;
    let month_id = 701;
    let mut tree = NativeTree::default();
    let date = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "date-field",
        &[
            (property::SCOPE, "due"),
            (property::CIVIL_VALUE, "2026-09-03"),
            (property::SEGMENT_ORDER, "mdy"),
        ],
        &[],
    );
    insert_component_node(&mut tree, date_id, ROOT_NODE, date);
    let month = component_part_node(
        NodeTag::View,
        date_id,
        "date-field-segment",
        &[(property::SCOPE, "due"), (property::SEGMENT, "month")],
        &[],
    );
    insert_component_node(&mut tree, month_id, date_id, month);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    cx.run_until_idle().unwrap();

    // The core owns the digits, so an empty segment declaration still reads "09" on screen and
    // to an assistive client, exactly as the docs describe.
    let update = cx.accessibility_update(window).unwrap();
    assert!(
        update
            .nodes
            .iter()
            .any(|(_, node)| node.label() == Some("09") || node.value() == Some("09")),
        "{:?}",
        update
            .nodes
            .iter()
            .map(|(_, node)| (node.role(), node.label().map(str::to_owned)))
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_declared_button_pressed_with_the_mouse_focuses_without_visible_focus_styles() {
    let button_id = 690;
    let mut tree = NativeTree::default();
    let mut button = NativeNode::new(NodeTag::Button);
    button.parent = Some(ROOT_NODE);
    button.set_property(property::WIDTH, Some(PropertyValue::Number(120.0)));
    button.set_property(property::HEIGHT, Some(PropertyValue::Number(32.0)));
    button.set_property(property::CLICK_LISTENER, Some(PropertyValue::Bool(true)));
    tree.nodes.insert(button_id, button);
    tree.nodes
        .get_mut(&ROOT_NODE)
        .unwrap()
        .children
        .push(button_id);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    let button = ElementId::new(button_id as u64);
    cx.run_until_idle().unwrap();

    // The hosted binding has no pointer path of its own: the press reaches the core's window-level
    // pointer dispatch, which focuses the button the way a native click does, so focus lands but
    // its styles stay hidden until the keyboard is used, like CSS `:focus-visible`.
    cx.simulate_mouse_down(
        window,
        button,
        quickgui::MouseDownEvent {
            button: quickgui::MouseButton::Left,
            position: quickgui::Point::new(10.0, 10.0),
            modifiers: quickgui::Modifiers::empty(),
            click_count: 1,
            first_mouse: false,
        },
    )
    .unwrap();
    cx.simulate_mouse_up(
        window,
        button,
        quickgui::MouseUpEvent {
            button: quickgui::MouseButton::Left,
            position: quickgui::Point::new(10.0, 10.0),
            modifiers: quickgui::Modifiers::empty(),
            click_count: 1,
        },
    )
    .unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(button));
    assert!(!cx.focus_visible(window).unwrap());

    // Tab is keyboard input, so the focus it lands — here the same button — shows its styles.
    cx.simulate_keystrokes(window, "tab").unwrap();
    assert_eq!(cx.focused(window).unwrap(), Some(button));
    assert!(cx.focus_visible(window).unwrap());
}

#[test]
fn declared_table_keeps_supplied_rows_during_delayed_scroll_range_updates() {
    const TABLE: u32 = 600;
    fn supply(tree: &mut NativeTree, range: std::ops::Range<usize>) {
        let mut tx = TreeTransaction::new(tree);
        for &id in &tree.nodes[&TABLE].children {
            let index = tree.nodes[&id].number(property::ROW_INDEX).unwrap() as usize;
            if !range.contains(&index) {
                tx.remove(TABLE, id).unwrap();
            }
        }
        for index in range {
            let row = 10_000 + index as u32 * 3;
            if tree.nodes.contains_key(&row) {
                continue;
            }
            tx.create(row, NodeTag::View, Arc::from("")).unwrap();
            tx.set_property(
                row,
                property::PART,
                Some(PropertyValue::String(Arc::from("table-row"))),
            )
            .unwrap();
            tx.set_property(
                row,
                property::ROW_INDEX,
                Some(PropertyValue::Number(index as f32)),
            )
            .unwrap();
            tx.insert(TABLE, row, None).unwrap();
            tx.create(row + 1, NodeTag::View, Arc::from("")).unwrap();
            tx.set_property(
                row + 1,
                property::PART,
                Some(PropertyValue::String(Arc::from("table-cell"))),
            )
            .unwrap();
            tx.set_property(
                row + 1,
                property::PART_VALUE,
                Some(PropertyValue::String(Arc::from("name"))),
            )
            .unwrap();
            tx.insert(row, row + 1, None).unwrap();
            tx.create(row + 2, NodeTag::Text, Arc::from(format!("Row {index}")))
                .unwrap();
            tx.insert(row + 1, row + 2, None).unwrap();
        }
        let overlay = tx.finish().unwrap();
        commit_overlay(tree, overlay);
    }

    let mut tree = NativeTree::default();
    let mut table = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "table",
        &[
            (property::SCOPE, "async-table"),
            (
                property::COLUMNS,
                r#"[{"id":"name","label":"Name","track":"1fr"}]"#,
            ),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    for (property, value) in [
        (property::ROW_COUNT, 100_000.0),
        (property::ROW_HEIGHT, 20.0),
        (property::HEADER_HEIGHT, 0.0),
        (property::WIDTH, 200.0),
        (property::HEIGHT, 200.0),
    ] {
        table.set_property(property, Some(PropertyValue::Number(value)));
    }
    insert_component_node(&mut tree, TABLE, ROOT_NODE, table);
    supply(&mut tree, 0..20);
    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, events.clone());
    let window = view.window_handle();
    let root = ElementId::named("async-table");
    let bounds = cx.element_bounds(window, root).unwrap();
    // Finish the initial range response after layout has measured the formerly unsized body.
    cx.update(view, |_view, cx| cx.invalidate()).unwrap();
    cx.element_bounds(window, root).unwrap();
    let row = |index| TableState::row_id(root, index);
    merged_component_change(&events, TABLE);
    let renders = cx.render_count(window).unwrap();
    assert!(
        cx.simulate_retained_scroll(window, root, quickgui::Vector::new(0.0, -80.0))
            .unwrap()
    );
    assert_eq!(
        cx.render_count(window).unwrap(),
        renders,
        "small scrolling reuses supplied rows"
    );
    assert!(
        cx.simulate_retained_scroll(window, root, quickgui::Vector::new(0.0, -40.0))
            .unwrap()
    );
    let refill = merged_component_change(&events, TABLE);
    assert_eq!(
        refill["visibleRange"],
        serde_json::json!({"start": 0, "end": 26})
    );
    assert_eq!(cx.element_bounds(window, row(6)).unwrap().y, bounds.y);

    // Do not deliver the requested rows yet. A large scroll still paints the previous supplied
    // window, instead of rebuilding it as empty cells at the destination.
    assert!(
        cx.simulate_retained_scroll(window, root, quickgui::Vector::new(0.0, -5_080.0))
            .unwrap()
    );
    let requested = merged_component_change(&events, TABLE);
    assert_eq!(
        requested["visibleRange"],
        serde_json::json!({"start": 250, "end": 280})
    );
    assert!(cx.contains_element(window, row(10)).unwrap());
    assert!(!cx.contains_element(window, row(260)).unwrap());
    assert_eq!(cx.element_bounds(window, row(10)).unwrap().y, bounds.y);
    assert_eq!(
        cx.element_bounds(window, row(19)).unwrap().bottom(),
        bounds.bottom()
    );
    let renders = cx.render_count(window).unwrap();
    cx.run_until_idle().unwrap();
    assert_eq!(
        cx.render_count(window).unwrap(),
        renders,
        "waiting for data does not poll"
    );
    assert!(merged_component_change(&events, TABLE).is_empty());

    cx.update(view, |view, cx| {
        supply(&mut view.tree.borrow_mut(), 250..280);
        cx.invalidate();
    })
    .unwrap();
    assert_eq!(cx.element_bounds(window, row(260)).unwrap().y, bounds.y);
    assert_eq!(
        cx.element_bounds(window, row(269)).unwrap().bottom(),
        bounds.bottom()
    );
    assert!(!cx.contains_element(window, row(10)).unwrap());
    let update = cx.accessibility_update(window).unwrap();
    assert!(
        update
            .nodes
            .iter()
            .any(|(id, node)| id.0 == 10_000 + 260 * 3 + 2 && node.label() == Some("Row 260"))
    );
    assert!(
        cx.read(view, |view| view.tree.borrow().nodes.len())
            .unwrap()
            < 100
    );
}

#[test]
fn a_declared_table_scrollbar_track_press_and_drag_scroll_the_body() {
    let table_id = 660;
    let header_id = 661;
    let row_id = 662;
    let cell_id = 663;
    let mut tree = NativeTree::default();
    // The gallery declares its table as an overflow-scroll view with a border around the core's
    // own virtual body, so the wrapper is itself a scroll container enclosing the body's track.
    let mut table = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "table",
        &[
            (property::SCOPE, "files"),
            (
                property::COLUMNS,
                r#"[{"id":"name","label":"Name","width":160},{"id":"size","label":"Size","track":"1fr"}]"#,
            ),
            (property::OVERFLOW_Y, "scroll"),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    table.set_property(property::ROW_COUNT, Some(PropertyValue::Number(200.0)));
    table.set_property(property::ROW_HEIGHT, Some(PropertyValue::Number(24.0)));
    table.set_property(property::HEADER_HEIGHT, Some(PropertyValue::Number(24.0)));
    table.set_property(property::WIDTH, Some(PropertyValue::Number(400.0)));
    table.set_property(property::HEIGHT, Some(PropertyValue::Number(240.0)));
    table.set_property(property::BORDER_WIDTH, Some(PropertyValue::Number(1.0)));
    insert_component_node(&mut tree, table_id, ROOT_NODE, table);
    let header = component_part_node(
        NodeTag::View,
        table_id,
        "table-header",
        &[(property::PART_VALUE, "name")],
        &[],
    );
    insert_component_node(&mut tree, header_id, table_id, header);
    let mut row = component_part_node(NodeTag::View, table_id, "table-row", &[], &[]);
    row.set_property(property::ROW_INDEX, Some(PropertyValue::Number(0.0)));
    insert_component_node(&mut tree, row_id, table_id, row);
    let cell = component_part_node(
        NodeTag::View,
        row_id,
        "table-cell",
        &[(property::PART_VALUE, "name")],
        &[],
    );
    insert_component_node(&mut tree, cell_id, row_id, cell);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let (mut cx, view) = mounted_component_view(tree, Rc::clone(&events));
    let window = view.window_handle();
    cx.run_until_idle().unwrap();
    let mounted = merged_component_change(&events, table_id);
    assert_eq!(mounted["visibleRange"]["start"], serde_json::json!(0));

    // The body fills the core root below the header, and its overlay track is the rightmost
    // twelve points of the body. Pressing the track away from the thumb centers the thumb there.
    let root = ElementId::named("files");
    let bounds = cx.element_bounds(window, root).unwrap();
    let track_x = bounds.right() - 6.0;
    let body_top = bounds.y + 24.0;
    let press = quickgui::Point::new(track_x, body_top + 60.0);
    assert!(
        cx.simulate_scrollbar_press(window, press).unwrap(),
        "the press at {press:?} lands on the body's scrollbar track inside {bounds:?}"
    );
    assert!(
        cx.scrollbar_drag_active(window).unwrap(),
        "the table rebuilt for the new range keeps the thumb drag"
    );
    let pressed = merged_component_change(&events, table_id);
    let start_after_press = pressed["visibleRange"]["start"]
        .as_u64()
        .expect("the track press scrolled the body and reported the new range");
    assert!(start_after_press > 0, "reported {pressed:?}");

    // Dragging the thumb further down scrolls further, through every rebuilt frame in between.
    let dragged_to = quickgui::Point::new(track_x, press.y + 40.0);
    assert!(cx.simulate_scrollbar_drag_to(window, dragged_to).unwrap());
    assert!(cx.scrollbar_drag_active(window).unwrap());
    let dragged = merged_component_change(&events, table_id);
    let start_after_drag = dragged["visibleRange"]["start"]
        .as_u64()
        .expect("the drag scrolled the body and reported the new range");
    assert!(
        start_after_drag > start_after_press,
        "the drag moved from {start_after_press} to {start_after_drag}"
    );

    assert!(cx.simulate_scrollbar_release(window).unwrap());
    assert!(!cx.scrollbar_drag_active(window).unwrap());
    // What JavaScript was told is exactly the range the core retained.
    let visible = cx
        .read(view, |view| {
            let table = view.components.tables.values().next().unwrap();
            table.state.visible_rows()
        })
        .unwrap();
    assert_eq!(visible.start as u64, start_after_drag);
}

#[test]
fn a_controlled_splitter_keeps_the_handle_under_the_pointer_while_declarations_lag() {
    let root_id = 470;
    let first_pane_id = 471;
    let handle_id = 472;
    let second_pane_id = 473;
    let mut tree = NativeTree::default();
    let mut root = component_part_node(
        NodeTag::View,
        ROOT_NODE,
        "splitter",
        &[
            (property::SCOPE, "lagging"),
            (property::VALUES, "[180,180]"),
            (property::ITEMS, r#"[{"min":20},{"min":20}]"#),
        ],
        &[(property::COMPONENT_CHANGE_LISTENER, true)],
    );
    root.set_property(property::WIDTH, Some(PropertyValue::Number(366.0)));
    root.set_property(property::HEIGHT, Some(PropertyValue::Number(60.0)));
    insert_component_node(&mut tree, root_id, ROOT_NODE, root);
    for (id, index) in [(first_pane_id, 0.0), (second_pane_id, 1.0)] {
        let mut pane = component_part_node(
            NodeTag::View,
            root_id,
            "splitter-pane",
            &[(property::SCOPE, "lagging")],
            &[],
        );
        pane.set_property(property::ITEM_INDEX, Some(PropertyValue::Number(index)));
        insert_component_node(&mut tree, id, root_id, pane);
    }
    let mut handle = component_part_node(
        NodeTag::View,
        root_id,
        "splitter-handle",
        &[(property::SCOPE, "lagging")],
        &[],
    );
    handle.set_property(property::ITEM_INDEX, Some(PropertyValue::Number(0.0)));
    handle.set_property(property::WIDTH, Some(PropertyValue::Number(6.0)));
    insert_component_node(&mut tree, handle_id, root_id, handle);
    let root_children = &mut tree.nodes.get_mut(&root_id).unwrap().children;
    root_children.clear();
    root_children.extend([first_pane_id, handle_id, second_pane_id]);

    let events: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let view = component_part_view(14, tree, Rc::clone(&events));
    let (mut cx, view) = quickgui::TestAppContext::from_application(
        component_application(),
        quickgui::WindowOptions::default(),
        view,
    )
    .unwrap();
    let window = view.window_handle();
    cx.run_until_idle().unwrap();

    let splitter = Splitter::new(
        ElementId::named("lagging"),
        &SplitterState::new(SplitterOrientation::Horizontal, &[180.0, 180.0]),
    );
    let handle = splitter.handle_id(0);
    let handle_bounds = cx.element_bounds(window, handle).unwrap();
    // The pointer presses three points into the handle and must stay there for the whole drag.
    let grip = 3.0;
    let origin = quickgui::Point::new(handle_bounds.x + grip, handle_bounds.y + 30.0);
    let pointer =
        |phase, position: quickgui::Point, previous: quickgui::Point| quickgui::PointerEvent {
            size: quickgui::Size::ZERO,
            phase,
            position,
            origin,
            local_position: position,
            local_origin: origin,
            delta: position - previous,
            button: quickgui::MouseButton::Left,
            modifiers: quickgui::Modifiers::empty(),
        };
    let redeclare = |cx: &mut quickgui::TestAppContext, sizes: &serde_json::Value| {
        cx.update(view, |view, cx| {
            view.tree
                .borrow_mut()
                .nodes
                .get_mut(&root_id)
                .unwrap()
                .set_property(
                    property::VALUES,
                    Some(PropertyValue::String(Arc::from(sizes.to_string().as_str()))),
                );
            cx.invalidate();
        })
        .unwrap();
        cx.run_until_idle().unwrap();
    };

    cx.simulate_pointer(
        window,
        handle,
        pointer(quickgui::PointerPhase::Down, origin, origin),
    )
    .unwrap();
    // A controlled owner echoes every reported size back as its next `value` declaration, but
    // the round trip through JavaScript is asynchronous: here each echo lands two moves late.
    let mut echoes: VecDeque<serde_json::Value> = VecDeque::new();
    let mut position = origin;
    for _ in 0..4 {
        let previous = position;
        position.x += 12.0;
        cx.simulate_pointer(
            window,
            handle,
            pointer(quickgui::PointerPhase::Move, position, previous),
        )
        .unwrap();
        let change = component_change(&events, root_id);
        echoes.push_back(change["sizes"].clone());
        if echoes.len() > 2 {
            let lagging = echoes.pop_front().unwrap();
            redeclare(&mut cx, &lagging);
        }
    }
    cx.simulate_pointer(
        window,
        handle,
        pointer(quickgui::PointerPhase::Up, position, position),
    )
    .unwrap();
    let handle_bounds = cx.element_bounds(window, handle).unwrap();
    assert!(
        (handle_bounds.x + grip - position.x).abs() < 0.5,
        "the handle at {handle_bounds:?} fell behind the pointer released at {position:?}"
    );

    // The remaining echoes land after the release, oldest first; none may rewind the handle.
    while let Some(lagging) = echoes.pop_front() {
        redeclare(&mut cx, &lagging);
        let handle_bounds = cx.element_bounds(window, handle).unwrap();
        assert!(
            (handle_bounds.x + grip - position.x).abs() < 0.5,
            "the stale echo {lagging} moved the handle to {handle_bounds:?}"
        );
    }
    let sizes = cx
        .read(view, |view| {
            let splitter = view.components.splitters.values().next().unwrap();
            splitter.state.sizes().to_vec()
        })
        .unwrap();
    assert_eq!(sizes, vec![228.0, 132.0]);

    // A genuinely new controlled value still reseeds the splitter once the drag has ended.
    redeclare(&mut cx, &serde_json::json!([100, 260]));
    let handle_bounds = cx.element_bounds(window, handle).unwrap();
    assert!(
        (handle_bounds.x - 100.0).abs() < 0.5,
        "the new declaration moved the handle to {handle_bounds:?}"
    );
}

#[test]
fn packaged_fonts_resolve_against_resources_and_preserve_absolute_paths() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("quickgui-fonts-{}-{unique}", std::process::id()));
    let resources = root.join("Resources");
    std::fs::create_dir_all(resources.join("fonts")).unwrap();
    std::fs::write(resources.join("fonts/bundled.ttf"), b"bundled font").unwrap();
    let external = root.join("external.ttf");
    std::fs::write(&external, b"external font").unwrap();
    let options = NativeAppOptions {
        resource_dir: Some(resources.to_string_lossy().into_owned()),
        fonts: Some(vec![
            "fonts/bundled.ttf".into(),
            external.to_string_lossy().into_owned(),
        ]),
        ..NativeAppOptions::default()
    };
    let fonts = crate::runtime::native_font_data(&options).unwrap();
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(fonts.len(), 2);
    assert_eq!(fonts[0].as_ref(), b"bundled font");
    assert_eq!(fonts[1].as_ref(), b"external font");
}

#[test]
fn fluent_layout_presets_reach_native_geometry() {
    let make_node = |parent, numbers: &[(u16, f32)], strings: &[(u16, &str)]| {
        let mut node = NativeNode::new(NodeTag::View);
        node.parent = Some(parent);
        for &(code, value) in numbers {
            node.set_property(code, Some(PropertyValue::Number(value)));
        }
        for &(code, value) in strings {
            node.set_property(code, Some(PropertyValue::String(Arc::from(value))));
        }
        node
    };
    let mut tree = NativeTree::default();
    let outer = make_node(
        ROOT_NODE,
        &[(property::WIDTH, 300.0), (property::HEIGHT, 220.0)],
        &[
            (property::DISPLAY, "flex"),
            (property::FLEX_DIRECTION, "column"),
        ],
    );
    tree.nodes.insert(799, outer);
    tree.nodes.get_mut(&ROOT_NODE).unwrap().children.push(799);
    let mut center = make_node(
        799,
        &[(property::WIDTH, 300.0), (property::HEIGHT, 100.0)],
        &[(property::DISPLAY, "flex")],
    );
    center.children.push(801);
    tree.nodes.insert(800, center);
    tree.nodes.insert(
        801,
        make_node(
            800,
            &[(property::WIDTH, 40.0), (property::HEIGHT, 20.0)],
            &[(property::MARGIN, "auto")],
        ),
    );
    tree.nodes.get_mut(&799).unwrap().children.push(800);
    for (id, width, template, first_width, second_width) in [
        (
            810,
            100.0,
            "repeat(2, minmax(min-content, 1fr))",
            120.0,
            20.0,
        ),
        (820, 300.0, "repeat(2, minmax(0, max-content))", 40.0, 80.0),
    ] {
        let mut grid = make_node(
            799,
            &[(property::WIDTH, width), (property::HEIGHT, 40.0)],
            &[
                (property::DISPLAY, "grid"),
                (property::GRID_TEMPLATE_COLUMNS, template),
            ],
        );
        grid.children.extend([id + 1, id + 2]);
        tree.nodes.insert(id, grid);
        for (child, width) in [(id + 1, first_width), (id + 2, second_width)] {
            tree.nodes.insert(
                child,
                make_node(
                    id,
                    &[(property::WIDTH, width), (property::HEIGHT, 20.0)],
                    &[],
                ),
            );
        }
        tree.nodes.get_mut(&799).unwrap().children.push(id);
    }
    let mut grid = make_node(
        799,
        &[(property::WIDTH, 300.0), (property::HEIGHT, 40.0)],
        &[
            (property::DISPLAY, "grid"),
            (property::GRID_TEMPLATE_COLUMNS, "repeat(3, minmax(0, 1fr))"),
        ],
    );
    grid.children.push(831);
    tree.nodes.insert(830, grid);
    tree.nodes.insert(
        831,
        make_node(
            830,
            &[(property::GRID_COLUMN_SPAN, 2.0), (property::HEIGHT, 20.0)],
            &[],
        ),
    );
    tree.nodes.get_mut(&799).unwrap().children.push(830);

    let view = component_part_view(91, tree, Rc::new(RefCell::new(VecDeque::new())));
    let (mut cx, view) = quickgui::TestAppContext::new(view).unwrap();
    let window = view.window_handle();
    let mut bounds = |id| cx.element_bounds(window, ElementId::new(id)).unwrap();
    let parent = bounds(800);
    let child = bounds(801);
    assert_eq!(
        child.x - parent.x,
        130.0,
        "parent={parent:?} child={child:?}"
    );
    assert_eq!(child.y - parent.y, 40.0);
    assert_eq!(bounds(812).x - bounds(810).x, 120.0);
    assert_eq!(bounds(822).x - bounds(820).x, 40.0);
    assert_eq!(bounds(831).width, 200.0);
    assert!(native_grid_tracks("none").is_empty());
    assert_eq!(
        native_text_overflow("ellipsis-start"),
        Some(quickgui::TextOverflow::ellipsis_start())
    );
    assert_eq!(
        native_text_overflow("ellipsis-middle"),
        Some(quickgui::TextOverflow::ellipsis_middle())
    );
    assert_eq!(native_text_overflow("unsupported"), None);
}
