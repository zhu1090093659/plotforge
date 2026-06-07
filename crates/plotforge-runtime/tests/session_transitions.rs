use plotforge_runtime::{RuntimeEngineError, RuntimeSession, interpret_action, summarize_delta};
use plotforge_schema::ActionIntentStatus;
use plotforge_storage::dynasty_embers_project;

#[test]
fn continue_action_does_not_advance_scene_or_turn() {
    let mut project = dynasty_embers_project();
    project.scenes[0].beats[0]
        .choices
        .push(plotforge_schema::Choice {
            id: "continue-council".into(),
            label: "Hear one more minister".into(),
            action_type: "continue".into(),
            dramatic_purpose: "Gather more pressure.".into(),
            change_scene: false,
        });
    let mut session = RuntimeSession::new(project);

    let step = session.play_once("听一位大臣继续陈情").expect("play");

    assert_eq!(step.scene.key, "court-crisis-001");
    assert_eq!(step.trace.story_state_after.turn, 0);
    assert!(step.trace.world_state_delta.is_empty());
    assert_eq!(step.trace.story_state_before, step.trace.story_state_after);
}

#[test]
fn multiple_turns_accumulate_world_state_and_completed_scenes() {
    let mut session = RuntimeSession::new(dynasty_embers_project());

    let first = session.play_once("朕决定加征辽饷").expect("first turn");
    let second = session
        .play_once("先拨内帑稳住边军军饷")
        .expect("second turn");

    assert_eq!(first.trace.story_state_after.turn, 1);
    assert_eq!(second.trace.story_state_after.turn, 2);
    assert!(second.trace.world_state_after.resources["army_morale"] > 45);
    assert_eq!(session.story_state().completed_scene_keys.len(), 2);
}

#[test]
fn missing_current_scene_is_explicit_error() {
    let mut project = dynasty_embers_project();
    project.scenes.clear();
    let mut session = RuntimeSession::new(project);

    let error = session
        .play_once("朕决定加征辽饷")
        .expect_err("missing scene");

    assert!(matches!(error, RuntimeEngineError::MissingScene(_)));
}

#[test]
fn unsupported_input_does_not_commit_state() {
    let mut session = RuntimeSession::new(dynasty_embers_project());
    let story_before = session.story_state().clone();
    let world_before = session.world_state().clone();

    let error = session
        .play_once("朕今日只想题诗赏月")
        .expect_err("unsupported input");

    assert!(matches!(error, RuntimeEngineError::UnsupportedAction(_)));
    assert_eq!(session.story_state(), &story_before);
    assert_eq!(session.world_state(), &world_before);
}

#[test]
fn action_intent_is_explicit_for_known_and_unknown_inputs() {
    let tax = interpret_action("朕决定加征辽饷");
    assert_eq!(tax.status, ActionIntentStatus::Supported);
    assert_eq!(tax.action_type(), Some("raise_tax"));
    assert!(tax.matched_terms.contains(&"加征".into()));

    let unknown = interpret_action("朕今日只想题诗赏月");
    assert_eq!(unknown.status, ActionIntentStatus::Unsupported);
    assert_eq!(unknown.action_type(), None);
    assert!(unknown.reason.is_some());
}

#[test]
fn summarize_delta_keeps_human_readable_lines() {
    let mut session = RuntimeSession::new(dynasty_embers_project());
    let step = session.play_once("朕决定加征辽饷").expect("play");

    let lines = summarize_delta(&step.trace.world_state_delta);

    assert!(lines.contains(&"treasury: +12".into()));
    assert!(lines.contains(&"event: local_tax_resistance".into()));
}
