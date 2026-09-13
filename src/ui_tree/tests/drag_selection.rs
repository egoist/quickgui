use super::*;

#[test]
fn typed_drag_hits_skip_incompatible_targets_but_respect_pointer_blockers() {
    let mut tree = UiTree::new();
    let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
    let region = |id, source, target, blocks, order| HitRegion {
        transform: None,
        id: ElementId::new(id),
        bounds,
        clip: bounds,
        clickable: false,
        pointer_listener: false,
        drag_source: source,
        drop_target: target,
        focusable: false,
        cursor_style: None,
        cursor_states: CursorStateStyles::default(),
        stateful: source || target,
        blocks_pointer: blocks,
        app_region: None,
        order: PaintOrder {
            layer: PaintLayerKey::default(),
            source: order,
        },
    };
    tree.hit_regions.push(region(1, false, true, false, 0));
    tree.hit_regions.push(region(2, true, true, false, 1));

    let point = Point::new(10.0, 10.0);
    assert_eq!(tree.drag_source_at(point), Some(ElementId::new(2)));
    assert_eq!(
        tree.drop_target_at(point, |id| id == ElementId::new(1)),
        Some(ElementId::new(1))
    );
    assert!(tree.begin_drag(ElementId::new(2)));
    assert!(tree.set_drag_over(Some(ElementId::new(1))));
    assert!(!tree.set_drag_over(Some(ElementId::new(1))));
    assert!(tree.end_drag());

    tree.hit_regions[1].blocks_pointer = true;
    assert_eq!(
        tree.drop_target_at(point, |id| id == ElementId::new(1)),
        None
    );
}

#[test]
fn typed_drop_predicates_control_highlight_and_delivery_acceptance() {
    let mut root = div().id(7_u64).can_drop::<u32>(|value| *value >= 10);
    assign_runtime_ids(&mut root);
    let mut tree = UiTree::new();
    tree.root = Some(root);
    tree.rebuild_drop_predicates();

    assert!(!tree.can_drop(ElementId::new(7), TypeId::of::<u32>(), &9_u32));
    assert!(tree.can_drop(ElementId::new(7), TypeId::of::<u32>(), &10_u32));
    assert!(tree.can_drop(ElementId::new(7), TypeId::of::<u64>(), &0_u64));
}

#[cfg(target_os = "macos")]
#[test]
fn native_drop_snapshot_matches_typed_targets_predicates_and_blockers() {
    use crate::{ExternalDragText, ExternalDragUrl};

    let mut tree = UiTree::new();
    let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
    let region = |id, pointer_listener, blocks_pointer, order| HitRegion {
        transform: None,
        id: ElementId::new(id),
        bounds,
        clip: bounds,
        clickable: false,
        pointer_listener,
        drag_source: false,
        drop_target: true,
        focusable: false,
        cursor_style: None,
        cursor_states: CursorStateStyles::default(),
        stateful: true,
        blocks_pointer,
        app_region: None,
        order: PaintOrder {
            layer: PaintLayerKey::default(),
            source: order,
        },
    };
    tree.hit_regions.push(region(1, false, false, 0));
    tree.hit_regions.push(region(2, false, false, 1));
    tree.drop_predicates.insert(
        (ElementId::new(1), TypeId::of::<ExternalDragText>()),
        Arc::new(|value| {
            value
                .downcast_ref::<ExternalDragText>()
                .is_some_and(|text| text.as_str() == "accepted")
        }),
    );

    let mut snapshot = ExternalDropSnapshot::new();
    tree.update_external_drop_snapshot(
        &mut snapshot,
        &[
            (ElementId::new(1), TypeId::of::<ExternalDragText>()),
            (ElementId::new(2), TypeId::of::<ExternalDragUrl>()),
        ],
    );
    let point = Point::new(20.0, 20.0);
    let text = ExternalDragText::new("accepted");
    assert_eq!(
        snapshot.target_at(point, TypeId::of::<ExternalDragText>(), &text),
        Some(ElementId::new(1))
    );
    let rejected = ExternalDragText::new("rejected");
    assert_eq!(
        snapshot.target_at(point, TypeId::of::<ExternalDragText>(), &rejected),
        None
    );
    let url = ExternalDragUrl::new("https://quickgui.dev").unwrap();
    assert_eq!(
        snapshot.target_at(point, TypeId::of::<ExternalDragUrl>(), &url),
        Some(ElementId::new(2))
    );

    tree.hit_regions[1].pointer_listener = true;
    tree.update_external_drop_snapshot(
        &mut snapshot,
        &[(ElementId::new(1), TypeId::of::<ExternalDragText>())],
    );
    assert_eq!(
        snapshot.target_at(point, TypeId::of::<ExternalDragText>(), &text),
        None
    );
}

