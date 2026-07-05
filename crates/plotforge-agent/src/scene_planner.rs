//! Scene planner trait, mock pipeline, and provider-backed pipelines.
//!
//! Owns the `ScenePlanner` contract and its three implementations: the
//! deterministic `MockAgentPipeline`, the text-only `ProviderAgentPipeline`,
//! and the image-aware `ImageProviderAgentPipeline`. Runtime owns scene/beat
//! progression; agents propose content and runtime/rules commit state.

use std::cell::RefCell;
use std::collections::BTreeMap;

use plotforge_job::{JobClock, JobQueue};
use plotforge_media::AssetRegistry;
use plotforge_schema::{
    AgentOutputEnvelope, AgentProposalPayload, AgentRole, Beat, BeatNext, Choice, NarrativeReview,
    ProjectData, ReproducibilityMetadata, RuntimeError, Scene, StoryState, WorldState,
    contains_secret_marker_text,
};
use plotforge_storycraft::review_scene;

use crate::pipelines::{ProviderPipelineError, repair_json_text, validate_agent_output_envelope};
use crate::providers_image::{ImageProvider, SceneImagePipeline, SceneImageRequest};
use crate::providers_text::{TextModelProvider, TextModelRequest};
use crate::shared::payload_kind;
use crate::validation::{scene_from_proposals, validate_agent_output_proposal};

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
    pub reproducibility: ReproducibilityMetadata,
    pub fallback_used: bool,
    pub error: Option<RuntimeError>,
}

