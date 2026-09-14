use super::*;
use crate::ElementUpdate;

const VIEWPORT: Size = Size::new(640.0, 480.0);

#[test]
fn subtree_replacement_reconciles_only_changed_nodes_and_reuses_keyed_state() {
    let mut tree = UiTree::new();
    set_root(&mut tree, panels("Before", Color::BLACK));
    let value = ElementId::from("value");
    let value_node = tree.layout_nodes.nodes[&value];
    let measurements = Measurements::start();
    tree.take_work();
    let replacement = div()
        .id("changing-panel")
        .size(240.0, 480.0)
        .text_size(20.0)
        .children([text("After").id("value"), text("Inserted").id("new")]);
    assert_eq!(
        tree.update_elements(&[
            ElementUpdate::Replace {
                id: "changing-panel".into(),
                element: Box::new(replacement.clone())
            },
            ElementUpdate::BackgroundColor {
                id: "stable-panel".into(),
                color: Color::WHITE
            },
        ])
        .unwrap(),
        Some(ElementUpdateKind::Layout)
    );
    tree.layout(VIEWPORT, 1.0, &mut TestTextLayout).unwrap();
    assert_eq!(tree.take_work().reconciled_nodes, 3);
    assert_eq!(tree.layout_nodes.nodes[&value], value_node);
    assert!(
        measurements
            .take()
            .iter()
            .all(|id| *id == TextId::new(value.value())
                || *id == TextId::new(ElementId::from("new").value()))
    );
    let mut expected = panels("Before", Color::BLACK);
    expected.children[0] = replacement;
    expected.children[1].visual.background = Some(Color::WHITE);
    assert_matches_fresh_layout(&tree, expected, VIEWPORT, 1.0);
}

#[test]
fn disjoint_replacements_can_move_a_key_between_parents_in_either_order() {
    let initial = || {
        div().id("root").flex_row().children([
            div().id("left").size(200.0, 100.0),
            div()
                .id("right")
                .size(200.0, 100.0)
                .child(text("Moved").id("key")),
        ])
    };
    for reverse in [false, true] {
        let mut tree = UiTree::new();
        set_root(&mut tree, initial());
        let key = tree.layout_nodes.nodes[&"key".into()];
        let left = div()
            .id("left")
            .size(200.0, 100.0)
            .child(text("Moved").id("key"));
        let right = div().id("right").size(200.0, 100.0);
        let mut updates = vec![
            ElementUpdate::Replace {
                id: "left".into(),
                element: Box::new(left.clone()),
            },
            ElementUpdate::Replace {
                id: "right".into(),
                element: Box::new(right.clone()),
            },
        ];
        if reverse {
            updates.reverse();
        }
        tree.update_elements(&updates).unwrap().unwrap();
        tree.layout(VIEWPORT, 1.0, &mut TestTextLayout).unwrap();
        assert_eq!(tree.layout_nodes.nodes[&"key".into()], key);
        assert_matches_fresh_layout(
            &tree,
            div().id("root").flex_row().children([left, right]),
            VIEWPORT,
            1.0,
        );
    }
}

#[test]
fn invalid_replacements_and_overlapping_batches_leave_the_tree_unchanged() {
    let mut tree = UiTree::new();
    set_root(&mut tree, panels("Original", Color::BLACK));
    let before = tree.layout_nodes.nodes.clone();
    let replace = |element| ElementUpdate::Replace {
        id: "changing-panel".into(),
        element: Box::new(element),
    };
    assert!(
        tree.update_elements(&[replace(
            div()
                .id("changing-panel")
                .child(text("Duplicate").id("stable-panel"))
        )])
        .is_err()
    );
    assert_eq!(tree.layout_nodes.nodes, before);
    assert_matches_fresh_layout(&tree, panels("Original", Color::BLACK), VIEWPORT, 1.0);
    assert_eq!(
        tree.update_elements(&[
            replace(div().id("changing-panel")),
            ElementUpdate::Text {
                id: "value".into(),
                content: Arc::from("Overlapping")
            },
        ])
        .unwrap(),
        None
    );
    assert_matches_fresh_layout(&tree, panels("Original", Color::BLACK), VIEWPORT, 1.0);
}