#[cfg(target_os = "macos")]
#[test]
fn native_drop_snapshot_selects_the_topmost_target_before_offer_order() {
    use crate::ExternalDragText;

    let mut tree = UiTree::new();
    let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
    let region = |id, source| HitRegion {
        transform: None,
        id: ElementId::new(id),
        bounds,
        clip: bounds,
        clickable: false,
        pointer_listener: false,
        drag_source: false,
        drop_target: true,
        focusable: false,
        cursor_style: None,
        cursor_states: CursorStateStyles::default(),
        stateful: true,
        blocks_pointer: false,
        app_region: None,
        order: PaintOrder {
            layer: PaintLayerKey::default(),
            source,
        },
    };
    // The retained hit stack is bottom-to-top. Element 2 visually covers element 1.
    tree.hit_regions.push(region(1, 0));
    tree.hit_regions.push(region(2, 1));

    let mut snapshot = ExternalDropSnapshot::new();
    tree.update_external_drop_snapshot(
        &mut snapshot,
        &[
            (ElementId::new(1), TypeId::of::<u32>()),
            (ElementId::new(2), TypeId::of::<ExternalDragText>()),
        ],
    );
    let number = 42_u32;
    let text = ExternalDragText::new("topmost");
    let offers = [
        (TypeId::of::<u32>(), &number as &dyn Any),
        (TypeId::of::<ExternalDragText>(), &text as &dyn Any),
    ];
    assert_eq!(
        snapshot.offer_target_at(Point::new(20.0, 20.0), offers.into_iter()),
        Some((ElementId::new(2), 1))
    );

    // Within one target the native offer order is stable: process-local typed data precedes
    // its public representation, so no serialization is needed between QuickGUI windows.
    tree.update_external_drop_snapshot(
        &mut snapshot,
        &[
            (ElementId::new(2), TypeId::of::<u32>()),
            (ElementId::new(2), TypeId::of::<ExternalDragText>()),
        ],
    );
    assert_eq!(
        snapshot.offer_target_at(Point::new(20.0, 20.0), offers.into_iter()),
        Some((ElementId::new(2), 0))
    );
}

