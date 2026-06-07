use std::fs;

use plotforge_schema::{
    ActionIntent, RuntimePlannerResult, RuntimeRuleResult, RuntimeTrace, RuntimeTraceDiagnostic,
    RuntimeTraceStage, RuntimeTraceStageStatus, StoryState, WorldDelta, WorldState,
};
use plotforge_storage::{
    StorageError, create_demo_project, dynasty_embers_project, load_project, validate_project,
    write_trace,
};

#[test]
fn create_demo_respects_force_flag() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");

    create_demo_project(&project, false).expect("first create");
    let error = create_demo_project(&project, false).expect_err("force=false should fail");
    assert!(matches!(error, StorageError::ProjectExists(_)));

    create_demo_project(&project, true).expect("force=true recreates");
}

#[test]
fn validate_fails_when_entry_scene_is_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");
    fs::remove_file(project.join("scenes/court-crisis-001.scene.json")).expect("remove scene");

    let error = validate_project(&project).expect_err("missing scene should fail");
    assert!(matches!(error, StorageError::MissingFile(_)));
}

#[test]
fn write_trace_writes_trace_id_and_latest() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");
    let story = StoryState {
        current_scene_key: "court-crisis-001".into(),
        completed_scene_keys: Vec::new(),
        turn: 1,
    };
    let trace = RuntimeTrace {
        id: "trace-test".into(),
        timestamp_ms: 1,
        player_input: Some("test".into()),
        selected_choice: Some("raise-tax".into()),
        action_intent: Some(ActionIntent::supported("raise_tax", vec!["test".into()])),
        rule_result: Some(RuntimeRuleResult {
            action_type: "raise_tax".into(),
            delta_empty: true,
            state_committed: true,
            error: None,
        }),
        planner_result: Some(RuntimePlannerResult {
            requested_action_type: "raise_tax".into(),
            scene_key: Some("court-crisis-002".into()),
            fallback_used: false,
            error: None,
        }),
        diagnostics: vec![RuntimeTraceDiagnostic::new_redacted(
            RuntimeTraceStage::CommitState,
            RuntimeTraceStageStatus::Completed,
            "committed",
        )],
        world_state_before: WorldState::default(),
        world_state_delta: WorldDelta::default(),
        world_state_after: WorldState::default(),
        story_state_before: story.clone(),
        story_state_after: story,
        narrative_review: None,
        errors: Vec::new(),
        fallback_used: false,
    };

    let trace_path = write_trace(&project, &trace).expect("write trace");

    assert_eq!(trace_path.file_name().unwrap(), "trace-test.json");
    assert!(project.join("traces/trace-test.json").is_file());
    assert!(project.join("traces/latest.json").is_file());
}

#[test]
fn committed_fixture_matches_generated_demo_semantics() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let committed = load_project(root.join("examples/dynasty-embers")).expect("load fixture");
    let generated = dynasty_embers_project();

    assert_eq!(committed.game.id, generated.game.id);
    assert_eq!(committed.game.entry_scene, generated.game.entry_scene);
    assert_eq!(committed.resources.len(), generated.resources.len());
    assert_eq!(committed.characters.len(), generated.characters.len());
    assert_eq!(committed.rules.len(), generated.rules.len());
    assert_eq!(
        committed.story_craft.plot_threads.len(),
        generated.story_craft.plot_threads.len()
    );
}