#[test]
fn paint_only_mutation_reuses_geometry_and_hit_allocations() {
    let mut tree = UiTree::new();
    set_root(
        &mut tree,
        div().size_full().child(
            div()
                .id("button")
                .size(100.0, 40.0)
                .clickable()
                .bg(Color::BLACK),
        ),
    );
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut TestTextLayout).unwrap();
    let before_hits = tree.hit_regions.as_ptr();
    let before_bounds = tree.element_bounds.clone();
    let initial = tree.take_work();
    assert!(initial.geometry_nodes > 0);
    tree.update_elements(&[ElementUpdate::BackgroundColor {
        id: ElementId::from("button"),
        color: Color::WHITE,
    }])
    .unwrap();
    scene.clear(Color::BLACK);
    tree.paint(&mut scene, &mut TestTextLayout).unwrap();
    let work = tree.take_work();
    assert_eq!(work.geometry_nodes, 0);
    assert_eq!(work.layout_passes, 0);
    assert_eq!(tree.hit_regions.as_ptr(), before_hits);
    assert_eq!(tree.element_bounds, before_bounds);
    assert!(
        tree.hit_regions
            .iter()
            .any(|hit| hit.id == ElementId::from("button"))
    );
}

#[test]
fn layout_hover_refresh_and_paint_share_natural_geometry() {
    let mut tree = UiTree::new();
    set_root(&mut tree, panels("one", Color::BLACK));
    tree.take_work();
    tree.refresh_hover_after_layout(Some(Point::new(20.0, 20.0)))
        .unwrap();
    let work = tree.take_work();
    assert!(work.geometry_nodes > 0);
    tree.paint(&mut Scene::new(), &mut TestTextLayout).unwrap();
    assert_eq!(tree.take_work().geometry_nodes, 0);
}

#[test]
fn toolbar_hover_work_is_independent_of_unrelated_panel_size() {
    let mut counts = Vec::new();
    let mut allocation_counts = Vec::new();
    for count in [100, 10_000] {
        let mut tree = UiTree::new();
        let root = div().size_full().flex_row().children([
            button()
                .id("toolbar")
                .size(40.0, 40.0)
                .flex_none()
                .bg(Color::BLACK)
                .hover(|style| style.bg(Color::WHITE)),
            div()
                .id("document")
                .flex_col()
                .size(500.0, 480.0)
                .overflow_hidden()
                .children((0..count).map(|i| text("Stable").id(ElementId::new(1000 + i)))),
        ]);
        set_root(&mut tree, root);
        let mut scene = Scene::new();
        tree.paint(&mut scene, &mut TestTextLayout).unwrap();
        let chunks: Vec<_> = scene
            .paint_layers()
            .iter()
            .filter(|layer| !layer.text_runs().is_empty())
            .cloned()
            .collect();
        tree.take_work();
        let (_, allocations) = crate::allocation_tests::measure(|| {
            assert!(tree.pointer_moved(Point::new(10.0, 10.0), &mut TestTextLayout));
            scene.clear(Color::BLACK);
            tree.paint(&mut scene, &mut TestTextLayout).unwrap();
        });
        allocation_counts.push(allocations);
        let work = tree.take_work();
        assert_eq!(work.geometry_nodes, 0);
        assert_eq!(work.layout_passes, 0);
        assert!(work.reused_subtrees > 0);
        assert!(work.cached_paint_bytes > 0);
        counts.push(work.painted_nodes);
        eprintln!(
            "mounted siblings={count}, painted={}, geometry={}, allocations={}, allocated bytes={}",
            work.painted_nodes, work.geometry_nodes, allocations.calls, allocations.bytes
        );
        for chunk in chunks {
            assert!(
                scene
                    .paint_layers()
                    .iter()
                    .any(|current| Arc::ptr_eq(current, &chunk)),
                "replay must share commands without copying every primitive"
            );
        }
    }
    assert_eq!(counts, vec![2, 2]);
    assert_eq!(allocation_counts[0].calls, allocation_counts[1].calls);
    assert_eq!(allocation_counts[0].bytes, allocation_counts[1].bytes);
}

