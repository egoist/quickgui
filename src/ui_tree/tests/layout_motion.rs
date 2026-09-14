use super::*;

#[test]
fn hit_slop_expands_only_the_interaction_bounds() {
    let visual = Rect::new(20.0, 30.0, 1.0, 80.0);
    let interaction = expand_hit_bounds(
        visual,
        Insets {
            top: 2.0,
            right: 3.0,
            bottom: 4.0,
            left: 5.0,
        },
    );

    assert_eq!(visual, Rect::new(20.0, 30.0, 1.0, 80.0));
    assert_eq!(interaction, Rect::new(15.0, 28.0, 9.0, 86.0));
}

#[test]
fn generated_ids_are_path_stable() {
    assert_eq!(mix_id(42, 3), mix_id(42, 3));
    assert_ne!(mix_id(42, 3), mix_id(42, 4));
}

#[test]
fn container_query_receives_its_fixed_box_and_isolates_child_layout() {
    let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let callback_observed = observed.clone();
    let root = div().size(500.0, 300.0).child(
        container_query(move |size| {
            callback_observed.borrow_mut().push(size);
            div().id("oversized-query-child").size(900.0, 700.0)
        })
        .size(240.0, 120.0)
        .flex_none(),
    );
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;

    tree.set_root(root, Size::new(500.0, 300.0), 1.0, &mut renderer)
        .unwrap();

    assert_eq!(observed.borrow().as_slice(), &[Size::new(240.0, 120.0)]);
    let query = &tree.root.as_ref().unwrap().children[0];
    let query_layout = tree.taffy.layout(query.taffy_node.unwrap()).unwrap();
    let child = &query.children[0];
    let child_layout = tree.taffy.layout(child.taffy_node.unwrap()).unwrap();
    assert_eq!(query_layout.size.width, 240.0);
    assert_eq!(query_layout.size.height, 120.0);
    assert_eq!(child_layout.size.width, 900.0);
    assert_eq!(child_layout.size.height, 700.0);

    let mut scene = Scene::new();
    tree.paint_at(&mut scene, &mut renderer, Instant::now())
        .unwrap();
    assert_eq!(
        tree.element_bounds("oversized-query-child".into()),
        Some(Rect::new(0.0, 0.0, 900.0, 700.0))
    );
}

#[test]
fn container_query_redeclares_only_when_its_assigned_size_changes() {
    let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let callback_calls = calls.clone();
    let root = container_query(move |size| {
        callback_calls.borrow_mut().push(size);
        if size.width < 300.0 {
            div().id("narrow-query-result")
        } else {
            div().id("wide-query-result")
        }
    });
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;

    tree.set_root(root, Size::new(240.0, 160.0), 1.0, &mut renderer)
        .unwrap();
    assert!(tree.contains_element("narrow-query-result".into()));
    assert!(!tree.contains_element("wide-query-result".into()));

    tree.layout(Size::new(520.0, 160.0), 1.0, &mut renderer)
        .unwrap();
    assert!(!tree.contains_element("narrow-query-result".into()));
    assert!(tree.contains_element("wide-query-result".into()));

    tree.layout(Size::new(520.0, 160.0), 1.0, &mut renderer)
        .unwrap();
    assert_eq!(
        calls.borrow().as_slice(),
        &[Size::new(240.0, 160.0), Size::new(520.0, 160.0)]
    );
}

#[test]
fn container_query_prepares_callback_subtrees_before_layout() {
    let prepared = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    let prepared_for_hook = prepared.clone();

    tree.set_root_with_prepare(
        container_query(|_| div().id("prepared-query-result")),
        Size::new(320.0, 200.0),
        1.0,
        &mut renderer,
        move |subtree| {
            assert_eq!(subtree.explicit_id, Some("prepared-query-result".into()));
            prepared_for_hook.set(prepared_for_hook.get() + 1);
        },
    )
    .unwrap();

    assert_eq!(prepared.get(), 1);
}

#[test]
fn container_query_descendants_participate_in_initial_focus_resolution() {
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;

    tree.set_root(
        container_query(|_| {
            crate::text_input("")
                .id("query-autofocus-input")
                .auto_focus()
        }),
        Size::new(320.0, 200.0),
        1.0,
        &mut renderer,
    )
    .unwrap();

    assert_eq!(tree.focused(), Some("query-autofocus-input".into()));
}

#[test]
fn container_query_shells_do_not_unmount_stable_retained_state() {
    let started = Instant::now();
    let animated = playback_animation(AnimationRepeat::Infinite);
    let declaration = || {
        let animated = animated.clone();
        container_query(move |_| {
            div().children([
                div()
                    .id("query-scroll-state")
                    .size(100.0, 100.0)
                    .overflow_y_scroll(),
                img(animated.clone()).id("query-animated-image"),
                div()
                    .id("query-transition-state")
                    .transition(Transition::colors(Duration::from_millis(100))),
            ])
        })
    };
    let mut tree = UiTree::new_at(started);
    let mut renderer = TestTextLayout;
    tree.set_root(declaration(), Size::new(360.0, 240.0), 1.0, &mut renderer)
        .unwrap();

    let scroll_id: ElementId = "query-scroll-state".into();
    let image_id: ElementId = "query-animated-image".into();
    let transition_id: ElementId = "query-transition-state".into();
    tree.scroll_offsets
        .insert(scroll_id, Vector::new(0.0, 37.0));
    tree.animations.get_mut(&image_id).unwrap().elapsed = Duration::from_millis(23);
    let transition = Transition::colors(Duration::from_millis(100));
    tree.style_transitions.insert(
        transition_id,
        StyleTransitionPlayback::new(
            transition_test_style(Color::BLACK, 0.0),
            &transition,
            started,
        ),
    );

    tree.set_root(declaration(), Size::new(360.0, 240.0), 1.0, &mut renderer)
        .unwrap();

    assert_eq!(tree.scroll_offsets[&scroll_id].y, 37.0);
    assert_eq!(
        tree.animations[&image_id].elapsed,
        Duration::from_millis(23)
    );
    assert!(tree.style_transitions.contains_key(&transition_id));
}

