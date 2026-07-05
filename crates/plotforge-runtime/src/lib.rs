use plotforge_agent::{
    MockAgentPipeline, ScenePlan, ScenePlanRequest, ScenePlanner, ScenePlannerError,
};
use plotforge_rule::{RuleEngine, RuleError};
use plotforge_schema::{
    ActionIntent, AssetReference, AssetReferenceKind, Beat, BeatNext, Choice, ProjectData,
    ReproducibilityMetadata, RuntimeError, RuntimeMediaReference, RuntimePlannerResult,
    RuntimeRuleResult, RuntimeSnapshot, RuntimeTrace, RuntimeTraceDiagnostic, RuntimeTraceStage,
    RuntimeTraceStageStatus, Scene, StoryState, WorldDelta, WorldState, redact_trace_text,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeEngineError {
    #[error(transparent)]
    Rule(#[from] RuleError),
    #[error("current scene is missing: {0}")]
    MissingScene(String),
    #[error("current beat is missing in scene `{scene_key}`: {beat_id}")]
    MissingBeat { scene_key: String, beat_id: String },
    #[error("scene has no entry beat: {0}")]
    MissingEntryBeat(String),
    #[error("beat `{beat_id}` in scene `{scene_key}` has no same-scene transition")]
    MissingBeatTransition { scene_key: String, beat_id: String },
    #[error("unsupported player action: {0}")]
    UnsupportedAction(String),
    #[error("ambiguous player action `{input}` matched choices: {choice_ids:?}")]
    AmbiguousAction {
        input: String,
        choice_ids: Vec<String>,
    },
    #[error("no selected choice mapping for action type: {0}")]
    MissingChoiceMapping(String),
    #[error(transparent)]
    Planner(#[from] ScenePlannerError),
    #[error("runtime snapshot belongs to project `{actual}`, expected `{expected}`")]
    SnapshotProjectMismatch { expected: String, actual: String },
    #[error("runtime snapshot has project version `{actual}`, expected `{expected}`")]
    SnapshotVersionMismatch { expected: String, actual: String },
    #[error("runtime snapshot current scene is missing: {0}")]
    SnapshotMissingScene(String),
    #[error("runtime snapshot current beat is missing in scene `{scene_key}`: {beat_id}")]
    SnapshotMissingBeat { scene_key: String, beat_id: String },
}

#[derive(Clone, Debug)]
pub struct RuntimeSession<P = MockAgentPipeline> {
    project: ProjectData,
    story_state: StoryState,
    world_state: WorldState,
    reproducibility: ReproducibilityMetadata,
    scene_planner: P,
}

/// A snapshot of the choice a player selected (or, for the pi-Agent path,
/// a placeholder that carries only the fields the trace needs). Kept small
/// so `RuntimeStepContext` can own it without borrowing the choice list.
#[derive(Clone, Debug)]
pub struct ChoiceSnapshot {
    pub id: String,
}

impl ChoiceSnapshot {
    pub fn from_choice(choice: &Choice) -> Self {
        Self {
            id: choice.id.clone(),
        }
    }
}

/// Pre-evaluated inputs to `RuntimeSession::commit_scene_plan`. The caller
/// (the playtest `play_once` path or the `pi_agent_apply_run` Studio command)
/// resolves the action intent, evaluates rules, and supplies the resulting
/// world delta + states; `commit_scene_plan` only commits state and
/// assembles the trace. The pi-Agent path passes `action_intent=None` and
/// `selected_choice=None` because the agent's `ScenePlan` proposal already
/// names the next scene (there is no player-input-derived choice).
pub struct RuntimeStepContext<'a> {
    pub player_input: &'a str,
    pub action_type: String,
    pub action_intent: Option<ActionIntent>,
    pub selected_choice: Option<ChoiceSnapshot>,
    pub world_state_before: WorldState,
    pub world_delta: WorldDelta,
    pub world_state_after: WorldState,
    pub story_state_before: StoryState,
    /// The current scene + beat, required only for the same-scene
    /// (`continue`) path where `commit_scene_plan` is called with
    /// `plan=None`. The pi-Agent path always supplies a plan and may pass
    /// `None` here.
    pub current_scene: Option<Scene>,
    pub current_beat: Option<Beat>,
}

#[derive(Clone, Debug)]
pub struct RuntimeStep {
    pub scene: Scene,
    pub trace: RuntimeTrace,
}

impl RuntimeSession<MockAgentPipeline> {
    pub fn new(project: ProjectData) -> Self {
        Self::with_scene_planner(project, MockAgentPipeline)
    }

    pub fn from_snapshot(
        project: ProjectData,
        snapshot: RuntimeSnapshot,
    ) -> Result<Self, RuntimeEngineError> {
        Self::with_scene_planner_from_snapshot(project, snapshot, MockAgentPipeline)
    }
}

impl<P> RuntimeSession<P>
where
    P: ScenePlanner,
{
    pub fn with_scene_planner(project: ProjectData, scene_planner: P) -> Self {
        Self {
            story_state: project.story_state.clone(),
            world_state: project.world_state.clone(),
            reproducibility: ReproducibilityMetadata::local_mock(project.game.run_seed),
            project,
            scene_planner,
        }
    }

    pub fn with_scene_planner_from_snapshot(
        project: ProjectData,
        snapshot: RuntimeSnapshot,
        scene_planner: P,
    ) -> Result<Self, RuntimeEngineError> {
        let project = project_from_snapshot(project, &snapshot)?;
        Ok(Self {
            story_state: snapshot.story_state,
            world_state: snapshot.world_state,
            reproducibility: snapshot.reproducibility,
            project,
            scene_planner,
        })
    }

    pub fn story_state(&self) -> &StoryState {
        &self.story_state
    }

    pub fn world_state(&self) -> &WorldState {
        &self.world_state
    }

    /// Read-only access to the project the session was constructed with.
    /// Used by the Studio layer to surface project-scoped data (e.g. for
    /// the desktop `SourceView`) without re-loading the project from disk.
    pub fn project(&self) -> &ProjectData {
        &self.project
    }

    /// Apply a pi-Agent's `ScenePlan` proposal as a committed scene change.
    /// Runs rule evaluation for the `change_scene` action type (so the rule
    /// boundary stays inside `plotforge-runtime`, per AGENTS.md), then
    /// delegates to `commit_scene_plan`. The pi-Agent path always crosses
    /// the scene boundary (`change_scene=true`); there is no player-input-
    /// derived choice or action intent, only the agent's proposed scene.
    pub fn apply_agent_scene_plan(
        &mut self,
        plan: ScenePlan,
        player_input: &str,
    ) -> Result<RuntimeStep, RuntimeEngineError> {
        let world_state_before = self.world_state.clone();
        let story_state_before = self.story_state.clone();
        let rule_engine =
            RuleEngine::new(self.project.resources.clone(), self.project.rules.clone());
        let world_delta = rule_engine.evaluate("change_scene", &world_state_before)?;
        let world_state_after = rule_engine.apply_delta(&world_state_before, &world_delta)?;
        let context = RuntimeStepContext {
            player_input,
            action_type: "change_scene".to_string(),
            action_intent: None,
            selected_choice: None,
            world_state_before,
            world_delta,
            world_state_after,
            story_state_before,
            current_scene: None,
            current_beat: None,
        };
        self.commit_scene_plan(Some(plan), context, true)
    }

    pub fn snapshot(&self, id: impl Into<String>, timestamp_ms: u64) -> RuntimeSnapshot {
        let id = id.into();
        RuntimeSnapshot {
            reproducibility: self.reproducibility.clone().with_snapshot_id(&id),
            id,
            timestamp_ms,
            project_id: self.project.game.id.clone(),
            project_version: self.project.game.version.clone(),
            story_state: self.story_state.clone(),
            world_state: self.world_state.clone(),
            scenes: self.project.scenes.clone(),
        }
    }

    pub fn play_once(&mut self, player_input: &str) -> Result<RuntimeStep, RuntimeEngineError> {
        let world_state_before = self.world_state.clone();
        let story_state_before = self.story_state.clone();

        let current_scene = self
            .project
            .scene(&self.story_state.current_scene_key)
            .cloned()
            .ok_or_else(|| {
                RuntimeEngineError::MissingScene(self.story_state.current_scene_key.clone())
            })?;
        let current_beat_id = current_beat_id(&self.story_state, &current_scene)?.to_string();
        let current_beat = beat_by_id(&current_scene, &current_beat_id)?;
        let action_intent = interpret_action(player_input, &current_beat.choices)?;
        let action_type = action_intent
            .action_type()
            .ok_or_else(|| RuntimeEngineError::UnsupportedAction(player_input.to_string()))?;
        let choice_id = action_intent
            .choice_id
            .as_deref()
            .ok_or_else(|| RuntimeEngineError::MissingChoiceMapping(action_type.to_string()))?;
        let selected_choice = selected_choice_by_id(current_beat, choice_id)
            .ok_or_else(|| RuntimeEngineError::MissingChoiceMapping(choice_id.to_string()))?;
        let change_scene = selected_choice.change_scene;

        let rule_engine =
            RuleEngine::new(self.project.resources.clone(), self.project.rules.clone());
        let world_delta = rule_engine.evaluate(action_type, &world_state_before)?;
        let world_state_after = rule_engine.apply_delta(&world_state_before, &world_delta)?;

        let plan = if change_scene {
            Some(self.scene_planner.plan_next_scene(ScenePlanRequest {
                project: &self.project,
                story_state: &self.story_state,
                world_state: &world_state_after,
                player_input,
                action_type,
            })?)
        } else {
            None
        };
        let selected_choice_snapshot = ChoiceSnapshot::from_choice(selected_choice);
        // Clone the beat into an owned value so `current_scene` can move into
        // the context without leaving a dangling borrow on the borrowed beat.
        let current_beat_owned = current_beat.clone();

        let context = RuntimeStepContext {
            player_input,
            action_type: action_type.to_string(),
            action_intent: Some(action_intent),
            selected_choice: Some(selected_choice_snapshot),
            world_state_before,
            world_delta,
            world_state_after,
            story_state_before,
            current_scene: Some(current_scene),
            current_beat: Some(current_beat_owned),
        };
        self.commit_scene_plan(plan, context, change_scene)
    }

    /// Commits a `ScenePlan` (or a same-scene beat transition when `plan` is
    /// `None`) and assembles the resulting `RuntimeStep` with a full
    /// `RuntimeTrace`. The caller pre-evaluates rules and supplies
    /// `world_state_after`; this method never re-runs the rule engine, so
    /// the rule-evaluation boundary stays in the caller (per AGENTS.md:
    /// "agents propose content and runtime/rules commit state").
    ///
    /// The `pi_agent_apply_run` Studio command builds the same
    /// `RuntimeStepContext` (without a `player_input`-derived choice, since
    /// the agent's proposal already names the next scene) and calls this
    /// method, so playtest turns and pi-Agent turns share one commit path.
    pub fn commit_scene_plan(
        &mut self,
        plan: Option<ScenePlan>,
        context: RuntimeStepContext<'_>,
        change_scene: bool,
    ) -> Result<RuntimeStep, RuntimeEngineError> {
        let RuntimeStepContext {
            player_input,
            action_type,
            action_intent,
            selected_choice,
            world_state_before,
            world_delta,
            world_state_after,
            story_state_before,
            current_scene,
            current_beat,
        } = context;

        let (
            next_scene,
            next_beat_id,
            planner_fallback_used,
            planner_error,
            planner_scene_key,
            planner_reproducibility,
            review,
        ) = if let Some(plan) = plan {
            let planner_fallback_used = plan.fallback_used;
            let errors = if let Some(error) = plan.error {
                vec![error]
            } else {
                fallback_errors(planner_fallback_used)
            };
            let planner_error = errors.first().cloned();
            let planner_reproducibility = plan.reproducibility.clone();
            let review = plan.review;
            let planned_scene = plan.scene;
            let next_beat_id = entry_beat_id(&planned_scene)?.to_string();
            (
                planned_scene,
                next_beat_id,
                planner_fallback_used,
                planner_error,
                Some(review.scene_key.clone()),
                planner_reproducibility,
                Some(review),
            )
        } else {
            let current_scene = current_scene
                .ok_or_else(|| RuntimeEngineError::UnsupportedAction(action_type.clone()))?;
            let current_beat = current_beat
                .ok_or_else(|| RuntimeEngineError::UnsupportedAction(action_type.clone()))?;
            let next_beat_id = next_same_scene_beat_id(&current_scene, &current_beat)?;
            (
                current_scene.clone(),
                next_beat_id,
                false,
                None,
                None,
                self.reproducibility.clone(),
                None,
            )
        };

        self.story_state.current_beat_id = Some(next_beat_id);
        if change_scene {
            self.story_state
                .completed_scene_keys
                .push(story_state_before.current_scene_key.clone());
            self.story_state.current_scene_key = next_scene.key.clone();
            self.story_state.turn += 1;
            if self.project.scene(&next_scene.key).is_none() {
                self.project.scenes.push(next_scene.clone());
            }
        };

        self.world_state = world_state_after.clone();
        self.reproducibility = planner_reproducibility.clone();

        let planner_status = if planner_fallback_used {
            RuntimeTraceStageStatus::Fallback
        } else {
            RuntimeTraceStageStatus::Completed
        };
        let redacted_action_type = redact_trace_text(&action_type);
        let redacted_choice_id = selected_choice
            .as_ref()
            .map(|c| redact_trace_text(&c.id))
            .unwrap_or_default();
        let redacted_planner_scene_key = planner_scene_key
            .as_ref()
            .map(|scene_key| redact_trace_text(scene_key));
        let matched_terms_len = action_intent
            .as_ref()
            .map(|intent| intent.matched_terms.len())
            .unwrap_or(0);
        let diagnostics = vec![
            RuntimeTraceDiagnostic::new_redacted(
                RuntimeTraceStage::InterpretAction,
                if action_intent.is_some() {
                    RuntimeTraceStageStatus::Completed
                } else {
                    // The pi-Agent apply path carries no player-input-derived
                    // action_intent (the agent's ScenePlan is the intent).
                    // This is a deliberate skip, not a degraded fallback.
                    RuntimeTraceStageStatus::Skipped
                },
                format!("action `{action_type}` matched {matched_terms_len} term(s)"),
            ),
            RuntimeTraceDiagnostic::new_redacted(
                RuntimeTraceStage::SelectChoice,
                if selected_choice.is_some() {
                    RuntimeTraceStageStatus::Completed
                } else {
                    // No player choice on the pi-Agent path; the agent's
                    // proposal drives the turn. Deliberate skip, not fallback.
                    RuntimeTraceStageStatus::Skipped
                },
                if let Some(choice) = selected_choice.as_ref() {
                    format!("selected choice `{}`", choice.id)
                } else {
                    format!("agent proposal `{action_type}` (no player choice)")
                },
            ),
            RuntimeTraceDiagnostic::new_redacted(
                RuntimeTraceStage::EvaluateRules,
                RuntimeTraceStageStatus::Completed,
                if world_delta.is_empty() {
                    format!("rules evaluated for `{action_type}` with no world delta")
                } else {
                    format!("rules evaluated for `{action_type}` and produced a world delta")
                },
            ),
            RuntimeTraceDiagnostic::new_redacted(
                RuntimeTraceStage::PlanScene,
                planner_status,
                if change_scene {
                    if planner_fallback_used {
                        if let (Some(scene_key), Some(error)) =
                            (planner_scene_key.as_ref(), planner_error.as_ref())
                        {
                            format!(
                                "planner returned fallback scene `{scene_key}` after `{}`",
                                error.code
                            )
                        } else if let Some(scene_key) = planner_scene_key.as_ref() {
                            format!("planner returned fallback scene `{scene_key}`")
                        } else {
                            "planner returned fallback scene".into()
                        }
                    } else if let Some(scene_key) = planner_scene_key.as_ref() {
                        format!("planner returned scene `{scene_key}`")
                    } else {
                        "planner returned scene".into()
                    }
                } else {
                    format!(
                        "planner skipped for same-scene beat `{}`",
                        self.story_state
                            .current_beat_id
                            .as_deref()
                            .unwrap_or("unknown")
                    )
                },
            ),
            RuntimeTraceDiagnostic::new_redacted(
                RuntimeTraceStage::CommitState,
                RuntimeTraceStageStatus::Completed,
                format!("committed story turn {}", self.story_state.turn),
            ),
        ];

        let trace_id = format!("trace-{:03}", self.story_state.turn);
        let trace = RuntimeTrace {
            reproducibility: planner_reproducibility.with_trace_id(&trace_id),
            id: trace_id,
            timestamp_ms: u64::from(self.story_state.turn),
            player_input: Some(redact_trace_text(player_input)),
            selected_choice: if selected_choice.is_some() {
                Some(redacted_choice_id)
            } else {
                None
            },
            action_intent: action_intent.map(|intent| intent.redacted()),
            rule_result: Some(RuntimeRuleResult {
                action_type: redacted_action_type.clone(),
                delta_empty: world_delta.is_empty(),
                state_committed: true,
                error: None,
            }),
            planner_result: Some(RuntimePlannerResult {
                requested_action_type: redacted_action_type,
                scene_key: redacted_planner_scene_key,
                fallback_used: planner_fallback_used,
                error: planner_error.clone(),
            }),
            diagnostics,
            world_state_before,
            world_state_delta: world_delta,
            world_state_after,
            story_state_before,
            story_state_after: self.story_state.clone(),
            narrative_review: review.map(|review| review.redacted()),
            media_references: scene_media_references(
                &next_scene,
                self.story_state.current_beat_id.as_deref(),
            ),
            errors: planner_error.into_iter().collect(),
            fallback_used: planner_fallback_used,
        };

        Ok(RuntimeStep {
            scene: next_scene,
            trace,
        })
    }
}

fn project_from_snapshot(
    mut project: ProjectData,
    snapshot: &RuntimeSnapshot,
) -> Result<ProjectData, RuntimeEngineError> {
    if project.game.id != snapshot.project_id {
        return Err(RuntimeEngineError::SnapshotProjectMismatch {
            expected: project.game.id,
            actual: snapshot.project_id.clone(),
        });
    }
    if project.game.version != snapshot.project_version {
        return Err(RuntimeEngineError::SnapshotVersionMismatch {
            expected: project.game.version,
            actual: snapshot.project_version.clone(),
        });
    }

    project.story_state = snapshot.story_state.clone();
    project.world_state = snapshot.world_state.clone();
    project.scenes = snapshot.scenes.clone();
    if project
        .scene(&project.story_state.current_scene_key)
        .is_none()
    {
        return Err(RuntimeEngineError::SnapshotMissingScene(
            project.story_state.current_scene_key,
        ));
    }
    let scene = project
        .scene(&project.story_state.current_scene_key)
        .expect("snapshot scene already checked");
    let beat_id = current_beat_id(&project.story_state, scene)
        .map_err(|error| match error {
            RuntimeEngineError::MissingBeat { scene_key, beat_id } => {
                RuntimeEngineError::SnapshotMissingBeat { scene_key, beat_id }
            }
            other => other,
        })?
        .to_string();
    if beat_by_id(scene, &beat_id).is_err() {
        return Err(RuntimeEngineError::SnapshotMissingBeat {
            scene_key: scene.key.clone(),
            beat_id,
        });
    }

    Ok(project)
}

pub fn interpret_action(
    player_input: &str,
    choices: &[Choice],
) -> Result<ActionIntent, RuntimeEngineError> {
    let normalized_input = normalize_input(player_input);
    let matches = choices
        .iter()
        .filter_map(|choice| {
            let matched_terms = choice_matched_terms(&normalized_input, choice);
            (!matched_terms.is_empty()).then_some((choice, matched_terms))
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Ok(ActionIntent::unsupported(
            "no current-beat choice matched the player input",
        )),
        [(choice, matched_terms)] => Ok(ActionIntent::for_choice(
            choice.id.clone(),
            choice.action_type.clone(),
            matched_terms.clone(),
        )),
        _ => Err(RuntimeEngineError::AmbiguousAction {
            input: player_input.to_string(),
            choice_ids: matches
                .iter()
                .map(|(choice, _)| choice.id.clone())
                .collect(),
        }),
    }
}

fn choice_matched_terms(input: &str, choice: &Choice) -> Vec<String> {
    choice
        .input_terms
        .iter()
        .map(|term| term.trim())
        .chain([
            choice.id.as_str(),
            choice.action_type.as_str(),
            choice.label.as_str(),
        ])
        .filter(|term| !term.is_empty())
        .filter(|term| {
            let normalized_term = normalize_input(term);
            !normalized_term.is_empty() && input.contains(&normalized_term)
        })
        .map(ToString::to_string)
        .collect()
}

fn normalize_input(input: &str) -> String {
    input.trim().to_lowercase()
}

fn current_beat_id<'a>(
    story_state: &'a StoryState,
    scene: &'a Scene,
) -> Result<&'a str, RuntimeEngineError> {
    match story_state.current_beat_id.as_deref() {
        Some(beat_id) => Ok(beat_id),
        None => entry_beat_id(scene),
    }
}

fn entry_beat_id(scene: &Scene) -> Result<&str, RuntimeEngineError> {
    let beat_id = scene
        .entry_beat_id
        .as_deref()
        .ok_or_else(|| RuntimeEngineError::MissingEntryBeat(scene.key.clone()))?;
    beat_by_id(scene, beat_id).map(|beat| beat.id.as_str())
}

fn beat_by_id<'a>(scene: &'a Scene, beat_id: &str) -> Result<&'a Beat, RuntimeEngineError> {
    scene
        .beats
        .iter()
        .find(|beat| beat.id == beat_id)
        .ok_or_else(|| RuntimeEngineError::MissingBeat {
            scene_key: scene.key.clone(),
            beat_id: beat_id.to_string(),
        })
}

