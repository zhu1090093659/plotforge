use plotforge_agent::{MockAgentPipeline, ScenePlanRequest, ScenePlanner};
use plotforge_rule::{RuleEngine, RuleError};
use plotforge_schema::{
    ProjectData, RuntimeError, RuntimeTrace, Scene, StoryState, WorldDelta, WorldState,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeEngineError {
    #[error(transparent)]
    Rule(#[from] RuleError),
    #[error("current scene is missing: {0}")]
    MissingScene(String),
}

#[derive(Clone, Debug)]
pub struct RuntimeSession {
    project: ProjectData,
    story_state: StoryState,
    world_state: WorldState,
}

#[derive(Clone, Debug)]
pub struct RuntimeStep {
    pub scene: Scene,
    pub trace: RuntimeTrace,
}

impl RuntimeSession {
    pub fn new(project: ProjectData) -> Self {
        Self {
            story_state: project.story_state.clone(),
            world_state: project.world_state.clone(),
            project,
        }
    }

    pub fn story_state(&self) -> &StoryState {
        &self.story_state
    }

    pub fn world_state(&self) -> &WorldState {
        &self.world_state
    }

    pub fn play_once(&mut self, player_input: &str) -> Result<RuntimeStep, RuntimeEngineError> {
        let action_type = interpret_action(player_input);
        let choice_id = selected_choice_for_action(action_type);
        let world_state_before = self.world_state.clone();
        let story_state_before = self.story_state.clone();

        let rule_engine =
            RuleEngine::new(self.project.resources.clone(), self.project.rules.clone());
        let world_delta = rule_engine.evaluate(action_type, &world_state_before)?;
        let world_state_after = rule_engine.apply_delta(&world_state_before, &world_delta)?;

        let current_scene = self
            .project
            .scene(&self.story_state.current_scene_key)
            .cloned()
            .ok_or_else(|| {
                RuntimeEngineError::MissingScene(self.story_state.current_scene_key.clone())
            })?;

        let plan = MockAgentPipeline.plan_next_scene(ScenePlanRequest {
            project: &self.project,
            story_state: &self.story_state,
            world_state: &world_state_after,
            player_input,
            action_type,
        });

        let next_scene = if action_type == "continue" {
            current_scene
        } else {
            plan.scene
        };

        if action_type != "continue" {
            self.story_state
                .completed_scene_keys
                .push(story_state_before.current_scene_key.clone());
            self.story_state.current_scene_key = next_scene.key.clone();
            self.story_state.turn += 1;
            if self.project.scene(&next_scene.key).is_none() {
                self.project.scenes.push(next_scene.clone());
            }
        }

        self.world_state = world_state_after.clone();

        let trace = RuntimeTrace {
            id: format!("trace-{:03}", self.story_state.turn),
            timestamp_ms: u64::from(self.story_state.turn),
            player_input: Some(player_input.to_string()),
            selected_choice: Some(choice_id.to_string()),
            world_state_before,
            world_state_delta: world_delta,
            world_state_after,
            story_state_before,
            story_state_after: self.story_state.clone(),
            narrative_review: Some(plan.review),
            errors: fallback_errors(plan.fallback_used),
            fallback_used: plan.fallback_used,
        };

        Ok(RuntimeStep {
            scene: next_scene,
            trace,
        })
    }
}

pub fn interpret_action(player_input: &str) -> &'static str {
    let normalized = player_input.to_lowercase();
    if normalized.contains("continue")
        || normalized.contains("minister")
        || normalized.contains("听")
    {
        "continue"
    } else if normalized.contains("army")
        || normalized.contains("pay")
        || normalized.contains("军饷")
        || normalized.contains("拨")
    {
        "pay_army"
    } else if normalized.contains("tax")
        || normalized.contains("levy")
        || normalized.contains("辽饷")
        || normalized.contains("加征")
    {
        "raise_tax"
    } else if normalized.contains("corruption")
        || normalized.contains("inspect")
        || normalized.contains("贪")
        || normalized.contains("查")
    {
        "inspect_corruption"
    } else {
        "raise_tax"
    }
}

fn selected_choice_for_action(action_type: &str) -> &'static str {
    match action_type {
        "continue" => "continue-council",
        "inspect_corruption" => "inspect-corruption",
        "pay_army" => "pay-army",
        "raise_tax" => "raise-tax",
        _ => "raise-tax",
    }
}

fn fallback_errors(fallback_used: bool) -> Vec<RuntimeError> {
    if fallback_used {
        vec![RuntimeError {
            code: "fallback_scene".into(),
            message: "Mock agent used a fallback scene because the requested scene was missing."
                .into(),
        }]
    } else {
        Vec::new()
    }
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
        assert_eq!(interpret_action("朕决定加征辽饷"), "raise_tax");
        assert_eq!(interpret_action("严查贪墨"), "inspect_corruption");
        assert_eq!(interpret_action("先拨内帑稳住边军军饷"), "pay_army");
    }

    #[test]
    fn play_once_commits_rule_delta_and_trace() {
        let project = dynasty_embers_project();
        let mut session = RuntimeSession::new(project);

        let step = session
            .play_once("朕决定加征辽饷，同时严查贪墨官员。")
            .expect("play");

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
