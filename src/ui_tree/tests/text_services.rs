use super::*;

use std::rc::Rc;

use crate::{
    Misspelling, SpellingMenuLabels, TestSpellCheckProvider, TextCheckingOverrides,
    TextServiceError, element::InputConstraints, spell::SPELL_CHECK_SETTLE_DELAY,
};

fn checking_constraints(overrides: TextCheckingOverrides) -> InputConstraints {
    InputConstraints {
        read_only: false,
        max_length: None,
        filter: None,
        text_checking: overrides,
    }
}

fn checked_tree(id: ElementId, value: &str) -> UiTree {
    let mut tree = UiTree::new();
    tree.focused = Some(id);
    tree.text_inputs.insert(
        id,
        TextInputState::with_constraints(
            value,
            false,
            checking_constraints(TextCheckingOverrides {
                spellcheck: Some(true),
                ..TextCheckingOverrides::default()
            }),
        ),
    );
    tree
}

fn install(provider: TestSpellCheckProvider) -> Rc<TestSpellCheckProvider> {
    let provider = Rc::new(provider);
    crate::set_shared_spell_check_provider(provider.clone());
    provider
}

/// Pump once to timestamp the armed deadline, then once more after the settle delay elapsed.
fn settle(tree: &mut UiTree) -> bool {
    settle_advance(tree).repaint
}

fn settle_advance(tree: &mut UiTree) -> crate::ui_tree::editing::SpellCheckAdvance {
    let now = Instant::now();
    tree.advance_spell_check(now);
    tree.advance_spell_check(now + SPELL_CHECK_SETTLE_DELAY)
}

#[test]
fn settled_checks_pump_on_one_deadline_and_leave_a_clean_tree_asleep() {
    let _provider = install(TestSpellCheckProvider::new().misspelling("helo"));
    let id = ElementId::new(701);
    let mut tree = checked_tree(id, "");

    let idle = tree.advance_spell_check(Instant::now());
    assert!(!idle.repaint);
    assert!(idle.next_deadline.is_none());

    let edit = tree.input_replace("helo world");
    assert!(edit.repaint);
    let armed = tree
        .advance_spell_check(Instant::now())
        .next_deadline
        .expect("an accepted edit arms exactly one deadline");

    assert!(tree.input_move_home(false).repaint);
    let due = tree.advance_spell_check(armed + SPELL_CHECK_SETTLE_DELAY);
    assert!(due.repaint);
    assert!(due.next_deadline.is_none());
    assert_eq!(
        tree.focused_input_misspellings()
            .iter()
            .map(Misspelling::range)
            .collect::<Vec<_>>(),
        vec![0..4]
    );
    crate::clear_spell_check_provider();
}

#[test]
fn spelling_actions_replace_learn_and_ignore_through_the_focused_input() {
    let provider = install(
        TestSpellCheckProvider::new()
            .misspelling("helo")
            .guess("helo", &["hello"]),
    );
    let id = ElementId::new(702);
    let mut tree = checked_tree(id, "");
    assert!(tree.input_replace("helo there").repaint);
    assert!(settle(&mut tree));
    assert_eq!(tree.focused_input_misspellings().len(), 1);

    let items = tree.input_spelling_menu_items(1, SpellingMenuLabels::default());
    assert_eq!(items.len(), 4);
    assert_eq!(items[0].label().as_ref(), "hello");

    let replaced = tree.input_replace_word(0..4, "hello");
    assert!(replaced.repaint);
    assert_eq!(
        replaced.change.as_ref().map(|change| change.value.as_ref()),
        Some("hello there")
    );

    assert!(!tree.input_learn_word("").repaint);
    tree.input_learn_word("hello");
    assert_eq!(provider.learned().len(), 1);
    tree.input_ignore_word("hello");
    assert_eq!(provider.ignored().len(), 1);
    crate::clear_spell_check_provider();
}

#[test]
fn autocorrections_are_applied_and_reversible_through_the_tree() {
    let _provider = install(TestSpellCheckProvider::new().correction_for("teh", "the"));
    let id = ElementId::new(703);
    let mut tree = UiTree::new();
    tree.focused = Some(id);
    tree.text_inputs.insert(
        id,
        TextInputState::with_constraints(
            "teh",
            false,
            checking_constraints(TextCheckingOverrides {
                autocorrect: Some(true),
                ..TextCheckingOverrides::default()
            }),
        ),
    );

    let typed = tree.input_replace(" ");
    assert_eq!(
        typed.change.as_ref().map(|change| change.value.as_ref()),
        Some("the ")
    );
    let reverted = tree.input_revert_autocorrection();
    assert_eq!(
        reverted.change.as_ref().map(|change| change.value.as_ref()),
        Some("teh ")
    );
    assert!(!tree.input_revert_autocorrection().repaint);
    crate::clear_spell_check_provider();
}

