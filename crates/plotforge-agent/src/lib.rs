use std::collections::BTreeMap;

use plotforge_schema::{Beat, Choice, NarrativeReview, ProjectData, Scene, StoryState, WorldState};
use plotforge_storycraft::review_scene;

#[derive(Clone, Debug)]
pub struct ScenePlanRequest<'a> {
    pub project: &'a ProjectData,
    pub story_state: &'a StoryState,
    pub world_state: &'a WorldState,
    pub player_input: &'a str,
    pub action_type: &'a str,
}

#[derive(Clone, Debug)]
pub struct ScenePlan {
    pub scene: Scene,
    pub review: NarrativeReview,
    pub fallback_used: bool,
}

pub trait ScenePlanner {
    fn plan_next_scene(&self, request: ScenePlanRequest<'_>) -> ScenePlan;
}

#[derive(Clone, Debug, Default)]
pub struct MockAgentPipeline;

impl ScenePlanner for MockAgentPipeline {
    fn plan_next_scene(&self, request: ScenePlanRequest<'_>) -> ScenePlan {
        let next_turn = request.story_state.turn + 1;

        if request.action_type == "continue" {
            let (scene, fallback_used) = if let Some(scene) = request
                .project
                .scene(&request.story_state.current_scene_key)
            {
                (scene.clone(), false)
            } else {
                (fallback_scene(next_turn, request.action_type), true)
            };
            let review = review_scene(
                &scene,
                &request.project.story_craft,
                &request.project.characters,
            );
            return ScenePlan {
                scene,
                review,
                fallback_used,
            };
        }

        let scene = dynasty_scene(next_turn, &request);
        let review = review_scene(
            &scene,
            &request.project.story_craft,
            &request.project.characters,
        );
        ScenePlan {
            scene,
            review,
            fallback_used: false,
        }
    }
}

fn dynasty_scene(turn: u32, request: &ScenePlanRequest<'_>) -> Scene {
    let scene_key = format!("court-crisis-{turn:03}");
    let (title, hook, thread_update) = match request.action_type {
        "raise_tax" => (
            "Tax Resistance Memorials",
            "Before the ink dries, three provinces report tax runners beaten outside county offices.",
            "The new levy fills ledgers while lighting provincial resistance.",
        ),
        "inspect_corruption" => (
            "The Sealed Corruption Ledger",
            "A trembling clerk presents accounts that name both border officers and palace brokers.",
            "The insider channel becomes harder to dismiss.",
        ),
        "pay_army" => (
            "Border Payroll Relief",
            "Courier drums announce that the garrison has received silver, but the capital coffers echo.",
            "The army is calmed for now, at a visible treasury cost.",
        ),
        _ => (
            "Council of Unsteady Ministers",
            "Every minister bows lower than usual while avoiding the map of burning counties.",
            "The court delays, and every delay becomes a political fact.",
        ),
    };

    Scene {
        key: scene_key.clone(),
        title: title.into(),
        location: "Qianqing Palace".into(),
        dramatic_purpose: format!(
            "Show the consequence of `{}` and push the dynasty toward a harder tradeoff.",
            request.action_type
        ),
        hook: hook.into(),
        background_asset: format!("assets/generated/{scene_key}.png"),
        character_ids: vec!["grand-secretary".into(), "eunuch-director".into()],
        plot_thread_updates: BTreeMap::from([(
            thread_for_action(request.action_type).into(),
            thread_update.into(),
        )]),
        beats: vec![Beat {
            id: format!("{scene_key}-beat-001"),
            text: format!(
                "The court absorbs the order: {} The treasury stands at {}, public order at {}, and army morale at {}.",
                request.player_input,
                resource(request.world_state, "treasury"),
                resource(request.world_state, "public_order"),
                resource(request.world_state, "army_morale")
            ),
            choices: vec![
                Choice {
                    id: "raise-tax".into(),
                    label: "Press another emergency levy".into(),
                    action_type: "raise_tax".into(),
                    dramatic_purpose: "Gain treasury while risking unrest.".into(),
                    change_scene: true,
                },
                Choice {
                    id: "inspect-corruption".into(),
                    label: "Investigate payroll corruption".into(),
                    action_type: "inspect_corruption".into(),
                    dramatic_purpose: "Seek hidden leakage while destabilizing court factions."
                        .into(),
                    change_scene: true,
                },
                Choice {
                    id: "continue-council".into(),
                    label: "Hear one more minister".into(),
                    action_type: "continue".into(),
                    dramatic_purpose:
                        "Stay in the scene to gather more pressure before committing.".into(),
                    change_scene: false,
                },
            ],
        }],
    }
}

fn fallback_scene(turn: u32, action_type: &str) -> Scene {
    let scene_key = format!("fallback-{turn:03}");
    Scene {
        key: scene_key.clone(),
        title: "Fallback Council".into(),
        location: "Qianqing Palace".into(),
        dramatic_purpose: "Keep the deterministic mock loop visible after a missing scene.".into(),
        hook: "A fallback council forms because the requested scene was missing.".into(),
        background_asset: format!("assets/generated/{scene_key}.png"),
        character_ids: Vec::new(),
        plot_thread_updates: BTreeMap::from([(
            "tax-disorder".into(),
            format!("Fallback response for action `{action_type}`."),
        )]),
        beats: vec![Beat {
            id: format!("{scene_key}-beat-001"),
            text: "The court waits for the engine to recover a valid scene.".into(),
            choices: vec![Choice {
                id: "continue".into(),
                label: "Continue".into(),
                action_type: "continue".into(),
                dramatic_purpose: "Remain in the current recovery beat.".into(),
                change_scene: false,
            }],
        }],
    }
}

fn resource(world_state: &WorldState, key: &str) -> i32 {
    world_state.resources.get(key).copied().unwrap_or_default()
}

fn thread_for_action(action_type: &str) -> &'static str {
    match action_type {
        "raise_tax" => "tax-disorder",
        "inspect_corruption" => "court-insider",
        "pay_army" => "border-payroll",
        _ => "tax-disorder",
    }
}