impl ScenePlan {
    /// Assembles a `ScenePlan` from a pi-Agent `ScenePlanProposal` envelope.
    /// The proposal carries the scene skeleton (key, title, location,
    /// dramatic purpose, hook, cast, entry beat id, background asset); this
    /// constructor seeds a single entry beat with `BeatNext::None` so the
    /// runtime has a committable scene. The narrative review is a neutral
    /// pass/fail baseline (the pi-Agent surface does not yet produce a
    /// `ReviewProposal`; that is deferred). Reproducibility is carried
    /// through from the envelope.
    pub fn from_proposal(
        proposal: &plotforge_schema::ScenePlanProposal,
        reproducibility: ReproducibilityMetadata,
    ) -> Result<Self, ScenePlannerError> {
        let beat_id = if proposal.entry_beat_id.trim().is_empty() {
            format!("{}-beat-001", proposal.scene_key)
        } else {
            proposal.entry_beat_id.clone()
        };
        let beat = Beat {
            id: beat_id.clone(),
            text: proposal.scene_summary.clone(),
            speaker: None,
            line_delivery: None,
            audio_refs: Vec::new(),
            choices: Vec::new(),
            next: BeatNext::None,
        };
        let scene = Scene {
            key: proposal.scene_key.clone(),
            title: proposal.title.clone(),
            location: proposal.location.clone(),
            dramatic_purpose: proposal.dramatic_purpose.clone(),
            hook: proposal.hook.clone(),
            background_asset: proposal.background_asset.clone().unwrap_or_default(),
            audio_refs: Vec::new(),
            character_ids: proposal.cast.clone(),
            plot_thread_updates: BTreeMap::new(),
            beats: vec![beat],
            entry_beat_id: Some(beat_id),
        };
        let review = NarrativeReview {
            scene_key: scene.key.clone(),
            score: 50,
            hook_score: 50,
            pacing_score: 50,
            character_consistency_score: 50,
            payoff_score: 50,
            choice_meaningfulness_score: 50,
            ai_slop_risk: 20,
            issues: Vec::new(),
        };
        Ok(Self {
            scene,
            review,
            reproducibility,
            fallback_used: false,
            error: None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenePlannerError {
    pub code: String,
    pub message: String,
}

impl ScenePlannerError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ScenePlannerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ScenePlannerError {}

pub trait ScenePlanner {
    fn plan_next_scene(
        &self,
        request: ScenePlanRequest<'_>,
    ) -> Result<ScenePlan, ScenePlannerError>;
}

#[derive(Clone, Debug)]
pub struct ImageProviderAgentPipeline<T, I, C> {
    text_provider: T,
    image_pipeline: SceneImagePipeline<I>,
    state: RefCell<SceneImagePipelineState<C>>,
}

impl<T, I, C> ImageProviderAgentPipeline<T, I, C>
where
    C: JobClock,
{
    pub fn new(text_provider: T, image_provider: I, clock: C) -> Self {
        Self {
            text_provider,
            image_pipeline: SceneImagePipeline::new(image_provider),
            state: RefCell::new(SceneImagePipelineState {
                asset_registry: AssetRegistry::new(),
                job_queue: JobQueue::new(clock),
            }),
        }
    }

    pub fn asset_records(&self) -> Vec<plotforge_schema::AssetRecord> {
        self.state
            .borrow()
            .asset_registry
            .records()
            .cloned()
            .collect()
    }

    pub fn job_records(&self) -> Vec<plotforge_schema::JobRecord> {
        self.state.borrow().job_queue.records().cloned().collect()
    }
}

impl<T, I, C> ScenePlanner for ImageProviderAgentPipeline<T, I, C>
where
    T: TextModelProvider,
    I: ImageProvider,
    C: JobClock,
{
    fn plan_next_scene(
        &self,
        request: ScenePlanRequest<'_>,
    ) -> Result<ScenePlan, ScenePlannerError> {
        let next_turn = request.story_state.turn + 1;
        let scene_key = format!("provider-scene-{next_turn:03}");

        if request.action_type == "continue" {
            let (scene, fallback_used, error) = if let Some(scene) = request
                .project
                .scene(&request.story_state.current_scene_key)
            {
                (scene.clone(), false, None)
            } else {
                (
                    fallback_scene(next_turn, request.action_type),
                    true,
                    Some(RuntimeError::redacted(
                        "fallback_scene",
                        "Image provider pipeline used a fallback scene because the requested scene was missing.",
                    )),
                )
            };
            let review = review_scene(
                &scene,
                &request.project.story_craft,
                &request.project.characters,
            );
            return Ok(ScenePlan {
                scene,
                review,
                reproducibility: local_reproducibility(&request),
                fallback_used,
                error,
            });
        }

        match provider_scene_plan(&self.text_provider, &request, &scene_key) {
            Ok(mut generated) => {
                let image_request = SceneImageRequest::background(&generated.scene);
                let image_result = {
                    let mut state = self.state.borrow_mut();
                    let SceneImagePipelineState {
                        asset_registry,
                        job_queue,
                    } = &mut *state;
                    self.image_pipeline.generate_scene_background(
                        image_request,
                        asset_registry,
                        job_queue,
                    )
                }
                .map_err(|error| {
                    ScenePlannerError::new("image_pipeline_error", error.to_string())
                })?;
                generated.scene.background_asset = image_result.asset_record.export_path.clone();

                Ok(ScenePlan {
                    scene: generated.scene,
                    review: generated.review,
                    reproducibility: generated.reproducibility,
                    fallback_used: image_result.fallback_used,
                    error: image_result.error,
                })
            }
            Err(error) => {
                let runtime_error = error.into_runtime_error();
                let scene = fallback_scene(next_turn, request.action_type);
                let review = review_scene(
                    &scene,
                    &request.project.story_craft,
                    &request.project.characters,
                );
                Ok(ScenePlan {
                    scene,
                    review,
                    reproducibility: local_reproducibility(&request),
                    fallback_used: true,
                    error: Some(runtime_error),
                })
            }
        }
    }
}

#[derive(Clone, Debug)]
struct SceneImagePipelineState<C> {
    asset_registry: AssetRegistry,
    job_queue: JobQueue<C>,
}

#[derive(Clone, Debug)]
pub struct ProviderAgentPipeline<P> {
    provider: P,
}

impl<P> ProviderAgentPipeline<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P> ScenePlanner for ProviderAgentPipeline<P>
where
    P: TextModelProvider,
{
    fn plan_next_scene(
        &self,
        request: ScenePlanRequest<'_>,
    ) -> Result<ScenePlan, ScenePlannerError> {
        let next_turn = request.story_state.turn + 1;
        let scene_key = format!("provider-scene-{next_turn:03}");

        if request.action_type == "continue" {
            let (scene, fallback_used, error) = if let Some(scene) = request
                .project
                .scene(&request.story_state.current_scene_key)
            {
                (scene.clone(), false, None)
            } else {
                (
                    fallback_scene(next_turn, request.action_type),
                    true,
                    Some(RuntimeError::redacted(
                        "fallback_scene",
                        "Provider pipeline used a fallback scene because the requested scene was missing.",
                    )),
                )
            };
            let review = review_scene(
                &scene,
                &request.project.story_craft,
                &request.project.characters,
            );
            return Ok(ScenePlan {
                scene,
                review,
                reproducibility: local_reproducibility(&request),
                fallback_used,
                error,
            });
        }

        match self.provider_scene_plan(&request, &scene_key) {
            Ok(generated) => Ok(ScenePlan {
                scene: generated.scene,
                review: generated.review,
                reproducibility: generated.reproducibility,
                fallback_used: false,
                error: None,
            }),
            Err(error) => {
                let runtime_error = error.into_runtime_error();
                let scene = fallback_scene(next_turn, request.action_type);
                let review = review_scene(
                    &scene,
                    &request.project.story_craft,
                    &request.project.characters,
                );
                Ok(ScenePlan {
                    scene,
                    review,
                    reproducibility: local_reproducibility(&request),
                    fallback_used: true,
                    error: Some(runtime_error),
                })
            }
        }
    }
}

impl<P> ProviderAgentPipeline<P>
where
    P: TextModelProvider,
{
    fn provider_scene_plan(
        &self,
        request: &ScenePlanRequest<'_>,
        scene_key: &str,
    ) -> Result<ProviderScenePlan, ProviderPipelineError> {
        provider_scene_plan(&self.provider, request, scene_key)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProviderScenePlan {
    scene: Scene,
    review: NarrativeReview,
    reproducibility: ReproducibilityMetadata,
}

fn provider_scene_plan<P>(
    provider: &P,
    request: &ScenePlanRequest<'_>,
    scene_key: &str,
) -> Result<ProviderScenePlan, ProviderPipelineError>
where
    P: TextModelProvider,
{
    let scene_plan_output =
        complete_agent_output(provider, AgentRole::ScenePlanner, request, scene_key)?;
    let scene_plan = match scene_plan_output.proposal.output {
        AgentProposalPayload::ScenePlan(scene_plan) => *scene_plan,
        output => {
            return Err(ProviderPipelineError::Validation {
                agent: AgentRole::ScenePlanner,
                message: format!("unexpected payload `{}`", payload_kind(&output)),
            });
        }
    };
    let beat_output = complete_agent_output(provider, AgentRole::BeatWriter, request, scene_key)?;
    let beat_drafts = match beat_output.proposal.output {
        AgentProposalPayload::BeatDrafts(beat_drafts) => *beat_drafts,
        output => {
            return Err(ProviderPipelineError::Validation {
                agent: AgentRole::BeatWriter,
                message: format!("unexpected payload `{}`", payload_kind(&output)),
            });
        }
    };
    let review_output = complete_agent_output(provider, AgentRole::PlotDoctor, request, scene_key)?;
    let review = match review_output.proposal.output {
        AgentProposalPayload::Review(review) => *review,
        output => {
            return Err(ProviderPipelineError::Validation {
                agent: AgentRole::PlotDoctor,
                message: format!("unexpected payload `{}`", payload_kind(&output)),
            });
        }
    };
    let scene =
        scene_from_proposals(&scene_plan, &beat_drafts, Some(&review)).map_err(|error| {
            ProviderPipelineError::Validation {
                agent: AgentRole::ScenePlanner,
                message: format!("scene assembly rejected provider proposals: {error:?}"),
            }
        })?;

    Ok(ProviderScenePlan {
        scene,
        review: review.review,
        reproducibility: scene_plan_output.reproducibility,
    })
}

fn complete_agent_output<P>(
    provider: &P,
    agent: AgentRole,
    request: &ScenePlanRequest<'_>,
    scene_key: &str,
) -> Result<AgentOutputEnvelope, ProviderPipelineError>
where
    P: TextModelProvider,
{
    let prompt = text_model_prompt(&agent, request, scene_key);
    if contains_secret_marker_text(&prompt) {
        return Err(ProviderPipelineError::Validation {
            agent,
            message: "text generation prompt contained a secret marker".into(),
        });
    }
    let reproducibility = provider.reproducibility_metadata(request.project.game.run_seed);
    let model_request = TextModelRequest {
        call_id: format!("{}-{}", scene_key, payload_call_suffix(&agent)),
        agent: agent.clone(),
        scene_key: scene_key.to_string(),
        run_seed: reproducibility.run_seed,
        prompt_version: reproducibility.prompt_version.clone(),
        model_version: reproducibility.model_version.clone(),
        provider_config_hash: reproducibility.provider_config_hash.clone(),
        prompt,
    };
    let response = provider
        .complete(&model_request)
        .map_err(ProviderPipelineError::Provider)?;
    if contains_secret_marker_text(&response.raw_json) {
        return Err(ProviderPipelineError::Validation {
            agent,
            message: "provider output contained a secret marker".into(),
        });
    }
    let repaired_json = repair_json_text(&response.raw_json).map_err(|message| {
        ProviderPipelineError::InvalidJson {
            agent: agent.clone(),
            message,
        }
    })?;
    let mut envelope =
        serde_json::from_str::<AgentOutputEnvelope>(&repaired_json).map_err(|error| {
            ProviderPipelineError::InvalidJson {
                agent: agent.clone(),
                message: error.to_string(),
            }
        })?;
    validate_agent_output_envelope(&envelope, &agent).map_err(|message| {
        ProviderPipelineError::Validation {
            agent: agent.clone(),
            message,
        }
    })?;
    // The runtime owns reproducibility identity — overwrite the envelope's
    // reproducibility block with the locally-expected values so real provider
    // responses (which cannot echo these values) still validate. See
    // `complete_text_agent_output` for the rationale.
    envelope.reproducibility = reproducibility.clone();
    validate_agent_output_proposal(&envelope.proposal).map_err(|error| {
        ProviderPipelineError::Validation {
            agent,
            message: format!("{error:?}"),
        }
    })?;

    Ok(envelope)
}

fn local_reproducibility(request: &ScenePlanRequest<'_>) -> ReproducibilityMetadata {
    ReproducibilityMetadata::local_mock(request.project.game.run_seed)
}

fn payload_call_suffix(agent: &AgentRole) -> &'static str {
    match agent {
        AgentRole::ScenePlanner => "scene-plan",
        AgentRole::BeatWriter => "beats",
        AgentRole::PlotDoctor => "review",
        AgentRole::StoryArchitect => "story-architect",
        AgentRole::StoryCraftPlanner => "story-craft-planner",
        AgentRole::CharacterDesigner => "character-designer",
        AgentRole::ConsistencyChecker => "consistency-checker",
        AgentRole::DeslopRefiner => "deslop-refiner",
    }
}

fn text_model_prompt(agent: &AgentRole, request: &ScenePlanRequest<'_>, scene_key: &str) -> String {
    format!(
        "agent={agent:?}; scene_key={scene_key}; action_type={}; player_input={}; turn={}",
        request.action_type, request.player_input, request.story_state.turn
    )
}

#[derive(Clone, Debug, Default)]
pub struct MockAgentPipeline;

impl ScenePlanner for MockAgentPipeline {
    fn plan_next_scene(
        &self,
        request: ScenePlanRequest<'_>,
    ) -> Result<ScenePlan, ScenePlannerError> {
        let next_turn = request.story_state.turn + 1;

        if request.action_type == "continue" {
            let (scene, fallback_used, error) = if let Some(scene) = request
                .project
                .scene(&request.story_state.current_scene_key)
            {
                (scene.clone(), false, None)
            } else {
                (
                    fallback_scene(next_turn, request.action_type),
                    true,
                    Some(RuntimeError::redacted(
                        "fallback_scene",
                        "Mock agent used a fallback scene because the requested scene was missing.",
                    )),
                )
            };
            let review = review_scene(
                &scene,
                &request.project.story_craft,
                &request.project.characters,
            );
            return Ok(ScenePlan {
                scene,
                review,
                reproducibility: local_reproducibility(&request),
                fallback_used,
                error,
            });
        }

        let scene = civic_scene(next_turn, &request);
        let review = review_scene(
            &scene,
            &request.project.story_craft,
            &request.project.characters,
        );
        Ok(ScenePlan {
            scene,
            review,
            reproducibility: local_reproducibility(&request),
            fallback_used: false,
            error: None,
        })
    }
}

fn civic_scene(turn: u32, request: &ScenePlanRequest<'_>) -> Scene {
    let scene_key = format!("civic-crisis-{turn:03}");
    let (title, hook, thread_update) = match request.action_type {
        "raise_tax" => (
            "Tax Resistance Memorials",
            "Before the ink dries, three provinces report tax runners beaten outside county offices.",
            "The new levy fills ledgers while lighting provincial resistance.",
        ),
        "inspect_corruption" => (
            "The Sealed Corruption Ledger",
            "A trembling clerk presents accounts that name both border officers and civic brokers.",
            "The insider channel becomes harder to dismiss.",
        ),
        "pay_army" => (
            "Border Payroll Relief",
            "Courier drums announce that the garrison has received silver, but the capital coffers echo.",
            "The army is calmed for now, at a visible treasury cost.",
        ),
        _ => (
            "Council of Unsteady Advisers",
            "Every minister bows lower than usual while avoiding the map of burning counties.",
            "The council delays, and every delay becomes a political fact.",
        ),
    };

    let first_beat_id = format!("{scene_key}-beat-001");
    let second_beat_id = format!("{scene_key}-beat-002");

    Scene {
        key: scene_key.clone(),
        title: title.into(),
        location: "Civic Hall".into(),
        dramatic_purpose: format!(
            "Show the consequence of `{}` and push the city toward a harder tradeoff.",
            request.action_type
        ),
        hook: hook.into(),
        background_asset: format!("assets/generated/{scene_key}.png"),
        audio_refs: Vec::new(),
        character_ids: vec!["city-treasurer".into(), "guild-liaison".into()],
        plot_thread_updates: BTreeMap::from([(
            thread_for_action(request.action_type).into(),
            thread_update.into(),
        )]),
        entry_beat_id: Some(first_beat_id.clone()),
        beats: vec![
            Beat {
                id: first_beat_id,
                text: format!(
                    "The council absorbs the order: {} The treasury stands at {}, public order at {}, and army morale at {}.",
                    request.player_input,
                    resource(request.world_state, "treasury"),
                    resource(request.world_state, "public_order"),
                    resource(request.world_state, "army_morale")
                ),
                speaker: Some("city-treasurer".into()),
                line_delivery: Some("controlled alarm".into()),
                audio_refs: Vec::new(),
                choices: vec![
                    Choice {
                        id: "raise-tax".into(),
                        label: "Press another emergency levy".into(),
                        action_type: "raise_tax".into(),
                        input_terms: crate::shared::choice_input_terms("raise_tax"),
                        dramatic_purpose: "Gain treasury while risking unrest.".into(),
                        change_scene: true,
                    },
                    Choice {
                        id: "inspect-corruption".into(),
                        label: "Investigate payroll corruption".into(),
                        action_type: "inspect_corruption".into(),
                        input_terms: crate::shared::choice_input_terms("inspect_corruption"),
                        dramatic_purpose: "Seek hidden leakage while destabilizing civic factions."
                            .into(),
                        change_scene: true,
                    },
                    Choice {
                        id: "continue-council".into(),
                        label: "Hear one more minister".into(),
                        action_type: "continue".into(),
                        input_terms: crate::shared::choice_input_terms("continue"),
                        dramatic_purpose:
                            "Stay in the scene to gather more pressure before committing.".into(),
                        change_scene: false,
                    },
                    Choice {
                        id: "pay-army".into(),
                        label: "Pay the border army first".into(),
                        action_type: "pay_army".into(),
                        input_terms: crate::shared::choice_input_terms("pay_army"),
                        dramatic_purpose: "Spend scarce treasury to buy military time.".into(),
                        change_scene: true,
                    },
                ],
                next: BeatNext::Beat(second_beat_id.clone()),
            },
            Beat {
                id: second_beat_id,
                text:
                    "A second minister adds a sharper warning: every answer now has a visible cost."
                        .into(),
                speaker: Some("war-minister".into()),
                line_delivery: Some("terse warning".into()),
                audio_refs: Vec::new(),
                choices: vec![
                    Choice {
                        id: "raise-tax".into(),
                        label: "Press another emergency levy".into(),
                        action_type: "raise_tax".into(),
                        input_terms: crate::shared::choice_input_terms("raise_tax"),
                        dramatic_purpose: "Gain treasury while risking unrest.".into(),
                        change_scene: true,
                    },
                    Choice {
                        id: "inspect-corruption".into(),
                        label: "Investigate payroll corruption".into(),
                        action_type: "inspect_corruption".into(),
                        input_terms: crate::shared::choice_input_terms("inspect_corruption"),
                        dramatic_purpose: "Seek hidden leakage while destabilizing civic factions."
                            .into(),
                        change_scene: true,
                    },
                    Choice {
                        id: "pay-army".into(),
                        label: "Pay the border army first".into(),
                        action_type: "pay_army".into(),
                        input_terms: crate::shared::choice_input_terms("pay_army"),
                        dramatic_purpose: "Spend scarce treasury to buy military time.".into(),
                        change_scene: true,
                    },
                ],
                next: BeatNext::Scene,
            },
        ],
    }
}

fn fallback_scene(turn: u32, action_type: &str) -> Scene {
    let scene_key = format!("fallback-{turn:03}");
    let first_beat_id = format!("{scene_key}-beat-001");
    let second_beat_id = format!("{scene_key}-beat-002");
    Scene {
        key: scene_key.clone(),
        title: "Fallback Council".into(),
        location: "Civic Hall".into(),
        dramatic_purpose: "Keep the deterministic mock loop visible after a missing scene.".into(),
        hook: "A fallback council forms because the requested scene was missing.".into(),
        background_asset: format!("assets/generated/{scene_key}.png"),
        audio_refs: Vec::new(),
        character_ids: Vec::new(),
        plot_thread_updates: BTreeMap::from([(
            "tax-disorder".into(),
            format!("Fallback response for action `{action_type}`."),
        )]),
        entry_beat_id: Some(first_beat_id.clone()),
        beats: vec![
            Beat {
                id: first_beat_id,
                text: "The council waits for the engine to recover a valid scene.".into(),
                speaker: None,
                line_delivery: None,
                audio_refs: Vec::new(),
                choices: vec![Choice {
                    id: "continue".into(),
                    label: "Continue".into(),
                    action_type: "continue".into(),
                    input_terms: crate::shared::choice_input_terms("continue"),
                    dramatic_purpose: "Remain in the current recovery beat.".into(),
                    change_scene: false,
                }],
                next: BeatNext::Beat(second_beat_id.clone()),
            },
            Beat {
                id: second_beat_id,
                text: "The recovery beat has no new scene request; the fallback remains visible."
                    .into(),
                speaker: None,
                line_delivery: None,
                audio_refs: Vec::new(),
                choices: Vec::new(),
                next: BeatNext::End,
            },
        ],
    }
}

fn resource(world_state: &WorldState, key: &str) -> i32 {
    world_state.resources.get(key).copied().unwrap_or_default()
}

fn thread_for_action(action_type: &str) -> &'static str {
    match action_type {
        "raise_tax" => "tax-disorder",
        "inspect_corruption" => "council-insider",
        "pay_army" => "border-payroll",
        _ => "tax-disorder",
    }
}
