use plotforge_agent::{MockAgentPipeline, ScenePlanRequest, ScenePlanner, ScenePlannerError};
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

        let (
            next_scene,
            next_beat_id,
            planner_fallback_used,
            planner_error,
            planner_scene_key,
            planner_reproducibility,
            review,
        ) = if change_scene {
            let plan = self.scene_planner.plan_next_scene(ScenePlanRequest {
                project: &self.project,
                story_state: &self.story_state,
                world_state: &world_state_after,
                player_input,
                action_type,
            })?;
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
            let next_beat_id = next_same_scene_beat_id(&current_scene, current_beat)?;
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
        let redacted_action_type = redact_trace_text(action_type);
        let redacted_choice_id = redact_trace_text(&selected_choice.id);
        let redacted_planner_scene_key = planner_scene_key
            .as_ref()
            .map(|scene_key| redact_trace_text(scene_key));
        let diagnostics = vec![
            RuntimeTraceDiagnostic::new_redacted(
                RuntimeTraceStage::InterpretAction,
                RuntimeTraceStageStatus::Completed,
                format!(
                    "action `{action_type}` matched {} term(s)",
                    action_intent.matched_terms.len()
                ),
            ),
            RuntimeTraceDiagnostic::new_redacted(
                RuntimeTraceStage::SelectChoice,
                RuntimeTraceStageStatus::Completed,
                format!("selected choice `{}`", selected_choice.id),
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
            selected_choice: Some(redacted_choice_id),
            action_intent: Some(action_intent.redacted()),
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
            media_references: scene_media_references(&next_scene),
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

fn scene_media_references(scene: &Scene) -> Vec<RuntimeMediaReference> {
    vec![RuntimeMediaReference {
        reference: AssetReference {
            reference_kind: AssetReferenceKind::Scene,
            reference_id: redact_trace_text(&scene.key),
            slot: "background_asset".into(),
        },
        project_path: redact_trace_text(&scene.background_asset),
    }]
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
    use plotforge_storage::dynasty_embers_project;

    use super::{RuntimeSession, interpret_action};

    #[test]
    fn interprets_chinese_action_text() {
        let project = dynasty_embers_project();
        let choices = &project.scenes[0].beats[0].choices;

        assert_eq!(
            interpret_action("朕决定加征辽饷", choices)
                .expect("tax intent")
                .action_type(),
            Some("raise_tax")
        );
        assert_eq!(
            interpret_action("严查贪墨", choices)
                .expect("corruption intent")
                .action_type(),
            Some("inspect_corruption")
        );
        assert_eq!(
            interpret_action("先拨内帑稳住边军军饷", choices)
                .expect("army intent")
                .action_type(),
            Some("pay_army")
        );
        assert_eq!(
            interpret_action("题诗赏月", choices)
                .expect("unsupported intent")
                .action_type(),
            None
        );
    }

    #[test]
    fn play_once_commits_rule_delta_and_trace() {
        let project = dynasty_embers_project();
        let mut session = RuntimeSession::new(project);

        let step = session.play_once("朕决定加征辽饷").expect("play");

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
        assert_eq!(step.scene.key, "court-crisis-001");
        assert!(step.trace.narrative_review.expect("review").passes());
    }
}