#[test]
fn inherited_color_invalidates_cached_descendants() {
    let mut tree = UiTree::new();
    set_root(
        &mut tree,
        div().id("root").size_full().text_color(Color::BLACK).child(
            div()
                .flex_col()
                .children((0..64).map(|i| text("Inherited").id(ElementId::new(1000 + i)))),
        ),
    );
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut TestTextLayout).unwrap();
    for color in [Color::WHITE, Color::rgb8(120, 10, 30)] {
        tree.update_elements(&[ElementUpdate::TextColor {
            id: ElementId::from("root"),
            color,
        }])
        .unwrap();
        scene.clear(Color::BLACK);
        tree.paint(&mut scene, &mut TestTextLayout).unwrap();
        assert!(!scene.text_runs().is_empty());
        assert!(scene.text_runs().iter().all(|run| run.style.color == color));
    }
}

#[test]
fn named_group_hover_reaches_cached_member_subtrees() {
    let mut tree = UiTree::new();
    set_root(
        &mut tree,
        div().id("group").group_named("outer").size_full().child(
            div().group().size_full().children((0..64).map(|i| {
                text("Member")
                    .id(ElementId::new(1000 + i))
                    .text_color(Color::BLACK)
                    .group_hover_named("outer", |style| style.text_color(Color::WHITE))
            })),
        ),
    );
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut TestTextLayout).unwrap();
    tree.hovered.insert(ElementId::from("group"));
    scene.clear(Color::BLACK);
    tree.paint(&mut scene, &mut TestTextLayout).unwrap();
    assert!(!scene.text_runs().is_empty());
    assert!(
        scene
            .text_runs()
            .iter()
            .all(|run| run.style.color == Color::WHITE)
    );
}

struct Measurements;

impl Measurements {
    fn start() -> Self {
        RECORDED_TEXT_IDS.with(|ids| *ids.borrow_mut() = Some(Vec::new()));
        Self
    }

    fn take(&self) -> Vec<TextId> {
        RECORDED_TEXT_IDS.with(|ids| std::mem::take(ids.borrow_mut().as_mut().unwrap()))
    }
}

impl Drop for Measurements {
    fn drop(&mut self) {
        RECORDED_TEXT_IDS.with(|ids| *ids.borrow_mut() = None);
    }
}

fn set_root(tree: &mut UiTree, root: Element) {
    tree.set_root(root, VIEWPORT, 1.0, &mut TestTextLayout)
        .unwrap();
}

fn assert_matches_fresh_layout(tree: &UiTree, root: Element, viewport: Size, scale: f32) {
    let mut fresh = UiTree::new();
    fresh
        .set_root(root, viewport, scale, &mut TestTextLayout)
        .unwrap();
    assert_eq!(
        tree.layout_nodes.nodes.len(),
        fresh.layout_nodes.nodes.len()
    );
    assert_eq!(
        tree.taffy.total_node_count(),
        fresh.taffy.total_node_count()
    );
    for (id, node) in &tree.layout_nodes.nodes {
        assert_eq!(
            tree.taffy.layout(*node).unwrap(),
            fresh.taffy.layout(fresh.layout_nodes.nodes[id]).unwrap(),
            "layout differs for {id:?}",
        );
    }
}

fn panels(value: &'static str, color: Color) -> Element {
    div().size_full().flex_row().text_color(color).children([
        div()
            .id("changing-panel")
            .size(240.0, 480.0)
            .child(text(value).id("value")),
        div()
            .id("stable-panel")
            .size(320.0, 480.0)
            .flex_col()
            .children((0..100).map(|i| text("Stable sibling").id(ElementId::new(1000 + i)))),
    ])
}

#[test]
fn redeclaration_and_paint_only_changes_do_not_measure_text() {
    let mut tree = UiTree::new();
    let measurements = Measurements::start();
    set_root(&mut tree, panels("One", Color::BLACK));
    let identities = tree.layout_nodes.nodes.clone();
    assert!(measurements.take().len() >= 101);
    let passes = tree.layout_nodes.layout_passes;

    set_root(&mut tree, panels("One", Color::BLACK));
    assert!(
        measurements.take().is_empty(),
        "identical declarations must reuse layout"
    );
    set_root(&mut tree, panels("One", Color::WHITE).bg(Color::BLACK));
    assert!(
        measurements.take().is_empty(),
        "inherited color and background are paint-only"
    );
    assert_eq!(tree.layout_nodes.nodes, identities);
    assert_eq!(
        tree.layout_nodes.layout_passes, passes,
        "paint-only declarations skip the layout pass entirely"
    );
    assert_matches_fresh_layout(
        &tree,
        panels("One", Color::WHITE).bg(Color::BLACK),
        VIEWPORT,
        1.0,
    );
}