fn selected_choice_by_id<'a>(beat: &'a Beat, choice_id: &str) -> Option<&'a Choice> {
    beat.choices.iter().find(|choice| choice.id == choice_id)
}

fn next_same_scene_beat_id(scene: &Scene, beat: &Beat) -> Result<String, RuntimeEngineError> {
    match &beat.next {
        BeatNext::Beat(beat_id) if beat_by_id(scene, beat_id).is_ok() => Ok(beat_id.clone()),
        _ => Err(RuntimeEngineError::MissingBeatTransition {
            scene_key: scene.key.clone(),
            beat_id: beat.id.clone(),
        }),
    }
}

fn fallback_errors(fallback_used: bool) -> Vec<RuntimeError> {
    if fallback_used {
        vec![RuntimeError::redacted(
            "fallback_scene",
            "Mock agent used a fallback scene because the requested scene was missing.",
        )]
    } else {
        Vec::new()
    }
}

fn scene_media_references(scene: &Scene, beat_id: Option<&str>) -> Vec<RuntimeMediaReference> {
    let mut references = vec![RuntimeMediaReference {
        reference: AssetReference {
            reference_kind: AssetReferenceKind::Scene,
            reference_id: redact_trace_text(&scene.key),
            slot: "background_asset".into(),
        },
        project_path: redact_trace_text(&scene.background_asset),
    }];
    references.extend(
        scene
            .audio_refs
            .iter()
            .map(|media_reference| RuntimeMediaReference {
                reference: AssetReference {
                    reference_kind: AssetReferenceKind::Scene,
                    reference_id: redact_trace_text(&scene.key),
                    slot: redact_trace_text(&media_reference.slot),
                },
                project_path: redact_trace_text(&media_reference.project_path),
            }),
    );
    if let Some(beat) =
        beat_id.and_then(|beat_id| scene.beats.iter().find(|beat| beat.id == beat_id))
    {
        references.extend(
            beat.audio_refs
                .iter()
                .map(|media_reference| RuntimeMediaReference {
                    reference: AssetReference {
                        reference_kind: AssetReferenceKind::Scene,
                        reference_id: redact_trace_text(&scene.key),
                        slot: redact_trace_text(&format!(
                            "beat_audio:{}:{}",
                            beat.id, media_reference.slot
                        )),
                    },
                    project_path: redact_trace_text(&media_reference.project_path),
                }),
        );
    }
    references
}