#[test]
fn keyed_query_motion_survives_a_size_redeclaration_and_unmounts_cleanly() {
    let start = Instant::now();
    let root = container_query(|size| {
        if size.width < 500.0 {
            div().with_animation(
                "query-motion",
                Animation::new(Duration::from_millis(100)),
                |element, phase| element.w(100.0 * phase),
            )
        } else {
            div()
        }
    });
    let mut tree = UiTree::new_at(start);
    let mut renderer = TestTextLayout;
    tree.set_root_unlaid(root, Size::new(240.0, 160.0), 1.0, start, false)
        .unwrap();
    tree.layout_with_prepare_at(
        Size::new(240.0, 160.0),
        1.0,
        &mut renderer,
        start,
        &mut |_| {},
    )
    .unwrap();

    let motion_id: ElementId = "query-motion".into();
    assert_eq!(tree.declarative_animations[&motion_id].last_value, 0.0);
    tree.layout_with_prepare_at(
        Size::new(360.0, 160.0),
        1.0,
        &mut renderer,
        start + Duration::from_millis(50),
        &mut |_| {},
    )
    .unwrap();
    assert!(
        (tree.declarative_animations[&motion_id].last_value - 0.5).abs() < 0.001,
        "the stable animation ID should retain elapsed time across callback replacement"
    );

    tree.layout_with_prepare_at(
        Size::new(600.0, 160.0),
        1.0,
        &mut renderer,
        start + Duration::from_millis(75),
        &mut |_| {},
    )
    .unwrap();
    assert!(!tree.declarative_animations.contains_key(&motion_id));
    assert!(!tree.declarative_animation_ids.contains(&motion_id));
}

#[test]
fn detached_container_queries_are_pointer_passive_and_resize_on_demand() {
    let start = Instant::now();
    let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let callback_observed = observed.clone();
    let template = container_query(move |size| {
        callback_observed.borrow_mut().push(size);
        div().clickable().cursor_pointer().with_animation(
            "detached-query-motion",
            Animation::new(Duration::from_millis(100)),
            |element, phase| element.w(10.0 + phase),
        )
    });
    let mut renderer = TestTextLayout;
    let mut tree = DetachedTree::new(
        template,
        ElementId::new(0xfeed),
        Size::new(240.0, 160.0),
        1.0,
        &mut renderer,
        start,
        start,
        true,
        false,
    )
    .unwrap();

    let child = &tree.root.children[0];
    assert!(!child.clickable);
    assert_eq!(child.cursor_style, None);
    assert!(child.animation.is_none());
    assert!(
        tree.motion
            .animations
            .contains_key(&ElementId::from("detached-query-motion"))
    );

    tree.layout(Size::new(420.0, 160.0), 1.0, &mut renderer)
        .unwrap();
    assert!(tree.motion.needs_resolve);
    assert!(
        tree.refresh(
            Size::new(420.0, 160.0),
            1.0,
            &mut renderer,
            start + Duration::from_millis(10),
            true,
            false,
        )
        .unwrap()
    );
    assert_eq!(
        observed.borrow().as_slice(),
        &[Size::new(240.0, 160.0), Size::new(420.0, 160.0)]
    );
}

#[test]
fn container_query_depth_is_hard_bounded_before_nested_layout_allocation() {
    fn nested_query(depth: usize) -> Element {
        if depth == 0 {
            div()
        } else {
            container_query(move |_| nested_query(depth - 1))
        }
    }

    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    let result = tree.set_root(
        nested_query(MAX_CONTAINER_QUERY_DEPTH + 1),
        Size::new(320.0, 200.0),
        1.0,
        &mut renderer,
    );

    assert!(matches!(result, Err(UiError::ContainerQueryDepthExceeded)));
}

#[test]
fn container_query_count_is_hard_bounded_before_layout() {
    let root =
        div().children((0..=MAX_CONTAINER_QUERIES_PER_WINDOW).map(|_| container_query(|_| div())));
    let mut tree = UiTree::new();
    let result = tree.set_root_for_test(root, Size::new(640.0, 480.0), 1.0, Instant::now());

    assert!(matches!(result, Err(UiError::TooManyContainerQueries)));
    assert!(tree.root_node.is_none());
}

#[test]
fn declarative_animation_chains_carry_elapsed_time_across_stages() {
    let start = Instant::now();
    let stages = [
        Animation::new(Duration::from_millis(100)),
        Animation::new(Duration::from_millis(200)),
    ];
    let mut playback = DeclarativeAnimationPlayback::new(start);

    let first = playback.sample(&stages, start, start, true, false);
    assert_eq!(first.animation_ix, 0);
    assert_eq!(first.value, 0.0);

    let second = playback.sample(
        &stages,
        start + Duration::from_millis(150),
        start,
        true,
        false,
    );
    assert_eq!(second.animation_ix, 1);
    assert!((second.value - 0.25).abs() < 0.001);

    let final_sample = playback.sample(
        &stages,
        start + Duration::from_millis(300),
        start,
        true,
        false,
    );
    assert_eq!(final_sample.animation_ix, 1);
    assert_eq!(final_sample.value, 1.0);
    assert!(!final_sample.request_frame);
    assert_eq!(final_sample.deadline, None);
}

#[test]
fn duplicate_declarative_motion_ids_fail_before_layout() {
    let animation = || Animation::new(Duration::from_millis(100));
    let root = div().children([
        div().with_animation("shared-motion", animation(), |element, _| element),
        div().with_animation("shared-motion", animation(), |element, _| element),
    ]);
    let mut tree = UiTree::new();
    let result = tree.set_root_for_test(root, Size::new(640.0, 480.0), 1.0, Instant::now());

    assert!(matches!(result, Err(UiError::DuplicateAnimationId(_))));
}

#[test]
fn style_transition_registry_is_hard_bounded_before_layout() {
    let root = div().children(
        (0..=MAX_STYLE_TRANSITIONS_PER_WINDOW)
            .map(|_| div().transition(Transition::colors(Duration::from_millis(100)))),
    );
    let mut tree = UiTree::new();
    let result = tree.set_root_for_test(root, Size::new(640.0, 480.0), 1.0, Instant::now());

    assert!(matches!(result, Err(UiError::TooManyStyleTransitions)));
    assert!(tree.root_node.is_none());
    assert!(tree.style_transitions.is_empty());
}

fn transition_test_style(background: Color, radius: f32) -> TransitionPaintStyle {
    TransitionPaintStyle {
        background,
        border_color: Color::TRANSPARENT,
        border_widths: Insets::default(),
        radius,
        opacity: 1.0,
        transform: Transform2D::IDENTITY,
        text_color: None,
        text_fallback: Color::WHITE,
        shadows: TransitionShadowList::empty(),
    }
}