#[test]
fn changed_text_measures_only_its_branch() {
    let mut tree = UiTree::new();
    let measurements = Measurements::start();
    set_root(&mut tree, panels("One", Color::BLACK));
    measurements.take();

    set_root(&mut tree, panels("A much longer value", Color::BLACK));
    let measured = measurements.take();
    assert!(!measured.is_empty());
    assert!(
        measured
            .iter()
            .all(|id| *id == TextId::new(ElementId::from("value").value())),
        "unchanged sibling subtrees were measured: {measured:?}"
    );
    assert_matches_fresh_layout(
        &tree,
        panels("A much longer value", Color::BLACK),
        VIEWPORT,
        1.0,
    );
}

#[test]
fn inherited_font_and_scale_changes_invalidate_measurements() {
    let mut tree = UiTree::new();
    let measurements = Measurements::start();
    set_root(&mut tree, panels("One", Color::BLACK));
    measurements.take();
    set_root(&mut tree, panels("One", Color::BLACK).text_size(24.0));
    assert!(measurements.take().len() >= 101);

    tree.relayout_with_prepare(VIEWPORT, 2.0, &mut TestTextLayout, |_| {})
        .unwrap();
    assert!(
        measurements.take().len() >= 101,
        "scale is a measurement input even at the same logical size"
    );
    assert_matches_fresh_layout(
        &tree,
        panels("One", Color::BLACK).text_size(24.0),
        VIEWPORT,
        2.0,
    );
}

#[test]
fn retained_layout_matches_fresh_after_reorder_reparent_and_unmount() {
    let leaf = || text("Movable").id("leaf");
    let first = || {
        div().id("root").flex_row().children([
            div()
                .id("a")
                .p(8.0)
                .child(div().id("b").p(4.0).child(leaf())),
            text("Sibling").id("sibling"),
        ])
    };
    let inverted = || {
        div().id("root").flex_col().children([
            text("Sibling").id("sibling"),
            div()
                .id("b")
                .p(4.0)
                .child(div().id("a").p(8.0).child(leaf())),
        ])
    };
    let mut tree = UiTree::new();
    set_root(&mut tree, first());
    let identities = tree.layout_nodes.nodes.clone();
    set_root(&mut tree, inverted());
    assert_eq!(identities, tree.layout_nodes.nodes);
    assert_matches_fresh_layout(&tree, inverted(), VIEWPORT, 1.0);
    set_root(&mut tree, first());
    assert_matches_fresh_layout(&tree, first(), VIEWPORT, 1.0);

    for index in 0..20 {
        let root = || {
            div()
                .id("root")
                .child(text("Replacement").id(ElementId::new(2000 + index)))
        };
        set_root(&mut tree, root());
        assert_matches_fresh_layout(&tree, root(), VIEWPORT, 1.0);
        assert_eq!(
            tree.taffy.total_node_count(),
            2,
            "unmounted nodes must leave the arena"
        );
    }
}

#[test]
fn container_query_layout_survives_redeclaration_and_resizing() {
    let root = || {
        div().size_full().child(
            container_query(|size| {
                div().id("query-content").child(
                    text(if size.width < 300.0 { "Narrow" } else { "Wide" }).id("query-text"),
                )
            })
            .id("query")
            .w_full()
            .h(100.0),
        )
    };
    let mut tree = UiTree::new();
    let measurements = Measurements::start();
    set_root(&mut tree, root());
    let identities = tree.layout_nodes.nodes.clone();
    assert!(!measurements.take().is_empty());
    set_root(&mut tree, root());
    assert!(measurements.take().is_empty());
    assert_eq!(tree.layout_nodes.nodes, identities);
    let small = Size::new(200.0, 480.0);
    tree.relayout_with_prepare(small, 1.0, &mut TestTextLayout, |_| {})
        .unwrap();
    assert!(!measurements.take().is_empty());
    assert_matches_fresh_layout(&tree, root(), small, 1.0);
}