#[cfg(target_os = "macos")]
#[test]
fn native_drop_snapshot_supports_arbitrary_typed_predicates() {
    let mut tree = UiTree::new();
    let bounds = Rect::new(0.0, 0.0, 40.0, 40.0);
    tree.hit_regions.push(HitRegion {
        transform: None,
        id: ElementId::new(9),
        bounds,
        clip: bounds,
        clickable: false,
        pointer_listener: false,
        drag_source: false,
        drop_target: true,
        focusable: false,
        cursor_style: None,
        cursor_states: CursorStateStyles::default(),
        stateful: true,
        blocks_pointer: false,
        app_region: None,
        order: PaintOrder {
            layer: PaintLayerKey::default(),
            source: 0,
        },
    });
    tree.drop_predicates.insert(
        (ElementId::new(9), TypeId::of::<u32>()),
        Arc::new(|value| {
            value
                .downcast_ref::<u32>()
                .is_some_and(|value| *value >= 10)
        }),
    );

    let mut snapshot = ExternalDropSnapshot::new();
    tree.update_external_drop_snapshot(&mut snapshot, &[(ElementId::new(9), TypeId::of::<u32>())]);
    let point = Point::new(10.0, 10.0);
    assert_eq!(snapshot.target_at(point, TypeId::of::<u32>(), &9_u32), None);
    assert_eq!(
        snapshot.target_at(point, TypeId::of::<u32>(), &10_u32),
        Some(ElementId::new(9))
    );

    tree.drop_predicates.insert(
        (ElementId::new(9), TypeId::of::<u32>()),
        Arc::new(|_| panic!("application predicate panic")),
    );
    tree.update_external_drop_snapshot(&mut snapshot, &[(ElementId::new(9), TypeId::of::<u32>())]);
    assert_eq!(
        snapshot.target_at(point, TypeId::of::<u32>(), &10_u32),
        None
    );
}

#[cfg(target_os = "macos")]
#[test]
fn native_drop_acceptance_cap_fails_closed_in_listener_order() {
    let mut tree = UiTree::new();
    let bounds = Rect::new(0.0, 0.0, 40.0, 40.0);
    let target = ElementId::new((MAX_EXTERNAL_DROP_ACCEPTANCES + 2) as u64);
    let region = |id, source| HitRegion {
        transform: None,
        id,
        bounds,
        clip: bounds,
        clickable: false,
        pointer_listener: false,
        drag_source: false,
        drop_target: true,
        focusable: false,
        cursor_style: None,
        cursor_states: CursorStateStyles::default(),
        stateful: true,
        blocks_pointer: false,
        app_region: None,
        order: PaintOrder {
            layer: PaintLayerKey::default(),
            source,
        },
    };
    // The lower target is within the acceptance cap. The omitted top target must not let the
    // offer reach through to it, so a truncated acceptance table rejects the complete offer.
    tree.hit_regions.push(region(ElementId::new(1), 0));
    tree.hit_regions.push(region(target, 1));
    let mut listeners = (0..MAX_EXTERNAL_DROP_ACCEPTANCES)
        .map(|index| (ElementId::new(index as u64 + 1), TypeId::of::<u32>()))
        .collect::<Vec<_>>();
    listeners.push((target, TypeId::of::<u32>()));

    let mut snapshot = ExternalDropSnapshot::new();
    tree.update_external_drop_snapshot(&mut snapshot, &listeners);
    assert!(snapshot.is_truncated());
    assert_eq!(
        snapshot.target_at(Point::new(10.0, 10.0), TypeId::of::<u32>(), &42_u32,),
        None
    );
}

#[cfg(target_os = "macos")]
#[test]
fn native_drop_snapshot_retains_only_the_topmost_bounded_stack() {
    use crate::ExternalDragText;

    let mut tree = UiTree::new();
    let bounds = Rect::new(0.0, 0.0, 20.0, 20.0);
    for source in 0..=MAX_EXTERNAL_DROP_HIT_REGIONS {
        tree.hit_regions.push(HitRegion {
            transform: None,
            id: ElementId::new(source as u64 + 1),
            bounds,
            clip: bounds,
            clickable: false,
            pointer_listener: false,
            drag_source: false,
            drop_target: true,
            focusable: false,
            cursor_style: None,
            cursor_states: CursorStateStyles::default(),
            stateful: true,
            blocks_pointer: false,
            app_region: None,
            order: PaintOrder {
                layer: PaintLayerKey::default(),
                source,
            },
        });
    }
    let mut snapshot = ExternalDropSnapshot::new();
    let listeners = (0..=MAX_EXTERNAL_DROP_HIT_REGIONS)
        .map(|source| {
            (
                ElementId::new(source as u64 + 1),
                TypeId::of::<ExternalDragText>(),
            )
        })
        .collect::<Vec<_>>();
    tree.update_external_drop_snapshot(&mut snapshot, &listeners);
    assert!(snapshot.is_truncated());
    assert_eq!(snapshot.regions.len(), MAX_EXTERNAL_DROP_HIT_REGIONS);
    assert_eq!(
        snapshot.target_at(
            Point::new(10.0, 10.0),
            TypeId::of::<ExternalDragText>(),
            &ExternalDragText::new("payload"),
        ),
        Some(ElementId::new(MAX_EXTERNAL_DROP_HIT_REGIONS as u64 + 1))
    );
}