#[test]
fn style_transitions_retarget_continuously_and_apply_unselected_properties_immediately() {
    let start = Instant::now();
    let from = transition_test_style(Color::BLACK, 0.0);
    let to = transition_test_style(Color::WHITE, 20.0);
    let config = Transition::colors(Duration::from_millis(100));
    let mut playback = StyleTransitionPlayback::new(from, &config, start);

    let initial = playback.sample(to, &config, start, true, false);
    assert_eq!(initial.background, Color::BLACK);
    assert_eq!(initial.radius, 20.0);
    assert!(playback.requests_frame());

    let midpoint = playback.sample(to, &config, start + Duration::from_millis(50), true, false);
    assert!((midpoint.background.r - 0.5).abs() < 0.001);
    assert_eq!(midpoint.radius, 20.0);

    let reversed = playback.sample(
        from,
        &config,
        start + Duration::from_millis(50),
        true,
        false,
    );
    assert_eq!(reversed.background, midpoint.background);
    assert_eq!(reversed.radius, 0.0);

    let reduced = playback.sample(to, &config, start + Duration::from_millis(60), true, true);
    assert_eq!(reduced, to);
    assert!(!playback.active);
    assert_eq!(playback.deadline(), None);
}

#[test]
fn style_transitions_fade_transparent_colors_without_a_dark_fringe() {
    let surface = Color::rgb8(22, 22, 21);
    let hover = Color::rgb8(36, 36, 35);
    let from = transition_test_style(Color::TRANSPARENT, 0.0);
    let to = transition_test_style(hover, 0.0);

    let midpoint = TransitionPaintStyle::interpolate(from, to, 0.5, TransitionProperties::COLORS);

    assert!((midpoint.background.a - 0.5).abs() < 0.001);
    assert!((midpoint.background.r - hover.r).abs() < 0.001);
    assert!((midpoint.background.g - hover.g).abs() < 0.001);
    assert!((midpoint.background.b - hover.b).abs() < 0.001);
    for (source, backdrop, target) in [
        (midpoint.background.r, surface.r, hover.r),
        (midpoint.background.g, surface.g, hover.g),
        (midpoint.background.b, surface.b, hover.b),
    ] {
        let composited = source * midpoint.background.a + backdrop * (1.0 - midpoint.background.a);
        assert!(composited > backdrop);
        assert!(composited < target);
    }
}

#[test]
fn style_transitions_interpolate_opacity_without_including_it_in_colors() {
    let start = Instant::now();
    let mut from = transition_test_style(Color::BLACK, 0.0);
    from.opacity = 0.0;
    let mut to = transition_test_style(Color::BLACK, 0.0);
    to.opacity = 1.0;

    let all = Transition::new(Duration::from_millis(100));
    let mut playback = StyleTransitionPlayback::new(from, &all, start);
    playback.sample(to, &all, start, true, false);
    let midpoint = playback.sample(to, &all, start + Duration::from_millis(50), true, false);
    assert!((midpoint.opacity - 0.5).abs() < 0.001);

    let colors = Transition::colors(Duration::from_millis(100));
    let mut playback = StyleTransitionPlayback::new(from, &colors, start);
    assert_eq!(
        playback.sample(to, &colors, start, true, false).opacity,
        1.0
    );
    assert!(!playback.requests_frame());
}

#[test]
fn throttled_style_transitions_wake_at_completion_even_below_one_fps() {
    let start = Instant::now();
    let from = transition_test_style(Color::BLACK, 0.0);
    let to = transition_test_style(Color::WHITE, 0.0);
    let config = Transition::colors(Duration::from_millis(100)).with_max_fps(0.5);
    let mut playback = StyleTransitionPlayback::new(from, &config, start);

    playback.sample(to, &config, start, true, false);
    assert_eq!(
        playback.deadline(),
        Some(start + Duration::from_millis(100))
    );
    assert!(!playback.requests_frame());
    assert!(!playback.deadline_due(start + Duration::from_millis(99)));
    assert!(playback.deadline_due(start + Duration::from_millis(100)));

    let final_style = playback.sample(to, &config, start + Duration::from_millis(100), true, false);
    assert_eq!(final_style, to);
    assert!(!playback.active);
    assert_eq!(playback.deadline(), None);
}

#[test]
fn transform_transitions_retarget_and_respect_reduced_motion() {
    let now = Instant::now();
    let from = transition_test_style(Color::WHITE, 10.0);
    let mut to = from;
    to.transform = Transform2D::translate(20.0, 0.0);
    let config = Transition::new(Duration::from_millis(100))
        .with_properties(TransitionProperties::TRANSFORM)
        .with_easing(crate::linear);
    let mut playback = StyleTransitionPlayback::new(from, &config, now);
    assert_eq!(
        playback.sample(to, &config, now, true, false).transform,
        from.transform
    );
    let halfway = now + Duration::from_millis(50);
    assert_eq!(
        playback.sample(to, &config, halfway, true, false).transform,
        Transform2D::translate(10.0, 0.0)
    );
    assert_eq!(
        playback
            .sample(from, &config, halfway, true, false)
            .transform,
        Transform2D::translate(10.0, 0.0)
    );
    assert_eq!(
        playback
            .sample(from, &config, now + Duration::from_millis(100), true, false)
            .transform,
        Transform2D::translate(5.0, 0.0)
    );
    assert_eq!(
        playback.sample(to, &config, now + Duration::from_millis(110), true, true),
        to
    );
    assert!(!playback.requests_frame());
    assert_eq!(playback.deadline(), None);
    let colors = Transition::colors(Duration::from_millis(100));
    let mut playback = StyleTransitionPlayback::new(from, &colors, now);
    assert_eq!(
        playback.sample(to, &colors, now, true, false).transform,
        to.transform
    );
    assert!(!playback.requests_frame());
}

#[test]
fn transform_transitions_preserve_rotation_and_singular_endpoints() {
    let from = Transform2D::IDENTITY;
    let to = Transform2D::rotate_degrees(90.0);
    let middle = interpolate_transition_transform(from, to, 0.5);
    let expected = Transform2D::rotate_degrees(45.0);
    assert!((middle.a - expected.a).abs() < 0.0001);
    assert!((middle.b - expected.b).abs() < 0.0001);
    assert!((middle.determinant() - 1.0).abs() < 0.0001);
    let across_wrap = interpolate_transition_transform(
        Transform2D::rotate_degrees(170.0),
        Transform2D::rotate_degrees(-170.0),
        0.5,
    );
    assert!((across_wrap.a + 1.0).abs() < 0.0001);
    assert!(across_wrap.b.abs() < 0.0001);
    let collapsed = Transform2D::scale(0.0, 1.0);
    assert_eq!(
        interpolate_transition_transform(from, collapsed, 0.5),
        Transform2D::scale(0.5, 1.0)
    );
    assert_eq!(interpolate_transition_transform(from, to, 0.0), from);
    assert_eq!(interpolate_transition_transform(from, to, 1.0), to);
}

