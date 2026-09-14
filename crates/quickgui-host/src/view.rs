use super::*;
use serde::Deserialize;

pub(super) struct NativeView {
    pub(super) window: u32,
    pub(super) handles: Option<Rc<RefCell<HashMap<WindowHandle, u32>>>>,
    pub(super) tree: Rc<RefCell<NativeTree>>,
    pub(super) events: EventQueue,
    pub(super) extensions: Rc<RefCell<HashMap<u32, NativeExtensionState>>>,
    pub(super) svgs: Rc<RefCell<HashMap<u32, NativeSvgState>>>,
    pub(super) lists: Rc<RefCell<HashMap<u32, NativeListState>>>,
    /// Retained decoded image sources keyed by their declaring node.
    pub(super) images: Rc<RefCell<HashMap<u32, NativeImageState>>>,
    /// Retained decoded raster backgrounds keyed by their declaring node.
    ///
    /// A `bg_image` needs a decoded core `Image` rather than the lazy resource an `<Image>` node
    /// uses, so each declared source is decoded exactly once and kept until it changes.
    pub(super) background_images: Rc<RefCell<HashMap<u32, NativeBackgroundImageState>>>,
    /// Retained validated application shaders keyed by their declaring node.
    pub(super) shaders: Rc<RefCell<HashMap<u32, NativeShaderState>>>,
    /// Retained in-window popover-menu models keyed by their declaring node.
    pub(super) menus: NativeMenuStates,
    /// The one core context-menu state this window owns.
    ///
    /// `ContextMenuState::element` takes a non-capturing accessor, so the state lives directly on
    /// the view. Opening a context menu anywhere in the window replaces the one already open,
    /// which is exactly the native invariant.
    pub(super) context_menu: ContextMenuState,
    /// Node that opened the current context menu, so only that target reports `expanded`.
    pub(super) context_menu_owner: Option<u32>,
    /// Node that currently holds keyboard focus, so blur is reported exactly once.
    pub(super) focused_node: Option<u32>,
    /// Retained range, ordering, and roving-focus component instances declared in this window.
    ///
    /// These are owned by the view rather than shared through a cell because each declared
    /// instance is reached through a [`StateAccessor`] that must return `&mut` into the view.
    pub(super) components: NativeComponentStates,
    #[cfg(target_os = "macos")]
    pub(super) swift_ui_hosts: Rc<RefCell<HashMap<u32, NativeSwiftUiHostState>>>,
    #[cfg(target_os = "macos")]
    pub(super) embedded_views: Rc<RefCell<HashMap<u32, MacEmbeddedView>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct NativeMenuAction(pub(super) u32);

impl View for NativeView {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        match event {
            // The core decides focus; the binding only reports the transition to the two nodes
            // that declared a listener.
            Event::FocusChanged(focused) => {
                // Base UI's `data-focused` and the `data-touched` edge it implies belong to the
                // core, so the focus the retained tree really moved is written straight into the
                // declared picker instances.
                if sync_picker_focus(&mut self.components, *focused) {
                    cx.invalidate();
                }
                let next = focused
                    .map(quickgui::ElementId::as_u64)
                    .and_then(|value| u32::try_from(value).ok())
                    .filter(|id| self.declares_focus_listener(*id));
                let previous = self.focused_node.take();
                if previous == next {
                    self.focused_node = next;
                    return;
                }
                let window = self.event_window(cx);
                if let Some(previous) = previous {
                    enqueue_event(
                        &self.events,
                        QueuedEvent {
                            kind: "blur",
                            window,
                            target: previous,
                            value: None,
                        },
                    );
                }
                if let Some(next) = next {
                    enqueue_event(
                        &self.events,
                        QueuedEvent {
                            kind: "focus",
                            window,
                            target: next,
                            value: None,
                        },
                    );
                }
                self.focused_node = next;
                return;
            }
            // A promoted drag ends outside the retained tree, so the core reports it on the
            // window and the binding routes it back to the declaring source node.
            Event::ExternalDragEnded(ended) => {
                let Some(source) = u32::try_from(ended.source.as_u64())
                    .ok()
                    .filter(|id| self.declares_drag_listener(*id))
                else {
                    return;
                };
                let window = self.event_window(cx);
                enqueue_event(
                    &self.events,
                    QueuedEvent {
                        kind: "dragend",
                        window,
                        target: source,
                        value: Some(Arc::from(
                            serde_json::json!({
                                "operation": match ended.operation {
                                    quickgui::ExternalDragOperation::Cancelled => "cancelled",
                                    quickgui::ExternalDragOperation::Copied => "copied",
                                    quickgui::ExternalDragOperation::Moved => "moved",
                                    quickgui::ExternalDragOperation::Linked => "linked",
                                    quickgui::ExternalDragOperation::Deleted => "deleted",
                                    quickgui::ExternalDragOperation::Other => "other",
                                },
                            })
                            .to_string()
                            .as_str(),
                        )),
                    },
                );
                return;
            }
            _ => {}
        }
        // Input events reach this callback on every frame; resolving the hosted window id costs a
        // map lookup, so the hot path leaves before it.
        if !crate::runtime::is_hosted_window_event(event) {
            return;
        }
        let window = cx.window_handle().map_or(self.window, |handle| {
            self.handles.as_ref().map_or(self.window, |handles| {
                handles
                    .borrow()
                    .get(&handle)
                    .copied()
                    .unwrap_or(self.window)
            })
        });
        // Window lifecycle notifications and the declared-ahead resize/move constraints live in
        // the hosted runtime module; everything else falls through to close interception.
        if crate::runtime::handle_window_lifecycle_event(window, event, cx, &self.events) {
            return;
        }
        if !matches!(event, Event::CloseRequested) {
            return;
        }
        // Interception was declared before the native decision, so the veto is answered here and
        // JavaScript completes the close later with an explicit command.
        if !crate::runtime::intercepts_close(window) {
            return;
        }
        cx.prevent_close();
        enqueue_event(
            &self.events,
            QueuedEvent {
                kind: "close-requested",
                window,
                target: ROOT_NODE,
                value: None,
            },
        );
    }

    fn render_scope(&mut self, id: ElementId, cx: &mut ViewContext<'_, Self>) -> Option<Element> {
        let id = u32::try_from(id.as_u64()).ok()?;
        if self.context_menu_owner.is_some() || !native_scope_supported(&self.tree.borrow(), id) {
            return None;
        }
        self.build_tree(cx, Some(id))
    }

    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let window = self.handles.as_ref().map_or(self.window, |handles| {
            handles
                .borrow()
                .get(&cx.window_handle())
                .copied()
                .expect("a native binding view must retain its core window handle")
        });
        let root = self.build_tree(cx, None).expect("native window root");
        let select_events = Rc::clone(&self.events);
        // A declared menu command keeps its concrete typed payload through the core's popover
        // chain and arrives here on the owner window's ordinary action path.
        let menu_select = cx.action_listener(
            ElementId::new(ROOT_ELEMENT_ID),
            move |view, action: &NativeMenuSelect, _cx| {
                // Selection is declared ahead of the core's decision, exactly like every other
                // listener, so a menu without an `onSelect` handler queues nothing.
                let (selects, clicks, changes) = {
                    let tree = view.tree.borrow();
                    match tree.nodes.get(&action.node) {
                        Some(node) => (
                            node.boolean(property::SELECT_LISTENER).unwrap_or(false),
                            node.boolean(property::CLICK_LISTENER).unwrap_or(false),
                            declares_change(node),
                        ),
                        None => return,
                    }
                };
                if selects {
                    enqueue_event(
                        &select_events,
                        QueuedEvent {
                            kind: "menuselect",
                            window,
                            target: action.node,
                            value: Some(action.event_value()),
                        },
                    );
                }
                // A Base UI-shaped row declared as a child node reports the same activation the
                // in-window compound reports, so one component works in either menu host.
                if clicks {
                    enqueue_event(
                        &select_events,
                        QueuedEvent {
                            kind: "click",
                            window,
                            target: action.node,
                            value: None,
                        },
                    );
                }
                if changes {
                    let mut payload = serde_json::Map::new();
                    payload.insert(
                        "activated".to_owned(),
                        serde_json::Value::String(action.item.to_string()),
                    );
                    if let Some(checked) = action.checked {
                        payload.insert("checked".to_owned(), serde_json::Value::Bool(checked));
                    }
                    enqueue_component_change(
                        &select_events,
                        window,
                        action.node,
                        serde_json::Value::Object(payload),
                    );
                }
            },
        );
        // QuickGUI has no document to navigate, so a declared `Menu.LinkItem` in a core-painted
        // menu reaches the platform through the core's own open-URL path.
        let open_link = cx.action_listener(
            ElementId::new(ROOT_ELEMENT_ID),
            move |_view, action: &quickgui::OpenMenuLink, cx| {
                let _ = cx.open_url(Arc::clone(&action.url));
            },
        );
        root.on_action(menu_select).on_action(open_link)
    }
}

impl NativeView {
    fn event_window(&self, cx: &mut EventContext) -> u32 {
        cx.window_handle().map_or(self.window, |handle| {
            self.handles.as_ref().map_or(self.window, |handles| {
                handles
                    .borrow()
                    .get(&handle)
                    .copied()
                    .unwrap_or(self.window)
            })
        })
    }

    fn declares_focus_listener(&self, id: u32) -> bool {
        self.tree
            .borrow()
            .nodes
            .get(&id)
            .is_some_and(|node| node.boolean(property::FOCUS_LISTENER).unwrap_or(false))
    }

    fn declares_drag_listener(&self, id: u32) -> bool {
        self.tree
            .borrow()
            .nodes
            .get(&id)
            .is_some_and(|node| node.boolean(property::DRAG_LISTENER).unwrap_or(false))
    }
}

pub(super) struct NativeElementStates<'a> {
    pub(super) extensions: &'a mut HashMap<u32, NativeExtensionState>,
    pub(super) svgs: &'a mut HashMap<u32, NativeSvgState>,
    pub(super) lists: &'a mut HashMap<u32, NativeListState>,
    /// Derived component-part element identities already mounted in this render pass.
    ///
    /// Nodes without a `part` property keep their unique node-derived identity and never touch
    /// this set, so the untouched element path costs nothing. Two parts that declare the same
    /// scope and value would otherwise register one core listener identity twice, which the core
    /// rejects with a panic; the duplicate mounts without listeners instead.
    pub(super) part_ids: &'a mut HashSet<u64>,
    /// Viewport portals lifted out of the subtree that declared them, in declaration order.
    ///
    /// A dialog root, drawer sheet, or toast viewport declared inside a panel is an overlay whose
    /// insets and percentage sizes must resolve against the window, exactly as a DOM portal
    /// renders into the document body. They mount under the window root once the ordinary tree
    /// is built; anchored popups stay where they are declared.
    pub(super) portals: Vec<Element>,
    pub(super) images: &'a mut HashMap<u32, NativeImageState>,
    /// Retained decoded raster backgrounds, decoded once per declared source.
    pub(super) background_images: &'a mut HashMap<u32, NativeBackgroundImageState>,
    pub(super) shaders: &'a mut HashMap<u32, NativeShaderState>,
    /// This pass's retained component instances, already reseeded from the declaration.
    ///
    /// Pickers and collections need an exclusive borrow while the core builds their popover or
    /// virtual scroll container, so the whole set travels as one mutable borrow.
    pub(super) components: &'a mut NativeComponentStates,
    /// Retained popover-menu models, shared with the listeners this pass installs.
    pub(super) menus: NativeMenuStates,
    /// Snapshot of the window's context-menu state for this render pass.
    pub(super) context_menu: ContextMenuState,
    pub(super) context_menu_owner: Option<u32>,
    #[cfg(target_os = "macos")]
    pub(super) swift_ui_hosts: &'a mut HashMap<u32, NativeSwiftUiHostState>,
    #[cfg(target_os = "macos")]
    pub(super) embedded_views: &'a HashMap<u32, MacEmbeddedView>,
}

#[cfg(target_os = "macos")]
pub(super) struct NativeSwiftUiHostState {
    host: std::result::Result<MacSwiftUiHost, String>,
}

#[cfg(target_os = "macos")]
impl NativeSwiftUiHostState {
    fn new(window: u32, events: &EventQueue, invalidator: quickgui::WindowInvalidator) -> Self {
        let action_events = Rc::clone(events);
        let presentation_events = Rc::clone(events);
        let value_events = Rc::clone(events);
        let submit_events = Rc::clone(events);
        let action_invalidator = invalidator.clone();
        let presentation_invalidator = invalidator.clone();
        let value_invalidator = invalidator.clone();
        let submit_invalidator = invalidator;
        Self {
            host: MacSwiftUiHost::new_with_control_events(
                move |target| {
                    let Ok(target) = u32::try_from(target) else {
                        return;
                    };
                    enqueue_event(
                        &action_events,
                        QueuedEvent {
                            kind: "click",
                            window,
                            target,
                            value: None,
                        },
                    );
                    action_invalidator.invalidate();
                },
                move |target, presented| {
                    let Ok(target) = u32::try_from(target) else {
                        return;
                    };
                    enqueue_event(
                        &presentation_events,
                        QueuedEvent {
                            kind: "presentationchange",
                            window,
                            target,
                            value: Some(Arc::from(if presented { "true" } else { "false" })),
                        },
                    );
                    presentation_invalidator.invalidate();
                },
                move |target, value| {
                    let Ok(target) = u32::try_from(target) else {
                        return;
                    };
                    enqueue_event(
                        &value_events,
                        QueuedEvent {
                            kind: "input",
                            window,
                            target,
                            value: Some(Arc::from(value)),
                        },
                    );
                    value_invalidator.invalidate();
                },
                move |target| {
                    let Ok(target) = u32::try_from(target) else {
                        return;
                    };
                    enqueue_event(
                        &submit_events,
                        QueuedEvent {
                            kind: "submit",
                            window,
                            target,
                            value: None,
                        },
                    );
                    submit_invalidator.invalidate();
                },
            ),
        }
    }

    fn element(
        &mut self,
        node: &NativeNode,
        tree: &NativeTree,
        embedded_views: &HashMap<u32, MacEmbeddedView>,
    ) -> std::result::Result<Element, String> {
        let host = self.host.as_mut().map_err(|error| error.clone())?;
        let elements = swift_ui_children(&node.children, tree, embedded_views)?;
        host.sync(&elements)?;
        let fitting = host.fitting_size()?;
        let mut element = native_view_with_outset(host.view(), host.effect_inset()?);
        if node
            .boolean(property::SWIFT_UI_MATCH_CONTENTS_HORIZONTAL)
            .unwrap_or(false)
        {
            element = element.w(fitting.width);
        }
        if node
            .boolean(property::SWIFT_UI_MATCH_CONTENTS_VERTICAL)
            .unwrap_or(false)
        {
            element = element.h(fitting.height);
        }
        Ok(element)
    }
}

