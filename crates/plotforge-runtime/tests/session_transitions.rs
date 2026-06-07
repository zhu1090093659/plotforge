use std::collections::BTreeMap;

use plotforge_agent::{ScenePlan, ScenePlanRequest, ScenePlanner, ScenePlannerError};
use plotforge_runtime::{RuntimeEngineError, RuntimeSession, interpret_action, summarize_delta};
use plotforge_schema::{
    ActionIntentStatus, Beat, Choice, NarrativeReview, REDACTED_TRACE_SECRET, RuntimeTraceStage,
    RuntimeTraceStageStatus, Scene,
};
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
            beats: vec![Beat {
                id: format!("{scene_key}-beat-001"),
                text: format!("Injected response to {}", request.player_input),
                choices: vec![Choice {
                    id: "continue-council".into(),
                    label: "Continue".into(),
                    action_type: "continue".into(),
                    dramatic_purpose: "Continue after injected planner.".into(),
                    change_scene: false,
                }],
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
            scene,
            review,
            fallback_used: self.fallback_used,
        })
    }
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