#[test]
fn transform_transitions_move_paint_and_hit_regions_without_moving_layout() {
    for (from, to) in [(0.0, 20.0), (-40.0, 40.0)] {
        let thumb: ElementId = "animated-thumb".into();
        let sibling: ElementId = "static-sibling".into();
        let declaration = |offset| {
            div().size(100.0, 30.0).flex_row().children([
                div()
                    .id(thumb)
                    .size(20.0, 20.0)
                    .flex_none()
                    .clickable()
                    .bg(Color::WHITE)
                    .translate(offset, 0.0)
                    .transition(
                        Transition::new(Duration::from_millis(100))
                            .with_properties(TransitionProperties::TRANSFORM)
                            .with_easing(crate::linear),
                    ),
                div().id(sibling).size(20.0, 20.0).flex_none(),
            ])
        };
        let now = Instant::now();
        let viewport = Size::new(100.0, 30.0);
        let mut tree = UiTree::new();
        let mut renderer = TestTextLayout;
        let mut scene = Scene::new();
        tree.set_root(declaration(from), viewport, 1.0, &mut renderer)
            .unwrap();
        tree.paint_at(&mut scene, &mut renderer, now).unwrap();
        assert_eq!(tree.element_bounds(thumb).unwrap().x, from);
        tree.set_root(declaration(to), viewport, 1.0, &mut renderer)
            .unwrap();
        scene.clear(Color::TRANSPARENT);
        tree.paint_at(&mut scene, &mut renderer, now).unwrap();
        assert_eq!(tree.element_bounds(thumb).unwrap().x, from);
        scene.clear(Color::TRANSPARENT);
        tree.paint_at(&mut scene, &mut renderer, now + Duration::from_millis(50))
            .unwrap();
        let middle = Rect::new((from + to) / 2.0, 0.0, 20.0, 20.0);
        assert_eq!(tree.element_bounds(thumb), Some(middle));
        assert_eq!(tree.element_bounds(sibling).unwrap().x, 20.0);
        assert!(
            scene
                .edge_quads()
                .iter()
                .any(|quad| quad.rect == middle && quad.fill == Color::WHITE)
        );
        // A pre-paint hover refresh must use the visible position, not jump ahead to the target.
        tree.refresh_hover_after_layout(Some(Point::new(middle.x + 5.0, 5.0)))
            .unwrap();
        assert_eq!(
            tree.hit_regions
                .iter()
                .find(|region| region.id == thumb)
                .unwrap()
                .bounds,
            middle
        );
        assert!(tree.style_transition_frame_requested());
        scene.clear(Color::TRANSPARENT);
        tree.paint_at(&mut scene, &mut renderer, now + Duration::from_millis(100))
            .unwrap();
        assert_eq!(tree.element_bounds(thumb).unwrap().x, to);
        assert!(!tree.style_transition_frame_requested());
    }
}

#[test]
fn paused_style_transitions_preserve_time_and_shadow_interpolation_is_bounded() {
    let start = Instant::now();
    let mut from = transition_test_style(Color::BLACK, 0.0);
    let mut to = transition_test_style(Color::WHITE, 0.0);
    to.shadows = TransitionShadowList::from_slice(&[BoxShadow::new(10.0, 20.0, Color::WHITE)
        .blur_radius(30.0)
        .spread_radius(4.0)]);
    from.shadows = TransitionShadowList::empty();
    let config = Transition::new(Duration::from_millis(100));
    let mut playback = StyleTransitionPlayback::new(from, &config, start);
    playback.sample(to, &config, start, true, false);

    playback.pause(start + Duration::from_millis(25));
    let paused = playback.current;
    assert_eq!(paused.shadows.as_slice().len(), 1);
    assert!((paused.shadows.as_slice()[0].offset().x - 1.25).abs() < 0.001);
    playback.resume(start + Duration::from_secs(10));
    let resumed = playback.sample(to, &config, start + Duration::from_secs(10), true, false);
    assert_eq!(resumed, paused);

    let later = playback.sample(
        to,
        &config,
        start + Duration::from_secs(10) + Duration::from_millis(25),
        true,
        false,
    );
    assert!((later.background.r - 0.5).abs() < 0.001);
    assert!((later.shadows.as_slice()[0].offset().x - 5.0).abs() < 0.001);
}

#[test]
fn capped_declarative_oneshots_always_schedule_their_terminal_frame() {
    let start = Instant::now();
    let stages = [Animation::new(Duration::from_millis(100)).with_max_fps(0.5)];
    let mut playback = DeclarativeAnimationPlayback::new(start);
    let sample = playback.sample(&stages, start, start, true, false);

    assert_eq!(sample.deadline, Some(start + Duration::from_millis(100)));
}