#[cfg(test)]
mod tests {
    use plotforge_schema::{GameProject, StoryState, WorldState};

    use super::{MockAgentPipeline, ScenePlanRequest, ScenePlanner};

    #[test]
    fn mock_pipeline_returns_valid_scene_and_review() {
        let project = plotforge_schema::ProjectData {
            game: GameProject {
                id: "demo".into(),
                title: "Demo".into(),
                version: "0.1.0".into(),
                description: "Demo".into(),
                entry_scene: "court-crisis-001".into(),
                run_seed: 7,
            },
            resources: Vec::new(),
            world_state: WorldState::default(),
            story_state: StoryState {
                current_scene_key: "court-crisis-001".into(),
                completed_scene_keys: Vec::new(),
                turn: 0,
            },
            story_craft: plotforge_storycraft::dynasty_embers_story_craft(),
            characters: vec![
                plotforge_schema::Character {
                    id: "grand-secretary".into(),
                    name: "Grand Secretary".into(),
                    role: "Court administrator".into(),
                    traits: vec!["cautious".into()],
                    visual_card: "elder official".into(),
                    voice_card: "restrained".into(),
                },
                plotforge_schema::Character {
                    id: "eunuch-director".into(),
                    name: "Eunuch Director".into(),
                    role: "Palace channel".into(),
                    traits: vec!["watchful".into()],
                    visual_card: "palace official".into(),
                    voice_card: "quiet".into(),
                },
            ],
            rules: Vec::new(),
            scenes: Vec::new(),
        };

        let plan = MockAgentPipeline.plan_next_scene(ScenePlanRequest {
            project: &project,
            story_state: &project.story_state,
            world_state: &project.world_state,
            player_input: "Raise the levy",
            action_type: "raise_tax",
        });

        assert_eq!(plan.scene.key, "court-crisis-001");
        assert!(plan.review.passes());
        assert!(!plan.fallback_used);
    }

    #[test]
    fn mock_pipeline_marks_continue_fallback() {
        let project = plotforge_schema::ProjectData {
            game: GameProject {
                id: "demo".into(),
                title: "Demo".into(),
                version: "0.1.0".into(),
                description: "Demo".into(),
                entry_scene: "missing".into(),
                run_seed: 7,
            },
            resources: Vec::new(),
            world_state: WorldState::default(),
            story_state: StoryState {
                current_scene_key: "missing".into(),
                completed_scene_keys: Vec::new(),
                turn: 0,
            },
            story_craft: plotforge_storycraft::dynasty_embers_story_craft(),
            characters: Vec::new(),
            rules: Vec::new(),
            scenes: Vec::new(),
        };

        let plan = MockAgentPipeline.plan_next_scene(ScenePlanRequest {
            project: &project,
            story_state: &project.story_state,
            world_state: &project.world_state,
            player_input: "continue",
            action_type: "continue",
        });

        assert!(plan.fallback_used);
        assert!(plan.scene.key.starts_with("fallback-"));
    }

    #[test]
    fn mock_pipeline_maps_actions_to_plot_threads() {
        let mut project = plotforge_storage::dynasty_embers_project();
        project.story_state.turn = 1;
        let pipeline = MockAgentPipeline;

        for (action_type, expected_thread) in [
            ("raise_tax", "tax-disorder"),
            ("inspect_corruption", "court-insider"),
            ("pay_army", "border-payroll"),
        ] {
            let plan = pipeline.plan_next_scene(ScenePlanRequest {
                project: &project,
                story_state: &project.story_state,
                world_state: &project.world_state,
                player_input: action_type,
                action_type,
            });
            assert!(
                plan.scene.plot_thread_updates.contains_key(expected_thread),
                "{action_type} should update {expected_thread}"
            );
            assert!(plan.scene.beats[0].text.contains("treasury"));
        }
    }
}