#[test]
fn element_kind_and_input_value_changes_update_intrinsic_layout() {
    let mut tree = UiTree::new();
    for child in [
        text("Label").id("content"),
        text_input("").placeholder("Placeholder").id("content"),
        text_input("A longer edited value").id("content"),
        div().size(50.0, 80.0).id("content"),
        text("Label again").id("content"),
    ] {
        let root = div().flex_col().child(child);
        set_root(&mut tree, root.clone());
        assert_matches_fresh_layout(&tree, root, VIEWPORT, 1.0);
    }
}

#[test]
fn targeted_text_updates_skip_unchanged_branches_and_preserve_selection() {
    let mut tree = UiTree::new();
    let measurements = Measurements::start();
    set_root(&mut tree, panels("Long selected text", Color::BLACK));
    measurements.take();
    let id = ElementId::from("value");
    tree.static_text_selection = Some(StaticTextSelection {
        anchor: StaticTextPosition { id, offset: 0 },
        focus: StaticTextPosition { id, offset: 18 },
    });
    let nodes = tree.layout_nodes.nodes.clone();
    assert_eq!(
        tree.update_elements(&[ElementUpdate::Text {
            id,
            content: Arc::from("Hi 🌍"),
        }])
        .unwrap(),
        Some(ElementUpdateKind::Layout)
    );
    assert!(tree.take_retained_semantics_dirty());
    assert!(!tree.take_retained_semantics_dirty());
    assert_eq!(
        tree.static_text_selection.unwrap().focus.offset,
        "Hi 🌍".len()
    );
    let entry = &tree.selectable_texts[tree.selectable_text_indices[&id]];
    assert_eq!(entry.content.as_ref(), "Hi 🌍");
    assert_eq!(entry.character_lengths.as_ref(), &[1, 1, 1, 4]);
    tree.layout(VIEWPORT, 1.0, &mut TestTextLayout).unwrap();
    assert_eq!(nodes, tree.layout_nodes.nodes);
    let measured = measurements.take();
    assert!(!measured.is_empty());
    assert!(
        measured
            .iter()
            .all(|text_id| *text_id == TextId::new(id.value()))
    );
    assert_matches_fresh_layout(&tree, panels("Hi 🌍", Color::BLACK), VIEWPORT, 1.0);
}

#[test]
fn targeted_paint_updates_respect_inheritance_and_do_not_dirty_layout() {
    let red = Color::rgb8(200, 0, 0);
    let root = || {
        div().id("root").text_color(Color::BLACK).children([
            div().id("inherited").child(text("Inherit").id("label")),
            div()
                .id("boundary")
                .text_color(red)
                .child(text("Explicit").id("red-label")),
        ])
    };
    let mut tree = UiTree::new();
    set_root(&mut tree, root());
    let measurements = Measurements::start();
    let updates = [
        ElementUpdate::TextColor {
            id: "root".into(),
            color: Color::WHITE,
        },
        ElementUpdate::BackgroundColor {
            id: "inherited".into(),
            color: Color::BLACK,
        },
        ElementUpdate::Opacity {
            id: "boundary".into(),
            opacity: 0.5,
        },
    ];
    assert_eq!(
        tree.update_elements(&updates).unwrap(),
        Some(ElementUpdateKind::Paint)
    );
    assert_eq!(
        tree.update_elements(&updates).unwrap(),
        Some(ElementUpdateKind::None)
    );
    assert!(!tree.take_retained_semantics_dirty());
    let root = tree.root.as_ref().unwrap();
    assert_eq!(
        root.children[0].children[0].resolved_typography.color,
        Color::WHITE
    );
    assert_eq!(root.children[1].children[0].resolved_typography.color, red);
    for node in tree.layout_nodes.nodes.values() {
        assert!(!tree.taffy.dirty(*node).unwrap());
    }
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut TestTextLayout).unwrap();
    assert_eq!(scene.text_runs()[0].style.color, Color::WHITE);
    assert_eq!(scene.text_runs()[1].style.color, red);
    assert!(measurements.take().is_empty());
}