#[cfg(target_os = "macos")]
fn swift_ui_button(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
) -> std::result::Result<SwiftUiButton, String> {
    let label = node
        .string(property::VALUE)
        .map(Arc::<str>::from)
        .or_else(|| {
            let text = swift_ui_text_content(node, tree);
            (!text.is_empty()).then(|| Arc::from(text))
        });
    let mut button = SwiftUiButton::new(u64::from(id)).role(match node.string(property::ROLE) {
        Some("cancel") => SwiftUiButtonRole::Cancel,
        Some("destructive") => SwiftUiButtonRole::Destructive,
        _ => SwiftUiButtonRole::Default,
    });
    if let Some(label) = label {
        button = button.label(label);
    }
    if let Some(target) = node.string(property::SWIFT_UI_TARGET) {
        button = button.target(Arc::<str>::from(target));
    }
    if let Some(test_id) = node.string(property::SWIFT_UI_TEST_ID) {
        button = button.test_id(Arc::<str>::from(test_id));
    }
    if let Some(style) = node.string(property::SWIFT_UI_BUTTON_STYLE) {
        button = button.style(match style {
            "bordered" => SwiftUiButtonStyle::Bordered,
            "borderedProminent" => SwiftUiButtonStyle::BorderedProminent,
            "borderless" => SwiftUiButtonStyle::Borderless,
            "plain" => SwiftUiButtonStyle::Plain,
            "glass" => SwiftUiButtonStyle::Glass,
            "glassProminent" => SwiftUiButtonStyle::GlassProminent,
            _ => SwiftUiButtonStyle::Automatic,
        });
    }
    if let Some(size) = node.string(property::SWIFT_UI_CONTROL_SIZE) {
        button = button.control_size(match size {
            "mini" => SwiftUiControlSize::Mini,
            "small" => SwiftUiControlSize::Small,
            "large" => SwiftUiControlSize::Large,
            "extraLarge" => SwiftUiControlSize::ExtraLarge,
            _ => SwiftUiControlSize::Regular,
        });
    }
    if let Some(disabled) = node.boolean(property::DISABLED) {
        button = button.disabled(disabled);
    }
    button = button
        .modifiers(swift_ui_modifiers(node)?)
        .action(node.boolean(property::CLICK_LISTENER).unwrap_or(false));
    if let Some(system_image) = node
        .string(property::SWIFT_UI_SYSTEM_IMAGE)
        .filter(|value| !value.is_empty())
    {
        button = button.system_image(Arc::<str>::from(system_image));
    }
    Ok(button)
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_slider(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
) -> std::result::Result<SwiftUiSlider, String> {
    let mut slider = SwiftUiSlider::new(
        u64::from(id),
        f64::from(node.number(property::VALUE).unwrap_or(0.0)),
    )
    .range(
        f64::from(node.number(property::MINIMUM).unwrap_or(0.0)),
        f64::from(node.number(property::MAXIMUM).unwrap_or(1.0)),
    )
    .modifiers(swift_ui_modifiers(node)?)
    .on_value_change(node.boolean(property::INPUT_LISTENER).unwrap_or(false));
    if let Some(step) = node.number(property::STEP).filter(|step| *step > 0.0) {
        slider = slider.step(f64::from(step));
    }
    let label = swift_ui_text_content(node, tree);
    if !label.is_empty() {
        slider = slider.label(label);
    }
    if let Some(test_id) = node.string(property::SWIFT_UI_TEST_ID) {
        slider = slider.test_id(Arc::<str>::from(test_id));
    }
    Ok(slider)
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_toggle(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
) -> std::result::Result<SwiftUiToggle, String> {
    let mut toggle = SwiftUiToggle::new(
        u64::from(id),
        node.boolean(property::CHECKED).unwrap_or(false),
    )
    .modifiers(swift_ui_modifiers(node)?)
    .on_value_change(node.boolean(property::INPUT_LISTENER).unwrap_or(false));
    let label = swift_ui_text_content(node, tree);
    if !label.is_empty() {
        toggle = toggle.label(label);
    }
    if let Some(test_id) = node.string(property::SWIFT_UI_TEST_ID) {
        toggle = toggle.test_id(Arc::<str>::from(test_id));
    }
    Ok(toggle)
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_progress_view(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
) -> std::result::Result<SwiftUiProgressView, String> {
    let mut progress = match node.number(property::VALUE) {
        Some(value) => SwiftUiProgressView::new(
            u64::from(id),
            f64::from(value),
            f64::from(node.number(property::MAXIMUM).unwrap_or(1.0)),
        ),
        None => SwiftUiProgressView::indeterminate(u64::from(id)),
    }
    .modifiers(swift_ui_modifiers(node)?);
    let label = swift_ui_text_content(node, tree);
    if !label.is_empty() {
        progress = progress.label(label);
    }
    if let Some(current_value_label) = node.string(property::VALUE_TEXT) {
        progress = progress.current_value_label(Arc::<str>::from(current_value_label));
    }
    if let Some(test_id) = node.string(property::SWIFT_UI_TEST_ID) {
        progress = progress.test_id(Arc::<str>::from(test_id));
    }
    Ok(progress)
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_stepper(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
) -> std::result::Result<SwiftUiStepper, String> {
    let mut stepper = SwiftUiStepper::new(
        u64::from(id),
        f64::from(node.number(property::VALUE).unwrap_or(0.0)),
    )
    .range(
        f64::from(node.number(property::MINIMUM).unwrap_or(0.0)),
        f64::from(node.number(property::MAXIMUM).unwrap_or(100.0)),
    )
    .step(f64::from(node.number(property::STEP).unwrap_or(1.0)))
    .modifiers(swift_ui_modifiers(node)?)
    .on_value_change(node.boolean(property::INPUT_LISTENER).unwrap_or(false));
    let label = swift_ui_text_content(node, tree);
    if !label.is_empty() {
        stepper = stepper.label(label);
    }
    if let Some(test_id) = node.string(property::SWIFT_UI_TEST_ID) {
        stepper = stepper.test_id(Arc::<str>::from(test_id));
    }
    Ok(stepper)
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_text_field(
    id: u32,
    node: &NativeNode,
) -> std::result::Result<SwiftUiTextField, String> {
    let mut field = SwiftUiTextField::new(
        u64::from(id),
        Arc::<str>::from(node.string(property::VALUE).unwrap_or_default()),
    )
    .secure(node.boolean(property::PASSWORD).unwrap_or(false))
    .modifiers(swift_ui_modifiers(node)?)
    .on_value_change(node.boolean(property::INPUT_LISTENER).unwrap_or(false))
    .on_submit(node.boolean(property::SUBMIT_LISTENER).unwrap_or(false));
    if let Some(placeholder) = node.string(property::PLACEHOLDER) {
        field = field.placeholder(Arc::<str>::from(placeholder));
    }
    if let Some(test_id) = node.string(property::SWIFT_UI_TEST_ID) {
        field = field.test_id(Arc::<str>::from(test_id));
    }
    Ok(field)
}

#[cfg(target_os = "macos")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeSwiftUiPickerOption {
    value: String,
    label: String,
    system_image: Option<String>,
    #[serde(default)]
    disabled: bool,
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_picker(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
) -> std::result::Result<SwiftUiPicker, String> {
    let payload = node.string(property::ITEMS).unwrap_or("[]");
    if payload.len() > MAX_OPTIONS_JSON_BYTES {
        return Err(format!(
            "SwiftUI Picker {id} options exceed the {MAX_OPTIONS_JSON_BYTES}-byte limit"
        ));
    }
    let declared: Vec<NativeSwiftUiPickerOption> = serde_json::from_str(payload)
        .map_err(|error| format!("invalid SwiftUI Picker {id} options: {error}"))?;
    let mut values = HashSet::with_capacity(declared.len());
    let mut options = Vec::with_capacity(declared.len());
    for option in declared {
        if option.value.is_empty() {
            return Err(format!("SwiftUI Picker {id} option values cannot be empty"));
        }
        if !values.insert(option.value.clone()) {
            return Err(format!(
                "SwiftUI Picker {id} contains duplicate option value {:?}",
                option.value
            ));
        }
        let mut converted = SwiftUiPickerOption::new(option.value, option.label);
        if let Some(system_image) = option.system_image.filter(|value| !value.is_empty()) {
            converted = converted.system_image(system_image);
        }
        options.push(converted.disabled(option.disabled));
    }
    let style = match (
        node.string(property::ROLE),
        node.string(property::SWIFT_UI_PICKER_STYLE),
    ) {
        (Some("tabs"), _) => SwiftUiPickerStyle::Tabs,
        (_, Some("menu")) => SwiftUiPickerStyle::Menu,
        (_, Some("segmented")) => SwiftUiPickerStyle::Segmented,
        (_, Some("radioGroup")) => SwiftUiPickerStyle::RadioGroup,
        (_, Some("inline")) => SwiftUiPickerStyle::Inline,
        _ => SwiftUiPickerStyle::Automatic,
    };
    let mut picker = SwiftUiPicker::new(
        u64::from(id),
        Arc::<str>::from(node.string(property::VALUE).unwrap_or_default()),
        options,
    )
    .style(style)
    .modifiers(swift_ui_modifiers(node)?)
    .on_value_change(node.boolean(property::INPUT_LISTENER).unwrap_or(false));
    let label = swift_ui_text_content(node, tree);
    if !label.is_empty() {
        picker = picker.label(label);
    }
    if let Some(test_id) = node.string(property::SWIFT_UI_TEST_ID) {
        picker = picker.test_id(Arc::<str>::from(test_id));
    }
    Ok(picker)
}

#[cfg(target_os = "macos")]
fn swift_ui_epoch(node: &NativeNode, property: u16) -> Option<f64> {
    node.string(property)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_date_picker(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
) -> std::result::Result<SwiftUiDatePicker, String> {
    let value = swift_ui_epoch(node, property::CIVIL_VALUE).unwrap_or(0.0);
    let mut picker = SwiftUiDatePicker::new(u64::from(id), value)
        .range(
            swift_ui_epoch(node, property::CIVIL_MINIMUM),
            swift_ui_epoch(node, property::CIVIL_MAXIMUM),
        )
        .components(
            match node.string(property::SWIFT_UI_DATE_PICKER_COMPONENTS) {
                Some("date") => SwiftUiDatePickerComponents::Date,
                Some("hourAndMinute") => SwiftUiDatePickerComponents::HourAndMinute,
                _ => SwiftUiDatePickerComponents::DateAndTime,
            },
        )
        .style(match node.string(property::SWIFT_UI_DATE_PICKER_STYLE) {
            Some("field") => SwiftUiDatePickerStyle::Field,
            Some("graphical") => SwiftUiDatePickerStyle::Graphical,
            Some("stepperField") => SwiftUiDatePickerStyle::StepperField,
            _ => SwiftUiDatePickerStyle::Automatic,
        })
        .modifiers(swift_ui_modifiers(node)?)
        .on_value_change(node.boolean(property::INPUT_LISTENER).unwrap_or(false));
    let label = swift_ui_text_content(node, tree);
    if !label.is_empty() {
        picker = picker.label(label);
    }
    if let Some(test_id) = node.string(property::SWIFT_UI_TEST_ID) {
        picker = picker.test_id(Arc::<str>::from(test_id));
    }
    Ok(picker)
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_color_picker(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
) -> std::result::Result<SwiftUiColorPicker, String> {
    let mut picker = SwiftUiColorPicker::new(
        u64::from(id),
        Arc::<str>::from(node.string(property::VALUE).unwrap_or("#000000ff")),
    )
    .supports_opacity(
        node.boolean(property::SWIFT_UI_COLOR_SUPPORTS_OPACITY)
            .unwrap_or(true),
    )
    .modifiers(swift_ui_modifiers(node)?)
    .on_value_change(node.boolean(property::INPUT_LISTENER).unwrap_or(false));
    let label = swift_ui_text_content(node, tree);
    if !label.is_empty() {
        picker = picker.label(label);
    }
    if let Some(test_id) = node.string(property::SWIFT_UI_TEST_ID) {
        picker = picker.test_id(Arc::<str>::from(test_id));
    }
    Ok(picker)
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_gauge(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
) -> std::result::Result<SwiftUiGauge, String> {
    let mut gauge = SwiftUiGauge::new(
        u64::from(id),
        f64::from(node.number(property::VALUE).unwrap_or(0.0)),
    )
    .range(
        f64::from(node.number(property::MINIMUM).unwrap_or(0.0)),
        f64::from(node.number(property::MAXIMUM).unwrap_or(1.0)),
    )
    .style(match node.string(property::SWIFT_UI_GAUGE_STYLE) {
        Some("accessoryCircular") => SwiftUiGaugeStyle::AccessoryCircular,
        Some("accessoryCircularCapacity") => SwiftUiGaugeStyle::AccessoryCircularCapacity,
        Some("accessoryLinear") => SwiftUiGaugeStyle::AccessoryLinear,
        Some("accessoryLinearCapacity") => SwiftUiGaugeStyle::AccessoryLinearCapacity,
        _ => SwiftUiGaugeStyle::Automatic,
    })
    .modifiers(swift_ui_modifiers(node)?);
    let label = swift_ui_text_content(node, tree);
    if !label.is_empty() {
        gauge = gauge.label(label);
    }
    if let Some(label) = node.string(property::VALUE_TEXT) {
        gauge = gauge.current_value_label(Arc::<str>::from(label));
    }
    if let Some(label) = node.string(property::SWIFT_UI_GAUGE_MINIMUM_VALUE_LABEL) {
        gauge = gauge.minimum_value_label(Arc::<str>::from(label));
    }
    if let Some(label) = node.string(property::SWIFT_UI_GAUGE_MAXIMUM_VALUE_LABEL) {
        gauge = gauge.maximum_value_label(Arc::<str>::from(label));
    }
    if let Some(test_id) = node.string(property::SWIFT_UI_TEST_ID) {
        gauge = gauge.test_id(Arc::<str>::from(test_id));
    }
    Ok(gauge)
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_children(
    children: &[u32],
    tree: &NativeTree,
    embedded_views: &HashMap<u32, MacEmbeddedView>,
) -> std::result::Result<Vec<SwiftUiElement>, String> {
    children
        .iter()
        .filter_map(|id| {
            tree.nodes
                .get(id)
                .and_then(|node| swift_ui_element(*id, node, tree, embedded_views))
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn swift_ui_element(
    id: u32,
    node: &NativeNode,
    tree: &NativeTree,
    embedded_views: &HashMap<u32, MacEmbeddedView>,
) -> Option<std::result::Result<SwiftUiElement, String>> {
    match node.tag {
        NodeTag::SwiftUiButton => Some(swift_ui_button(id, node, tree).map(SwiftUiElement::Button)),
        NodeTag::SwiftUiSlider => Some(swift_ui_slider(id, node, tree).map(SwiftUiElement::Slider)),
        NodeTag::SwiftUiToggle => Some(swift_ui_toggle(id, node, tree).map(SwiftUiElement::Toggle)),
        NodeTag::SwiftUiProgressView => {
            Some(swift_ui_progress_view(id, node, tree).map(SwiftUiElement::ProgressView))
        }
        NodeTag::SwiftUiStepper => {
            Some(swift_ui_stepper(id, node, tree).map(SwiftUiElement::Stepper))
        }
        NodeTag::SwiftUiTextField => {
            Some(swift_ui_text_field(id, node).map(SwiftUiElement::TextField))
        }
        NodeTag::SwiftUiPicker => Some(swift_ui_picker(id, node, tree).map(SwiftUiElement::Picker)),
        NodeTag::SwiftUiDatePicker => {
            Some(swift_ui_date_picker(id, node, tree).map(SwiftUiElement::DatePicker))
        }
        NodeTag::SwiftUiColorPicker => {
            Some(swift_ui_color_picker(id, node, tree).map(SwiftUiElement::ColorPicker))
        }
        NodeTag::SwiftUiGauge => Some(swift_ui_gauge(id, node, tree).map(SwiftUiElement::Gauge)),
        NodeTag::SwiftUiQuickGuiHost => Some((|| {
            let embedded_id = node
                .number(property::SWIFT_UI_EMBEDDED_WINDOW)
                .filter(|id| *id >= 1.0 && *id <= u32::MAX as f32)
                .map(|id| id as u32);
            let mut host = match embedded_id {
                Some(embedded_id) => {
                    let embedded = embedded_views.get(&embedded_id).cloned().ok_or_else(|| {
                        format!(
                            "SwiftUI QuickGUIHostView {id} references unknown embedded window {embedded_id}"
                        )
                    })?;
                    SwiftUiQuickGuiHost::new(u64::from(id), embedded)
                }
                None => SwiftUiQuickGuiHost::pending(u64::from(id)),
            };
            host.match_horizontal = node
                .boolean(property::SWIFT_UI_MATCH_CONTENTS_HORIZONTAL)
                .unwrap_or(false);
            host.match_vertical = node
                .boolean(property::SWIFT_UI_MATCH_CONTENTS_VERTICAL)
                .unwrap_or(false);
            host.width = node.number(property::WIDTH).filter(|value| *value > 0.0);
            host.height = node.number(property::HEIGHT).filter(|value| *value > 0.0);
            host.test_id = node
                .string(property::SWIFT_UI_TEST_ID)
                .filter(|value| !value.is_empty())
                .map(Arc::<str>::from);
            Ok(SwiftUiElement::QuickGuiHost(host))
        })()),
        NodeTag::SwiftUiPopover => Some((|| {
            let mut trigger = None;
            let mut content = None;
            for child in &node.children {
                let Some(slot) = tree.nodes.get(child) else {
                    continue;
                };
                match slot.tag {
                    NodeTag::SwiftUiPopoverTrigger if trigger.is_none() => {
                        trigger = Some(swift_ui_children(&slot.children, tree, embedded_views)?);
                    }
                    NodeTag::SwiftUiPopoverContent if content.is_none() => {
                        content = Some(swift_ui_children(&slot.children, tree, embedded_views)?);
                    }
                    _ => {}
                }
            }
            let mut popover = SwiftUiPopover::new(u64::from(id));
            popover.is_presented = node
                .boolean(property::SWIFT_UI_IS_PRESENTED)
                .unwrap_or(false);
            popover.attachment_anchor = match node.string(property::SWIFT_UI_ATTACHMENT_ANCHOR) {
                Some("top") => SwiftUiPopoverAttachmentAnchor::Top,
                Some("bottom") => SwiftUiPopoverAttachmentAnchor::Bottom,
                Some("leading") => SwiftUiPopoverAttachmentAnchor::Leading,
                Some("trailing") => SwiftUiPopoverAttachmentAnchor::Trailing,
                _ => SwiftUiPopoverAttachmentAnchor::Center,
            };
            popover.arrow_edge = match node.string(property::SWIFT_UI_ARROW_EDGE) {
                Some("top") => SwiftUiPopoverArrowEdge::Top,
                Some("leading") => SwiftUiPopoverArrowEdge::Leading,
                Some("trailing") => SwiftUiPopoverArrowEdge::Trailing,
                _ => SwiftUiPopoverArrowEdge::Bottom,
            };
            popover.trigger = trigger
                .ok_or_else(|| format!("SwiftUI Popover {id} requires one Popover.Trigger"))?;
            popover.content = content
                .ok_or_else(|| format!("SwiftUI Popover {id} requires one Popover.Content"))?;
            popover.test_id = node
                .string(property::SWIFT_UI_TEST_ID)
                .filter(|value| !value.is_empty())
                .map(Arc::<str>::from);
            Ok(SwiftUiElement::Popover(popover))
        })()),
        NodeTag::SwiftUiPopoverTrigger | NodeTag::SwiftUiPopoverContent => None,
        // A region (`Show`, `For`, ...) keeps its content before a sentinel it inserts into the
        // same parent; the content is what the host mounts.
        NodeTag::Sentinel => None,
        _ => Some(Err(format!(
            "node {id} is not a SwiftUI component and cannot be mounted directly in Host"
        ))),
    }
}

#[cfg(target_os = "macos")]
#[derive(serde::Deserialize)]
struct NativeSwiftUiModifier {
    #[serde(rename = "$type")]
    kind: String,
    style: Option<String>,
    size: Option<String>,
    shape: Option<String>,
    #[serde(rename = "cornerRadius")]
    corner_radius: Option<f32>,
    color: Option<String>,
    disabled: Option<bool>,
}

#[cfg(target_os = "macos")]
pub(super) fn swift_ui_modifiers(
    node: &NativeNode,
) -> std::result::Result<Vec<SwiftUiModifier>, String> {
    let Some(payload) = node.string(property::SWIFT_UI_MODIFIERS) else {
        return Ok(Vec::new());
    };
    let modifiers = serde_json::from_str::<Vec<NativeSwiftUiModifier>>(payload)
        .map_err(|error| format!("invalid SwiftUI modifiers: {error}"))?;
    modifiers
        .into_iter()
        .map(|modifier| match modifier.kind.as_str() {
            "buttonStyle" => Ok(SwiftUiModifier::ButtonStyle(
                match modifier.style.as_deref() {
                    Some("automatic") => SwiftUiButtonStyle::Automatic,
                    Some("bordered") => SwiftUiButtonStyle::Bordered,
                    Some("borderedProminent") => SwiftUiButtonStyle::BorderedProminent,
                    Some("borderless") => SwiftUiButtonStyle::Borderless,
                    Some("glass") => SwiftUiButtonStyle::Glass,
                    Some("glassProminent") => SwiftUiButtonStyle::GlassProminent,
                    Some("plain") => SwiftUiButtonStyle::Plain,
                    _ => return Err("buttonStyle modifier has an invalid style".to_owned()),
                },
            )),
            "buttonBorderShape" => {
                let shape = match modifier.shape.as_deref() {
                    Some("automatic") => SwiftUiButtonBorderShape::Automatic,
                    Some("capsule") => SwiftUiButtonBorderShape::Capsule,
                    Some("roundedRectangle") => SwiftUiButtonBorderShape::RoundedRectangle,
                    Some("circle") => SwiftUiButtonBorderShape::Circle,
                    _ => return Err("buttonBorderShape modifier has an invalid shape".to_owned()),
                };
                if modifier
                    .corner_radius
                    .is_some_and(|radius| !radius.is_finite() || radius < 0.0)
                {
                    return Err("buttonBorderShape modifier has an invalid cornerRadius".to_owned());
                }
                Ok(SwiftUiModifier::ButtonBorderShape {
                    shape,
                    corner_radius: modifier.corner_radius,
                })
            }
            "controlSize" => Ok(SwiftUiModifier::ControlSize(
                match modifier.size.as_deref() {
                    Some("mini") => SwiftUiControlSize::Mini,
                    Some("small") => SwiftUiControlSize::Small,
                    Some("regular") => SwiftUiControlSize::Regular,
                    Some("large") => SwiftUiControlSize::Large,
                    Some("extraLarge") => SwiftUiControlSize::ExtraLarge,
                    _ => return Err("controlSize modifier has an invalid size".to_owned()),
                },
            )),
            "labelStyle" => Ok(SwiftUiModifier::LabelStyle(
                match modifier.style.as_deref() {
                    Some("automatic") => SwiftUiLabelStyle::Automatic,
                    Some("iconOnly") => SwiftUiLabelStyle::IconOnly,
                    Some("titleAndIcon") => SwiftUiLabelStyle::TitleAndIcon,
                    Some("titleOnly") => SwiftUiLabelStyle::TitleOnly,
                    _ => return Err("labelStyle modifier has an invalid style".to_owned()),
                },
            )),
            "tint" => modifier
                .color
                .filter(|color| !color.is_empty())
                .map(|color| SwiftUiModifier::Tint(Arc::from(color)))
                .ok_or_else(|| "tint modifier has an invalid color".to_owned()),
            "disabled" => modifier
                .disabled
                .map(SwiftUiModifier::Disabled)
                .ok_or_else(|| "disabled modifier has no disabled value".to_owned()),
            kind => Err(format!("unsupported SwiftUI modifier `{kind}`")),
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn swift_ui_text_content(node: &NativeNode, tree: &NativeTree) -> String {
    let mut text = String::new();
    let mut pending = node.children.iter().copied().rev().collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(id) = pending.pop() {
        visited += 1;
        if visited > MAX_TREE_DEPTH {
            break;
        }
        let Some(child) = tree.nodes.get(&id) else {
            continue;
        };
        if child.tag == NodeTag::Text {
            text.push_str(&child.text);
        } else {
            pending.extend(child.children.iter().copied().rev());
        }
    }
    text
}

impl NativeView {
    fn build_tree(
        &mut self,
        cx: &mut ViewContext<'_, Self>,
        scope: Option<u32>,
    ) -> Option<Element> {
        let window = self.handles.as_ref().map_or(self.window, |handles| {
            handles
                .borrow()
                .get(&cx.window_handle())
                .copied()
                .expect("a native binding view must retain its core window handle")
        });
        let tree = self.tree.borrow();
        if scope.is_none() {
            self.components.sync(&tree, window, &self.events);
        }
        let components = &mut self.components;
        let mut extensions = self.extensions.borrow_mut();
        extensions.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::Extension)
        });
        let mut svgs = self.svgs.borrow_mut();
        svgs.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::Svg)
        });
        let mut lists = self.lists.borrow_mut();
        lists.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::VirtualList)
        });
        let mut images = self.images.borrow_mut();
        images.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::Image)
        });
        let mut background_images = self.background_images.borrow_mut();
        background_images.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.string(property::BACKGROUND_IMAGE).is_some())
        });
        let mut shaders = self.shaders.borrow_mut();
        shaders.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::Shader)
        });
        self.menus.borrow_mut().retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.string(property::PART) == Some(POPOVER_MENU_POPUP_PART))
        });
        let context_menu = self.context_menu;
        let context_menu_owner = self.context_menu_owner;
        #[cfg(target_os = "macos")]
        let mut swift_ui_hosts = self.swift_ui_hosts.borrow_mut();
        #[cfg(target_os = "macos")]
        let embedded_views = self.embedded_views.borrow();
        #[cfg(target_os = "macos")]
        swift_ui_hosts.retain(|id, _| {
            tree.nodes
                .get(id)
                .is_some_and(|node| node.tag == NodeTag::SwiftUiHost)
        });
        let mut root = div()
            .id(ElementId::new(ROOT_ELEMENT_ID))
            .size_full()
            .min_w(0.0)
            .min_h(0.0);
        if let Some(node) = tree.nodes.get(&ROOT_NODE) {
            let mut part_ids = HashSet::new();
            let mut states = NativeElementStates {
                extensions: &mut extensions,
                svgs: &mut svgs,
                lists: &mut lists,
                part_ids: &mut part_ids,
                portals: Vec::new(),
                images: &mut images,
                background_images: &mut background_images,
                shaders: &mut shaders,
                components,
                menus: Rc::clone(&self.menus),
                context_menu,
                context_menu_owner,
                #[cfg(target_os = "macos")]
                swift_ui_hosts: &mut swift_ui_hosts,
                #[cfg(target_os = "macos")]
                embedded_views: &embedded_views,
            };
            if let Some(id) = scope {
                let mut current = id;
                let mut depth = 0;
                while let Some(parent) = tree.nodes.get(&current).and_then(|node| node.parent) {
                    if parent == ROOT_NODE {
                        break;
                    }
                    depth += 1;
                    current = parent;
                }
                let element =
                    build_element(id, window, &tree, &self.events, &mut states, cx, depth)?;
                if !states.portals.is_empty() || element.is_viewport_portal() {
                    return None;
                }
                return Some(element);
            }
            root = root.children(node.children.iter().filter_map(|id| {
                build_element(*id, window, &tree, &self.events, &mut states, cx, 0)
                    .and_then(|element| hoist_portal(element, &mut states))
            }));
            root = root.children(std::mem::take(&mut states.portals));
        }
        Some(root)
    }
}