#[test]
fn immutable_text_selection_copies_across_visual_nodes_in_document_order() {
    let ids = [
        ElementId::new(101),
        ElementId::new(102),
        ElementId::new(103),
    ];
    let mut tree = UiTree::new();
    tree.selectable_texts = vec![
        SelectableTextEntry {
            id: ids[0],
            content: Arc::from("alpha"),
            character_lengths: selectable_character_lengths("alpha").into(),
        },
        SelectableTextEntry {
            id: ids[1],
            content: Arc::from("beta"),
            character_lengths: selectable_character_lengths("beta").into(),
        },
        SelectableTextEntry {
            id: ids[2],
            content: Arc::from("gamma"),
            character_lengths: selectable_character_lengths("gamma").into(),
        },
    ];
    tree.selectable_text_indices = ids
        .into_iter()
        .enumerate()
        .map(|(index, id)| (id, index))
        .collect();
    tree.element_bounds
        .insert(ids[0], Rect::new(0.0, 0.0, 50.0, 20.0));
    tree.element_bounds
        .insert(ids[1], Rect::new(58.0, 0.0, 40.0, 20.0));
    tree.element_bounds
        .insert(ids[2], Rect::new(0.0, 28.0, 60.0, 20.0));
    tree.static_text_selection = Some(StaticTextSelection {
        anchor: StaticTextPosition {
            id: ids[0],
            offset: 2,
        },
        focus: StaticTextPosition {
            id: ids[2],
            offset: 2,
        },
    });

    assert_eq!(tree.selected_static_text().as_deref(), Some("pha beta\nga"));

    tree.static_text_selection = tree
        .static_text_selection
        .map(|selection| StaticTextSelection {
            anchor: selection.focus,
            focus: selection.anchor,
        });
    assert_eq!(tree.selected_static_text().as_deref(), Some("pha beta\nga"));
}

#[test]
fn immutable_text_selection_clips_ranges_per_leaf() {
    let ids = [
        ElementId::new(201),
        ElementId::new(202),
        ElementId::new(203),
    ];
    let indices = ids
        .into_iter()
        .enumerate()
        .map(|(index, id)| (id, index))
        .collect::<HashMap<_, _>>();
    let selection = Some(StaticTextSelection {
        anchor: StaticTextPosition {
            id: ids[0],
            offset: 2,
        },
        focus: StaticTextPosition {
            id: ids[2],
            offset: 3,
        },
    });

    assert_eq!(
        static_selection_range_for_entry(selection, &indices, 0, 5),
        Some(2..5)
    );
    assert_eq!(
        static_selection_range_for_entry(selection, &indices, 1, 4),
        Some(0..4)
    );
    assert_eq!(
        static_selection_range_for_entry(selection, &indices, 2, 5),
        Some(0..3)
    );
}

#[test]
fn immutable_text_copy_limit_keeps_utf8_boundaries() {
    let mut output = String::new();
    push_bounded_text(&mut output, "a🙂b", 4);
    assert_eq!(output, "a");
    assert!(output.is_char_boundary(output.len()));
}