#[test]
fn unsupported_target_rejects_a_whole_retained_batch() {
    let mut tree = UiTree::new();
    set_root(
        &mut tree,
        div().id("root").children([
            text("Original").id("label"),
            container_query(|_| text("Callback").id("callback-text")).size(100.0, 50.0),
        ]),
    );
    for id in ["missing", "root", "callback-text"] {
        assert_eq!(
            tree.update_elements(&[
                ElementUpdate::Text {
                    id: "label".into(),
                    content: Arc::from("Should not apply")
                },
                ElementUpdate::Text {
                    id: id.into(),
                    content: Arc::from("Unsupported")
                },
            ])
            .unwrap(),
            None
        );
        assert!(matches!(&tree.root.as_ref().unwrap().children[0].kind,
            ElementKind::Text(content) if content.as_ref() == "Original"));
    }
}

#[test]
fn resolved_motion_callbacks_keep_ownership_of_their_subtrees() {
    let mut tree = UiTree::new();
    set_root(
        &mut tree,
        div().id("root").children([
            text("Independent").id("independent"),
            div().id("animated").with_animation(
                "clock",
                Animation::new(Duration::from_secs(1)),
                |element, _| element.child(text("Callback value").id("animated-text")),
            ),
        ]),
    );
    for id in ["animated", "animated-text"] {
        assert_eq!(
            tree.update_elements(&[ElementUpdate::TextColor {
                id: id.into(),
                color: Color::WHITE
            }])
            .unwrap(),
            None
        );
    }
    assert_eq!(
        tree.update_elements(&[ElementUpdate::TextColor {
            id: "independent".into(),
            color: Color::WHITE
        }])
        .unwrap(),
        Some(ElementUpdateKind::Paint)
    );
}

#[test]
fn targeted_background_updates_keep_transition_playback() {
    let now = Instant::now();
    let mut tree = UiTree::new_at(now);
    set_root(
        &mut tree,
        div()
            .id("box")
            .size(40.0, 40.0)
            .bg(Color::BLACK)
            .transition(Transition::colors(Duration::from_millis(100)).with_easing(crate::linear)),
    );
    let mut scene = Scene::new();
    tree.paint_at(&mut scene, &mut TestTextLayout, now).unwrap();
    assert_eq!(
        tree.update_elements(&[ElementUpdate::BackgroundColor {
            id: "box".into(),
            color: Color::WHITE,
        }])
        .unwrap(),
        Some(ElementUpdateKind::Paint)
    );
    scene.clear(Color::TRANSPARENT);
    tree.paint_at(&mut scene, &mut TestTextLayout, now).unwrap();
    assert_eq!(scene.edge_quads()[0].fill, Color::BLACK);
    scene.clear(Color::TRANSPARENT);
    tree.paint_at(
        &mut scene,
        &mut TestTextLayout,
        now + Duration::from_millis(50),
    )
    .unwrap();
    let middle = scene.edge_quads()[0].fill;
    assert_ne!(middle, Color::BLACK);
    assert_ne!(middle, Color::WHITE);
    assert!(tree.style_transition_frame_requested());
    scene.clear(Color::TRANSPARENT);
    tree.paint_at(
        &mut scene,
        &mut TestTextLayout,
        now + Duration::from_millis(100),
    )
    .unwrap();
    assert_eq!(scene.edge_quads()[0].fill, Color::WHITE);
    assert!(!tree.style_transition_frame_requested());
}

#[test]
fn root_layout_rounding_can_switch_without_losing_fractional_geometry() {
    let mut tree = UiTree::new();
    let declaration = |round| {
        div()
            .layout_rounding(round)
            .size(100., 100.)
            .items_center()
            .justify_center()
            .child(div().id("fractional").size(13., 13.))
    };
    set_root(&mut tree, declaration(false));
    assert_eq!(
        tree.taffy
            .layout(tree.layout_nodes.nodes[&ElementId::from("fractional")])
            .unwrap()
            .location
            .x,
        43.5
    );
    set_root(&mut tree, declaration(true));
    assert_eq!(
        tree.taffy
            .layout(tree.layout_nodes.nodes[&ElementId::from("fractional")])
            .unwrap()
            .location
            .x,
        44.
    );
    set_root(&mut tree, declaration(false));
    assert_eq!(
        tree.taffy
            .layout(tree.layout_nodes.nodes[&ElementId::from("fractional")])
            .unwrap()
            .location
            .x,
        43.5
    );
}