pub(super) fn build_element(
    id: u32,
    window: u32,
    tree: &NativeTree,
    events: &EventQueue,
    states: &mut NativeElementStates<'_>,
    cx: &mut ViewContext<'_, NativeView>,
    depth: usize,
) -> Option<Element> {
    if native_scope_identity(tree, id) {
        let mut present = false;
        let element = cx.with_scope(ElementId::new(u64::from(id)), |cx| {
            let element = build_element_inner(id, window, tree, events, states, cx, depth, true);
            present = element.is_some();
            element.unwrap_or_else(|| div().hidden())
        });
        present.then_some(element)
    } else {
        build_element_inner(id, window, tree, events, states, cx, depth, true)
    }
}

/// Build one declared node's own element — its styles, states, and semantics — without its
/// children, for a container whose children the core lays out itself, such as a table row.
pub(super) fn build_element_shell(
    id: u32,
    window: u32,
    tree: &NativeTree,
    events: &EventQueue,
    states: &mut NativeElementStates<'_>,
    cx: &mut ViewContext<'_, NativeView>,
    depth: usize,
) -> Option<Element> {
    build_element_inner(id, window, tree, events, states, cx, depth, false)
}

#[allow(clippy::too_many_arguments)]
fn build_element_inner(
    id: u32,
    window: u32,
    tree: &NativeTree,
    events: &EventQueue,
    states: &mut NativeElementStates<'_>,
    cx: &mut ViewContext<'_, NativeView>,
    depth: usize,
    with_children: bool,
) -> Option<Element> {
    if depth >= MAX_TREE_DEPTH {
        return None;
    }
    let node = tree.nodes.get(&id)?;
    // A component part adopts the Rust descriptor's derived identity so the core's own
    // `aria-controls`/`labelled-by` relationships and mount policy resolve without a JavaScript
    // registry. Ordinary nodes keep their unique node identity on the untouched fast path.
    let part_element_id = native_part_element_id_with(id, node, states.components);
    // A declared collection header, row, or cell is content, not identity: the core assigns the
    // exact grid, tree-item, and active-descendant identity itself, and an element can carry only
    // one stable id. These nodes therefore mount without one and register no listener of their
    // own; every interaction on them belongs to the core's own row and cell decorators.
    let core_owned_identity = matches!(
        node.string(property::PART),
        Some(TABLE_HEADER_PART | TABLE_ROW_PART | TABLE_CELL_PART | TREE_ROW_PART)
    );
    let listeners_enabled = !core_owned_identity
        && match part_element_id {
            Some(part_element_id) => states.part_ids.insert(part_element_id.as_u64()),
            None => true,
        };
    let element_id = part_element_id.unwrap_or_else(|| ElementId::new(id as u64));
    // A Base UI part whose activation, editing, gesture, or dismissal the core owns registers
    // exactly one listener for that identity. The declared listener is dropped rather than
    // registered twice; the result still reaches JavaScript through `componentchange`.
    let declared_part = node.string(property::PART).unwrap_or("");
    let mut element = match node.tag {
        NodeTag::Root => return None,
        NodeTag::View => div(),
        NodeTag::Button => button().cursor_default(),
        NodeTag::Text => text(node.text.clone()),
        NodeTag::Sentinel => div().hidden(),
        NodeTag::Input => {
            let multiline = node.boolean(property::MULTILINE).unwrap_or(false);
            // A number field's editing text belongs to the core: it parses, clamps, and reformats
            // it, so the declared `value` seeds the state instead of overwriting it every frame.
            // An OTP slot's character belongs to the core in exactly the same way: the retained
            // state owns every slot, so the declared `value` on the root seeds it and the slot
            // itself renders whatever the core decided.
            let otp_slot_text = match node.string(property::PART) {
                Some(OTP_FIELD_INPUT_PART) => states
                    .components
                    .base_ui
                    .otp_fields
                    .get(&component_key(id, node))
                    .map(|field| field.state.slot_text(declared_index(node))),
                _ => None,
            };
            let declared_value = match node.string(property::PART) {
                Some(NUMBER_FIELD_INPUT_PART) => states
                    .components
                    .number_fields
                    .get(&component_key(id, node))
                    .map_or_else(
                        || node.string(property::VALUE).unwrap_or_default(),
                        |field| field.state.text().as_ref(),
                    ),
                Some(OTP_FIELD_INPUT_PART) => otp_slot_text.as_deref().unwrap_or_default(),
                _ => node.string(property::VALUE).unwrap_or_default(),
            };
            let mut input = if multiline {
                text_area(declared_value)
            } else {
                text_input(declared_value)
            }
            .bg(Color::TRANSPARENT)
            .border(0.0, Color::TRANSPARENT)
            .rounded(0.0);
            if let Some(placeholder) = node.string(property::PLACEHOLDER) {
                input = input.placeholder(placeholder);
            }
            if !multiline && node.boolean(property::PASSWORD).unwrap_or(false) {
                input = input.password(true);
            }
            if listeners_enabled
                && !base_ui_owns_input(declared_part)
                && node.boolean(property::INPUT_LISTENER).unwrap_or(false)
            {
                let events = Rc::clone(events);
                // The retained input state already schedules its paint. Rebuilding here would read
                // the previous JavaScript-controlled value before Bun drains this queued event,
                // resetting every keystroke before Solid can commit the matching mutation batch.
                let listener = cx.input_listener(element_id, move |_view, value, _cx| {
                    enqueue_event(
                        &events,
                        QueuedEvent {
                            kind: "input",
                            window,
                            target: id,
                            value: Some(Arc::from(value)),
                        },
                    );
                });
                input = input.on_input(listener);
            }
            if listeners_enabled
                && !multiline
                && node.boolean(property::SUBMIT_LISTENER).unwrap_or(false)
            {
                let events = Rc::clone(events);
                // Submit has the same controlled-state boundary as input: JavaScript must consume
                // the queued value before a render can safely read the controlled property again.
                let listener = cx.submit_listener(element_id, move |_view, value, _cx| {
                    enqueue_event(
                        &events,
                        QueuedEvent {
                            kind: "submit",
                            window,
                            target: id,
                            value: Some(Arc::from(value)),
                        },
                    );
                });
                input = input.on_submit(listener);
            }
            input
        }
        NodeTag::Extension => {
            let package = node.string(property::EXTENSION_PACKAGE).unwrap_or_default();
            let name = node
                .string(property::EXTENSION_COMPONENT)
                .unwrap_or_default();
            let props = node.string(property::EXTENSION_PROPS).unwrap_or("{}");
            let result = (|| -> Result<Element, String> {
                let mut props: serde_json::Value =
                    serde_json::from_str(props).map_err(|error| error.to_string())?;
                // Standard root styles are available to every component package, including
                // bindings whose reusable style builder is opaque to application code.
                if let Some(properties) = props.as_object_mut() {
                    let style = properties
                        .entry("style")
                        .or_insert_with(|| serde_json::json!({}));
                    if !style.is_object() {
                        *style = serde_json::json!({});
                    }
                    for (key, name) in [
                        (property::FONT_SIZE, "fontSize"),
                        (property::LINE_HEIGHT, "lineHeight"),
                        (property::FONT_FAMILY, "fontFamily"),
                        (property::COLOR, "color"),
                        (property::BACKGROUND_COLOR, "backgroundColor"),
                        (property::BORDER_COLOR, "borderColor"),
                        (property::BORDER_WIDTH, "borderWidth"),
                        (property::BORDER_RADIUS, "borderRadius"),
                        (property::PADDING, "padding"),
                        (property::PADDING_TOP, "paddingTop"),
                        (property::PADDING_RIGHT, "paddingRight"),
                        (property::PADDING_BOTTOM, "paddingBottom"),
                        (property::PADDING_LEFT, "paddingLeft"),
                    ] {
                        if let Some(value) = node.property(key) {
                            style[name] = match value {
                                PropertyValue::Number(value) => serde_json::json!(value),
                                PropertyValue::Color(value) => serde_json::json!(value),
                                PropertyValue::String(value) => serde_json::json!(value.as_ref()),
                                PropertyValue::Bool(value) => serde_json::json!(value),
                            };
                        }
                    }
                }
                if states
                    .extensions
                    .get(&id)
                    .is_none_or(|state| state.package != package || state.name != name)
                {
                    let events = Rc::clone(events);
                    let component = quickgui::ExtensionComponent::new(
                        package,
                        name,
                        props.clone(),
                        element_id,
                        cx.window_invalidator(),
                        move |kind, value| {
                            enqueue_event(
                                &events,
                                QueuedEvent {
                                    kind: "componentchange",
                                    window,
                                    target: id,
                                    value: Some(
                                        serde_json::json!({"kind":kind,"value":value})
                                            .to_string()
                                            .into(),
                                    ),
                                },
                            );
                        },
                    )?;
                    states.extensions.insert(
                        id,
                        NativeExtensionState {
                            package: package.into(),
                            name: name.into(),
                            component,
                        },
                    );
                }
                let component = &states.extensions.get(&id).unwrap().component;
                component.set_props(&props)?;
                component.element(cx)
            })();
            result.unwrap_or_else(|error| {
                div()
                    .child(text(error))
                    .text_color(Color::rgb8(248, 113, 113))
            })
        }
        NodeTag::Image => {
            let source = node.string(property::VALUE).unwrap_or_default();
            let state = states
                .images
                .entry(id)
                .or_insert_with(|| NativeImageState::new(Arc::from(source)));
            state.sync(source);
            state.element(node)
        }
        NodeTag::Shader => {
            let source = node.string(property::VALUE).unwrap_or_default();
            let state = states
                .shaders
                .entry(id)
                .or_insert_with(|| NativeShaderState::new(Arc::from(source)));
            state.sync(source);
            state.element(node)
        }
        NodeTag::Svg => {
            let source = node.string(property::VALUE).unwrap_or_default();
            let state = states
                .svgs
                .entry(id)
                .or_insert_with(|| NativeSvgState::new(Arc::from(source)));
            state.sync(source);
            state.element()
        }
        NodeTag::VirtualList => div(),
        NodeTag::SwiftUiHost => {
            #[cfg(target_os = "macos")]
            {
                let state = states.swift_ui_hosts.entry(id).or_insert_with(|| {
                    NativeSwiftUiHostState::new(window, events, cx.window_invalidator())
                });
                match state.element(node, tree, states.embedded_views) {
                    Ok(element) => element,
                    Err(error) => div()
                        .size_full()
                        .bg(Color::rgb8(255, 255, 255))
                        .text_color(Color::rgb8(185, 28, 28))
                        .p_4()
                        .child(text(format!("SwiftUI host error: {error}"))),
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                div().hidden()
            }
        }
        NodeTag::SwiftUiButton
        | NodeTag::SwiftUiSlider
        | NodeTag::SwiftUiToggle
        | NodeTag::SwiftUiProgressView
        | NodeTag::SwiftUiStepper
        | NodeTag::SwiftUiTextField
        | NodeTag::SwiftUiPicker
        | NodeTag::SwiftUiDatePicker
        | NodeTag::SwiftUiColorPicker
        | NodeTag::SwiftUiGauge
        | NodeTag::SwiftUiQuickGuiHost
        | NodeTag::SwiftUiPopover
        | NodeTag::SwiftUiPopoverTrigger
        | NodeTag::SwiftUiPopoverContent => return None,
    };
    if !core_owned_identity {
        element = element.id(element_id);
    }

    element = apply_properties(element, node);
    element = apply_background_image(element, id, node, states.background_images);
    element = apply_tooltip(element, node);
    // The core part descriptor decides identity, semantics, and whether an inactive panel is
    // mounted at all. A part that is not mounted contributes no layout, paint, input, or
    // accessibility node, exactly as the Rust component guides describe.
    element = apply_part(element, id, node)?;
    element = apply_component_part(element, id, node, states.components, cx, listeners_enabled);
    if let Some(part) = node.string(property::PART) {
        element = apply_field_part(
            element,
            part,
            id,
            window,
            node,
            events,
            states.components,
            cx,
            listeners_enabled,
        )?;
        element = apply_picker_part(
            element,
            part,
            id,
            window,
            node,
            events,
            states.components,
            cx,
            listeners_enabled,
        )?;
        if owns_base_ui_part(part) {
            element = apply_base_ui_part(
                element,
                part,
                id,
                node,
                states.components,
                cx,
                listeners_enabled,
            )?;
        }
        element = apply_dialog_part(
            element,
            part,
            id,
            node,
            states.components,
            cx,
            listeners_enabled,
        )?;
        if owns_popover_part(part) {
            element = apply_popover_part(
                element,
                part,
                id,
                node,
                states.components,
                cx,
                listeners_enabled,
            )?;
        }
        if owns_menu_part(part) {
            element = apply_menu_part(
                element,
                part,
                id,
                window,
                node,
                events,
                states.components,
                cx,
                listeners_enabled,
            )?;
        }
    }
    element = apply_controls(element, node, tree);

    let part = node.string(property::PART);
    if part == Some(POPOVER_MENU_POPUP_PART) {
        // The declared model is retained across renders so the core keeps its highlighted item,
        // typeahead prefix, and toggle state through an atomic source replacement.
        let source = node.string(property::MENU).unwrap_or("");
        {
            let mut menus = states.menus.borrow_mut();
            match menus.get_mut(&id) {
                Some(state) => state.sync(id, source),
                None => {
                    if let Some(state) = NativeMenuState::new(id, source) {
                        menus.insert(id, state);
                    }
                }
            }
        }
        if listeners_enabled {
            let menus = Rc::clone(&states.menus);
            element =
                popover_menu_surface(element, element_id, id, window, events, &menus, node, cx);
        }
    }
    if listeners_enabled && part == Some(CONTEXT_MENU_TRIGGER_PART) {
        let state = if states.context_menu_owner == Some(id) {
            states.context_menu
        } else {
            ContextMenuState::new()
        };
        element = apply_context_menu(element, element_id, id, node, tree, state, cx);
    }

    let anchor_id = node
        .string(property::ANCHOR_TARGET)
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|anchor_id| *anchor_id != id && tree.nodes.contains_key(anchor_id));
    if let Some(anchor_id) = anchor_id {
        let dismiss_on_escape = node.boolean(property::DISMISS_ON_ESCAPE).unwrap_or(true);
        let dismiss_on_pointer_outside = node
            .boolean(property::DISMISS_ON_POINTER_OUTSIDE)
            .unwrap_or(true);
        // The anchor is another retained node, so it is anchored to the identity that node
        // really mounts under rather than to its raw node id.
        let anchor_element_id = tree
            .nodes
            .get(&anchor_id)
            .and_then(|anchor| native_part_element_id(anchor_id, anchor))
            .unwrap_or_else(|| ElementId::new(u64::from(anchor_id)));
        let mut popover = Popover::new(anchor_element_id, element_id, true)
            .kind(if part == Some(POPOVER_MENU_POPUP_PART) {
                PopoverKind::Menu
            } else {
                PopoverKind::Dialog
            })
            .placement(
                node.string(property::ANCHOR_PLACEMENT)
                    .and_then(parse_anchor_placement)
                    .unwrap_or_default(),
            )
            .dismiss_on_escape(dismiss_on_escape)
            .dismiss_on_pointer_outside(dismiss_on_pointer_outside);
        if let Some(gap) = node.number(property::ANCHOR_GAP) {
            popover = popover.anchor_gap(gap);
        }
        if let Some(margin) = node.number(property::VIEWPORT_MARGIN) {
            popover = popover.viewport_margin(margin);
        }
        element = popover.surface_with(element);

        element = attach_dismiss_listener(
            element,
            listeners_enabled
                && !base_ui_owns_dismiss(declared_part)
                && !popover_part_owns_dismiss(declared_part)
                && !menu_part_owns_dismiss(declared_part)
                && node.boolean(property::DISMISS_LISTENER).unwrap_or(false)
                && (dismiss_on_escape || dismiss_on_pointer_outside),
            element_id,
            id,
            window,
            events,
            cx,
        );
    } else {
        let dismiss_on_escape = node.boolean(property::DISMISS_ON_ESCAPE).unwrap_or(false);
        let dismiss_on_pointer_outside = node
            .boolean(property::DISMISS_ON_POINTER_OUTSIDE)
            .unwrap_or(false);
        if dismiss_on_escape {
            element = element.dismiss_on_escape();
        }
        if dismiss_on_pointer_outside {
            element = element.dismiss_on_pointer_outside();
        }
        element = attach_dismiss_listener(
            element,
            listeners_enabled
                && !base_ui_owns_dismiss(declared_part)
                && !popover_part_owns_dismiss(declared_part)
                && !menu_part_owns_dismiss(declared_part)
                && node.boolean(property::DISMISS_LISTENER).unwrap_or(false)
                && (dismiss_on_escape || dismiss_on_pointer_outside),
            element_id,
            id,
            window,
            events,
            cx,
        );
    }

    if listeners_enabled
        && !base_ui_owns_click(declared_part)
        && !menu_part_owns_click(declared_part)
        && node.boolean(property::CLICK_LISTENER).unwrap_or(false)
    {
        let events = Rc::clone(events);
        let listener = cx.listener(element_id, move |_view, _cx| {
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: "click",
                    window,
                    target: id,
                    value: None,
                },
            );
        });
        element = element.on_click(listener);
    }
    if listeners_enabled && node.boolean(property::HOVER_LISTENER).unwrap_or(false) {
        let events = Rc::clone(events);
        let listener = cx.hover_listener(element_id, move |_view, hovered, _cx| {
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: if *hovered { "mouseenter" } else { "mouseleave" },
                    window,
                    target: id,
                    value: None,
                },
            );
        });
        element = element.on_hover(listener);
    }
    if listeners_enabled
        && !base_ui_owns_pointer(declared_part)
        && node.boolean(property::POINTER_LISTENER).unwrap_or(false)
    {
        let events = Rc::clone(events);
        let listener = cx.pointer_listener(element_id, move |_view, event, cx| {
            enqueue_event(
                &events,
                QueuedEvent {
                    kind: "pointer",
                    window,
                    target: id,
                    value: Some(pointer_event_json(event).into()),
                },
            );
            cx.prevent_default();
            cx.stop_propagation();
        });
        element = element.on_pointer(listener);
    }

    if listeners_enabled {
        element = attach_input_listeners(
            element,
            element_id,
            id,
            window,
            events,
            node,
            cx,
            base_ui_owns_scroll_wheel(declared_part),
        );
    }

    // A declared collection owns its subtree: the binding builds the declared header, row, and
    // cell elements itself and hands the core finished elements, so the recursion below never
    // reaches them twice.
    match node.string(property::PART) {
        Some(TABLE_PART) => {
            return Some(build_table(
                element, id, window, tree, events, states, cx, depth,
            ));
        }
        Some(TREE_PART) => {
            return Some(build_tree(
                element, id, window, tree, events, states, cx, depth,
            ));
        }
        _ => {}
    }

    match node.tag {
        NodeTag::VirtualList => {
            let state = states
                .lists
                .entry(id)
                .or_insert_with(|| NativeListState::new(node));
            state.sync(node);
            let list = state.list.clone();
            let children = state.children.clone();
            let visible = list.visible_rows().range;
            let gap = node
                .number(property::ROW_GAP)
                .or_else(|| node.number(property::GAP))
                .unwrap_or(0.0)
                .max(0.0);
            let alignment = node.string(property::ALIGN_ITEMS);
            let item_count = children.len();
            let rows = list.render_rows(visible, |index| {
                let child =
                    build_element(children[index], window, tree, events, states, cx, depth + 1)
                        .unwrap_or_else(|| div().hidden());
                let mut row = div().w_full().flex_none().flex_row().child(child);
                row = match alignment {
                    Some("center") => row.justify_center(),
                    Some("flex-end" | "end") => row.justify_end(),
                    _ => row.justify_start(),
                };
                if gap > 0.0 && index + 1 < item_count {
                    row = row.padding(0.0, 0.0, gap, 0.0);
                }
                row
            });
            element = element.child(rows).variable_virtual_scroll(&list);
        }
        NodeTag::Text
        | NodeTag::Sentinel
        | NodeTag::Input
        | NodeTag::Svg
        | NodeTag::Image
        | NodeTag::Shader
        | NodeTag::Extension => {}
        NodeTag::SwiftUiHost
        | NodeTag::SwiftUiButton
        | NodeTag::SwiftUiSlider
        | NodeTag::SwiftUiToggle
        | NodeTag::SwiftUiProgressView
        | NodeTag::SwiftUiStepper
        | NodeTag::SwiftUiTextField
        | NodeTag::SwiftUiPicker
        | NodeTag::SwiftUiDatePicker
        | NodeTag::SwiftUiColorPicker
        | NodeTag::SwiftUiGauge
        | NodeTag::SwiftUiQuickGuiHost
        | NodeTag::SwiftUiPopover
        | NodeTag::SwiftUiPopoverTrigger
        | NodeTag::SwiftUiPopoverContent => {}
        NodeTag::Root | NodeTag::View | NodeTag::Button => {
            if with_children {
                element = element.children(node.children.iter().filter_map(|child| {
                    build_element(*child, window, tree, events, states, cx, depth + 1)
                        .and_then(|element| hoist_portal(element, states))
                }));
            }
        }
    }
    Some(element)
}