#[test]
fn detached_motion_retains_exact_time_and_sanitizes_every_resolved_frame() {
    let start = Instant::now();
    let template = div()
        .clickable()
        .cursor_pointer()
        .hover(|style| style.cursor_crosshair())
        .with_animation(
            "detached-motion",
            Animation::new(Duration::from_millis(100)).with_max_fps(20.0),
            |element, phase| {
                element
                    .w(10.0 + 90.0 * phase)
                    .clickable()
                    .cursor_copy()
                    .active(|style| style.cursor_grabbing())
                    .tooltip("animator-added interaction")
            },
        );
    let mut motion = DetachedMotionState::new(template, start);

    let initial = motion.resolve(start, true, false).unwrap();
    assert_eq!(absolute_length(initial.layout.size.width), Some(10.0));
    assert!(!initial.clickable);
    assert_eq!(initial.cursor_style, None);
    assert_eq!(initial.hover.cursor_style, None);
    assert_eq!(initial.active.cursor_style, None);
    assert!(initial.tooltip.is_none());
    assert!(initial.animation.is_none());
    assert!(!motion.frame_requested);
    assert_eq!(motion.deadline, Some(start + Duration::from_millis(50)));

    let midpoint = start + Duration::from_millis(50);
    assert!(!motion.deadline_due(midpoint - Duration::from_millis(1)));
    assert!(motion.deadline_due(midpoint));
    let midpoint_root = motion.resolve(midpoint, true, false).unwrap();
    assert_eq!(absolute_length(midpoint_root.layout.size.width), Some(55.0));
    assert!(!midpoint_root.clickable);
    assert_eq!(midpoint_root.cursor_style, None);
    assert_eq!(midpoint_root.hover.cursor_style, None);
    assert_eq!(midpoint_root.active.cursor_style, None);
    assert!(midpoint_root.tooltip.is_none());

    motion.pause(midpoint);
    let resumed_at = start + Duration::from_secs(10);
    motion.resume(resumed_at);
    let resumed = motion.resolve(resumed_at, true, false).unwrap();
    assert_eq!(absolute_length(resumed.layout.size.width), Some(55.0));
    assert_eq!(
        motion.deadline,
        Some(resumed_at + Duration::from_millis(50))
    );

    let completed_at = resumed_at + Duration::from_millis(50);
    assert!(motion.deadline_due(completed_at));
    let completed = motion.resolve(completed_at, true, false).unwrap();
    assert_eq!(absolute_length(completed.layout.size.width), Some(100.0));
    assert!(!motion.frame_requested);
    assert_eq!(motion.deadline, None);
    assert_eq!(motion.active_count(), 0);
}

#[test]
fn detached_motion_uses_a_local_duplicate_id_namespace() {
    let start = Instant::now();
    let animation = || Animation::new(Duration::from_millis(100));
    let template = div().children([
        div().with_animation("duplicate-detached", animation(), |element, _| element),
        div().with_animation("duplicate-detached", animation(), |element, _| element),
    ]);
    let mut motion = DetachedMotionState::new(template, start);

    assert!(matches!(
        motion.resolve(start, true, false),
        Err(UiError::DuplicateAnimationId(_))
    ));
}

#[test]
fn detached_springs_keep_velocity_and_snap_under_reduce_motion() {
    let start = Instant::now();
    let template = div().with_spring(
        "detached-spring",
        SpringAnimation::new(SpringConfig::new(170.0, 14.0, 1.0))
            .to(100.0)
            .from(0.0),
        |element, value| element.w(value),
    );
    let mut motion = DetachedMotionState::new(template, start);

    let initial = motion.resolve(start, true, false).unwrap();
    assert_eq!(absolute_length(initial.layout.size.width), Some(0.0));
    assert!(motion.frame_requested);
    assert_eq!(motion.active_count(), 1);

    let advanced = motion
        .resolve(start + Duration::from_millis(50), true, false)
        .unwrap();
    let width = absolute_length(advanced.layout.size.width).unwrap();
    assert!(width > 0.0 && width < 100.0);
    let velocity = motion
        .springs
        .values()
        .next()
        .expect("detached spring playback")
        .state
        .velocity;
    assert!(velocity > 0.0);

    motion.mark_needs_resolve();
    motion.pause(start + Duration::from_millis(50));
    assert!(motion.needs_resolve);
    let reduced = motion
        .resolve(start + Duration::from_millis(50), false, true)
        .unwrap();
    assert_eq!(absolute_length(reduced.layout.size.width), Some(100.0));
    assert!(!motion.frame_requested);
    assert_eq!(motion.active_count(), 0);
}

#[test]
fn box_shadows_keep_css_declaration_order_around_the_element_quad() {
    let first_drop = Color::rgb8(220, 38, 38);
    let first_inset = Color::rgb8(22, 163, 74);
    let second_drop = Color::rgb8(37, 99, 235);
    let second_inset = Color::rgb8(147, 51, 234);
    let shadows = [
        BoxShadow::new(0.0, 2.0, first_drop),
        BoxShadow::new(0.0, 1.0, first_inset).inset(true),
        BoxShadow::new(0.0, 4.0, second_drop),
        BoxShadow::new(0.0, 2.0, second_inset).inset(true),
    ];
    let bounds = Rect::new(10.0, 10.0, 80.0, 40.0);
    let clip = Rect::new(0.0, 0.0, 100.0, 100.0);
    let mut scene = Scene::new();
    push_element_shadows(
        &mut scene,
        PaintLayerKey::default(),
        bounds,
        crate::Corners::all(8.0),
        clip,
        &shadows,
        false,
    );
    scene.push_quad_in(PaintLayerKey::default(), Quad::new(bounds, Color::WHITE));
    push_element_shadows(
        &mut scene,
        PaintLayerKey::default(),
        bounds,
        crate::Corners::all(8.0),
        clip,
        &shadows,
        true,
    );

    let layer = &scene.paint_layers()[0];
    assert_eq!(
        layer.shapes(),
        &[
            crate::scene::ShapeRef::Shadow(0),
            crate::scene::ShapeRef::Shadow(1),
            crate::scene::ShapeRef::Quad(0),
            crate::scene::ShapeRef::Shadow(2),
            crate::scene::ShapeRef::Shadow(3),
        ]
    );
    let colors: Vec<_> = layer
        .shadows()
        .iter()
        .map(|shadow| shadow.style.color())
        .collect();
    assert_eq!(
        colors,
        vec![second_drop, first_drop, second_inset, first_inset]
    );
}

#[test]
fn images_preserve_intrinsic_ratio_when_one_axis_is_known() {
    assert_eq!(
        measure_image(Some(200.0), None, Size::new(400.0, 100.0)),
        TaffySize {
            width: 200.0,
            height: 50.0,
        }
    );
    assert_eq!(
        measure_image(None, Some(75.0), Size::new(400.0, 100.0)),
        TaffySize {
            width: 300.0,
            height: 75.0,
        }
    );
}

#[test]
fn path_fitting_accounts_for_nonzero_source_bounds_and_cover_cropping() {
    let mut builder = PathBuilder::fill();
    builder.move_to(Point::new(10.0, 20.0));
    builder.line_to(Point::new(110.0, 20.0));
    builder.line_to(Point::new(110.0, 70.0));
    builder.line_to(Point::new(10.0, 70.0));
    builder.close();
    let path = builder.build().unwrap();
    let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);

    let (contain_scale, contain_translation) = fit_path(bounds, &path, ObjectFit::Contain).unwrap();
    assert_eq!(contain_scale, [1.0, 1.0]);
    assert_eq!(contain_translation, Vector::new(-10.0, 5.0));

    let (cover_scale, cover_translation) = fit_path(bounds, &path, ObjectFit::Cover).unwrap();
    assert_eq!(cover_scale, [2.0, 2.0]);
    assert_eq!(cover_translation, Vector::new(-70.0, -40.0));
}

