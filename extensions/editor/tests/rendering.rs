use quickgui::{
    Application, ElementId, ExtensionComponent, IntoElement, Vector, View, ViewContext,
    WindowOptions,
};
use quickgui_extension_sdk::{ComponentFactory, WakeHandle, abi, schema};
use serde_json::{Value, json};

unsafe extern "C" fn no_wake(_: *mut std::ffi::c_void) {}
fn inert_wake() -> WakeHandle {
    unsafe {
        WakeHandle::from_raw(abi::Wake {
            context: std::ptr::null_mut(),
            wake: no_wake,
            release: no_wake,
        })
    }
}

fn register_editor() {
    static REGISTER: std::sync::Once = std::sync::Once::new();
    REGISTER.call_once(|| unsafe {
        quickgui::extensions::register_extension(
            quickgui_editor::quickgui_extension_v1(),
            b"editor",
        )
        .unwrap();
    });
}

struct Demo {
    props: Value,
    component: Option<ExtensionComponent>,
}
impl View for Demo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let component = self.component.get_or_insert_with(|| {
            ExtensionComponent::new(
                "editor",
                "diff-view",
                self.props.clone(),
                ElementId::new(1),
                cx.window_invalidator(),
                |_, _| {},
            )
            .unwrap()
        });
        component.element(cx).unwrap().size(420.0, 180.0)
    }
}

fn list_nodes(node: &schema::Node, output: &mut Vec<u64>) {
    if node.list.is_some() {
        output.push(node.key);
    }
    for child in &node.children {
        list_nodes(child, output);
    }
}

// Appearance is application-owned, including the fixtures for styled rendering.
fn themed(mut props: Value) -> Value {
    let values = props.as_object_mut().unwrap();
    let style = values
        .entry("style")
        .or_insert(json!({}))
        .as_object_mut()
        .unwrap();
    for (key, value) in json!({"backgroundColor":"#1e1f24","color":"#dcdfe6","borderColor":"#393c44","borderWidth":1,"borderRadius":6}).as_object().unwrap() {
        style.entry(key.clone()).or_insert(value.clone());
    }
    values
        .entry("gutterPadding")
        .or_insert(json!({"left":8,"right":10}));
    values
        .entry("contentPadding")
        .or_insert(json!({"vertical":8}));
    values.entry("gutterColor").or_insert(json!("#686d78"));
    values.entry("theme").or_insert(json!({
        "addedBackground":"#00c28110","removedBackground":"#ff353f10",
        "addedColor":"#00c281","removedColor":"#ff353f",
        "inlineAddedBackground":"#00c28137","inlineRemovedBackground":"#ff353f37"
    }));
    let options = values
        .entry("options")
        .or_insert(json!({}))
        .as_object_mut()
        .unwrap();
    options.entry("indicators").or_insert(json!("bars"));
    props
}

#[test]
fn components_are_unstyled_unless_the_application_supplies_appearance() {
    fn check(node: &schema::Node) {
        for style in &node.styles {
            match style {
                schema::Style::Background(paint) => assert_eq!(paint[3], 0.0),
                schema::Style::Foreground(_) => panic!("unstyled text must inherit its foreground"),
                schema::Style::Border(widths, _) => assert_eq!(*widths, [0.0; 4]),
                schema::Style::Radius(radius) => assert_eq!(*radius, 0.0),
                schema::Style::CornerRadii(radii) => assert_eq!(*radii, [0.0; 4]),
                schema::Style::Padding(padding) => assert_eq!(*padding, [0.0; 4]),
                _ => {}
            }
        }
        for highlight in &node.spans {
            assert!(highlight.foreground.is_none());
            assert!(
                highlight
                    .background
                    .as_ref()
                    .is_none_or(|paint| paint[3] == 0.0)
            );
        }
        if let Some(gutter) = node.input.as_ref().and_then(|input| input.gutter.as_ref()) {
            assert!(gutter.foreground.is_none());
            assert!(gutter.active_foreground.is_none());
            assert_eq!(gutter.content_padding, [0.0; 4]);
            assert_eq!(gutter.active_line_background[3], 0.0);
        }
        for child in &node.children {
            check(child);
        }
    }
    fn lists(node: &schema::Node, out: &mut Vec<u64>) {
        if let Some(list) = &node.list {
            out.push(list.renderer);
        }
        for child in &node.children {
            lists(child, out);
        }
    }
    for name in ["editor", "code-block", "diff-view"] {
        let props = json!({"value":"fn answer() -> u32 { 42 }", "language":"rust", "oldText":"old", "newText":"new"});
        let mut component = quickgui_editor::Factory::create(name, props, inert_wake()).unwrap();
        let frame = component.render(schema::RenderRequest::default()).unwrap();
        let mut renderers = Vec::new();
        for node in &frame.nodes {
            check(node);
            lists(node, &mut renderers);
        }
        for renderer in renderers {
            let rows = component
                .render(schema::RenderRequest {
                    renderer: Some(renderer),
                    start: 0,
                    end: 1,
                    ..Default::default()
                })
                .unwrap();
            for row in &rows.nodes {
                check(row);
            }
        }
    }
}