#[test]
fn immutable_text_drag_uses_visible_bounds_in_shared_clips() {
    let clip = Rect::new(0.0, 0.0, 400.0, 400.0);
    let region = |document_index, bounds, source| SelectableTextRegion {
        links: Vec::new(),
        document_index,
        bounds,
        clip,
        style: TextStyle::new(14.0, Color::WHITE),
        highlights: None,
        order: PaintOrder {
            layer: PaintLayerKey::default(),
            source,
        },
    };
    let regions = [
        region(0, Rect::new(20.0, 20.0, 200.0, 20.0), 0),
        region(1, Rect::new(20.0, 120.0, 200.0, 20.0), 1),
    ];

    assert_eq!(
        nearest_selectable_text_region(&regions, Point::new(40.0, 55.0))
            .map(|region| region.document_index),
        Some(0)
    );
    assert_eq!(
        nearest_selectable_text_region(&regions, Point::new(40.0, 105.0))
            .map(|region| region.document_index),
        Some(1)
    );
}

#[test]
fn focusing_an_input_clears_static_selection_and_keeps_clipboards_separate() {
    let text_id = ElementId::new(401);
    let input_id = ElementId::new(402);
    let mut tree = UiTree::new();
    tree.selectable_texts.push(SelectableTextEntry {
        id: text_id,
        content: Arc::from("document"),
        character_lengths: selectable_character_lengths("document").into(),
    });
    tree.selectable_text_indices.insert(text_id, 0);
    tree.static_text_selection = Some(StaticTextSelection {
        anchor: StaticTextPosition {
            id: text_id,
            offset: 0,
        },
        focus: StaticTextPosition {
            id: text_id,
            offset: "document".len(),
        },
    });
    tree.focus_order.push(input_id);
    tree.focusable_ids.insert(input_id);
    let mut input = TextInputState::new("input value");
    input.set_selection(0, "input".len());
    tree.text_inputs.insert(input_id, input);

    assert!(tree.focus_next(false));
    assert!(tree.static_text_selection.is_none());
    assert_eq!(tree.selected_input_text().as_deref(), Some("input"));
    assert_eq!(tree.selected_text().as_deref(), Some("input"));
}

#[test]
fn immutable_text_multi_clicks_select_unicode_words_and_complete_lines() {
    let content = "alpha 世界!\nsecond line\n";
    let world = content.find('世').unwrap();
    assert_eq!(word_range_at(content, world), world..world + "世界".len());
    assert_eq!(
        line_range_at(content, world),
        0..content.find('\n').unwrap() + 1
    );

    let mut tree = UiTree::new();
    let id = ElementId::new(301);
    let position = StaticTextPosition { id, offset: world };
    let point = Point::new(20.0, 20.0);
    let now = Instant::now();
    assert_eq!(
        tree.static_text_click_unit(point, position, now, false),
        StaticTextSelectionUnit::Character
    );
    assert_eq!(
        tree.static_text_click_unit(point, position, now + Duration::from_millis(100), false,),
        StaticTextSelectionUnit::Word
    );
    assert_eq!(
        tree.static_text_click_unit(point, position, now + Duration::from_millis(200), false,),
        StaticTextSelectionUnit::Line
    );
}

#[test]
fn automatic_user_selection_matches_web_control_boundaries() {
    let mut root = div()
        .child(text("plain"))
        .child(button().child(text("button")))
        .child(div().user_select_none().child(text("disabled")))
        .child(button().user_select_text().child(text("forced")));
    let mut taffy = TaffyTree::new();
    let mut seen = HashSet::new();
    let inherited = TextStyle::new(14.0, Color::WHITE);
    build_layout_node(
        &mut taffy,
        &mut seen,
        &mut root,
        ElementId::new(999),
        0,
        &inherited,
        true,
        Direction::Ltr,
    )
    .unwrap();

    assert!(root.children[0].resolved_user_select);
    assert!(!root.children[1].resolved_user_select);
    assert!(!root.children[1].children[0].resolved_user_select);
    assert!(!root.children[2].children[0].resolved_user_select);
    assert!(root.children[3].resolved_user_select);
    assert!(root.children[3].children[0].resolved_user_select);
}