pub(super) struct NativeExtensionState {
    package: String,
    name: String,
    component: quickgui::ExtensionComponent,
}

/// Keep an ordinary child in place, or lift a viewport portal out to mount under the window root.
fn hoist_portal(element: Element, states: &mut NativeElementStates<'_>) -> Option<Element> {
    if !element.is_viewport_portal() || states.portals.len() >= MAX_HOSTED_PORTALS {
        return Some(element);
    }
    states.portals.push(element);
    None
}

/// Portals lifted to the window root per render pass; anything beyond this stays in place.
const MAX_HOSTED_PORTALS: usize = 64;

pub(super) fn attach_dismiss_listener(
    mut element: Element,
    enabled: bool,
    element_id: ElementId,
    node_id: u32,
    window: u32,
    events: &EventQueue,
    cx: &mut ViewContext<'_, NativeView>,
) -> Element {
    if !enabled {
        return element;
    }
    let events = Rc::clone(events);
    let listener = cx.dismiss_listener(element_id, move |_view, _cx| {
        enqueue_event(
            &events,
            QueuedEvent {
                kind: "dismiss",
                window,
                target: node_id,
                value: None,
            },
        );
    });
    element = element.on_dismiss(listener);
    element
}

pub(super) fn enqueue_event(events: &EventQueue, event: QueuedEvent) {
    let first = {
        let mut events = events.borrow_mut();
        if events.len() >= MAX_QUEUED_EVENTS {
            return;
        }
        events.push_back(event);
        events.len() == 1
    };
    // Events are published to the application thread only between two pumps. The pump returns
    // on its own after a redraw or an OS wake-up, but an event produced by a dispatch that
    // triggers neither (a click that changes nothing visually) would otherwise wait for the next
    // mouse move. One wake per queue fill keeps the cost bounded under pointer-move bursts.
    if first {
        HOST.wake();
    }
}