#[test]
fn nested_opacity_multiplies_subtrees_and_restores_for_following_siblings() {
    let root = div().size(40.0, 20.0).flex_row().children([
        div()
            .size(20.0, 20.0)
            .opacity(0.5)
            .child(div().size_full().opacity(0.5).bg(Color::WHITE)),
        div().size(20.0, 20.0).bg(Color::WHITE),
    ]);
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(40.0, 20.0), 1.0, &mut renderer)
        .unwrap();

    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    assert_eq!(scene.edge_quads().len(), 2);
    assert_eq!(scene.edge_quads()[0].fill.a, 0.25);
    assert_eq!(scene.edge_quads()[1].fill.a, 1.0);
    assert_eq!(scene.current_opacity(), 1.0);
}

#[test]
fn zero_opacity_keeps_pointer_and_accessibility_semantics() {
    let control = ElementId::named("transparent-control");
    let root = button()
        .id(control)
        .size(80.0, 24.0)
        .opacity(0.0)
        .clickable()
        .child("Still interactive");
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(100.0, 40.0), 1.0, &mut renderer)
        .unwrap();

    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    assert!(scene.text_runs().is_empty());
    assert!(tree.hit_regions.iter().any(|region| region.id == control));
    assert!(
        tree.accessibility_update("Opacity test")
            .nodes
            .iter()
            .any(|(id, _)| *id == accessibility_id(control))
    );
}

#[test]
fn retained_scroll_accessibility_updates_only_the_container() {
    let scroll_id = ElementId::named("accessibility-scroll");
    let child_id = ElementId::named("accessibility-scroll-child");
    let root = div()
        .id(scroll_id)
        .size(100.0, 100.0)
        .overflow_y_scroll()
        .child(
            div()
                .h(300.0)
                .flex_none()
                .child(button().id(child_id).size(80.0, 24.0).child("Open")),
        );
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(100.0, 100.0), 1.0, &mut renderer)
        .unwrap();

    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let initial_child_bounds = tree
        .accessibility_update("Scroll test")
        .nodes
        .into_iter()
        .find_map(|(id, node)| (id == accessibility_id(child_id)).then(|| node.bounds()))
        .flatten()
        .expect("accessible child bounds");

    tree.scroll_offsets
        .insert(scroll_id, Vector::new(0.0, 80.0));
    scene.clear(Color::TRANSPARENT);
    tree.paint(&mut scene, &mut renderer).unwrap();

    let update = tree.accessibility_scroll_update();
    assert_eq!(update.nodes.len(), 1);
    let (node_id, node) = &update.nodes[0];
    assert_eq!(*node_id, accessibility_id(scroll_id));
    assert_eq!(node.bounds().expect("scroll bounds").y0, 80.0);
    assert_eq!(
        node.transform(),
        Some(&Affine::translate(AccessibilityVector::new(0.0, -80.0)))
    );
    assert!(node.clips_children());

    let scrolled_child_bounds = tree
        .accessibility_update("Scroll test")
        .nodes
        .into_iter()
        .find_map(|(id, node)| (id == accessibility_id(child_id)).then(|| node.bounds()))
        .flatten()
        .expect("accessible child bounds after scrolling");
    assert_eq!(scrolled_child_bounds, initial_child_bounds);
}

#[test]
fn retained_scroll_accessibility_includes_newly_visible_nested_children() {
    let outer = ElementId::named("outer-accessibility-scroll");
    let inner = ElementId::named("inner-accessibility-scroll");
    let control = ElementId::named("nested-accessibility-button");
    let root = div()
        .id(outer)
        .size(100.0, 100.0)
        .flex_col()
        .overflow_y_scroll()
        .child(div().h(240.0).flex_none())
        .child(
            div()
                .id(inner)
                .size(100.0, 100.0)
                .flex_none()
                .overflow_y_scroll()
                .child(
                    div()
                        .h(300.0)
                        .flex_none()
                        .child(button().id(control).size(80.0, 24.0).child("Nested button")),
                ),
        );
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(100.0, 100.0), 1.0, &mut renderer)
        .unwrap();
    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let mut published: HashMap<_, _> = tree
        .accessibility_update("Nested scrolling")
        .nodes
        .into_iter()
        .collect();
    assert!(!published.contains_key(&accessibility_id(control)));

    for offset in [240.0, 260.0, 0.0, 240.0] {
        tree.scroll_offsets.insert(outer, Vector::new(0.0, offset));
        scene.clear(Color::TRANSPARENT);
        tree.paint(&mut scene, &mut renderer).unwrap();
        let update = tree.accessibility_scroll_update();
        published.extend(update.nodes);

        // Match an accessibility client's retention: referenced children must exist,
        // and removing a parent reference prunes that entire subtree.
        let mut reachable = HashSet::new();
        let mut pending = vec![ACCESSIBILITY_ROOT_ID];
        while let Some(id) = pending.pop() {
            assert!(reachable.insert(id), "duplicate accessible child {id:?}");
            let node = published.get(&id).unwrap_or_else(|| {
                panic!("scroll offset {offset} references unpublished child {id:?}")
            });
            pending.extend(node.children().iter().copied());
        }
        published.retain(|id, _| reachable.contains(id));
        assert!(published.contains_key(&update.focus));
        assert_eq!(
            published.contains_key(&accessibility_id(control)),
            offset > 0.0,
        );
        if let Some(label) = published[&ACCESSIBILITY_ROOT_ID].label() {
            assert_eq!(label, "Nested scrolling");
        }
    }
}

#[test]
fn clipped_offscreen_text_keeps_bounds_without_emitting_a_text_run() {
    let text_id = ElementId::named("offscreen-text");
    let root = div().relative().size(100.0, 100.0).overflow_hidden().child(
        text("outside")
            .id(text_id)
            .absolute()
            .top(160.0)
            .left(0.0)
            .size(100.0, 20.0),
    );
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(100.0, 100.0), 1.0, &mut renderer)
        .unwrap();

    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();

    assert!(scene.text_runs().is_empty());
    assert_eq!(
        tree.element_bounds(text_id),
        Some(Rect::new(0.0, 160.0, 100.0, 20.0))
    );
    assert!(
        tree.accessibility_update("Cull test")
            .nodes
            .iter()
            .any(|(id, _)| *id == accessibility_id(text_id))
    );
}