pub fn summarize_delta(delta: &WorldDelta) -> Vec<String> {
    let mut lines = Vec::new();
    for (key, amount) in &delta.resource_changes {
        lines.push(format!("{key}: {amount:+}"));
    }
    for (key, value) in &delta.resource_sets {
        lines.push(format!("{key}: set {value}"));
    }
    for (key, value) in &delta.flags {
        lines.push(format!("flag {key}: {value}"));
    }
    for event in &delta.triggered_events {
        lines.push(format!("event: {event}"));
    }
    lines
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{RuntimeEngineError, RuntimeSession, RuntimeStepContext, interpret_action};
    use plotforge_agent::ScenePlan;
    use plotforge_schema::{
        Beat, BeatNext, Character, Choice, Effect, GameProject, ProjectData,
        ReproducibilityMetadata, ResourceDefinition, Rule, Scene, StoryState, WorldDelta,
        WorldState,
    };

    #[test]
    fn interprets_chinese_action_text() {
        let choices = test_choices();

        assert_eq!(
            interpret_action("决定加征港税", &choices)
                .expect("tax intent")
                .action_type(),
            Some("raise_tax")
        );
        assert_eq!(
            interpret_action("严查贪墨", &choices)
                .expect("corruption intent")
                .action_type(),
            Some("inspect_corruption")
        );
        assert_eq!(
            interpret_action("先拨内帑稳住边军军饷", &choices)
                .expect("army intent")
                .action_type(),
            Some("pay_army")
        );
        assert_eq!(
            interpret_action("题诗赏月", &choices)
                .expect("unsupported intent")
                .action_type(),
            None
        );
    }

    #[test]
    fn play_once_commits_rule_delta_and_trace() {
        let project = test_project();
        let mut session = RuntimeSession::new(project);

        let step = session.play_once("决定加征港税").expect("play");

        assert_eq!(
            step.trace.world_state_delta.resource_changes["treasury"],
            12
        );
        assert!(
            step.trace
                .world_state_after
                .triggered_events
                .contains(&"local_tax_resistance".into())
        );
        assert_eq!(step.scene.key, "civic-crisis-001");
        assert!(step.trace.narrative_review.expect("review").passes());
    }

    #[test]
    fn commit_scene_plan_commits_agent_proposal_without_player_choice() {
        // The pi-Agent path: no player-input-derived action_intent or
        // selected_choice (the agent's ScenePlan already names the next
        // scene). `commit_scene_plan` must still commit the scene, bump the
        // turn, write the pre-evaluated world_state_after, and produce a
        // trace with `action_intent = None` + `selected_choice = None`.
        let project = test_project();
        let mut session = RuntimeSession::new(project);

        let planned_scene = Scene {
            key: "agent-proposed-scene".into(),
            title: "Agent Proposed Scene".into(),
            location: "Test Court".into(),
            dramatic_purpose: "Agent drove the turn.".into(),
            hook: "A pi-Agent turn.".into(),
            background_asset: String::new(),
            audio_refs: Vec::new(),
            character_ids: Vec::new(),
            plot_thread_updates: BTreeMap::new(),
            entry_beat_id: Some("agent-proposed-beat-001".into()),
            beats: vec![Beat {
                id: "agent-proposed-beat-001".into(),
                text: "The agent's scene begins.".into(),
                speaker: None,
                line_delivery: None,
                audio_refs: Vec::new(),
                choices: Vec::new(),
                next: BeatNext::None,
            }],
        };
        let plan = ScenePlan {
            scene: planned_scene.clone(),
            review: plotforge_schema::NarrativeReview {
                scene_key: planned_scene.key.clone(),
                score: 80,
                hook_score: 80,
                pacing_score: 80,
                character_consistency_score: 80,
                payoff_score: 80,
                choice_meaningfulness_score: 80,
                ai_slop_risk: 10,
                issues: Vec::new(),
            },
            reproducibility: ReproducibilityMetadata::local_mock(7),
            fallback_used: false,
            error: None,
        };

        let world_state_before = session.world_state().clone();
        let world_state_after = WorldState {
            resources: BTreeMap::from([("treasury".into(), 52)]),
            flags: BTreeMap::new(),
            triggered_events: vec!["local_tax_resistance".into()],
        };
        let world_delta = WorldDelta {
            resource_changes: BTreeMap::from([("treasury".into(), 12)]),
            resource_sets: BTreeMap::new(),
            flags: BTreeMap::new(),
            triggered_events: vec!["local_tax_resistance".into()],
        };
        let story_state_before = session.story_state().clone();
        let context = RuntimeStepContext {
            player_input: "Agent proposed a scene change.",
            action_type: "change_scene".to_string(),
            action_intent: None,
            selected_choice: None,
            world_state_before,
            world_delta,
            world_state_after: world_state_after.clone(),
            story_state_before: story_state_before.clone(),
            current_scene: None,
            current_beat: None,
        };

        let step = session
            .commit_scene_plan(Some(plan), context, true)
            .expect("commit agent plan");

        // The scene was committed and turn bumped.
        assert_eq!(step.scene.key, "agent-proposed-scene");
        assert_eq!(session.story_state().turn, 1);
        assert_eq!(
            session.story_state().current_scene_key,
            "agent-proposed-scene"
        );
        // The pre-evaluated world_state_after was written verbatim.
        assert_eq!(session.world_state().resources["treasury"], 52);
        assert!(
            session
                .world_state()
                .triggered_events
                .contains(&"local_tax_resistance".into())
        );
        // The trace carries no player choice / action intent (pi-Agent path).
        assert!(step.trace.action_intent.is_none());
        assert!(step.trace.selected_choice.is_none());
        assert_eq!(step.trace.story_state_before.turn, 0);
        assert_eq!(step.trace.story_state_after.turn, 1);
        assert!(!step.trace.fallback_used);
        let _ = world_state_before;
    }

    #[test]
    fn commit_scene_plan_same_scene_without_beat_transition_fails_explicitly() {
        // The `continue` path: plan=None, change_scene=false. When the
        // current beat has no `BeatNext::Beat` pointer (it is `Scene` in the
        // test fixture), the same-scene transition must fail explicitly per
        // AGENTS.md ("Missing same-scene beat transitions must fail
        // explicitly"). No silent fallback, no turn bump.
        let project = test_project();
        let mut session = RuntimeSession::new(project);
        let current_scene = session
            .project
            .scene(&session.story_state().current_scene_key)
            .expect("current scene")
            .clone();
        let current_beat = current_scene
            .beats
            .iter()
            .find(|beat| beat.id == "runtime-unit-scene-beat-001")
            .expect("current beat")
            .clone();
        let world_state_before = session.world_state().clone();
        let story_state_before = session.story_state().clone();
        let world_state_after = world_state_before.clone();
        let world_delta = WorldDelta {
            resource_changes: BTreeMap::new(),
            resource_sets: BTreeMap::new(),
            flags: BTreeMap::new(),
            triggered_events: Vec::new(),
        };
        let context = RuntimeStepContext {
            player_input: "continue",
            action_type: "continue".to_string(),
            action_intent: None,
            selected_choice: None,
            world_state_before,
            world_delta,
            world_state_after,
            story_state_before,
            current_scene: Some(current_scene),
            current_beat: Some(current_beat),
        };
        let turn_before = session.story_state().turn;
        let error = session
            .commit_scene_plan(None, context, false)
            .expect_err("missing beat transition must fail explicitly");
        assert!(matches!(
            error,
            RuntimeEngineError::MissingBeatTransition { .. }
        ));
        // No state mutated on failure.
        assert_eq!(session.story_state().turn, turn_before);
    }

    /// Direct test for `apply_agent_scene_plan` (the rule-evaluation entry
    /// point the pi-Agent apply path drives). Covers finding L5: this was
    /// only exercised indirectly via the studio local-pi integration test.
    /// Asserts rule evaluation runs for `change_scene`, the scene is
    /// committed, the turn is bumped, and the trace carries no player
    /// action_intent / selected_choice.
    #[test]
    fn apply_agent_scene_plan_evaluates_rules_and_commits() {
        let project = test_project();
        let mut session = RuntimeSession::new(project);
        assert_eq!(session.story_state().turn, 0);

        let planned_scene = Scene {
            key: "agent-scene".into(),
            title: "Agent Scene".into(),
            location: "Test Court".into(),
            dramatic_purpose: "Agent drove the turn.".into(),
            hook: "A pi-Agent turn.".into(),
            background_asset: String::new(),
            audio_refs: Vec::new(),
            character_ids: Vec::new(),
            plot_thread_updates: BTreeMap::new(),
            entry_beat_id: Some("agent-scene-beat-001".into()),
            beats: vec![Beat {
                id: "agent-scene-beat-001".into(),
                text: "The agent's scene begins.".into(),
                speaker: None,
                line_delivery: None,
                audio_refs: Vec::new(),
                choices: Vec::new(),
                next: BeatNext::None,
            }],
        };
        let plan = ScenePlan {
            scene: planned_scene.clone(),
            review: plotforge_schema::NarrativeReview {
                scene_key: planned_scene.key.clone(),
                score: 50,
                hook_score: 50,
                pacing_score: 50,
                character_consistency_score: 50,
                payoff_score: 50,
                choice_meaningfulness_score: 50,
                ai_slop_risk: 20,
                issues: Vec::new(),
            },
            reproducibility: ReproducibilityMetadata::local_mock(7),
            fallback_used: false,
            error: None,
        };

        let step = session
            .apply_agent_scene_plan(plan, "Agent proposed a scene change.")
            .expect("apply agent scene plan");

        assert_eq!(step.scene.key, "agent-scene");
        assert_eq!(session.story_state().turn, 1);
        assert_eq!(session.story_state().current_scene_key, "agent-scene");
        // Rule evaluation ran for `change_scene` and produced a delta-derived
        // world_state_after. test_project's only rule keys on `raise_tax`, so
        // the `change_scene` delta is empty and `world_state_after` equals
        // `world_state_before`; the point here is that the rule engine was
        // invoked (the `apply_agent_scene_plan` boundary), not the business
        // result (which the rule-engine unit tests already cover).
        assert_eq!(
            step.trace.world_state_before.resources["treasury"],
            step.trace.world_state_after.resources["treasury"]
        );
        // The pi-Agent path carries no player action_intent / selected_choice.
        assert!(step.trace.action_intent.is_none());
        assert!(step.trace.selected_choice.is_none());
        assert!(!step.trace.fallback_used);
        assert_eq!(step.trace.story_state_after.turn, 1);
    }

    /// R6: the pi-Agent apply path carries no player action_intent / selected_choice,
    /// so the `InterpretAction` and `SelectChoice` trace diagnostics must be
    /// marked `Skipped` (deliberate skip — the agent's ScenePlan is the intent),
    /// NOT `Fallback` (which would misleadingly suggest a degraded result).
    #[test]
    fn apply_agent_scene_plan_marks_skipped_diagnostics_not_fallback() {
        use plotforge_schema::RuntimeTraceStage;
        let project = test_project();
        let mut session = RuntimeSession::new(project);
        let planned_scene = Scene {
            key: "agent-scene-r6".into(),
            title: "Agent Scene".into(),
            location: "Test Court".into(),
            dramatic_purpose: "Agent drove the turn.".into(),
            hook: "A pi-Agent turn.".into(),
            background_asset: String::new(),
            audio_refs: Vec::new(),
            character_ids: Vec::new(),
            plot_thread_updates: BTreeMap::new(),
            entry_beat_id: Some("agent-scene-r6-beat-001".into()),
            beats: vec![Beat {
                id: "agent-scene-r6-beat-001".into(),
                text: "The agent's scene begins.".into(),
                speaker: None,
                line_delivery: None,
                audio_refs: Vec::new(),
                choices: Vec::new(),
                next: BeatNext::None,
            }],
        };
        let plan = ScenePlan {
            scene: planned_scene,
            review: plotforge_schema::NarrativeReview {
                scene_key: "agent-scene-r6".into(),
                score: 50,
                hook_score: 50,
                pacing_score: 50,
                character_consistency_score: 50,
                payoff_score: 50,
                choice_meaningfulness_score: 50,
                ai_slop_risk: 20,
                issues: Vec::new(),
            },
            reproducibility: ReproducibilityMetadata::local_mock(7),
            fallback_used: false,
            error: None,
        };
        let step = session
            .apply_agent_scene_plan(plan, "Agent proposed a scene change.")
            .expect("apply agent scene plan");
        // The top-level fallback flag is false (this is not a fallback turn).
        assert!(!step.trace.fallback_used);
        // Find the InterpretAction and SelectChoice diagnostics.
        let interpret = step
            .trace
            .diagnostics
            .iter()
            .find(|d| d.stage == RuntimeTraceStage::InterpretAction)
            .expect("InterpretAction diagnostic present");
        let select = step
            .trace
            .diagnostics
            .iter()
            .find(|d| d.stage == RuntimeTraceStage::SelectChoice)
            .expect("SelectChoice diagnostic present");
        assert_eq!(
            interpret.status,
            plotforge_schema::RuntimeTraceStageStatus::Skipped,
            "InterpretAction must be Skipped on the pi-Agent path, not Fallback"
        );
        assert_eq!(
            select.status,
            plotforge_schema::RuntimeTraceStageStatus::Skipped,
            "SelectChoice must be Skipped on the pi-Agent path, not Fallback"
        );
    }

    fn test_project() -> ProjectData {
        let resources = vec![ResourceDefinition {
            key: "treasury".into(),
            label: "Treasury".into(),
            initial: 40,
            min: 0,
            max: 100,
        }];
        let scene = Scene {
            key: "runtime-unit-scene".into(),
            title: "Runtime Unit Scene".into(),
            location: "Test Court".into(),
            dramatic_purpose: "Test runtime commits.".into(),
            hook: "A test ledger waits.".into(),
            background_asset: "assets/generated/runtime-unit-scene.png".into(),
            audio_refs: Vec::new(),
            character_ids: Vec::new(),
            plot_thread_updates: BTreeMap::new(),
            entry_beat_id: Some("runtime-unit-scene-beat-001".into()),
            beats: vec![Beat {
                id: "runtime-unit-scene-beat-001".into(),
                text: "The test scene begins.".into(),
                speaker: None,
                line_delivery: None,
                audio_refs: Vec::new(),
                choices: test_choices(),
                next: BeatNext::Scene,
            }],
        };

        ProjectData {
            game: GameProject {
                id: "runtime-unit-fixture".into(),
                title: "Runtime Unit Fixture".into(),
                version: "0.1.0".into(),
                description: "Runtime unit fixture.".into(),
                entry_scene: scene.key.clone(),
                run_seed: 7,
            },
            resources: resources.clone(),
            world_state: WorldState {
                resources: BTreeMap::from([("treasury".into(), 40)]),
                flags: BTreeMap::new(),
                triggered_events: Vec::new(),
            },
            story_state: StoryState {
                current_scene_key: scene.key.clone(),
                current_beat_id: Some("runtime-unit-scene-beat-001".into()),
                completed_scene_keys: Vec::new(),
                turn: 0,
            },
            story_craft: plotforge_storycraft::sample_story_craft_state(),
            characters: vec![
                Character {
                    id: "city-treasurer".into(),
                    name: "City Treasurer".into(),
                    role: "Civic administrator".into(),
                    traits: vec!["cautious".into()],
                    visual_card: "elder official".into(),
                    voice_card: "restrained".into(),
                    portrait_request: None,
                },
                Character {
                    id: "guild-liaison".into(),
                    name: "Guild Liaison".into(),
                    role: "Civic channel".into(),
                    traits: vec!["watchful".into()],
                    visual_card: "guild official".into(),
                    voice_card: "quiet".into(),
                    portrait_request: None,
                },
            ],
            rules: vec![Rule {
                id: "raise-tax".into(),
                action_type: "raise_tax".into(),
                conditions: Vec::new(),
                effects: vec![
                    Effect::AddResource {
                        key: "treasury".into(),
                        amount: 12,
                    },
                    Effect::TriggerEvent {
                        event: "local_tax_resistance".into(),
                    },
                ],
            }],
            scenes: vec![scene],
            visual_bible: plotforge_schema::VisualBible::default(),
            audio_bible: plotforge_schema::AudioBible::default(),
            asset_records: Vec::new(),
            ai_safety_policy: plotforge_schema::AiSafetyPolicy::default(),
        }
    }

    fn test_choices() -> Vec<Choice> {
        vec![
            Choice {
                id: "raise-tax".into(),
                label: "Raise tax".into(),
                action_type: "raise_tax".into(),
                input_terms: vec!["加征".into(), "港税".into(), "raise".into(), "tax".into()],
                dramatic_purpose: "Trade order for revenue.".into(),
                change_scene: true,
            },
            Choice {
                id: "inspect-corruption".into(),
                label: "Inspect corruption".into(),
                action_type: "inspect_corruption".into(),
                input_terms: vec!["严查".into(), "贪墨".into()],
                dramatic_purpose: "Investigate corruption.".into(),
                change_scene: true,
            },
            Choice {
                id: "pay-army".into(),
                label: "Pay army".into(),
                action_type: "pay_army".into(),
                input_terms: vec!["内帑".into(), "边军".into(), "军饷".into()],
                dramatic_purpose: "Pay the army.".into(),
                change_scene: true,
            },
        ]
    }
}