pub(super) fn pointer_event_json(event: &quickgui::PointerEvent) -> String {
    let phase = match event.phase {
        PointerPhase::Down => "down",
        PointerPhase::Move => "move",
        PointerPhase::Up => "up",
        PointerPhase::Cancel => "cancel",
    };
    let button = match event.button {
        quickgui::MouseButton::Left => "left",
        quickgui::MouseButton::Right => "right",
        quickgui::MouseButton::Middle => "middle",
        quickgui::MouseButton::Back => "back",
        quickgui::MouseButton::Forward => "forward",
        quickgui::MouseButton::Other(_) => "other",
    };
    serde_json::json!({
        "phase": phase,
        "position": { "x": event.position.x, "y": event.position.y },
        "origin": { "x": event.origin.x, "y": event.origin.y },
        "localPosition": { "x": event.local_position.x, "y": event.local_position.y },
        "localOrigin": { "x": event.local_origin.x, "y": event.local_origin.y },
        "delta": { "x": event.delta.x, "y": event.delta.y },
        "button": button,
    })
    .to_string()
}

/// Bounded compound scope key shared by every part of one component instance.
///
/// The renderer allocates the key; the Rust binding hashes it into the same [`ElementId`] the
/// core component would have used, so every derived part identity matches without a registry.
pub(super) fn native_part_scope(id: u32, node: &NativeNode) -> ElementId {
    match node
        .string(property::SCOPE)
        .filter(|scope| !scope.is_empty() && scope.len() <= MAX_COMPONENT_VALUE_BYTES)
    {
        Some(scope) => ElementId::named(scope),
        None => ElementId::new(id as u64),
    }
}

pub(super) fn native_part_value(node: &NativeNode, key: u16) -> Option<ElementId> {
    node.string(key)
        .filter(|value| !value.is_empty() && value.len() <= MAX_COMPONENT_VALUE_BYTES)
        .map(ElementId::named)
}

/// Build the declared checkbox descriptor.
///
/// A checkbox that declares `parent` folds its declared children's checked booleans through the
/// core, so its on/mixed/off state is derived rather than retained separately anywhere.
fn native_checkbox(node: &NativeNode) -> Checkbox {
    let checkbox = if node.boolean(property::PARENT).unwrap_or(false) {
        Checkbox::parent(declared_flags(node, property::VALUES))
    } else {
        Checkbox::new(native_toggle_state(node))
    };
    checkbox.read_only(node.boolean(property::READ_ONLY).unwrap_or(false))
}

fn native_toggle_state(node: &NativeNode) -> ToggleState {
    if node.boolean(property::INDETERMINATE) == Some(true) {
        ToggleState::Mixed
    } else {
        ToggleState::from(node.boolean(property::CHECKED).unwrap_or(false))
    }
}

fn native_tabs(id: u32, node: &NativeNode) -> Tabs {
    let scope = native_part_scope(id, node);
    let tabs = match native_part_value(node, property::ACTIVE_VALUE) {
        Some(active) => Tabs::new(scope, active),
        None => Tabs::without_selection(scope),
    };
    let tabs = if node.string(property::ORIENTATION) == Some("vertical") {
        tabs.vertical()
    } else {
        tabs
    };
    tabs.activate_on_focus(node.boolean(property::ACTIVATE_ON_FOCUS).unwrap_or(false))
        .loop_focus(node.boolean(property::LOOP_FOCUS).unwrap_or(true))
        .keep_mounted(node.boolean(property::KEEP_MOUNTED).unwrap_or(false))
}

fn native_tab(id: u32, node: &NativeNode) -> Option<Tab> {
    let value = native_part_value(node, property::PART_VALUE)?;
    Some(
        native_tabs(id, node)
            .tab(value)
            .disabled(node.boolean(property::DISABLED).unwrap_or(false)),
    )
}

fn native_collapsible(id: u32, node: &NativeNode) -> Collapsible {
    Collapsible::new(
        native_part_scope(id, node),
        node.boolean(property::OPEN).unwrap_or(false),
    )
    .disabled(node.boolean(property::DISABLED).unwrap_or(false))
    .keep_mounted(node.boolean(property::KEEP_MOUNTED).unwrap_or(false))
}

fn native_accordion_item(id: u32, node: &NativeNode) -> Option<AccordionItem> {
    let value = native_part_value(node, property::PART_VALUE)?;
    let index = node
        .number(property::ITEM_INDEX)
        .unwrap_or(0.0)
        .clamp(0.0, MAX_NODES as f32) as usize;
    let mut accordion = Accordion::new(native_part_scope(id, node))
        .keep_mounted(node.boolean(property::KEEP_MOUNTED).unwrap_or(false));
    if let Some(level) = node.number(property::HEADING_LEVEL) {
        accordion = accordion.heading_level(level.clamp(1.0, 6.0) as usize);
    }
    Some(
        accordion
            .item(value, index, node.boolean(property::OPEN).unwrap_or(false))
            .disabled(node.boolean(property::DISABLED).unwrap_or(false)),
    )
}

pub(super) fn native_field(id: u32, node: &NativeNode) -> Field {
    let field = Field::new(native_part_scope(id, node))
        .validation_mode(declared_validation_mode(node))
        .validation_debounce(declared_validation_debounce(node))
        .disabled(node.boolean(property::DISABLED).unwrap_or(false))
        .invalid(node.boolean(property::INVALID).unwrap_or(false))
        .required(node.boolean(property::REQUIRED).unwrap_or(false))
        .touched(node.boolean(property::TOUCHED).unwrap_or(false))
        .dirty(node.boolean(property::DIRTY).unwrap_or(false))
        .filled(node.boolean(property::FILLED).unwrap_or(false));
    match node.string(property::VALIDATION_MESSAGE) {
        Some(message) => field.validation_message(message),
        None => field,
    }
}

pub(super) fn native_dialog(id: u32, node: &NativeNode) -> CoreDialog {
    let kind = match node.string(property::VARIANT) {
        Some("alertdialog") => DialogKind::AlertDialog,
        _ => DialogKind::Dialog,
    };
    let mut dialog = CoreDialog::with_kind(
        native_part_scope(id, node),
        node.boolean(property::OPEN).unwrap_or(false),
        kind,
    );
    if let Some(dismiss) = node.boolean(property::DISMISS_ON_ESCAPE) {
        dialog = dialog.dismiss_on_escape(dismiss);
    }
    if let Some(dismiss) = node.boolean(property::DISMISS_ON_POINTER_OUTSIDE) {
        dialog = dialog.dismiss_on_backdrop(dismiss);
    }
    dialog
}

fn native_fieldset(id: u32, node: &NativeNode) -> Fieldset {
    Fieldset::new(native_part_scope(id, node))
        .disabled(node.boolean(property::DISABLED).unwrap_or(false))
}

/// Resolve a part identity that depends on retained per-instance state, such as a queued toast.
///
/// Everything else falls through to [`native_part_element_id`], which needs no state at all.
pub(super) fn native_part_element_id_with(
    id: u32,
    node: &NativeNode,
    components: &NativeComponentStates,
) -> Option<ElementId> {
    let part = node.string(property::PART)?;
    if let Some(derived) = field_part_element_id(part, id, node, components) {
        return Some(derived);
    }
    if let Some(derived) = base_ui_part_element_id(part, id, node, components) {
        return Some(derived);
    }
    if let Some(derived) = popover_part_element_id(part, id, node, components) {
        return Some(derived);
    }
    if owns_menu_part(part)
        && let Some(derived) = menu_part_element_id(part, id, node, components)
    {
        return Some(derived);
    }
    native_part_element_id(id, node)
}