#[test]
fn padded_text_measures_and_paints_in_one_content_box() {
    let plain_id = ElementId::named("padded-plain-text");
    let styled_id = ElementId::named("padded-styled-text");
    let root = div().size(240.0, 50.0).flex_row().items_start().children([
        text("plain")
            .id(plain_id)
            .w(100.0)
            .px(10.0)
            .py(5.0)
            .line_height(20.0)
            .border(2.0, Color::WHITE),
        crate::styled_text("styled")
            .into_element()
            .id(styled_id)
            .w(100.0)
            .px(4.0)
            .py(3.0)
            .line_height(20.0)
            .border(1.0, Color::WHITE),
    ]);
    let mut tree = UiTree::new();
    RECORDED_TEXT_WIDTHS.with(|recording| *recording.borrow_mut() = Some(Vec::new()));
    let mut renderer = TestTextLayout;
    tree.set_root(root, Size::new(240.0, 50.0), 1.0, &mut renderer)
        .unwrap();
    let measured_widths = RECORDED_TEXT_WIDTHS.with(|recording| {
        recording
            .borrow_mut()
            .take()
            .expect("active width recording")
    });

    assert!(measured_widths.contains(&Some(76.0)));
    assert!(measured_widths.contains(&Some(90.0)));
    assert!(
        !measured_widths.contains(&Some(100.0)),
        "assigned border-box widths must never reach text shaping: {:?}",
        measured_widths
    );

    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    let plain = scene
        .text_runs()
        .iter()
        .find(|run| run.id == TextId::new(plain_id.value()))
        .expect("plain text run");
    let styled = scene
        .text_runs()
        .iter()
        .find(|run| run.id == TextId::new(styled_id.value()))
        .expect("styled text run");

    assert_eq!(
        tree.element_bounds(plain_id),
        Some(Rect::new(0.0, 0.0, 100.0, 34.0))
    );
    assert_eq!(plain.bounds, Rect::new(12.0, 7.0, 76.0, 20.0));
    assert_eq!(
        tree.element_bounds(styled_id),
        Some(Rect::new(100.0, 0.0, 100.0, 28.0))
    );
    assert_eq!(styled.bounds, Rect::new(105.0, 4.0, 90.0, 20.0));
}

#[test]
fn animation_playback_uses_exact_deadlines_and_pauses_without_catching_up() {
    let started = Instant::now();
    let mut playback =
        AnimationPlayback::new(playback_animation(AnimationRepeat::Infinite), started);
    playback.activate(started, true);
    assert_eq!(
        playback.deadline(),
        Some(started + Duration::from_millis(40))
    );

    assert!(!playback.advance(started + Duration::from_millis(39)));
    assert_eq!(playback.frame_index, 0);
    assert!(playback.advance(started + Duration::from_millis(40)));
    assert_eq!(playback.frame_index, 1);
    assert_eq!(
        playback.deadline(),
        Some(started + Duration::from_millis(100))
    );

    playback.active = false;
    playback.activate(started + Duration::from_secs(10), true);
    assert_eq!(
        playback.deadline(),
        Some(started + Duration::from_secs(10) + Duration::from_millis(60))
    );

    playback.active = false;
    playback.activate(started + Duration::from_secs(20), false);
    assert!(!playback.active);
    assert_eq!(playback.deadline(), None);

    playback.activate(started + Duration::from_secs(30), true);
    playback.seen = false;
    playback.finish_visibility();
    assert!(!playback.active);
    assert_eq!(playback.deadline(), None);
}

#[test]
fn finite_animation_playback_stops_scheduling_on_the_last_frame() {
    let started = Instant::now();
    let repeat = AnimationRepeat::Finite(1);
    let mut playback = AnimationPlayback::new(playback_animation(repeat), started);
    playback.activate(started, true);
    assert!(playback.advance(started + Duration::from_millis(100)));
    assert_eq!(playback.frame_index, 1);
    assert!(playback.completed);
    assert_eq!(playback.deadline(), None);
}

#[test]
fn duplicate_explicit_ids_are_rejected_before_layout() {
    let mut seen = HashSet::new();
    let root = div()
        .child(text("one").id("same"))
        .child(text("two").id("same"));
    let error = collect_explicit_ids(&root, &mut seen).unwrap_err();
    assert!(matches!(error, UiError::DuplicateId(_)));
}

#[test]
fn accessibility_root_id_is_reserved() {
    let mut seen = HashSet::from([ElementId::new(ACCESSIBILITY_ROOT_ID.0)]);
    let root = div().id(ElementId::new(ACCESSIBILITY_ROOT_ID.0));
    let error = collect_explicit_ids(&root, &mut seen).unwrap_err();
    assert!(matches!(error, UiError::ReservedId(_)));
}