#[test]
fn code_block_document_padding_is_explicit_and_optional() {
    fn first_list(node: &schema::Node) -> Option<&schema::List> {
        node.list
            .as_ref()
            .or_else(|| node.children.iter().find_map(first_list))
    }
    for padding in [0.0, 12.0] {
        let mut component = quickgui_editor::Factory::create(
            "code-block",
            json!({
                "value":"fn answer() -> u32 { 42 }",
                "contentPadding":{"left":padding,"right":padding,"vertical":8}
            }),
            inert_wake(),
        )
        .unwrap();
        let frame = component.render(schema::RenderRequest::default()).unwrap();
        let list = first_list(&frame.nodes[0]).unwrap();
        let rows = component
            .render(schema::RenderRequest {
                renderer: Some(list.renderer),
                start: 0,
                end: 1,
                ..Default::default()
            })
            .unwrap();
        assert!(
            rows.nodes[0].children[0]
                .styles
                .contains(&schema::Style::Padding([8.0, padding, 8.0, padding]))
        );
    }
}

#[test]
fn component_abi_keeps_both_diff_gutters_fixed_while_code_scrolls() {
    register_editor();
    for layout in ["split", "unified"] {
        let old = (0..120)
            .map(|n| format!("old {n} {}\n", "wide".repeat(40)))
            .collect::<String>();
        let new = (0..120)
            .map(|n| format!("new {n} {}\n", "wider".repeat(40)))
            .collect::<String>();
        let props = themed(json!({"oldText":old,"newText":new,"options":{"layout":layout}}));
        let mut declaration =
            quickgui_editor::Factory::create("diff-view", props.clone(), inert_wake()).unwrap();
        let frame = declaration
            .render(schema::RenderRequest::default())
            .unwrap();
        let mut lists = Vec::new();
        list_nodes(&frame.nodes[0], &mut lists);
        let props = themed(props);
        let (mut cx, view) = Application::new()
            .into_test_context(
                WindowOptions::default().size(440.0, 200.0),
                Demo {
                    props,
                    component: None,
                },
            )
            .unwrap();
        let window = view.window_handle();
        for list_key in lists {
            let list_id = cx
                .read(view, |view| {
                    view.component.as_ref().unwrap().element_id(list_key)
                })
                .unwrap();
            fn find_list(node: &schema::Node, key: u64) -> Option<schema::List> {
                node.list
                    .as_ref()
                    .filter(|_| node.key == key)
                    .cloned()
                    .or_else(|| node.children.iter().find_map(|child| find_list(child, key)))
            }
            let list = find_list(&frame.nodes[0], list_key).unwrap();
            let rows = declaration
                .render(schema::RenderRequest {
                    renderer: Some(list.renderer),
                    start: 0,
                    end: 1,
                    ..Default::default()
                })
                .unwrap();
            let gutter = &rows.nodes[0].children[0];
            assert!(
                !rows.nodes[0].children[1]
                    .styles
                    .iter()
                    .any(|style| matches!(style, schema::Style::Padding(..))),
                "diff code must not add horizontal padding"
            );
            let gutter_id = cx
                .read(view, |view| {
                    view.component.as_ref().unwrap().element_id(gutter.key)
                })
                .unwrap();
            let before = cx.element_bounds(window, gutter_id).unwrap();
            #[cfg(target_os = "macos")]
            let pane = cx.element_bounds(window, list_id).unwrap();
            #[cfg(target_os = "macos")]
            let snapshot = cx.capture_screenshot(window).unwrap();
            assert!(
                cx.simulate_retained_scroll(window, list_id, Vector::new(-160.0, 0.0))
                    .unwrap(),
                "{layout}: pane did not scroll"
            );
            let after = cx.element_bounds(window, gutter_id).unwrap();
            assert_eq!(after.x, before.x, "{layout}: gutter moved");
            assert_eq!(
                after.width, 52.0,
                "{layout}: marker and line number share one fixed lane"
            );
            assert!(cx.retained_scroll_offset(window, list_id).unwrap().x > 0.0);
            let marker = &gutter.children[0];
            assert!(marker.styles.contains(&schema::Style::Width(4.0)));
            assert!(
                !gutter
                    .children
                    .last()
                    .unwrap()
                    .styles
                    .iter()
                    .any(|style| matches!(style, schema::Style::Border(..)))
            );
            #[cfg(target_os = "macos")]
            {
                let scrolled = cx.capture_screenshot(window).unwrap();
                let scale = snapshot.width() as f32 / 440.0;
                for x in ((before.x + 1.0) * scale).round() as u32
                    ..((before.right() - 1.0) * scale).round() as u32
                {
                    for y in ((before.y + 1.0) * scale).round() as u32
                        ..((before.bottom() - 1.0) * scale).round() as u32
                    {
                        assert_eq!(
                            scrolled.pixel(x, y),
                            snapshot.pixel(x, y),
                            "{layout}: code painted into the fixed gutter"
                        );
                    }
                }
            }
            #[cfg(target_os = "macos")]
            if layout == "split" && pane.x > 10.0 {
                assert_eq!(
                    before.x,
                    pane.x + 1.0,
                    "only the divider may precede the right gutter"
                );
                let scrolled = cx.capture_screenshot(window).unwrap();
                let scale = snapshot.width() as f32 / 440.0;
                // The right pane owns a one-point left border. None of its scrolled
                // rows, inline highlights, or glyphs may paint into that strip.
                for x in (pane.x * scale).round() as u32..((pane.x + 1.0) * scale).round() as u32 {
                    for y in ((pane.y + 1.0) * scale).round() as u32
                        ..((pane.bottom() - 12.0) * scale).round() as u32
                    {
                        assert_eq!(
                            scrolled.pixel(x, y),
                            snapshot.pixel(x, y),
                            "scrolled content leaked into the split divider at ({x}, {y})"
                        );
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn unstyled_components_inherit_the_parent_background_and_foreground() {
    register_editor();
    for name in ["editor", "code-block", "diff-view"] {
        let (mut cx, view) = Application::new()
            .into_test_context(
                WindowOptions::default().size(180.0, 160.0),
                CornerDemo {
                    name,
                    props: json!({"value":"fn answer() {}", "language":"rust", "lineNumbers":true, "oldText":"old\n", "newText":"new\n", "options":{"fileHeader":false}}),
                    component: None,
                },
            ).unwrap();
        let snapshot = cx.capture_screenshot(view.window_handle()).unwrap();
        for (x, y) in [(16, 16), (335, 16), (16, 295), (335, 295), (100, 180)] {
            assert_eq!(
                snapshot.pixel(x, y),
                Some([240, 80, 200, 255]),
                "{name}: added a background or frame"
            );
        }
        assert!(
            (18..70).any(|y| (18..325).any(|x| {
                let pixel = snapshot.pixel(x, y).unwrap();
                pixel[0] < 130 && pixel[1] > 180 && pixel[2] < 160
            })),
            "{name}: text did not inherit its parent's foreground"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn gutters_share_code_row_backgrounds_without_a_divider() {
    register_editor();
    for (name, layout) in [
        ("editor", ""),
        ("code-block", ""),
        ("diff-view", "split"),
        ("diff-view", "unified"),
    ] {
        let props = if name == "diff-view" {
            json!({"oldText":"a\n","newText":"b\n","options":{"layout":layout,"fileHeader":false}})
        } else {
            json!({"value":"short","lineNumbers":true})
        };
        let (mut cx, view) = Application::new()
            .into_test_context(
                WindowOptions::default().size(180.0, 160.0),
                CornerDemo {
                    name,
                    props,
                    component: None,
                },
            )
            .unwrap();
        let snapshot = cx.capture_screenshot(view.window_handle()).unwrap();
        if layout == "unified" {
            for y in [48, 88] {
                assert_eq!(
                    snapshot.pixel(38, y),
                    snapshot.pixel(318, y),
                    "unified gutter"
                );
                assert_eq!(
                    snapshot.pixel(121, y),
                    snapshot.pixel(318, y),
                    "unified divider remains"
                );
            }
        } else if name == "diff-view" {
            // Blank code and gutter pixels on each changed row share one background.
            assert_eq!(
                snapshot.pixel(38, 48),
                snapshot.pixel(158, 48),
                "deletion gutter"
            );
            assert_eq!(
                snapshot.pixel(198, 48),
                snapshot.pixel(318, 48),
                "addition gutter"
            );
            assert_eq!(
                snapshot.pixel(121, 56),
                snapshot.pixel(158, 56),
                "deletion divider remains"
            );
            assert_eq!(
                snapshot.pixel(281, 56),
                snapshot.pixel(318, 56),
                "addition divider remains"
            );
        } else {
            assert_eq!(
                snapshot.pixel(38, 70),
                snapshot.pixel(296, 70),
                "{name}: gutter differs from its code row"
            );
            assert_eq!(
                snapshot.pixel(85, 70),
                snapshot.pixel(296, 70),
                "{name}: gutter divider remains"
            );
        }
    }
}

#[cfg(target_os = "macos")]
struct CornerDemo {
    name: &'static str,
    props: Value,
    component: Option<ExtensionComponent>,
}

#[cfg(target_os = "macos")]
impl View for CornerDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let component = self.component.get_or_insert_with(|| {
            ExtensionComponent::new(
                "editor",
                self.name,
                self.props.clone(),
                ElementId::new(1),
                cx.window_invalidator(),
                |_, _| {},
            )
            .unwrap()
        });
        quickgui::div()
            .size_full()
            .padding(8.0, 8.0, 8.0, 8.0)
            .bg(quickgui::Color::rgb8(240, 80, 200))
            .text_color(quickgui::Color::rgb8(42, 240, 90))
            .child(component.element(cx).unwrap().size(160.0, 140.0))
    }
}

#[cfg(target_os = "macos")]
#[test]
fn editable_and_read_only_gutter_corners_survive_scrolling_without_a_padding_band() {
    register_editor();
    let mut editable_corner: Option<quickgui::VisualSnapshot> = None;
    for name in ["editor", "code-block"] {
        let source = (0..120)
            .map(|n| format!("let value_{n} = {};\n", "1234567890".repeat(30)))
            .collect::<String>();
        let props = themed(
            json!({"value":source,"lineNumbers":true,"gutterBackground":"#0000ff","style":{"borderColor":"#ffffff"}}),
        );
        let mut declaration =
            quickgui_editor::Factory::create(name, props.clone(), inert_wake()).unwrap();
        let frame = declaration
            .render(schema::RenderRequest::default())
            .unwrap();
        let (mut cx, view) = Application::new()
            .into_test_context(
                WindowOptions::default().size(180.0, 160.0),
                CornerDemo {
                    name,
                    props,
                    component: None,
                },
            )
            .unwrap();
        let window = view.window_handle();
        let mut lists = Vec::new();
        list_nodes(&frame.nodes[0], &mut lists);
        let scroll_key = lists.first().copied().unwrap_or(0);
        let scroll_id = cx
            .read(view, |view| {
                view.component.as_ref().unwrap().element_id(scroll_key)
            })
            .unwrap();
        if name == "editor" {
            let policy = &frame.nodes[0].input.as_ref().unwrap().text_checking;
            assert_eq!(
                [
                    policy.spellcheck,
                    policy.grammar_check,
                    policy.autocorrect,
                    policy.smart_quotes,
                    policy.smart_dashes,
                    policy.text_replacement
                ],
                [Some(false); 6]
            );
            let gutter = frame.nodes[0]
                .input
                .as_ref()
                .unwrap()
                .gutter
                .as_ref()
                .unwrap();
            assert_eq!(gutter.content_padding, [8.0, 0.0, 8.0, 0.0]);
        } else {
            assert_eq!(
                cx.element_bounds(window, scroll_id).unwrap().y,
                9.0,
                "read-only scroll viewport must meet the inside border"
            );
        }
        let first = cx.capture_screenshot(window).unwrap();
        if let Some(reference) = &editable_corner {
            for y in 16..28 {
                for x in 16..28 {
                    assert_eq!(
                        first.pixel(x, y),
                        reference.pixel(x, y),
                        "editable and read-only corner strokes must match at ({x},{y})"
                    );
                }
            }
        } else {
            editable_corner = Some(first.clone());
        }
        for delta in [
            Vector::new(-160.0, -103.0),
            Vector::new(-240.0, -67.0),
            Vector::new(400.0, 170.0),
        ] {
            assert!(
                cx.simulate_retained_scroll(window, scroll_id, delta)
                    .unwrap()
            );
            let snapshot = cx.capture_screenshot(window).unwrap();
            for (x, y) in [(18, 18), (18, 293), (333, 18), (333, 293)] {
                let pixel = snapshot.pixel(x, y).unwrap();
                assert!(
                    pixel[0] > 200 && pixel[1] > 50 && pixel[2] > 150,
                    "{name}: gutter covered the outside of its rounded corner at ({x},{y}): {pixel:?}"
                );
                assert_eq!(
                    Some(pixel),
                    first.pixel(x, y),
                    "{name}: the corner changed on scroll at ({x},{y})"
                );
            }
            // A settled repaint must preserve the edge pixels after scrolling.
            let settled = cx.capture_screenshot(window).unwrap();
            settled
                .assert_matches(&snapshot, quickgui::VisualTolerance::EXACT)
                .unwrap();
        }
    }
}
