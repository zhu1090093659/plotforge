use std::collections::BTreeMap;

use plotforge_agent::{
    FakeImageProvider, FakeTextModelProvider, ImageProviderAgentPipeline, ProviderAgentPipeline,
    ScenePlan, ScenePlanRequest, ScenePlanner, ScenePlannerError,
};
use plotforge_job::JobClock;
use plotforge_runtime::{RuntimeEngineError, RuntimeSession, interpret_action, summarize_delta};
use plotforge_schema::{
    ActionIntentStatus, AgentRole, Beat, BeatNext, Choice, Effect, NarrativeReview,
    REDACTED_TRACE_SECRET, ReproducibilityMetadata, Rule, RuntimeTraceStage,
    RuntimeTraceStageStatus, Scene,
};
use plotforge_storage::dynasty_embers_project;

#[derive(Clone, Debug)]
struct FakeClock {
    now_ms: u64,
}

impl JobClock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.now_ms
    }
}

#[test]
fn continue_action_does_not_advance_scene_or_turn() {
    let mut session = RuntimeSession::with_scene_planner(dynasty_embers_project(), ErrorPlanner);

    let step = session.play_once("听一位大臣继续陈情").expect("play");

    assert_eq!(step.scene.key, "court-crisis-001");
    assert_eq!(
        step.scene.background_asset,
        "assets/generated/court-crisis-001.png"
    );
    assert_eq!(
        step.trace.story_state_before.current_beat_id.as_deref(),
        Some("court-crisis-001-beat-001")
    );
    assert_eq!(
        step.trace.story_state_after.current_beat_id.as_deref(),
        Some("court-crisis-001-beat-002")
    );
    assert_eq!(step.trace.story_state_after.turn, 0);
    assert!(step.trace.story_state_after.completed_scene_keys.is_empty());
    assert!(step.trace.world_state_delta.is_empty());
    assert_eq!(session.story_state(), &step.trace.story_state_after);
    assert!(step.trace.narrative_review.is_none());
    let planner_result = step.trace.planner_result.as_ref().expect("planner result");
    assert_eq!(planner_result.requested_action_type, "continue");
    assert_eq!(planner_result.scene_key, None);
    assert!(!planner_result.fallback_used);
    assert!(planner_result.error.is_none());
    assert_eq!(step.trace.media_references.len(), 1);
    assert_eq!(
        step.trace.media_references[0].project_path,
        "assets/generated/court-crisis-001.png"
    );
    assert!(step.trace.diagnostics.iter().any(|diagnostic| {
        diagnostic.stage == RuntimeTraceStage::PlanScene
            && diagnostic.status == RuntimeTraceStageStatus::Completed
            && diagnostic.message.contains("planner skipped")
    }));
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
fn snapshot_restore_continues_deterministic_multi_turn_session() {
    let project = dynasty_embers_project();
    let mut uninterrupted =
        RuntimeSession::with_scene_planner(project.clone(), FakePlanner::success());
    uninterrupted.play_once("朕决定加征辽饷").expect("first");
    let expected_second = uninterrupted
        .play_once("先拨内帑稳住边军军饷")
        .expect("second");

    let mut original = RuntimeSession::with_scene_planner(project.clone(), FakePlanner::success());
    original.play_once("朕决定加征辽饷").expect("first");
    let snapshot = original.snapshot("save-001", 42);
    assert_eq!(snapshot.project_id, "dynasty-embers");
    assert_eq!(snapshot.story_state.turn, 1);
    assert!(
        snapshot
            .scenes
            .iter()
            .any(|scene| scene.key == "injected-scene-001")
    );

    let mut restored =
        RuntimeSession::with_scene_planner_from_snapshot(project, snapshot, FakePlanner::success())
            .expect("restore");
    let restored_second = restored
        .play_once("先拨内帑稳住边军军饷")
        .expect("restored second");

    assert_eq!(restored.story_state(), uninterrupted.story_state());
    assert_eq!(restored.world_state(), uninterrupted.world_state());
    assert_eq!(restored_second.scene, expected_second.scene);
    assert_eq!(restored_second.trace.story_state_before.turn, 1);
    assert_eq!(restored_second.trace.story_state_after.turn, 2);
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
fn missing_current_beat_is_explicit_error_without_commit() {
    let mut project = dynasty_embers_project();
    project.story_state.current_beat_id = Some("missing-beat".into());
    let mut session = RuntimeSession::new(project);
    let story_before = session.story_state().clone();
    let world_before = session.world_state().clone();

    let error = session
        .play_once("朕决定加征辽饷")
        .expect_err("missing current beat");

    assert!(matches!(
        error,
        RuntimeEngineError::MissingBeat { beat_id, .. } if beat_id == "missing-beat"
    ));
    assert_eq!(session.story_state(), &story_before);
    assert_eq!(session.world_state(), &world_before);
}

#[test]
fn missing_entry_beat_is_explicit_error_without_first_beat_fallback() {
    let mut project = dynasty_embers_project();
    project.story_state.current_beat_id = None;
    project.scenes[0].entry_beat_id = None;
    let mut session = RuntimeSession::new(project);
    let story_before = session.story_state().clone();
    let world_before = session.world_state().clone();

    let error = session
        .play_once("朕决定加征辽饷")
        .expect_err("missing entry beat");

    assert!(matches!(
        error,
        RuntimeEngineError::MissingEntryBeat(scene_key) if scene_key == "court-crisis-001"
    ));
    assert_eq!(session.story_state(), &story_before);
    assert_eq!(session.world_state(), &world_before);
}

#[test]
fn missing_same_scene_beat_transition_is_explicit_error_without_commit() {
    let mut project = dynasty_embers_project();
    project.scenes[0].beats[0].next = BeatNext::None;
    let mut session = RuntimeSession::with_scene_planner(project, ErrorPlanner);
    let story_before = session.story_state().clone();
    let world_before = session.world_state().clone();

    let error = session
        .play_once("听一位大臣继续陈情")
        .expect_err("missing same-scene transition");

    assert!(matches!(
        error,
        RuntimeEngineError::MissingBeatTransition { beat_id, .. }
            if beat_id == "court-crisis-001-beat-001"
    ));
    assert_eq!(session.story_state(), &story_before);
    assert_eq!(session.world_state(), &world_before);
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
fn ambiguous_input_does_not_commit_state() {
    let mut session = RuntimeSession::new(dynasty_embers_project());
    let story_before = session.story_state().clone();
    let world_before = session.world_state().clone();

    let error = session
        .play_once("朕决定加征辽饷，同时严查贪墨官员。")
        .expect_err("ambiguous input");

    assert!(matches!(
        error,
        RuntimeEngineError::AmbiguousAction { choice_ids, .. }
            if choice_ids == vec!["raise-tax".to_string(), "inspect-corruption".to_string()]
    ));
    assert_eq!(session.story_state(), &story_before);
    assert_eq!(session.world_state(), &world_before);
}

#[test]
fn action_intent_is_explicit_for_known_and_unknown_inputs() {
    let project = dynasty_embers_project();
    let choices = &project.scenes[0].beats[0].choices;

    let tax = interpret_action("朕决定加征辽饷", choices).expect("tax intent");
    assert_eq!(tax.status, ActionIntentStatus::Supported);
    assert_eq!(tax.choice_id.as_deref(), Some("raise-tax"));
    assert_eq!(tax.action_type(), Some("raise_tax"));
    assert!(tax.matched_terms.contains(&"加征".into()));

    let unknown = interpret_action("朕今日只想题诗赏月", choices).expect("unsupported intent");
    assert_eq!(unknown.status, ActionIntentStatus::Unsupported);
    assert_eq!(unknown.action_type(), None);
    assert!(unknown.reason.is_some());
}

#[test]
fn runtime_uses_injected_planner_success_path() {
    let mut session =
        RuntimeSession::with_scene_planner(dynasty_embers_project(), FakePlanner::success());

    let step = session.play_once("朕决定加征辽饷").expect("play");

    assert_eq!(step.scene.key, "injected-scene-001");
    assert_eq!(
        step.trace.narrative_review.expect("review").scene_key,
        "injected-scene-001"
    );
    assert!(!step.trace.fallback_used);
}

#[test]
fn injected_planner_fallback_is_trace_visible() {
    let mut session =
        RuntimeSession::with_scene_planner(dynasty_embers_project(), FakePlanner::fallback());

    let step = session.play_once("朕决定加征辽饷").expect("play");

    assert!(step.trace.fallback_used);
    assert!(
        step.trace
            .errors
            .iter()
            .any(|error| error.code == "fallback_scene")
    );
    let planner_result = step.trace.planner_result.as_ref().expect("planner result");
    assert!(planner_result.fallback_used);
    assert_eq!(
        planner_result.error.as_ref().expect("planner error").code,
        "fallback_scene"
    );
    assert!(step.trace.diagnostics.iter().any(|diagnostic| {
        diagnostic.stage == RuntimeTraceStage::PlanScene
            && diagnostic.status == RuntimeTraceStageStatus::Fallback
    }));
}

#[test]
fn injected_planner_error_does_not_commit_state() {
    let mut session = RuntimeSession::with_scene_planner(dynasty_embers_project(), ErrorPlanner);
    let story_before = session.story_state().clone();
    let world_before = session.world_state().clone();

    let error = session
        .play_once("朕决定加征辽饷")
        .expect_err("planner error");

    assert!(matches!(error, RuntimeEngineError::Planner(_)));
    assert_eq!(session.story_state(), &story_before);
    assert_eq!(session.world_state(), &world_before);
}

#[test]
fn planner_scene_with_missing_entry_beat_is_explicit_error_without_commit() {
    let mut session =
        RuntimeSession::with_scene_planner(dynasty_embers_project(), MissingEntryBeatPlanner);
    let story_before = session.story_state().clone();
    let world_before = session.world_state().clone();

    let error = session
        .play_once("朕决定加征辽饷")
        .expect_err("planner scene missing entry beat");

    assert!(matches!(
        error,
        RuntimeEngineError::MissingBeat { scene_key, beat_id }
            if scene_key == "invalid-entry-scene" && beat_id == "missing-entry-beat"
    ));
    assert_eq!(session.story_state(), &story_before);
    assert_eq!(session.world_state(), &world_before);
}

#[test]
fn failed_rule_after_restore_does_not_corrupt_snapshot_state() {
    let mut project = dynasty_embers_project();
    project.rules.push(Rule {
        id: "invalid-unknown-resource".into(),
        action_type: "raise_tax".into(),
        conditions: Vec::new(),
        effects: vec![Effect::AddResource {
            key: "missing_resource".into(),
            amount: 1,
        }],
    });
    let initial_snapshot = RuntimeSession::new(project.clone()).snapshot("save-001", 42);
    let mut restored =
        RuntimeSession::from_snapshot(project, initial_snapshot.clone()).expect("restore");

    let error = restored
        .play_once("朕决定加征辽饷")
        .expect_err("rule failure");

    assert!(matches!(error, RuntimeEngineError::Rule(_)));
    let after_error_snapshot = restored.snapshot("save-after-error", 43);
    assert_eq!(
        after_error_snapshot.story_state,
        initial_snapshot.story_state
    );
    assert_eq!(
        after_error_snapshot.world_state,
        initial_snapshot.world_state
    );
    assert_eq!(after_error_snapshot.scenes, initial_snapshot.scenes);
}

#[test]
fn provider_pipeline_errors_are_trace_visible() {
    let planner = ProviderAgentPipeline::new(FakeTextModelProvider::timeout(AgentRole::BeatWriter));
    let mut session = RuntimeSession::with_scene_planner(dynasty_embers_project(), planner);

    let step = session.play_once("朕决定加征辽饷").expect("fallback play");

    assert!(step.trace.fallback_used);
    assert!(
        step.trace
            .errors
            .iter()
            .any(|error| error.code == "text_provider_timeout")
    );
    let planner_result = step.trace.planner_result.as_ref().expect("planner result");
    assert!(planner_result.fallback_used);
    assert_eq!(
        planner_result.error.as_ref().expect("planner error").code,
        "text_provider_timeout"
    );
    assert!(step.trace.diagnostics.iter().any(|diagnostic| {
        diagnostic.stage == RuntimeTraceStage::PlanScene
            && diagnostic.status == RuntimeTraceStageStatus::Fallback
            && diagnostic.message.contains("text_provider_timeout")
    }));
}

#[test]
fn image_provider_failures_are_trace_visible() {
    let planner = ImageProviderAgentPipeline::new(
        FakeTextModelProvider::success(),
        FakeImageProvider::timeout(),
        FakeClock { now_ms: 1_000 },
    );
    let mut session = RuntimeSession::with_scene_planner(dynasty_embers_project(), planner);

    let step = session
        .play_once("朕决定加征辽饷")
        .expect("image fallback play");

    assert_eq!(step.scene.key, "provider-scene-001");
    assert!(step.trace.fallback_used);
    assert!(
        step.trace
            .errors
            .iter()
            .any(|error| error.code == "image_provider_timeout")
    );
    let planner_result = step.trace.planner_result.as_ref().expect("planner result");
    assert!(planner_result.fallback_used);
    assert_eq!(
        planner_result.error.as_ref().expect("planner error").code,
        "image_provider_timeout"
    );
    assert!(step.trace.diagnostics.iter().any(|diagnostic| {
        diagnostic.stage == RuntimeTraceStage::PlanScene
            && diagnostic.status == RuntimeTraceStageStatus::Fallback
            && diagnostic.message.contains("image_provider_timeout")
    }));
}

#[test]
fn summarize_delta_keeps_human_readable_lines() {
    let mut session = RuntimeSession::new(dynasty_embers_project());
    let step = session.play_once("朕决定加征辽饷").expect("play");

    let lines = summarize_delta(&step.trace.world_state_delta);

    assert!(lines.contains(&"treasury: +12".into()));
    assert!(lines.contains(&"event: local_tax_resistance".into()));
}

#[test]
fn runtime_trace_records_intent_rule_planner_and_diagnostics() {
    let mut session = RuntimeSession::new(dynasty_embers_project());

    let step = session.play_once("朕决定加征辽饷").expect("play");

    let intent = step.trace.action_intent.as_ref().expect("action intent");
    assert_eq!(intent.status, ActionIntentStatus::Supported);
    assert_eq!(intent.action_type(), Some("raise_tax"));

    let rule_result = step.trace.rule_result.as_ref().expect("rule result");
    assert_eq!(rule_result.action_type, "raise_tax");
    assert!(!rule_result.delta_empty);
    assert!(rule_result.state_committed);
    assert!(rule_result.error.is_none());

    let planner_result = step.trace.planner_result.as_ref().expect("planner result");
    assert_eq!(planner_result.requested_action_type, "raise_tax");
    assert_eq!(
        planner_result.scene_key.as_deref(),
        Some("court-crisis-001")
    );
    assert!(!planner_result.fallback_used);
    assert!(planner_result.error.is_none());

    assert_eq!(step.trace.media_references.len(), 1);
    let media_reference = &step.trace.media_references[0];
    assert_eq!(media_reference.reference.reference_id, "court-crisis-001");
    assert_eq!(media_reference.reference.slot, "background_asset");
    assert_eq!(
        media_reference.project_path,
        "assets/generated/court-crisis-001.png"
    );

    assert!(step.trace.diagnostics.iter().any(|diagnostic| {
        diagnostic.stage == RuntimeTraceStage::InterpretAction
            && diagnostic.status == RuntimeTraceStageStatus::Completed
    }));
    assert!(step.trace.diagnostics.iter().any(|diagnostic| {
        diagnostic.stage == RuntimeTraceStage::EvaluateRules
            && diagnostic.status == RuntimeTraceStageStatus::Completed
    }));
    assert!(step.trace.diagnostics.iter().any(|diagnostic| {
        diagnostic.stage == RuntimeTraceStage::PlanScene
            && diagnostic.status == RuntimeTraceStageStatus::Completed
    }));
}

#[test]
fn runtime_trace_json_redacts_secret_markers() {
    let mut session = RuntimeSession::new(dynasty_embers_project());

    let step = session
        .play_once("朕决定加征辽饷 OPENAI_API_KEY=sk-test-secret-marker bearer token=value")
        .expect("play");
    let json = serde_json::to_string(&step.trace).expect("serialize trace");

    assert!(
        step.trace
            .player_input
            .as_deref()
            .expect("player input")
            .contains(REDACTED_TRACE_SECRET)
    );
    assert!(json.contains(REDACTED_TRACE_SECRET));
    assert!(!json.contains("OPENAI_API_KEY"));
    assert!(!json.contains("sk-test-secret-marker"));
    assert!(!json.contains("token=value"));
}

#[derive(Clone, Debug)]
struct FakePlanner {
    fallback_used: bool,
}

impl FakePlanner {
    fn success() -> Self {
        Self {
            fallback_used: false,
        }
    }

    fn fallback() -> Self {
        Self {
            fallback_used: true,
        }
    }
}

impl ScenePlanner for FakePlanner {
    fn plan_next_scene(
        &self,
        request: ScenePlanRequest<'_>,
    ) -> Result<ScenePlan, ScenePlannerError> {
        let scene_key = format!("injected-scene-{:03}", request.story_state.turn + 1);
        let first_beat_id = format!("{scene_key}-beat-001");
        let second_beat_id = format!("{scene_key}-beat-002");
        let scene = Scene {
            key: scene_key.clone(),
            title: "Injected Planner Scene".into(),
            location: "Test Court".into(),
            dramatic_purpose: "Prove runtime uses the injected planner.".into(),
            hook: "A test planner interrupts the court protocol.".into(),
            background_asset: format!("assets/generated/{scene_key}.png"),
            character_ids: Vec::new(),
            plot_thread_updates: BTreeMap::from([(
                "tax-disorder".into(),
                "Injected planner advanced the thread.".into(),
            )]),
            entry_beat_id: Some(first_beat_id.clone()),
            beats: vec![
                Beat {
                    id: first_beat_id,
                    text: format!("Injected response to {}", request.player_input),
                    choices: vec![
                        Choice {
                            id: "raise-tax".into(),
                            label: "Raise taxes".into(),
                            action_type: "raise_tax".into(),
                            input_terms: choice_input_terms("raise_tax"),
                            dramatic_purpose: "Advance after injected tax pressure.".into(),
                            change_scene: true,
                        },
                        Choice {
                            id: "inspect-corruption".into(),
                            label: "Inspect corruption".into(),
                            action_type: "inspect_corruption".into(),
                            input_terms: choice_input_terms("inspect_corruption"),
                            dramatic_purpose: "Advance after injected corruption pressure.".into(),
                            change_scene: true,
                        },
                        Choice {
                            id: "pay-army".into(),
                            label: "Pay the army".into(),
                            action_type: "pay_army".into(),
                            input_terms: choice_input_terms("pay_army"),
                            dramatic_purpose: "Advance after injected army pressure.".into(),
                            change_scene: true,
                        },
                        Choice {
                            id: "continue-council".into(),
                            label: "Continue".into(),
                            action_type: "continue".into(),
                            input_terms: choice_input_terms("continue"),
                            dramatic_purpose: "Continue after injected planner.".into(),
                            change_scene: false,
                        },
                    ],
                    next: BeatNext::Beat(second_beat_id.clone()),
                },
                Beat {
                    id: second_beat_id,
                    text: "The injected planner leaves the council with a sharper second beat."
                        .into(),
                    choices: vec![
                        Choice {
                            id: "raise-tax".into(),
                            label: "Raise taxes".into(),
                            action_type: "raise_tax".into(),
                            input_terms: choice_input_terms("raise_tax"),
                            dramatic_purpose: "Advance after injected tax pressure.".into(),
                            change_scene: true,
                        },
                        Choice {
                            id: "inspect-corruption".into(),
                            label: "Inspect corruption".into(),
                            action_type: "inspect_corruption".into(),
                            input_terms: choice_input_terms("inspect_corruption"),
                            dramatic_purpose: "Advance after injected corruption pressure.".into(),
                            change_scene: true,
                        },
                        Choice {
                            id: "pay-army".into(),
                            label: "Pay the army".into(),
                            action_type: "pay_army".into(),
                            input_terms: choice_input_terms("pay_army"),
                            dramatic_purpose: "Advance after injected army pressure.".into(),
                            change_scene: true,
                        },
                    ],
                    next: BeatNext::Scene,
                },
            ],
        };
        let review = NarrativeReview {
            scene_key,
            score: 100,
            hook_score: 100,
            pacing_score: 100,
            character_consistency_score: 100,
            payoff_score: 100,
            choice_meaningfulness_score: 100,
            ai_slop_risk: 0,
            issues: Vec::new(),
        };
        Ok(ScenePlan {
            reproducibility: ReproducibilityMetadata::local_mock(request.project.game.run_seed),
            scene,
            review,
            fallback_used: self.fallback_used,
            error: None,
        })
    }
}

fn choice_input_terms(action_type: &str) -> Vec<String> {
    let terms: &[&str] = match action_type {
        "continue" => &["continue", "hear", "minister", "听", "继续", "陈情"],
        "raise_tax" => &["raise", "tax", "levy", "加征", "辽饷"],
        "inspect_corruption" => &["inspect", "corruption", "严查", "贪墨", "查"],
        "pay_army" => &["pay", "army", "军饷", "拨", "内帑", "边军"],
        _ => &[],
    };
    terms.iter().map(|term| (*term).to_string()).collect()
}

#[derive(Clone, Debug)]
struct ErrorPlanner;

impl ScenePlanner for ErrorPlanner {
    fn plan_next_scene(
        &self,
        _request: ScenePlanRequest<'_>,
    ) -> Result<ScenePlan, ScenePlannerError> {
        Err(ScenePlannerError::new(
            "fake_planner",
            "planner failed before scene proposal",
        ))
    }
}

#[derive(Clone, Debug)]
struct MissingEntryBeatPlanner;

impl ScenePlanner for MissingEntryBeatPlanner {
    fn plan_next_scene(
        &self,
        request: ScenePlanRequest<'_>,
    ) -> Result<ScenePlan, ScenePlannerError> {
        let scene_key = "invalid-entry-scene".to_string();
        let scene = Scene {
            key: scene_key.clone(),
            title: "Invalid Entry Scene".into(),
            location: "Test Court".into(),
            dramatic_purpose: "Prove invalid planner beat graphs fail before commit.".into(),
            hook: "The scene points at a missing entry beat.".into(),
            background_asset: "assets/generated/invalid-entry-scene.png".into(),
            character_ids: Vec::new(),
            plot_thread_updates: BTreeMap::new(),
            entry_beat_id: Some("missing-entry-beat".into()),
            beats: vec![Beat {
                id: "valid-but-not-entry".into(),
                text: "This beat exists but is not the declared entry beat.".into(),
                choices: Vec::new(),
                next: BeatNext::End,
            }],
        };
        let review = NarrativeReview {
            scene_key,
            score: 100,
            hook_score: 100,
            pacing_score: 100,
            character_consistency_score: 100,
            payoff_score: 100,
            choice_meaningfulness_score: 100,
            ai_slop_risk: 0,
            issues: Vec::new(),
        };
        Ok(ScenePlan {
            reproducibility: ReproducibilityMetadata::local_mock(request.project.game.run_seed),
            scene,
            review,
            fallback_used: false,
            error: None,
        })
    }
}