#[test]
fn display_none_deactivates_the_complete_mounted_subtree() {
    let animated = playback_animation(AnimationRepeat::Infinite);
    let declaration = |hidden: bool| {
        let subtree = div()
            .id("hidden-subtree")
            .w(120.0)
            .children([
                button()
                    .id("hidden-button")
                    .clickable()
                    .tooltip("Hidden tooltip")
                    .auto_focus(),
                text_input("").id("hidden-input"),
                text("hidden selectable")
                    .id("hidden-text")
                    .user_select_text(),
                crate::img(animated.clone()).id("hidden-image"),
                div().id("hidden-motion").with_animation(
                    "hidden-motion-clock",
                    Animation::new(Duration::from_millis(100)).repeat(),
                    |element, value| element.w(value * 10.0),
                ),
                div()
                    .id("hidden-transition")
                    .transition(Transition::colors(Duration::from_millis(100))),
            ])
            .when(hidden, Element::hidden);
        div()
            .size(300.0, 100.0)
            .flex_row()
            .children([subtree, div().id("visible-sibling").size(24.0, 24.0)])
    };

    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    tree.set_root(
        declaration(false),
        Size::new(300.0, 100.0),
        1.0,
        &mut renderer,
    )
    .unwrap();

    let button = ElementId::named("hidden-button");
    let subtree = ElementId::named("hidden-subtree");
    assert_eq!(tree.focused(), Some(button));
    assert!(tree.clickable_ids.contains(&button));
    assert!(tree.text_inputs.contains_key(&"hidden-input".into()));
    assert!(
        tree.selectable_text_indices
            .contains_key(&"hidden-text".into())
    );
    assert!(tree.animations.contains_key(&"hidden-image".into()));
    assert!(
        tree.declarative_animation_ids
            .contains(&"hidden-motion-clock".into())
    );
    assert!(
        tree.style_transition_ids
            .contains(&"hidden-transition".into())
    );
    assert!(tree.tooltips.contains_key(&button));

    tree.hovered.insert(button);
    tree.pressed = Some(button);
    tree.dragging = Some(button);
    tree.drag_over = Some(button);
    tree.scroll_offsets.insert(subtree, Vector::new(0.0, 12.0));
    tree.scrollbar_states
        .insert(subtree, ScrollbarState::default());
    tree.hovered_scrollbar = Some(subtree);
    tree.scrollbar_drag = Some(ScrollbarDrag {
        id: subtree,
        axis: ScrollbarAxis::Vertical,
        pointer_origin: 0.0,
        scroll_origin: 0.0,
    });

    tree.set_root(
        declaration(true),
        Size::new(300.0, 100.0),
        1.0,
        &mut renderer,
    )
    .unwrap();

    // The declarations and their stable IDs still exist, but no hidden descendant remains
    // mounted in an interactive, semantic, resource, or animation registry.
    assert!(tree.contains_element(button));
    assert!(!tree.displayed_ids.contains(&subtree));
    assert!(!tree.displayed_ids.contains(&button));
    assert!(!tree.focusable_ids.contains(&button));
    assert!(!tree.clickable_ids.contains(&button));
    assert_eq!(tree.focused(), None);
    assert!(!tree.text_inputs.contains_key(&"hidden-input".into()));
    assert!(
        !tree
            .selectable_text_indices
            .contains_key(&"hidden-text".into())
    );
    assert!(!tree.animations.contains_key(&"hidden-image".into()));
    assert!(tree.declarative_animation_ids.is_empty());
    assert!(tree.declarative_animations.is_empty());
    assert!(tree.style_transition_ids.is_empty());
    assert!(tree.style_transitions.is_empty());
    assert!(!tree.tooltips.contains_key(&button));
    assert!(!tree.hovered.contains(&button));
    assert_eq!(tree.pressed, None);
    assert_eq!(tree.dragging, None);
    assert_eq!(tree.drag_over, None);
    assert!(!tree.scroll_offsets.contains_key(&subtree));
    assert!(!tree.scrollbar_states.contains_key(&subtree));
    assert_eq!(tree.hovered_scrollbar, None);
    assert!(tree.scrollbar_drag.is_none());

    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    assert_eq!(tree.element_bounds(subtree), None);
    assert_eq!(
        tree.element_bounds("visible-sibling".into()),
        Some(Rect::new(0.0, 0.0, 24.0, 24.0))
    );
    let update = tree.accessibility_update("Display none test");
    assert!(
        !update
            .nodes
            .iter()
            .any(|(id, _)| *id == accessibility_id(subtree))
    );
    assert!(
        !update
            .nodes
            .iter()
            .any(|(id, _)| *id == accessibility_id(button))
    );
    assert_eq!(tree.accessibility_element(accessibility_id(button)), None);
}

#[test]
fn display_none_does_not_resolve_descendant_container_queries() {
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let callback_calls = calls.clone();
    let root = div().hidden().child(container_query(move |_| {
        callback_calls.set(callback_calls.get() + 1);
        div().id("hidden-query-result")
    }));
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;

    tree.set_root(root, Size::new(320.0, 200.0), 1.0, &mut renderer)
        .unwrap();

    assert_eq!(calls.get(), 0);
    assert!(!tree.contains_element("hidden-query-result".into()));
    assert!(!tree.declarative_animation_frame_requested());
}

#[test]
fn visibility_hidden_retains_layout_and_controlled_state_but_not_runtime_activity() {
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let callback_calls = calls.clone();
    let invisible = div()
        .id("invisible-subtree")
        .w(120.0)
        .invisible()
        .children([
            button()
                .id("invisible-button")
                .clickable()
                .auto_focus()
                .tooltip("Invisible tooltip"),
            text_input("controlled value").id("invisible-input"),
            container_query(move |_| {
                callback_calls.set(callback_calls.get() + 1);
                div().id("invisible-query-result")
            }),
            div().id("invisible-motion").with_animation(
                "invisible-motion-clock",
                Animation::new(Duration::from_millis(100)).repeat(),
                |element, value| element.w(value * 10.0),
            ),
        ]);
    let root = div()
        .size(300.0, 100.0)
        .flex_row()
        .children([invisible, div().id("visibility-sibling").size(24.0, 24.0)]);
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;

    tree.set_root(root, Size::new(300.0, 100.0), 1.0, &mut renderer)
        .unwrap();

    let subtree = ElementId::named("invisible-subtree");
    let button = ElementId::named("invisible-button");
    assert_eq!(
        calls.get(),
        1,
        "visibility must not suppress layout callbacks"
    );
    assert!(tree.contains_element("invisible-query-result".into()));
    assert!(tree.displayed_ids.contains(&subtree));
    assert!(!tree.visible_ids.contains(&subtree));
    assert!(!tree.visible_ids.contains(&button));
    assert!(!tree.focusable_ids.contains(&button));
    assert!(!tree.clickable_ids.contains(&button));
    assert_eq!(tree.focused(), None);
    assert!(
        tree.text_inputs.contains_key(&"invisible-input".into()),
        "layout-preserving visibility keeps controlled input state warm"
    );
    assert!(tree.tooltips.is_empty());
    assert!(tree.declarative_animation_ids.is_empty());
    assert!(tree.declarative_animations.is_empty());
    assert!(!tree.declarative_animation_frame_requested());

    let mut scene = Scene::new();
    tree.paint(&mut scene, &mut renderer).unwrap();
    assert_eq!(tree.element_bounds(subtree), None);
    assert_eq!(
        tree.element_bounds("visibility-sibling".into()),
        Some(Rect::new(120.0, 0.0, 24.0, 24.0))
    );
    let update = tree.accessibility_update("Visibility test");
    assert!(
        !update
            .nodes
            .iter()
            .any(|(id, _)| *id == accessibility_id(subtree))
    );
    assert_eq!(tree.accessibility_element(accessibility_id(button)), None);
}