#[test]
fn definition_requests_use_the_caret_word_and_stay_unsupported_without_a_platform() {
    let _provider = install(TestSpellCheckProvider::new());
    let id = ElementId::new(704);
    let mut tree = UiTree::new();
    tree.focused = Some(id);
    tree.text_inputs.insert(
        id,
        TextInputState::with_constraints(
            "alpha beta",
            false,
            checking_constraints(TextCheckingOverrides::default()),
        ),
    );

    let (value, anchor) = tree
        .input_definition_request()
        .expect("the caret word is defined");
    assert_eq!(value.as_ref(), "beta");
    assert_eq!(anchor, Point::default());
    assert_eq!(
        tree.input_look_up_selection(),
        Err(TextServiceError::Unsupported)
    );
    // Force click is opt-in per input.
    assert_eq!(
        tree.input_force_click_definition(),
        Err(TextServiceError::InvalidRequest)
    );

    tree.text_inputs.insert(
        id,
        TextInputState::with_constraints(
            "alpha",
            false,
            checking_constraints(TextCheckingOverrides {
                lookup_on_force_click: Some(true),
                ..TextCheckingOverrides::default()
            }),
        ),
    );
    assert_eq!(
        tree.input_force_click_definition(),
        Err(TextServiceError::Unsupported)
    );
    crate::clear_spell_check_provider();
}

#[test]
fn undo_routing_prefers_a_focused_text_input() {
    let id = ElementId::new(705);
    let mut tree = UiTree::new();
    tree.text_inputs.insert(
        id,
        TextInputState::with_constraints(
            "value",
            false,
            checking_constraints(TextCheckingOverrides::default()),
        ),
    );
    assert!(!tree.text_input_claims_undo());

    tree.focused = Some(id);
    assert!(tree.text_input_claims_undo());
    assert!(!tree.input_can_undo());
    assert!(tree.input_replace("!").repaint);
    assert!(tree.input_can_undo());
    assert!(tree.input_undo().repaint);

    let mut manager = crate::UndoManager::<usize>::new();
    let mut applied = 0_usize;
    manager.register(crate::UndoEntry::new(
        "Reorder",
        |count: &mut usize| *count += 1,
        |count: &mut usize| *count += 1,
    ));
    assert!(!manager.undo_unless_handled(tree.text_input_claims_undo(), &mut applied));
    tree.focused = None;
    assert!(manager.undo_unless_handled(tree.text_input_claims_undo(), &mut applied));
    assert_eq!(applied, 1);
}

#[test]
fn the_focused_caret_blinks_on_one_deadline_per_toggle_and_restarts_on_edits() {
    use crate::CARET_BLINK_HALF_PERIOD;
    use web_time::Duration;

    let id = ElementId::new(702);
    let mut tree = UiTree::new();
    let mut renderer = TestTextLayout;
    let mut scene = Scene::new();
    tree.set_root(
        div()
            .size(300.0, 100.0)
            .child(text_input("hello").id(id).w(200.0)),
        Size::new(300.0, 100.0),
        1.0,
        &mut renderer,
    )
    .unwrap();
    let t0 = Instant::now();

    // Nothing is focused, so nothing is armed: an idle window schedules no wake-up at all.
    tree.paint_at(&mut scene, &mut renderer, t0).unwrap();
    assert_eq!(tree.next_caret_blink_deadline(t0), None);
    assert!(!tree.advance_caret_blink(t0 + Duration::from_secs(5)));

    // Focus starts the solid phase; the first toggle is exactly one half period away.
    assert!(tree.focus(id));
    tree.paint_at(&mut scene, &mut renderer, t0).unwrap();
    assert_eq!(
        tree.next_caret_blink_deadline(t0),
        Some(t0 + CARET_BLINK_HALF_PERIOD)
    );
    assert!(!tree.advance_caret_blink(t0 + Duration::from_millis(100)));
    let toggled = t0 + CARET_BLINK_HALF_PERIOD + Duration::from_millis(70);
    assert!(tree.advance_caret_blink(toggled));

    // The paint that follows records the hidden phase, so the same toggle is not due twice, and
    // the next deadline is the next toggle rather than the next frame.
    tree.paint_at(&mut scene, &mut renderer, toggled).unwrap();
    assert!(!tree.advance_caret_blink(toggled));
    assert_eq!(
        tree.next_caret_blink_deadline(toggled),
        Some(t0 + CARET_BLINK_HALF_PERIOD * 2)
    );

    // An edit makes the caret solid again from the moment it is painted.
    assert!(tree.input_replace("hello!").repaint);
    let edited = toggled + Duration::from_millis(50);
    tree.paint_at(&mut scene, &mut renderer, edited).unwrap();
    assert_eq!(
        tree.next_caret_blink_deadline(edited),
        Some(edited + CARET_BLINK_HALF_PERIOD)
    );

    // Blurring disarms the blink entirely.
    assert!(tree.blur());
    tree.paint_at(&mut scene, &mut renderer, edited).unwrap();
    assert_eq!(tree.next_caret_blink_deadline(edited), None);
}