/// Resolve the identity a component part must mount with, or `None` for an ordinary node.
pub(super) fn native_part_element_id(id: u32, node: &NativeNode) -> Option<ElementId> {
    let part = node.string(property::PART)?;
    if let Some(component) = component_part_element_id(id, node) {
        return Some(component);
    }
    if let Some(picker) = picker_part_element_id(part, id, node) {
        return Some(picker);
    }
    Some(match part {
        "tabs" | "collapsible" | "accordion" | "fieldset" | "field-control" => {
            native_part_scope(id, node)
        }
        "tabs-list" => native_tabs(id, node).list_id(),
        "tab" => native_tab(id, node)?.tab_id(),
        "tab-indicator" => native_tab(id, node)?.indicator_id(),
        "tab-panel" => native_tab(id, node)?.panel_id(),
        "collapsible-trigger" => native_collapsible(id, node).trigger_id(),
        "collapsible-panel" => native_collapsible(id, node).panel_id(),
        "accordion-item" => native_accordion_item(id, node)?.root_id(),
        "accordion-header" => native_accordion_item(id, node)?.header_id(),
        "accordion-trigger" => native_accordion_item(id, node)?.trigger_id(),
        "accordion-panel" => native_accordion_item(id, node)?.panel_id(),
        "field" => native_field(id, node).root_id(),
        "field-label" | "field-passive-label" => native_field(id, node).label_id(),
        "field-description" => native_field(id, node).description_id(),
        "field-error" => native_field(id, node).error_id(),
        "field-item" => native_field(id, node).item_id(),
        "field-validity" => native_field(id, node).validity_id(),
        "progress" | "meter" => native_part_scope(id, node),
        "progress-track" => native_progress(id, node).track_id()?,
        "progress-indicator" => native_progress(id, node).indicator_id()?,
        "progress-label" => native_progress(id, node).label_id()?,
        "progress-value" => native_progress(id, node).value_id()?,
        "meter-track" => native_meter(id, node).track_id()?,
        "meter-indicator" => native_meter(id, node).indicator_id()?,
        "meter-label" => native_meter(id, node).label_id()?,
        "meter-value" => native_meter(id, node).value_id()?,
        "dialog-viewport" => native_dialog(id, node).viewport_id(),
        "fieldset-legend" => native_fieldset(id, node).legend_id(),
        "fieldset-description" => native_fieldset(id, node).description_id(),
        "dialog" => native_dialog(id, node).root_id(),
        "dialog-backdrop" => native_dialog(id, node).backdrop_id(),
        "dialog-popup" => native_dialog(id, node).popover_id(),
        "dialog-title" => native_dialog(id, node).title_id(),
        "dialog-description" => native_dialog(id, node).description_id(),
        "dialog-close" => native_dialog(id, node).close_id(),
        // The stateless halves of the Base UI-aligned popover and tooltip compounds, so an
        // anchored surface elsewhere in the tree resolves the same identity the part mounts under.
        POPOVER_TRIGGER_PART => popover_trigger_id(native_part_scope(id, node)),
        POPOVER_POPUP_PART => popover_surface_id(native_part_scope(id, node)),
        TOOLTIP_TRIGGER_PART => tooltip_trigger_id(native_part_scope(id, node)),
        TOOLTIP_POPUP_PART => tooltip_popup_id(native_part_scope(id, node)),
        _ => return None,
    })
}

/// Apply the Rust core part descriptor named by the `part` property.
///
/// `None` means the core decided this part is not mounted, such as an inactive tab panel or a
/// closed collapsible panel without `keepMounted`.
pub(super) fn apply_part(element: Element, id: u32, node: &NativeNode) -> Option<Element> {
    let Some(part) = node.string(property::PART) else {
        return Some(element);
    };
    Some(match part {
        "checkbox" => native_checkbox(node).root_with(element),
        "checkbox-indicator" => Checkbox::new(ToggleState::Off).indicator_with(element),
        "radio" => Radio::new(node.boolean(property::CHECKED).unwrap_or(false))
            .read_only(node.boolean(property::READ_ONLY).unwrap_or(false))
            .root_with(element),
        "radio-indicator" => Radio::new(false).indicator_with(element),
        "radio-group" => RadioGroup::new()
            .read_only(node.boolean(property::READ_ONLY).unwrap_or(false))
            .required(node.boolean(property::REQUIRED).unwrap_or(false))
            .root_with(element),
        "switch" => Switch::new(node.boolean(property::CHECKED).unwrap_or(false))
            .read_only(node.boolean(property::READ_ONLY).unwrap_or(false))
            .root_with(element),
        "switch-thumb" => Switch::new(false).thumb_with(element),
        "tabs" => native_tabs(id, node).root_with(element),
        "tabs-list" => native_tabs(id, node).list_with(element),
        "tab" => match native_tab(id, node) {
            Some(tab) => tab.tab_with(element),
            None => element,
        },
        "tab-indicator" => native_tab(id, node)?.indicator_with(element)?,
        "tab-panel" => native_tab(id, node)?.panel_with(element)?,
        "collapsible" => native_collapsible(id, node).root_with(element),
        "collapsible-trigger" => native_collapsible(id, node).trigger_with(element),
        "collapsible-panel" => native_collapsible(id, node).panel_with(element)?,
        "accordion" => Accordion::new(native_part_scope(id, node)).root_with(element),
        "accordion-item" => match native_accordion_item(id, node) {
            Some(item) => item.root_with(element),
            None => element,
        },
        "accordion-header" => match native_accordion_item(id, node) {
            Some(item) => item.header_with(element),
            None => element,
        },
        "accordion-trigger" => match native_accordion_item(id, node) {
            Some(item) => item.trigger_with(element),
            None => element,
        },
        "accordion-panel" => native_accordion_item(id, node)?.panel_with(element)?,
        "field" => native_field(id, node).root_with(element),
        "field-label" => native_field(id, node).label_with(element),
        "field-passive-label" => native_field(id, node).passive_label_with(element),
        "field-control" => native_field(id, node).control_with(element),
        "field-description" => native_field(id, node).description_with(element),
        "field-error" => native_field(id, node).error_with(element),
        "field-item" => native_field(id, node).item_with(element),
        "field-validity" => native_field(id, node)
            .validity_with(node.boolean(property::OPEN).unwrap_or(true), element),
        "progress" => native_progress(id, node).root_with(element),
        "progress-indicator" => native_progress(id, node).indicator_with(element),
        "progress-track" => native_progress(id, node).track_with(element),
        "progress-label" => native_progress(id, node).label_with(element),
        "progress-value" => native_progress(id, node).value_with(element),
        "meter" => native_meter(id, node).root_with(element),
        "meter-indicator" => native_meter(id, node).indicator_with(element),
        "meter-track" => native_meter(id, node).track_with(element),
        "meter-label" => native_meter(id, node).label_with(element),
        "meter-value" => native_meter(id, node).value_with(element),
        "toggle" => {
            Toggle::new(node.boolean(property::PRESSED).unwrap_or(false)).root_with(element)
        }
        "toggle-indicator" => Toggle::new(false).indicator_with(element),
        "fieldset" => native_fieldset(id, node).root_with(element),
        "fieldset-legend" => native_fieldset(id, node).legend_with(element),
        "fieldset-description" => native_fieldset(id, node).description_with(element),
        "fieldset-control" => native_fieldset(id, node).control_with(element),
        // The Rust guide requires the portal root to be mounted only while the dialog is open,
        // so a closed dialog contributes no overlay, focus trap, backdrop, or accessibility node.
        // `apply_dialog_part` owns that decision, because the core may hold a closing dialog
        // mounted for its own exit transition.
        "dialog" => element,
        "dialog-trigger" => {
            native_dialog(id, node).trigger_with(ElementId::new(id as u64), element)
        }
        "dialog-backdrop" => native_dialog(id, node).backdrop_with(element),
        "dialog-popup" => native_dialog(id, node).popup_with(element),
        "dialog-title" => native_dialog(id, node).title_with(element),
        "dialog-description" => native_dialog(id, node).description_with(element),
        // A menu trigger declares only `has-popup` and expansion here; the mounted relationship to
        // its surface travels through the validated `controls` property instead of a guessed id.
        POPOVER_MENU_TRIGGER_PART => {
            Popover::new(ElementId::new(id as u64), ElementId::new(id as u64), false)
                .kind(PopoverKind::Menu)
                .trigger_with(element)
                .accessibility_expanded(node.boolean(property::OPEN).unwrap_or(false))
        }
        "dialog-close" => native_dialog(id, node).close_with(
            node.string(property::ACCESSIBILITY_LABEL)
                .unwrap_or("Close"),
            element,
        ),
        _ => element,
    })
}

/// Materialize a declared CSS grid track list into the core's bounded track descriptors.
///
/// The list is a declaration, so an unparsable track becomes `auto` instead of failing the whole
/// template, and the result is bounded by [`MAX_GRID_TRACKS`].
pub(super) fn native_grid_tracks(value: &str) -> Vec<GridTrack> {
    let mut tracks = Vec::new();
    if value.len() > MAX_GRID_TRACK_LIST_BYTES || value.trim() == "none" {
        return tracks;
    }
    for token in split_grid_tokens(value) {
        if tracks.len() >= MAX_GRID_TRACKS {
            break;
        }
        if let Some(arguments) = function_arguments(&token, "repeat") {
            let mut parts = split_grid_arguments(arguments);
            if parts.len() < 2 {
                continue;
            }
            let count = parts
                .remove(0)
                .trim()
                .parse::<usize>()
                .unwrap_or(0)
                .min(MAX_GRID_TRACKS);
            let repeated = parts
                .iter()
                .flat_map(|part| split_grid_tokens(part))
                .map(|part| native_grid_track(&part))
                .collect::<Vec<_>>();
            for _ in 0..count {
                for track in &repeated {
                    if tracks.len() >= MAX_GRID_TRACKS {
                        break;
                    }
                    tracks.push(*track);
                }
            }
            continue;
        }
        tracks.push(native_grid_track(&token));
    }
    tracks
}

fn native_grid_track(token: &str) -> GridTrack {
    let token = token.trim();
    if let Some(arguments) = function_arguments(token, "minmax") {
        let parts = split_grid_arguments(arguments);
        if let [minimum, maximum] = parts.as_slice() {
            let minimum = grid_length(minimum).unwrap_or(0.0);
            let fraction = grid_fraction(maximum).unwrap_or(1.0);
            return GridTrack::minmax_px_fr(minimum, fraction);
        }
        return GridTrack::auto();
    }
    if let Some(arguments) = function_arguments(token, "fit-content") {
        return GridTrack::fit_content_px(grid_length(arguments).unwrap_or(0.0));
    }
    match token {
        "auto" => GridTrack::auto(),
        "min-content" => GridTrack::min_content(),
        "max-content" => GridTrack::max_content(),
        _ => {
            if let Some(fraction) = grid_fraction(token) {
                return GridTrack::fr(fraction);
            }
            if let Some(percent) = token.strip_suffix('%').and_then(parse_finite) {
                return GridTrack::percent(percent / 100.0);
            }
            match grid_length(token) {
                Some(length) => GridTrack::px(length),
                None => GridTrack::auto(),
            }
        }
    }
}

fn function_arguments<'a>(token: &'a str, name: &str) -> Option<&'a str> {
    let rest = token.strip_prefix(name)?.trim_start();
    rest.strip_prefix('(')?.strip_suffix(')')
}

fn grid_fraction(token: &str) -> Option<f32> {
    token.trim().strip_suffix("fr").and_then(parse_finite)
}

fn grid_length(token: &str) -> Option<f32> {
    let token = token.trim();
    parse_finite(token.strip_suffix("px").unwrap_or(token))
}

fn parse_finite(value: &str) -> Option<f32> {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

/// Split a track list on whitespace while keeping parenthesized functions intact.
fn split_grid_tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    for character in value.chars() {
        match character {
            '(' => {
                depth += 1;
                current.push(character);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            character if character.is_whitespace() && depth == 0 => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            character => current.push(character),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn split_grid_arguments(value: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    for character in value.chars() {
        match character {
            '(' => {
                depth += 1;
                current.push(character);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            ',' if depth == 0 => parts.push(std::mem::take(&mut current)),
            character => current.push(character),
        }
    }
    parts.push(current);
    parts
        .into_iter()
        .map(|part| part.trim().to_owned())
        .filter(|part| !part.is_empty())
        .collect()
}

/// Apply the declared CSS grid template, flow, and item placement.
pub(super) fn apply_grid(mut element: Element, node: &NativeNode) -> Element {
    if let Some(columns) = node.string(property::GRID_TEMPLATE_COLUMNS) {
        element = apply_grid_template(element, columns, false);
    } else if let Some(count) = node.number(property::GRID_TEMPLATE_COLUMNS) {
        element = element.grid_cols(bounded_track_count(count));
    }
    if let Some(rows) = node.string(property::GRID_TEMPLATE_ROWS) {
        element = apply_grid_template(element, rows, true);
    } else if let Some(count) = node.number(property::GRID_TEMPLATE_ROWS) {
        element = element.grid_rows(bounded_track_count(count));
    }
    if let Some(flow) = node.string(property::GRID_AUTO_FLOW) {
        element = match flow {
            "column" | "col" => element.grid_flow_col(),
            "row dense" | "dense row" => element.grid_flow_row_dense(),
            "column dense" | "dense column" | "col dense" => element.grid_flow_col_dense(),
            _ => element.grid_flow_row(),
        };
    }
    if let Some(span) = node.number(property::GRID_COLUMN_SPAN) {
        element = element.col_span(bounded_track_count(span));
    }
    if let Some(start) = node.number(property::GRID_COLUMN_START) {
        element = element.col_start(bounded_grid_line(start));
    }
    if let Some(end) = node.number(property::GRID_COLUMN_END) {
        element = element.col_end(bounded_grid_line(end));
    }
    if let Some(span) = node.number(property::GRID_ROW_SPAN) {
        element = element.row_span(bounded_track_count(span));
    }
    if let Some(start) = node.number(property::GRID_ROW_START) {
        element = element.row_start(bounded_grid_line(start));
    }
    if let Some(end) = node.number(property::GRID_ROW_END) {
        element = element.row_end(bounded_grid_line(end));
    }
    element
}

// Preserve the core's compact equal-track presets, including their intrinsic
// minimum/maximum sizing. The generic CSS parser handles other track lists.
fn apply_grid_template(element: Element, value: &str, rows: bool) -> Element {
    if let Some(arguments) = function_arguments(value.trim(), "repeat") {
        let parts = split_grid_arguments(arguments);
        if let [count, track] = parts.as_slice()
            && let Ok(count) = count.trim().parse::<u16>()
            && let Some(minmax) = function_arguments(track.trim(), "minmax")
        {
            let bounds = split_grid_arguments(minmax);
            if let [minimum, maximum] = bounds.as_slice() {
                match (rows, minimum.trim(), maximum.trim()) {
                    (false, "0" | "0px", "1fr") => return element.grid_cols(count),
                    (true, "0" | "0px", "1fr") => return element.grid_rows(count),
                    (false, "min-content", "1fr") => return element.grid_cols_min_content(count),
                    (true, "min-content", "1fr") => return element.grid_rows_min_content(count),
                    (false, "0" | "0px", "max-content") => {
                        return element.grid_cols_max_content(count);
                    }
                    (true, "0" | "0px", "max-content") => {
                        return element.grid_rows_max_content(count);
                    }
                    _ => {}
                }
            }
        }
    }
    if rows {
        element.grid_template_rows(native_grid_tracks(value))
    } else {
        element.grid_template_columns(native_grid_tracks(value))
    }
}

fn bounded_track_count(value: f32) -> u16 {
    if value.is_finite() {
        value.clamp(0.0, MAX_GRID_TRACKS as f32) as u16
    } else {
        0
    }
}

fn bounded_grid_line(value: f32) -> i16 {
    if value.is_finite() {
        let limit = (MAX_GRID_TRACKS + 1) as f32;
        value.clamp(-limit, limit) as i16
    } else {
        0
    }
}

/// Build the declared paint-only transition.
///
/// The core owns interpolation, cadence, and which paint properties can transition; the binding
/// only maps the declared CSS-shaped names onto the core's own flags and easing curves.
pub(super) fn native_transition(node: &NativeNode) -> Option<Transition> {
    let milliseconds = node
        .number(property::TRANSITION_DURATION)
        .or_else(|| node.number(property::TRANSITION))?;
    let mut transition = Transition::new(Duration::from_secs_f32(
        (milliseconds / 1_000.0).clamp(0.0, 10.0),
    ));
    transition = transition.with_properties(match node.string(property::TRANSITION_PROPERTIES) {
        Some(list) => native_transition_properties(list),
        // The previous shorthand-only binding transitioned colors, so an undeclared property set
        // keeps that exact behavior.
        None => TransitionProperties::COLORS,
    });
    transition = match node.string(property::TRANSITION_EASING) {
        Some("linear") => transition.with_easing(quickgui::linear),
        Some("ease-in") => transition.with_easing(quickgui::quadratic),
        Some("ease-out") => transition.with_easing(quickgui::ease_out_quint()),
        _ => transition,
    };
    if let Some(fps) = node.number(property::TRANSITION_MAX_FPS) {
        transition = transition.with_max_fps(fps);
    }
    Some(transition)
}

fn native_transition_properties(list: &str) -> TransitionProperties {
    let mut properties = TransitionProperties::empty();
    for name in list.split(',') {
        properties |= match name.trim() {
            "all" => TransitionProperties::ALL,
            "background" | "background-color" => TransitionProperties::BACKGROUND,
            "border-color" => TransitionProperties::BORDER_COLOR,
            "border-width" => TransitionProperties::BORDER_WIDTH,
            "border-radius" => TransitionProperties::BORDER_RADIUS,
            "color" => TransitionProperties::TEXT_COLOR,
            "box-shadow" => TransitionProperties::BOX_SHADOW,
            "opacity" => TransitionProperties::OPACITY,
            "transform" => TransitionProperties::TRANSFORM,
            _ => TransitionProperties::empty(),
        };
    }
    if properties.is_empty() {
        TransitionProperties::COLORS
    } else {
        properties
    }
}

/// Build the declared progress descriptor.
///
/// Declaring the compound scope derives the track, label, and value identities and points the
/// root's accessible name and description at whichever of them the application mounted.
pub(super) fn native_progress(id: u32, node: &NativeNode) -> Progress {
    let maximum = f64::from(node.number(property::MAXIMUM).unwrap_or(1.0));
    let indeterminate = node.boolean(property::INDETERMINATE).unwrap_or(false);
    let progress = match node.number(property::VALUE) {
        Some(value) if !indeterminate => Progress::new(f64::from(value), maximum),
        _ => Progress::indeterminate(),
    };
    let progress = progress.id(native_part_scope(id, node));
    let progress = match declared_value_format(node.string(property::FORMAT)) {
        Some(format) => progress.format(format),
        None => progress,
    };
    match node.string(property::VALUE_TEXT) {
        Some(text) => progress.value_text(text),
        None => progress,
    }
}

/// Build the declared meter descriptor.
pub(super) fn native_meter(id: u32, node: &NativeNode) -> Meter {
    let mut meter = Meter::new(
        f64::from(node.number(property::VALUE).unwrap_or(0.0)),
        f64::from(node.number(property::MINIMUM).unwrap_or(0.0)),
        f64::from(node.number(property::MAXIMUM).unwrap_or(1.0)),
    );
    if let Some(low) = node.number(property::LOW) {
        meter = meter.low(f64::from(low));
    }
    if let Some(high) = node.number(property::HIGH) {
        meter = meter.high(f64::from(high));
    }
    if let Some(optimum) = node.number(property::OPTIMUM) {
        meter = meter.optimum(f64::from(optimum));
    }
    meter = meter.id(native_part_scope(id, node));
    if let Some(format) = declared_value_format(node.string(property::FORMAT)) {
        meter = meter.format(format);
    }
    match node.string(property::VALUE_TEXT) {
        Some(text) => meter.value_text(text),
        None => meter,
    }
}

/// Relate a caller-declared `controls` target that is still mounted in the retained tree.
///
/// A dangling or self-referential target is omitted, matching the core's own relation rules.
pub(super) fn apply_controls(element: Element, node: &NativeNode, tree: &NativeTree) -> Element {
    let Some(target) = node
        .string(property::CONTROLS)
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|target| tree.nodes.contains_key(target))
    else {
        return element;
    };
    element.accessibility_controls(ElementId::new(target as u64))
}

/// Attach the core's delayed, pointer-passive tooltip declared by the `tooltip` property.
pub(super) fn apply_tooltip(element: Element, node: &NativeNode) -> Element {
    let Some(label) = node
        .string(property::TOOLTIP)
        .filter(|label| !label.is_empty())
    else {
        return element;
    };
    let mut tooltip = Tooltip::text(bounded_tooltip_text(label));
    if let Some(placement) = node
        .string(property::TOOLTIP_PLACEMENT)
        .and_then(parse_anchor_placement)
    {
        tooltip = tooltip.placement(placement);
    }
    if let Some(milliseconds) = node.number(property::TOOLTIP_DELAY) {
        tooltip = tooltip.delay(Duration::from_secs_f32(
            (milliseconds / 1_000.0).clamp(0.0, 10.0),
        ));
    }
    if let Some(gap) = node.number(property::TOOLTIP_GAP) {
        tooltip = tooltip.gap(gap);
    }
    if let Some(margin) = node.number(property::TOOLTIP_VIEWPORT_MARGIN) {
        tooltip = tooltip.viewport_margin(margin);
    }
    element.tooltip(tooltip)
}

fn bounded_tooltip_text(label: &str) -> Arc<str> {
    if label.len() <= MAX_TOOLTIP_TEXT_BYTES {
        return Arc::from(label);
    }
    let mut end = MAX_TOOLTIP_TEXT_BYTES;
    while end > 0 && !label.is_char_boundary(end) {
        end -= 1;
    }
    Arc::from(&label[..end])
}

pub(super) fn native_text_overflow(value: &str) -> Option<quickgui::TextOverflow> {
    match value {
        "ellipsis" => Some(quickgui::TextOverflow::ellipsis()),
        "ellipsis-start" => Some(quickgui::TextOverflow::ellipsis_start()),
        "ellipsis-middle" => Some(quickgui::TextOverflow::ellipsis_middle()),
        _ => None,
    }
}

pub(super) fn apply_properties(mut element: Element, node: &NativeNode) -> Element {
    if let Some(display) = node.string(property::DISPLAY) {
        element = match display {
            "none" => element.hidden(),
            "flex" => match node.string(property::FLEX_DIRECTION) {
                Some("row") => element.flex_row(),
                Some("row-reverse") => element.flex_row_reverse(),
                Some("column") => element.flex_col(),
                Some("column-reverse") => element.flex_col_reverse(),
                _ => element.flex(),
            },
            "grid" => element.grid(),
            _ => element.block(),
        };
    }
    if let Some(wrap) = node.string(property::FLEX_WRAP) {
        element = match wrap {
            "wrap" => element.flex_wrap(),
            "wrap-reverse" => element.flex_wrap_reverse(),
            _ => element.flex_nowrap(),
        };
    }
    if let Some(value) = node.number(property::FLEX_GROW) {
        element = element.flex_grow(value);
    }
    if let Some(value) = node.number(property::FLEX_SHRINK) {
        element = element.flex_shrink(value);
    }
    if let Some(value) = node.number(property::FLEX_BASIS) {
        element = element.flex_basis(value);
    } else if node.string(property::FLEX_BASIS) == Some("auto") {
        element = element.flex_basis_auto();
    }
    if let Some(value) = node.string(property::ALIGN_ITEMS) {
        element = match value {
            "center" => element.items_center(),
            "flex-end" | "end" => element.items_end(),
            "baseline" => element.items_baseline(),
            "stretch" => element.items_stretch(),
            _ => element.items_start(),
        };
    }
    if let Some(value) = node.string(property::ALIGN_SELF) {
        element = match value {
            "center" => element.self_center(),
            "flex-end" => element.self_flex_end(),
            "end" => element.self_end(),
            "baseline" => element.self_baseline(),
            "stretch" => element.self_stretch(),
            "start" => element.self_start(),
            _ => element.self_flex_start(),
        };
    }
    if let Some(value) = node.string(property::JUSTIFY_CONTENT) {
        element = match value {
            "center" => element.justify_center(),
            "flex-end" | "end" => element.justify_end(),
            "space-between" => element.justify_between(),
            "space-around" => element.justify_around(),
            "space-evenly" => element.justify_evenly(),
            _ => element.justify_start(),
        };
    }
    if let Some(value) = node.string(property::ALIGN_CONTENT) {
        element = match value {
            "center" => element.content_center(),
            "flex-end" | "end" => element.content_end(),
            "space-between" => element.content_between(),
            "space-around" => element.content_around(),
            "space-evenly" => element.content_evenly(),
            "stretch" => element.content_stretch(),
            "normal" => element.content_normal(),
            _ => element.content_start(),
        };
    }
    if let Some(value) = node.number(property::GAP) {
        element = element.gap(value);
    }
    if let Some(value) = node.number(property::COLUMN_GAP) {
        element = element.gap_x(value);
    }
    if let Some(value) = node.number(property::ROW_GAP) {
        element = element.gap_y(value);
    }

    element = apply_dimension(element, node, property::WIDTH, DimensionKind::Width);
    element = apply_dimension(element, node, property::HEIGHT, DimensionKind::Height);
    if let Some(value) = node.number(property::MIN_WIDTH) {
        element = element.min_w(value);
    }
    if let Some(value) = node.number(property::MIN_HEIGHT) {
        element = element.min_h(value);
    }
    if let Some(value) = node.number(property::MAX_WIDTH) {
        element = element.max_w(value);
    }
    if let Some(value) = node.number(property::MAX_HEIGHT) {
        element = element.max_h(value);
    }

    let padding = node.number(property::PADDING).unwrap_or(0.0);
    let padding_top = node.number(property::PADDING_TOP).unwrap_or(padding);
    let padding_right = node.number(property::PADDING_RIGHT).unwrap_or(padding);
    let padding_bottom = node.number(property::PADDING_BOTTOM).unwrap_or(padding);
    let padding_left = node.number(property::PADDING_LEFT).unwrap_or(padding);
    if [padding_top, padding_right, padding_bottom, padding_left]
        .iter()
        .any(|value| *value != 0.0)
    {
        element = element.padding(padding_top, padding_right, padding_bottom, padding_left);
    }
    let margin = node.number(property::MARGIN).unwrap_or(0.0);
    let margin_top = node.number(property::MARGIN_TOP).unwrap_or(margin);
    let margin_right = node.number(property::MARGIN_RIGHT).unwrap_or(margin);
    let margin_bottom = node.number(property::MARGIN_BOTTOM).unwrap_or(margin);
    let margin_left = node.number(property::MARGIN_LEFT).unwrap_or(margin);
    if [margin_top, margin_right, margin_bottom, margin_left]
        .iter()
        .any(|value| *value != 0.0)
    {
        element = element.margin(margin_top, margin_right, margin_bottom, margin_left);
    }
    let automatic = |code| match node.property(code) {
        Some(PropertyValue::String(value)) => value.as_ref() == "auto",
        None => node.string(property::MARGIN) == Some("auto"),
        _ => false,
    };
    if automatic(property::MARGIN_TOP) {
        element = element.mt_auto();
    }
    if automatic(property::MARGIN_RIGHT) {
        element = element.mr_auto();
    }
    if automatic(property::MARGIN_BOTTOM) {
        element = element.mb_auto();
    }
    if automatic(property::MARGIN_LEFT) {
        element = element.ml_auto();
    }

    if let Some(color) = node.color(property::BACKGROUND_COLOR) {
        element = element.bg(color);
    }
    if let Some(color) = node.color(property::COLOR) {
        element = element.text_color(color);
    }
    // Interaction states: each is one nested declaration the core's own `ElementStateStyle`
    // carries whole, so a state that declares nothing registers no state style at all.
    if let Some(style) = native_state_style(node, &HOVER_STYLE_CODES) {
        element = element.hover(move |_| style);
    }
    if let Some(style) = native_state_style(node, &ACTIVE_STYLE_CODES) {
        element = element.active(move |_| style);
    }
    if let Some(style) = native_state_style(node, &FOCUS_STYLE_CODES) {
        element = element.focus(move |_| style);
    }
    if let Some(style) = native_state_style(node, &DISABLED_STYLE_CODES) {
        element = element.disabled_style(move |_| style);
    }
    if let Some(style) = native_state_style(node, &INVALID_STYLE_CODES) {
        element = element.invalid_style(move |_| style);
    }
    if let Some(style) = native_state_style(node, &SELECTED_STYLE_CODES) {
        element = element.selected_style(move |_| style);
    }
    if let Some(style) = native_state_style(node, &DRAGGING_STYLE_CODES) {
        element = element.dragging(move |_| style);
    }
    if let Some(style) = native_state_style(node, &DRAG_OVER_STYLE_CODES) {
        element = element.drag_over(move |_| style);
    }
    if let Some(style) = native_state_style(node, &FOCUS_WITHIN_STYLE_CODES) {
        element = element.focus_within(move |_| style);
    }
    // Group states in a fixed order — hover, then active — so a held press wins over the hover
    // beneath it whatever order the declaration listed them in. The core bounds the two together.
    let hover_styles = native_group_styles(node, property::GROUP_HOVER_STYLE);
    let active_styles = native_group_styles(node, property::GROUP_ACTIVE_STYLE)
        .into_iter()
        .take(MAX_GROUP_STYLES_PER_ELEMENT.saturating_sub(hover_styles.len()));
    for (target, style) in hover_styles {
        element = match target {
            Some(name) => element.group_hover_named(name, move |_| style),
            None => element.group_hover(move |_| style),
        };
    }
    for (target, style) in active_styles {
        element = match target {
            Some(name) => element.group_active_named(name, move |_| style),
            None => element.group_active(move |_| style),
        };
    }
    match native_group(node) {
        Some(Some(name)) => element = element.group_named(name),
        Some(None) => element = element.group(),
        None => {}
    }
    if let Some(transition) = native_transition(node) {
        element = element.transition(transition);
    }
    element = apply_grid(element, node);
    if let Some(value) = node.number(property::OPACITY) {
        element = element.opacity(value);
    }
    let border_widths = native_border_widths(node);
    if border_widths.top > 0.0
        || border_widths.right > 0.0
        || border_widths.bottom > 0.0
        || border_widths.left > 0.0
    {
        element = element.border_widths(border_widths).border_color(
            node.color(property::BORDER_COLOR)
                .unwrap_or(Color::TRANSPARENT),
        );
    }
    if let Some(value) = node.number(property::BORDER_RADIUS) {
        element = element.rounded(value);
    }
    if let Some(shadows) = native_box_shadows(node) {
        element = element.shadows(shadows);
    }
    if let Some(value) = node.number(property::FONT_SIZE) {
        element = element.text_size(value);
    }
    if let Some(family) = node
        .string(property::FONT_FAMILY)
        .and_then(native_font_family)
    {
        element = element.font_family(family);
    }
    if let Some(value) = node.number(property::LINE_HEIGHT) {
        element = element.line_height(value);
    }
    if let Some(weight) = font_weight(node.property(property::FONT_WEIGHT)) {
        element = element.font_weight(weight);
    }
    if let Some(value) = node.string(property::TEXT_ALIGN) {
        element = element.text_align(match value {
            "center" => TextAlign::Center,
            "right" => TextAlign::Right,
            "justify" => TextAlign::Justify,
            // `start` and `end` are direction relative in the core; they must not collapse into
            // the physical left and right edges before an RTL subtree can resolve them.
            "start" => TextAlign::Start,
            "end" => TextAlign::End,
            _ => TextAlign::Left,
        });
    }
    if let Some(value) = node.string(property::WHITE_SPACE) {
        element = match value {
            "nowrap" => element.whitespace_nowrap(),
            _ => element.whitespace_normal(),
        };
    }
    if let Some(overflow) = node
        .string(property::TEXT_OVERFLOW)
        .and_then(native_text_overflow)
    {
        element = element.text_overflow(overflow);
    }
    if let Some(value) = node.number(property::LINE_CLAMP) {
        element = element.line_clamp(value.max(1.0) as usize);
    }
    if [
        node.string(property::OVERFLOW),
        node.string(property::OVERFLOW_X),
        node.string(property::OVERFLOW_Y),
    ]
    .into_iter()
    .flatten()
    .any(|value| value == "hidden")
    {
        element = element.overflow_hidden();
    }
    let scrolls =
        |value: Option<&str>| value.is_some_and(|value| matches!(value, "auto" | "scroll"));
    let scrolls_y =
        scrolls(node.string(property::OVERFLOW_Y)) || scrolls(node.string(property::OVERFLOW));
    let scrolls_x = scrolls(node.string(property::OVERFLOW_X));
    element = match (scrolls_x, scrolls_y) {
        (true, true) => element.overflow_scroll(),
        (true, false) => element.overflow_x_scroll(),
        (false, true) => element.overflow_y_scroll(),
        (false, false) => element,
    };
    if let Some(value) = node.number(property::SCROLL_TO_END_REVISION) {
        element = element.scroll_to_end(value.max(0.0) as u64);
    }
    if let Some(value) = node.string(property::CURSOR) {
        element = element.cursor(cursor(value));
    }
    if let Some(value) = node.string(property::APP_REGION) {
        element = element.app_region(if value == "drag" {
            AppRegion::Drag
        } else {
            AppRegion::NoDrag
        });
    }
    if let Some(value) = node.boolean(property::DISABLED) {
        element = element.disabled(value);
    }
    // Web-style invalid state on any element, so the `invalid` state style and the native
    // accessibility flag follow one declaration; text inputs already declared it on their own.
    if let Some(value) = node.boolean(property::INVALID) {
        element = element.invalid(value);
    }
    // Web-style selected state on any element: a custom list row declares `selected` and the
    // `selected` state style plus the native accessibility flag follow it.
    if let Some(value) = node.boolean(property::SELECTED) {
        element = element.selected(value);
    }
    if let Some(value) = node.string(property::ACCESSIBILITY_LABEL) {
        element = element.accessibility_label(value.to_owned());
    }
    if let Some(value) = node.string(property::ROLE).and_then(accessibility_role) {
        element = element.accessibility_role(value);
    }
    if let Some(value) = node.number(property::TAB_INDEX) {
        // A declared tab index makes an element focusable, exactly as the web attribute does, so
        // focused key, action, and focus listeners can reach an ordinary container.
        element = element
            .tab_index(value.clamp(i16::MIN as f32, i16::MAX as f32) as i16)
            .focusable();
    }
    if let Some(value) = node.boolean(property::FOCUS_ON_POINTER) {
        element = element.focus_on_pointer(value);
    }
    if node.boolean(property::OVERLAY) == Some(true) {
        element = element.overlay();
    }
    if node.boolean(property::FOCUS_TRAP) == Some(true) {
        element = element.focus_trap();
    }
    if node.boolean(property::RESTORE_PREVIOUS_FOCUS) == Some(true) {
        element = element.restore_previous_focus();
    }
    if node.boolean(property::AUTO_FOCUS) == Some(true) {
        element = element.auto_focus();
    }
    if let Some(value) = node.boolean(property::ACCESSIBILITY_MODAL) {
        element = element.accessibility_modal(value);
    }
    let hit_slop = node.number(property::HIT_SLOP).unwrap_or(0.0);
    let hit_slop = quickgui::Insets {
        top: node.number(property::HIT_SLOP_TOP).unwrap_or(hit_slop),
        right: node.number(property::HIT_SLOP_RIGHT).unwrap_or(hit_slop),
        bottom: node.number(property::HIT_SLOP_BOTTOM).unwrap_or(hit_slop),
        left: node.number(property::HIT_SLOP_LEFT).unwrap_or(hit_slop),
    };
    if hit_slop != quickgui::Insets::default() {
        element = element.hit_slop(hit_slop);
    }
    // A sticky element keeps its in-flow box, so its insets declare where it pins inside the
    // nearest scroll container instead of displacing it.
    let sticky = node.string(property::POSITION) == Some("sticky");
    if let Some(value) = node.string(property::POSITION) {
        element = match value {
            "absolute" => element.absolute(),
            "sticky" => element.sticky(),
            _ => element.relative(),
        };
    }
    if let Some(value) = node.number(property::TOP) {
        element = if sticky {
            element.sticky_top(value)
        } else {
            element.top(value)
        };
    }
    if let Some(value) = node.number(property::RIGHT) {
        element = if sticky {
            element.sticky_right(value)
        } else {
            element.right(value)
        };
    }
    if let Some(value) = node.number(property::BOTTOM) {
        element = if sticky {
            element.sticky_bottom(value)
        } else {
            element.bottom(value)
        };
    }
    if let Some(value) = node.number(property::LEFT) {
        element = if sticky {
            element.sticky_left(value)
        } else {
            element.left(value)
        };
    }
    if let Some(value) = node.string(property::USER_SELECT) {
        element = match value {
            "none" => element.user_select_none(),
            "text" => element.user_select_text(),
            _ => element,
        };
    }
    if let Some(value) = node.string(property::VISIBILITY) {
        element = if value == "hidden" {
            element.invisible()
        } else {
            element.visible()
        };
    }
    if let Some(value) = node.number(property::ASPECT_RATIO) {
        element = element.aspect_ratio(value);
    }
    element = apply_text_styles(element, node);
    element = apply_box_styles(element, node);
    element = apply_layout_styles(element, node);
    element
}

pub(super) enum DimensionKind {
    Width,
    Height,
}

pub(super) fn apply_dimension(
    element: Element,
    node: &NativeNode,
    property: u16,
    kind: DimensionKind,
) -> Element {
    match node.property(property) {
        Some(PropertyValue::Number(value)) => match kind {
            DimensionKind::Width => element.w(*value),
            DimensionKind::Height => element.h(*value),
        },
        // A CSS percentage sizes against the parent, exactly like the `100%` shorthand did.
        Some(PropertyValue::String(value)) => match value
            .strip_suffix('%')
            .and_then(|percent| percent.trim().parse::<f32>().ok())
            .filter(|percent| percent.is_finite() && *percent >= 0.0)
        {
            Some(percent) => match kind {
                DimensionKind::Width => element.w_fraction(percent / 100.0),
                DimensionKind::Height => element.h_fraction(percent / 100.0),
            },
            None => element,
        },
        _ => element,
    }
}

pub(super) fn unpack_color(value: u32) -> Color {
    Color::rgba8(
        value as u8,
        (value >> 8) as u8,
        (value >> 16) as u8,
        (value >> 24) as u8,
    )
}

fn native_border_widths(node: &NativeNode) -> Insets {
    let border_width = node.number(property::BORDER_WIDTH).unwrap_or(0.0);
    Insets {
        top: node
            .number(property::BORDER_TOP_WIDTH)
            .unwrap_or(border_width),
        right: node
            .number(property::BORDER_RIGHT_WIDTH)
            .unwrap_or(border_width),
        bottom: node
            .number(property::BORDER_BOTTOM_WIDTH)
            .unwrap_or(border_width),
        left: node
            .number(property::BORDER_LEFT_WIDTH)
            .unwrap_or(border_width),
    }
}

pub(super) fn native_font_family(value: &str) -> Option<quickgui::FontFamily> {
    match value.trim() {
        "sans-serif" | "system-ui" => Some(quickgui::FontFamily::SansSerif),
        "serif" => Some(quickgui::FontFamily::Serif),
        "monospace" => Some(quickgui::FontFamily::Monospace),
        name if !name.is_empty() && name.len() <= quickgui::MAX_FONT_FAMILY_BYTES => {
            Some(quickgui::FontFamily::named(Arc::<str>::from(name)))
        }
        _ => None,
    }
}

pub(super) fn font_weight(value: Option<&PropertyValue>) -> Option<FontWeight> {
    let weight = match value {
        Some(PropertyValue::Number(value)) => *value,
        Some(PropertyValue::String(value)) => match value.as_ref() {
            "thin" => 100.0,
            "extralight" | "extra-light" => 200.0,
            "light" => 300.0,
            "medium" => 500.0,
            "semibold" | "semi-bold" => 600.0,
            "bold" => 700.0,
            "extrabold" | "extra-bold" => 800.0,
            "black" => 900.0,
            _ => 400.0,
        },
        _ => return None,
    };
    Some(if weight < 150.0 {
        FontWeight::THIN
    } else if weight < 250.0 {
        FontWeight::EXTRA_LIGHT
    } else if weight < 350.0 {
        FontWeight::LIGHT
    } else if weight < 450.0 {
        FontWeight::NORMAL
    } else if weight < 550.0 {
        FontWeight::MEDIUM
    } else if weight < 650.0 {
        FontWeight::SEMIBOLD
    } else if weight < 750.0 {
        FontWeight::BOLD
    } else if weight < 850.0 {
        FontWeight::EXTRA_BOLD
    } else {
        FontWeight::BLACK
    })
}

pub(super) fn cursor(value: &str) -> CursorStyle {
    match value {
        "text" => CursorStyle::IBeam,
        "pointer" => CursorStyle::PointingHand,
        "crosshair" => CursorStyle::Crosshair,
        "grab" => CursorStyle::OpenHand,
        "grabbing" => CursorStyle::ClosedHand,
        "not-allowed" | "no-drop" => CursorStyle::OperationNotAllowed,
        "copy" => CursorStyle::DragCopy,
        "alias" => CursorStyle::DragLink,
        "context-menu" => CursorStyle::ContextualMenu,
        "ew-resize" => CursorStyle::ResizeLeftRight,
        "ns-resize" => CursorStyle::ResizeUpDown,
        "nwse-resize" => CursorStyle::ResizeUpLeftDownRight,
        "nesw-resize" => CursorStyle::ResizeUpRightDownLeft,
        "col-resize" => CursorStyle::ResizeColumn,
        "row-resize" => CursorStyle::ResizeRow,
        "n-resize" => CursorStyle::ResizeUp,
        "e-resize" => CursorStyle::ResizeRight,
        "s-resize" => CursorStyle::ResizeDown,
        "w-resize" => CursorStyle::ResizeLeft,
        "vertical-text" => CursorStyle::IBeamCursorForVerticalLayout,
        _ => CursorStyle::Arrow,
    }
}

pub(super) fn accessibility_role(value: &str) -> Option<AccessibilityRole> {
    Some(match value {
        "button" => AccessibilityRole::Button,
        "link" => AccessibilityRole::Link,
        "img" | "image" => AccessibilityRole::Image,
        "list" => AccessibilityRole::List,
        "listitem" => AccessibilityRole::ListItem,
        "heading" => AccessibilityRole::Heading,
        "checkbox" => AccessibilityRole::CheckBox,
        "radio" => AccessibilityRole::RadioButton,
        "radiogroup" => AccessibilityRole::RadioGroup,
        "switch" => AccessibilityRole::Switch,
        "dialog" => AccessibilityRole::Dialog,
        "alertdialog" => AccessibilityRole::AlertDialog,
        "menu" => AccessibilityRole::Menu,
        "menuitem" => AccessibilityRole::MenuItem,
        "separator" => AccessibilityRole::Separator,
        "group" => AccessibilityRole::Group,
        "region" => AccessibilityRole::Region,
        "listbox" => AccessibilityRole::ListBox,
        "option" => AccessibilityRole::ListBoxOption,
        "combobox" => AccessibilityRole::ComboBox,
        "table" => AccessibilityRole::Table,
        "tree" => AccessibilityRole::Tree,
        "grid" => AccessibilityRole::Grid,
        "row" => AccessibilityRole::Row,
        "columnheader" => AccessibilityRole::ColumnHeader,
        "rowheader" => AccessibilityRole::RowHeader,
        "gridcell" => AccessibilityRole::GridCell,
        "treeitem" => AccessibilityRole::TreeItem,
        "tab" => AccessibilityRole::Tab,
        "tablist" => AccessibilityRole::TabList,
        "tabpanel" => AccessibilityRole::TabPanel,
        "tooltip" => AccessibilityRole::Tooltip,
        "form" => AccessibilityRole::Form,
        "label" => AccessibilityRole::Label,
        _ => return None,
    })
}

#[cfg(test)]
mod paint_tests {
    use super::*;

    #[test]
    fn native_border_edges_override_the_uniform_width_independently() {
        let mut node = NativeNode::new(NodeTag::View);
        node.set_property(property::BORDER_WIDTH, Some(PropertyValue::Number(1.0)));
        node.set_property(property::BORDER_TOP_WIDTH, Some(PropertyValue::Number(0.0)));
        node.set_property(
            property::BORDER_LEFT_WIDTH,
            Some(PropertyValue::Number(4.0)),
        );

        assert_eq!(
            native_border_widths(&node),
            Insets {
                top: 0.0,
                right: 1.0,
                bottom: 1.0,
                left: 4.0,
            }
        );
    }

    #[test]
    fn native_box_shadow_json_uses_explicit_and_current_text_colors() {
        let mut node = NativeNode::new(NodeTag::View);
        node.set_property(property::COLOR, Some(PropertyValue::Color(0xff665544)));
        node.set_property(
            property::BOX_SHADOW,
            Some(PropertyValue::String(Arc::from(
                r#"[{"offsetX":2,"offsetY":3,"blurRadius":8,"spreadRadius":-1,"color":2150834689,"inset":false},{"offsetX":0,"offsetY":1,"blurRadius":0,"spreadRadius":0,"color":null,"inset":true}]"#,
            ))),
        );

        let shadows = native_box_shadows(&node).expect("valid box shadows");
        assert_eq!(shadows.len(), 2);
        assert_eq!(shadows[0].offset(), quickgui::Vector::new(2.0, 3.0));
        assert_eq!(shadows[0].blur(), 8.0);
        assert_eq!(shadows[0].spread(), -1.0);
        assert_eq!(shadows[0].color(), unpack_color(0x80332201));
        assert!(!shadows[0].is_inset());
        assert_eq!(shadows[1].color(), unpack_color(0xff665544));
        assert!(shadows[1].is_inset());
    }
}
