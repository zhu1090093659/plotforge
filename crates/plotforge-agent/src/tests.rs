use std::{cell::RefCell, fs, rc::Rc};

use plotforge_job::{JobClock, JobQueue, UsageLedger};
use plotforge_media::AssetRegistry;
use plotforge_schema::{
    AgentOutputProposal, AgentProposalPayload, AgentRole, AssetKind, AssetReferenceKind,
    AssetSourceKind, BeatDraftProposal, BeatDraftsProposal, CharacterGenerationRequest, Choice,
    GameProject, GenerationStatus, JobStatus, NarrativeFunction, NarrativeReview,
    ProjectCreationRequest, ProjectTemplateId, REDACTED_TRACE_SECRET, ReviewProposal,
    ScenePlanProposal, Severity, StoryCraftEditDocument, StoryCraftGenerationRequest, StoryState,
    WorldEditDocument, WorldGenerationRequest, WorldState,
};

use super::{
    AgentProposalValidationError, ConfiguredTextModelProvider, FakeImageProvider,
    FakeTextModelProvider, FakeTtsProvider, ImageGenerationRequest, ImageGenerationResponse,
    ImageProvider, ImageProviderAgentPipeline, ImageProviderError, MockAgentPipeline,
    ProviderAgentPipeline, ProviderCredentialError, ProviderCredentialResolver, SceneImagePipeline,
    SceneImageRequest, ScenePlanRequest, ScenePlanner, TextModelClient, TextModelClientRequest,
    TextModelProviderError, TextModelResponse, TextProviderConfig, TtsPipeline, TtsProvider,
    TtsProviderError, TtsProviderOutput, TtsRequest, fake_success_response,
    generate_character_with_provider, generate_story_craft_with_provider,
    generate_world_expansion_with_provider, scene_from_proposals, validate_agent_output_proposal,
};

#[derive(Clone, Debug)]
struct ReportingImageProvider;

impl ImageProvider for ReportingImageProvider {
    fn reports_usage(&self) -> bool {
        true
    }

    fn generate(
        &self,
        request: &ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse, ImageProviderError> {
        Ok(ImageGenerationResponse::png(
            vec![1, 2, 3],
            "real-image",
            Some("image-model".into()),
            Some(format!("request-{}", request.scene_key)),
            17,
        ))
    }
}

#[derive(Clone, Debug)]
struct ReportingTtsProvider;

impl TtsProvider for ReportingTtsProvider {
    fn reports_usage(&self) -> bool {
        true
    }

    fn synthesize(&self, _request: &TtsRequest) -> Result<TtsProviderOutput, TtsProviderError> {
        Ok(TtsProviderOutput::audio(
            vec![4, 5, 6],
            "real-tts",
            Some("tts-model".into()),
            Some("tts-request".into()),
            23,
        ))
    }
}

fn agent_test_project() -> plotforge_schema::ProjectData {
    let temp = tempfile::tempdir().expect("tempdir");
    plotforge_storage::create_project_from_request(
        temp.path().join("agent-test-project"),
        sample_creation_request(),
        false,
    )
    .expect("create project")
    .project
}

fn create_starter_project(project_path: &std::path::Path) {
    plotforge_storage::create_project_from_request(project_path, sample_creation_request(), false)
        .expect("create project");
}

fn sample_creation_request() -> ProjectCreationRequest {
    ProjectCreationRequest {
        template: ProjectTemplateId::HistoricalCrisis,
        concept: "A local starter project for agent tests.".into(),
        visual_style: "clear readable test style".into(),
        voice_enabled: false,
        initial_scene_request: "A creator opens a fresh PlotForge project.".into(),
    }
}

#[derive(Clone, Debug)]
struct FakeClock {
    now_ms: u64,
}

impl FakeClock {
    fn new(now_ms: u64) -> Self {
        Self { now_ms }
    }
}

impl JobClock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.now_ms
    }
}

#[derive(Clone, Debug)]
struct StaticCredentialResolver {
    credential: Option<String>,
}

impl StaticCredentialResolver {
    fn token(value: impl Into<String>) -> Self {
        Self {
            credential: Some(value.into()),
        }
    }

    fn missing() -> Self {
        Self { credential: None }
    }
}

impl ProviderCredentialResolver for StaticCredentialResolver {
    fn resolve(&self, env_var: &str) -> Result<String, ProviderCredentialError> {
        self.credential
            .clone()
            .ok_or_else(|| ProviderCredentialError::Missing {
                env_var: env_var.into(),
            })
    }
}

#[derive(Clone, Debug)]
struct RecordingTextModelClient {
    credentials: Rc<RefCell<Vec<String>>>,
    prompts: Rc<RefCell<Vec<String>>>,
    error: Option<TextModelProviderError>,
}

impl RecordingTextModelClient {
    fn success() -> Self {
        Self {
            credentials: Rc::new(RefCell::new(Vec::new())),
            prompts: Rc::new(RefCell::new(Vec::new())),
            error: None,
        }
    }

    fn provider_error(message: impl Into<String>) -> Self {
        Self {
            credentials: Rc::new(RefCell::new(Vec::new())),
            prompts: Rc::new(RefCell::new(Vec::new())),
            error: Some(TextModelProviderError::provider(
                "text_client_error",
                message.into(),
            )),
        }
    }

    fn credentials(&self) -> Vec<String> {
        self.credentials.borrow().clone()
    }

    fn prompts(&self) -> Vec<String> {
        self.prompts.borrow().clone()
    }
}

impl TextModelClient for RecordingTextModelClient {
    fn complete(
        &self,
        request: TextModelClientRequest<'_>,
    ) -> Result<TextModelResponse, TextModelProviderError> {
        self.credentials
            .borrow_mut()
            .push(request.credential.to_string());
        self.prompts
            .borrow_mut()
            .push(request.request.prompt.to_string());
        if let Some(error) = self.error.clone() {
            return Err(error);
        }
        fake_success_response(request.request)
    }
}

#[test]
fn mock_pipeline_returns_valid_scene_and_review() {
    let project = plotforge_schema::ProjectData {
        game: GameProject {
            id: "demo".into(),
            title: "Demo".into(),
            version: "0.1.0".into(),
            description: "Demo".into(),
            entry_scene: "opening-scene".into(),
            run_seed: 7,
        },
        resources: Vec::new(),
        world_state: WorldState::default(),
        story_state: StoryState {
            current_scene_key: "opening-scene".into(),
            current_beat_id: Some("opening-scene-beat-001".into()),
            completed_scene_keys: Vec::new(),
            turn: 0,
        },
        story_craft: plotforge_storycraft::sample_story_craft_state(),
        characters: vec![
            plotforge_schema::Character {
                id: "city-treasurer".into(),
                name: "City Treasurer".into(),
                role: "Civic administrator".into(),
                traits: vec!["cautious".into()],
                visual_card: "elder official".into(),
                voice_card: "restrained".into(),
                portrait_request: None,
            },
            plotforge_schema::Character {
                id: "guild-liaison".into(),
                name: "Guild Liaison".into(),
                role: "Civic channel".into(),
                traits: vec!["watchful".into()],
                visual_card: "guild official".into(),
                voice_card: "quiet".into(),
                portrait_request: None,
            },
        ],
        rules: Vec::new(),
        scenes: Vec::new(),
        visual_bible: plotforge_schema::VisualBible::default(),
        audio_bible: plotforge_schema::AudioBible::default(),
        asset_records: Vec::new(),
        ai_safety_policy: plotforge_schema::AiSafetyPolicy::default(),
    };

    let plan = MockAgentPipeline
        .plan_next_scene(ScenePlanRequest {
            project: &project,
            story_state: &project.story_state,
            world_state: &project.world_state,
            player_input: "Raise the levy",
            action_type: "raise_tax",
        })
        .expect("plan");

    assert_eq!(plan.scene.key, "civic-crisis-001");
    assert!(plan.review.as_ref().expect("review").passes());
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
            current_beat_id: Some("missing-beat-001".into()),
            completed_scene_keys: Vec::new(),
            turn: 0,
        },
        story_craft: plotforge_schema::StoryCraftState::default(),
        characters: Vec::new(),
        rules: Vec::new(),
        scenes: Vec::new(),
        visual_bible: plotforge_schema::VisualBible::default(),
        audio_bible: plotforge_schema::AudioBible::default(),
        asset_records: Vec::new(),
        ai_safety_policy: plotforge_schema::AiSafetyPolicy::default(),
    };

    let plan = MockAgentPipeline
        .plan_next_scene(ScenePlanRequest {
            project: &project,
            story_state: &project.story_state,
            world_state: &project.world_state,
            player_input: "continue",
            action_type: "continue",
        })
        .expect("plan");

    assert!(plan.fallback_used);
    assert!(plan.scene.key.starts_with("fallback-"));
}

#[test]
fn mock_pipeline_maps_actions_to_plot_threads() {
    let mut project = agent_test_project();
    project.story_state.turn = 1;
    let pipeline = MockAgentPipeline;

    for (action_type, expected_thread) in [
        ("raise_tax", "tax-disorder"),
        ("inspect_corruption", "council-insider"),
        ("pay_army", "border-payroll"),
    ] {
        let plan = pipeline
            .plan_next_scene(ScenePlanRequest {
                project: &project,
                story_state: &project.story_state,
                world_state: &project.world_state,
                player_input: action_type,
                action_type,
            })
            .expect("plan");
        assert!(
            plan.scene.plot_thread_updates.contains_key(expected_thread),
            "{action_type} should update {expected_thread}"
        );
        assert!(plan.scene.beats[0].text.contains("treasury"));
    }
}

#[test]
fn valid_agent_output_proposals_validate_and_assemble_scene() {
    for proposal in [
        sample_scene_plan_output_proposal(),
        sample_beat_drafts_output_proposal(),
        sample_review_output_proposal(),
    ] {
        validate_agent_output_proposal(&proposal).expect("proposal valid");
    }

    let scene_plan = sample_scene_plan_proposal();
    let beat_drafts = sample_beat_drafts_proposal();
    let review = sample_review_proposal();
    let scene = scene_from_proposals(&scene_plan, &beat_drafts, Some(&review)).expect("scene");

    assert_eq!(scene.key, "civic-crisis-002");
    assert_eq!(scene.beats[0].id, "civic-crisis-002-beat-001");
    assert_eq!(scene.character_ids, vec!["city-treasurer"]);
    assert!(scene.plot_thread_updates.is_empty());
}

#[test]
fn agent_output_proposal_rejects_beat_scene_mismatch() {
    let mut proposal = sample_beat_drafts_output_proposal();
    if let AgentProposalPayload::BeatDrafts(beat_drafts) = &mut proposal.output {
        beat_drafts.beats[0].scene_key = "other-scene".into();
    }

    let error = validate_agent_output_proposal(&proposal).expect_err("mismatch");

    assert!(matches!(
        error,
        AgentProposalValidationError::BeatSceneMismatch { .. }
    ));
}

#[test]
fn agent_output_proposal_rejects_role_payload_mismatch() {
    let mut proposal = sample_beat_drafts_output_proposal();
    proposal.agent = AgentRole::ScenePlanner;

    let error = validate_agent_output_proposal(&proposal).expect_err("mismatch");

    assert!(matches!(
        error,
        AgentProposalValidationError::AgentPayloadMismatch {
            agent: AgentRole::ScenePlanner,
            payload_kind: "beat_drafts"
        }
    ));
}

#[test]
fn agent_output_proposal_rejects_missing_entry_beat() {
    let mut scene_plan = sample_scene_plan_proposal();
    scene_plan.entry_beat_id = "missing-beat".into();
    let beat_drafts = sample_beat_drafts_proposal();

    let error = scene_from_proposals(&scene_plan, &beat_drafts, None).expect_err("missing entry");

    assert!(matches!(
        error,
        AgentProposalValidationError::EntryBeatMissing(id) if id == "missing-beat"
    ));
}

#[test]
fn fake_text_provider_pipeline_builds_scene_from_json_proposals() {
    let project = agent_test_project();
    let pipeline = ProviderAgentPipeline::new(FakeTextModelProvider::success());

    let plan = pipeline
        .plan_next_scene(provider_request(&project))
        .expect("provider plan");

    assert_eq!(plan.scene.key, "provider-scene-001");
    assert_eq!(
        plan.review.as_ref().expect("review").scene_key,
        "provider-scene-001"
    );
    assert_eq!(plan.review.as_ref().expect("review").score, 96);
    assert_eq!(plan.reproducibility.run_seed, 7);
    assert_eq!(
        plan.reproducibility.prompt_version,
        "plotforge-agent-text-prompt-v1"
    );
    assert_eq!(plan.reproducibility.model_version, "fake-text-model-v1");
    assert_eq!(
        plan.reproducibility.provider_config_hash,
        "sha256:fake-text-provider-config-v1"
    );
    assert!(!plan.fallback_used);
    assert!(plan.error.is_none());
    assert!(plan.scene.plot_thread_updates.is_empty());
}

#[test]
fn fake_text_provider_pipeline_repairs_wrapped_json_output() {
    let project = agent_test_project();
    let pipeline =
        ProviderAgentPipeline::new(FakeTextModelProvider::wrapped_json(AgentRole::ScenePlanner));

    let plan = pipeline
        .plan_next_scene(provider_request(&project))
        .expect("provider plan");

    assert_eq!(plan.scene.key, "provider-scene-001");
    assert_eq!(
        plan.review.as_ref().expect("review").scene_key,
        "provider-scene-001"
    );
    assert_eq!(
        plan.reproducibility.provider_config_hash,
        "sha256:fake-text-provider-config-v1"
    );
    assert!(!plan.fallback_used);
    assert!(plan.error.is_none());
}

#[test]
fn fake_text_provider_pipeline_falls_back_on_provider_error() {
    let project = agent_test_project();
    let pipeline = ProviderAgentPipeline::new(FakeTextModelProvider::provider_error(
        AgentRole::ScenePlanner,
    ));

    let plan = pipeline
        .plan_next_scene(provider_request(&project))
        .expect("fallback plan");

    assert!(plan.fallback_used);
    assert_eq!(
        plan.error.as_ref().expect("provider error").code,
        "text_provider_error"
    );
    assert_eq!(plan.scene.key, "fallback-001");
}

#[test]
fn fake_text_provider_pipeline_falls_back_on_timeout() {
    let project = agent_test_project();
    let pipeline =
        ProviderAgentPipeline::new(FakeTextModelProvider::timeout(AgentRole::BeatWriter));

    let plan = pipeline
        .plan_next_scene(provider_request(&project))
        .expect("fallback plan");

    assert!(plan.fallback_used);
    assert_eq!(
        plan.error.as_ref().expect("timeout error").code,
        "text_provider_timeout"
    );
}

#[test]
fn fake_text_provider_pipeline_falls_back_on_schema_validation() {
    let project = agent_test_project();
    let pipeline =
        ProviderAgentPipeline::new(FakeTextModelProvider::invalid_schema(AgentRole::BeatWriter));

    let plan = pipeline
        .plan_next_scene(provider_request(&project))
        .expect("fallback plan");

    assert!(plan.fallback_used);
    assert_eq!(
        plan.error.as_ref().expect("validation error").code,
        "text_provider_schema_validation"
    );
}

#[test]
fn fake_text_provider_pipeline_falls_back_on_secret_marker_output() {
    let project = agent_test_project();
    let pipeline = ProviderAgentPipeline::new(FakeTextModelProvider::secret_marker(
        AgentRole::ScenePlanner,
    ));

    let plan = pipeline
        .plan_next_scene(provider_request(&project))
        .expect("fallback plan");

    let error = plan.error.as_ref().expect("validation error");
    assert!(plan.fallback_used);
    assert_eq!(error.code, "text_provider_schema_validation");
    assert!(error.message.contains("secret marker"));
    assert!(!error.message.contains("api_key"));
    assert!(!error.message.contains("sk-test-secret-marker"));
}

#[test]
fn fake_text_provider_pipeline_falls_back_on_invalid_json() {
    let project = agent_test_project();
    let pipeline =
        ProviderAgentPipeline::new(FakeTextModelProvider::invalid_json(AgentRole::PlotDoctor));

    let plan = pipeline
        .plan_next_scene(provider_request(&project))
        .expect("fallback plan");

    assert!(plan.fallback_used);
    assert_eq!(
        plan.error.as_ref().expect("invalid json error").code,
        "text_provider_invalid_json"
    );
}

#[test]
fn world_generation_success_records_envelope_evidence() {
    let report = generate_world_expansion_with_provider(
        &FakeTextModelProvider::success(),
        world_generation_request(),
        41,
    );

    assert_eq!(report.evidence.status, GenerationStatus::Succeeded);
    assert!(!report.evidence.fallback_used);
    assert!(report.evidence.error.is_none());
    assert_eq!(report.evidence.envelopes.len(), 1);
    assert!(
        report
            .document
            .world_bible_markdown
            .contains("# World Bible")
    );
    assert!(
        report
            .document
            .canon_markdown
            .contains("Generated facts must not contradict forbidden facts")
    );
    assert_eq!(report.evidence.reproducibility.run_seed, 41);
    assert_eq!(
        report.evidence.reproducibility.provider_config_hash,
        "sha256:fake-text-provider-config-v1"
    );
    let encoded = serde_json::to_string(&report).expect("world report json");
    assert!(!encoded.contains("sk-test-secret-marker"));
    assert!(!encoded.contains("api_key"));
}

#[test]
fn story_craft_generation_success_builds_plot_threads_and_arc() {
    let report = generate_story_craft_with_provider(
        &FakeTextModelProvider::success(),
        story_craft_generation_request(),
        42,
    );

    assert_eq!(report.evidence.status, GenerationStatus::Succeeded);
    assert!(!report.evidence.fallback_used);
    assert!(report.evidence.error.is_none());
    assert_eq!(report.evidence.envelopes.len(), 1);
    assert!(
        report
            .document
            .story_bible_markdown
            .contains("# Story Bible")
    );
    assert!(
        report
            .document
            .style_guide_markdown
            .contains("# Style Guide")
    );
    assert!(report.document.story_craft.emotional_arc.len() >= 3);
    assert!(report.document.story_craft.plot_threads.len() >= 3);
    assert!(
        report
            .document
            .story_craft
            .bible
            .central_question
            .contains("preserve legitimacy")
    );
}

#[test]
fn character_generation_success_includes_visual_voice_and_portrait_request() {
    let report = generate_character_with_provider(
        &FakeTextModelProvider::success(),
        character_generation_request(),
        43,
    );

    assert_eq!(report.evidence.status, GenerationStatus::Succeeded);
    assert!(!report.evidence.fallback_used);
    assert!(report.evidence.error.is_none());
    assert_eq!(report.character.id, "generated-character");
    assert!(!report.character.visual_card.trim().is_empty());
    assert!(!report.character.voice_card.trim().is_empty());

    let portrait_request = report
        .character
        .portrait_request
        .as_ref()
        .expect("portrait request");
    assert_eq!(portrait_request.target_asset_slot, "portrait");
    assert!(portrait_request.fallback_allowed);
    assert!(portrait_request.prompt_hash.starts_with("sha256:"));
    assert_eq!(
        portrait_request.provider_config_hash,
        "sha256:fake-text-provider-config-v1"
    );
}

#[test]
fn generation_fallback_is_visible_and_redacted_on_invalid_schema() {
    let provider = FakeTextModelProvider::invalid_schema(AgentRole::CharacterDesigner);
    let report = generate_character_with_provider(&provider, character_generation_request(), 44);

    assert_eq!(report.evidence.status, GenerationStatus::Fallback);
    assert!(report.evidence.fallback_used);
    assert!(report.evidence.envelopes.is_empty());
    assert_eq!(report.character.id, "fallback-character");
    assert!(
        report
            .character
            .portrait_request
            .as_ref()
            .expect("fallback portrait request")
            .fallback_allowed
    );

    let error = report.evidence.error.as_ref().expect("fallback error");
    assert_eq!(error.code, "text_provider_schema_validation");
    assert!(error.message.contains("EmptyField"));
    let encoded = serde_json::to_string(&report).expect("character report json");
    assert!(!encoded.contains("sk-test-secret-marker"));
    assert!(!encoded.contains("api_key"));
}

#[test]
fn generation_fallback_redacts_secret_marker_output() {
    let provider = FakeTextModelProvider::secret_marker(AgentRole::StoryArchitect);
    let report = generate_world_expansion_with_provider(&provider, world_generation_request(), 45);

    assert_eq!(report.evidence.status, GenerationStatus::Fallback);
    assert!(report.evidence.fallback_used);
    assert!(report.evidence.envelopes.is_empty());
    assert_eq!(
        report.evidence.error.as_ref().expect("secret error").code,
        "text_provider_schema_validation"
    );

    let encoded = serde_json::to_string(&report).expect("world report json");
    assert!(encoded.contains("secret marker"));
    assert!(!encoded.contains("sk-test-secret-marker"));
    assert!(!encoded.contains("api_key"));
}

#[test]
fn generation_wrapped_json_is_repaired_for_story_craft() {
    let provider = FakeTextModelProvider::wrapped_json(AgentRole::StoryCraftPlanner);
    let report =
        generate_story_craft_with_provider(&provider, story_craft_generation_request(), 46);

    assert_eq!(report.evidence.status, GenerationStatus::Succeeded);
    assert!(!report.evidence.fallback_used);
    assert_eq!(report.evidence.envelopes.len(), 1);
    assert!(report.document.story_craft.plot_threads.len() >= 3);
}

#[test]
fn text_provider_config_hash_excludes_credentials() {
    let config = text_provider_config();
    let hash = config.provider_config_hash();

    assert!(hash.starts_with("sha256:"));
    assert_eq!(
        hash,
        config.reproducibility_metadata(7).provider_config_hash
    );
    assert!(!hash.contains("sk-test-secret-marker"));
    assert!(!hash.contains("api_key"));
    assert!(!hash.contains("secret_key"));
}

#[test]
fn configured_text_provider_uses_local_credential_without_persisting_it() {
    let project = agent_test_project();
    let client = RecordingTextModelClient::success();
    let client_probe = client.clone();
    let config = text_provider_config();
    let expected_hash = config.provider_config_hash();
    let pipeline = ProviderAgentPipeline::new(ConfiguredTextModelProvider::new(
        config,
        client,
        StaticCredentialResolver::token("sk-test-secret-marker"),
    ));

    let plan = pipeline
        .plan_next_scene(provider_request(&project))
        .expect("provider plan");

    assert_eq!(plan.scene.key, "provider-scene-001");
    assert_eq!(plan.reproducibility.model_version, "gpt-plotforge-test");
    assert_eq!(plan.reproducibility.provider_config_hash, expected_hash);
    assert!(
        client_probe
            .credentials()
            .iter()
            .all(|credential| credential == "sk-test-secret-marker")
    );
    let encoded = serde_json::to_string(&plan.reproducibility).expect("metadata json");
    assert!(!encoded.contains("sk-test-secret-marker"));
}

#[test]
fn configured_text_provider_disabled_falls_back_explicitly() {
    let project = agent_test_project();
    let pipeline = ProviderAgentPipeline::new(ConfiguredTextModelProvider::new(
        TextProviderConfig::disabled(),
        RecordingTextModelClient::success(),
        StaticCredentialResolver::token("sk-test-secret-marker"),
    ));

    let plan = pipeline
        .plan_next_scene(provider_request(&project))
        .expect("fallback plan");

    assert!(plan.fallback_used);
    let error = plan.error.as_ref().expect("disabled error");
    assert_eq!(error.code, "text_provider_disabled");
    assert!(error.message.contains("disabled"));
    assert!(!error.message.contains("sk-test-secret-marker"));
}

#[test]
fn configured_text_provider_missing_credential_falls_back_without_secret_leak() {
    let project = agent_test_project();
    let pipeline = ProviderAgentPipeline::new(ConfiguredTextModelProvider::new(
        text_provider_config(),
        RecordingTextModelClient::success(),
        StaticCredentialResolver::missing(),
    ));

    let plan = pipeline
        .plan_next_scene(provider_request(&project))
        .expect("fallback plan");

    assert!(plan.fallback_used);
    let error = plan.error.as_ref().expect("credential error");
    assert_eq!(error.code, "text_provider_missing_credential");
    assert!(error.message.contains("PLOTFORGE_TEXT_PROVIDER_TOKEN"));
    assert!(!error.message.contains("sk-test-secret-marker"));
}

#[test]
fn configured_text_provider_rejects_secret_player_input_before_client_call() {
    let project = agent_test_project();
    let client = RecordingTextModelClient::success();
    let client_probe = client.clone();
    let pipeline = ProviderAgentPipeline::new(ConfiguredTextModelProvider::new(
        text_provider_config(),
        client,
        StaticCredentialResolver::token("sk-test-secret-marker"),
    ));
    let request = ScenePlanRequest {
        project: &project,
        story_state: &project.story_state,
        world_state: &project.world_state,
        player_input: "Authorization: bearer token=value",
        action_type: "raise_tax",
    };

    let plan = pipeline.plan_next_scene(request).expect("fallback plan");

    assert!(plan.fallback_used);
    let error = plan.error.as_ref().expect("prompt validation error");
    assert_eq!(error.code, "text_provider_schema_validation");
    assert!(error.message.contains("secret marker"));
    assert!(!error.message.contains("Authorization"));
    assert!(!error.message.contains("token=value"));
    assert!(client_probe.credentials().is_empty());
    assert!(client_probe.prompts().is_empty());
}

#[test]
fn configured_text_provider_client_errors_are_redacted_and_trace_visible() {
    let project = agent_test_project();
    let pipeline = ProviderAgentPipeline::new(ConfiguredTextModelProvider::new(
        text_provider_config(),
        RecordingTextModelClient::provider_error(
            "upstream rejected request OPENAI_API_KEY=sk-test-secret-marker bearer token=value",
        ),
        StaticCredentialResolver::token("sk-test-secret-marker"),
    ));

    let plan = pipeline
        .plan_next_scene(provider_request(&project))
        .expect("fallback plan");

    assert!(plan.fallback_used);
    let error = plan.error.as_ref().expect("client error");
    assert_eq!(error.code, "text_client_error");
    assert!(error.message.contains(REDACTED_TRACE_SECRET));
    assert!(!error.message.contains("OPENAI_API_KEY"));
    assert!(!error.message.contains("sk-test-secret-marker"));
    assert!(!error.message.contains("token=value"));
}

#[test]
fn fake_image_provider_registers_generated_asset_and_successful_job() {
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(100));
    let pipeline = SceneImagePipeline::new(FakeImageProvider::success());

    let result = pipeline
        .generate_scene_background(scene_image_request(), &mut registry, &mut jobs)
        .expect("image generation");

    assert!(!result.fallback_used);
    assert!(result.error.is_none());
    assert_eq!(result.asset_record.source, AssetSourceKind::Generated);
    assert_eq!(
        result.asset_record.project_path,
        "assets/generated/scene-one.png"
    );
    assert_eq!(
        result
            .asset_record
            .provider_metadata
            .as_ref()
            .expect("metadata")
            .provider,
        "fake-image"
    );
    assert!(
        !result
            .asset_record
            .provider_metadata
            .as_ref()
            .expect("metadata")
            .fallback_used
    );
    assert_eq!(
        result
            .asset_record
            .references
            .iter()
            .filter(|reference| {
                reference.reference_kind == AssetReferenceKind::Scene
                    && reference.reference_id == "scene-one"
            })
            .count(),
        1
    );
    let job = result.job_record.expect("job");
    assert_eq!(job.status, JobStatus::Succeeded);
    assert_eq!(job.cost.spent_units, 1);
}

#[test]
fn image_provider_reports_spent_cost_units_to_usage_ledger() {
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(101));
    let mut ledger = UsageLedger::new(FakeClock::new(102));
    let pipeline = SceneImagePipeline::new(ReportingImageProvider);

    pipeline
        .generate_scene_background_with_usage_reporter(
            scene_image_request(),
            &mut registry,
            &mut jobs,
            Some(&mut ledger),
        )
        .expect("image generation reports usage");

    let summary = ledger.summary();
    assert_eq!(summary.total_spent_cost_units, 17);
    assert_eq!(summary.by_provider["real-image"].image_calls, 1);
}

#[test]
fn fake_image_provider_failure_registers_placeholder_and_failed_job() {
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(200));
    let pipeline = SceneImagePipeline::new(FakeImageProvider::provider_error());

    let result = pipeline
        .generate_scene_background(scene_image_request(), &mut registry, &mut jobs)
        .expect("placeholder fallback");

    assert!(result.fallback_used);
    assert_eq!(
        result.error.as_ref().expect("runtime error").code,
        "image_provider_error"
    );
    assert!(
        result
            .error
            .as_ref()
            .expect("runtime error")
            .message
            .contains(REDACTED_TRACE_SECRET)
    );
    assert_eq!(result.asset_record.source, AssetSourceKind::Placeholder);
    let metadata = result
        .asset_record
        .provider_metadata
        .as_ref()
        .expect("metadata");
    assert_eq!(metadata.provider, "plotforge-placeholder");
    assert!(metadata.fallback_used);
    let job = result.job_record.expect("job");
    assert_eq!(job.status, JobStatus::Failed);
    assert_eq!(
        job.failure.as_ref().expect("failure").code,
        "image_provider_error"
    );
    assert!(
        job.failure
            .as_ref()
            .expect("failure")
            .message
            .contains(REDACTED_TRACE_SECRET)
    );
}

#[test]
fn scene_image_pipeline_reuses_matching_cached_asset_without_new_job() {
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(300));
    let provider = FakeImageProvider::success();
    let pipeline = SceneImagePipeline::new(provider.clone());

    let first = pipeline
        .generate_scene_background(scene_image_request(), &mut registry, &mut jobs)
        .expect("first image");
    let second = pipeline
        .generate_scene_background(scene_image_request(), &mut registry, &mut jobs)
        .expect("cached image");

    assert_eq!(provider.call_count(), 1);
    assert_eq!(registry.len(), 1);
    assert_eq!(first.asset_record.id, second.asset_record.id);
    assert!(second.job_record.is_none());
}

#[test]
fn scene_image_pipeline_does_not_cache_placeholder_as_success() {
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(350));
    let failing_pipeline = SceneImagePipeline::new(FakeImageProvider::provider_error());
    let success_provider = FakeImageProvider::success();
    let success_pipeline = SceneImagePipeline::new(success_provider.clone());

    let fallback = failing_pipeline
        .generate_scene_background(scene_image_request(), &mut registry, &mut jobs)
        .expect("placeholder fallback");
    let generated = success_pipeline
        .generate_scene_background(scene_image_request(), &mut registry, &mut jobs)
        .expect("generated retry");

    assert_eq!(fallback.asset_record.source, AssetSourceKind::Placeholder);
    assert_eq!(generated.asset_record.source, AssetSourceKind::Generated);
    assert_eq!(success_provider.call_count(), 1);
    assert_eq!(jobs.records().count(), 2);
}

#[test]
fn fake_tts_provider_registers_scene_audio_and_successful_job() {
    let project = agent_test_project();
    let scene = &project.scenes[0];
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(360));
    let pipeline = TtsPipeline::new(FakeTtsProvider::success());

    let result = pipeline
        .synthesize(
            TtsRequest::scene_narration(scene, scene.hook.clone(), "calm narrator"),
            &mut registry,
            &mut jobs,
        )
        .expect("tts generation");

    assert!(!result.fallback_used);
    assert!(result.error.is_none());
    assert_eq!(result.asset_record.kind, AssetKind::Audio);
    assert_eq!(result.asset_record.source, AssetSourceKind::Generated);
    let metadata = result
        .asset_record
        .provider_metadata
        .as_ref()
        .expect("metadata");
    assert_eq!(metadata.provider, "fake-tts");
    assert!(!metadata.fallback_used);
    assert!(result.asset_record.references.iter().any(|reference| {
        reference.reference_kind == AssetReferenceKind::Scene
            && reference.reference_id == scene.key
            && reference.slot == "scene_audio"
    }));
    let job = result.job_record.expect("job");
    assert_eq!(job.kind, plotforge_schema::JobKind::TtsGeneration);
    assert_eq!(job.status, JobStatus::Succeeded);
    assert_eq!(job.cost.spent_units, 1);
}

#[test]
fn tts_provider_reports_spent_cost_units_to_usage_ledger() {
    let project = agent_test_project();
    let scene = &project.scenes[0];
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(361));
    let mut ledger = UsageLedger::new(FakeClock::new(362));
    let pipeline = TtsPipeline::new(ReportingTtsProvider);

    pipeline
        .synthesize_with_usage_reporter(
            TtsRequest::scene_narration(scene, scene.hook.clone(), "calm narrator"),
            &mut registry,
            &mut jobs,
            Some(&mut ledger),
        )
        .expect("TTS generation reports usage");

    let summary = ledger.summary();
    assert_eq!(summary.total_spent_cost_units, 23);
    assert_eq!(summary.by_provider["real-tts"].tts_calls, 1);
}

#[test]
fn fake_tts_provider_registers_character_voice_asset() {
    let character = plotforge_schema::Character {
        id: "test-speaker".into(),
        name: "Test Speaker".into(),
        role: "Fixture character".into(),
        traits: vec!["clear".into()],
        visual_card: "simple portrait".into(),
        voice_card: "measured".into(),
        portrait_request: None,
    };
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(365));
    let pipeline = TtsPipeline::new(FakeTtsProvider::success());

    let result = pipeline
        .synthesize(
            TtsRequest::character_voice(&character, "The treasury crisis has a price."),
            &mut registry,
            &mut jobs,
        )
        .expect("voice generation");

    assert_eq!(result.asset_record.kind, AssetKind::Voice);
    assert_eq!(result.asset_record.source, AssetSourceKind::Generated);
    assert!(result.asset_record.references.iter().any(|reference| {
        reference.reference_kind == AssetReferenceKind::Character
            && reference.reference_id == character.id
            && reference.slot == "voice"
    }));
    assert_eq!(
        result
            .asset_record
            .provider_metadata
            .as_ref()
            .expect("metadata")
            .provider,
        "fake-tts"
    );
    assert_eq!(result.job_record.expect("job").status, JobStatus::Succeeded);
}

#[test]
fn fake_tts_provider_failure_registers_silent_fallback_and_failed_job() {
    let project = agent_test_project();
    let scene = &project.scenes[0];
    let beat = &scene.beats[0];
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(370));
    let pipeline = TtsPipeline::new(FakeTtsProvider::provider_error());

    let result = pipeline
        .synthesize(
            TtsRequest::beat_narration(scene.key.clone(), beat, "civic narrator"),
            &mut registry,
            &mut jobs,
        )
        .expect("silent fallback");

    assert!(result.fallback_used);
    assert_eq!(
        result.error.as_ref().expect("runtime error").code,
        "tts_provider_error"
    );
    assert!(
        result
            .error
            .as_ref()
            .expect("runtime error")
            .message
            .contains(REDACTED_TRACE_SECRET)
    );
    assert_eq!(result.asset_record.kind, AssetKind::Audio);
    assert_eq!(result.asset_record.source, AssetSourceKind::Placeholder);
    let metadata = result
        .asset_record
        .provider_metadata
        .as_ref()
        .expect("metadata");
    assert_eq!(metadata.provider, "plotforge-silent-fallback");
    assert!(metadata.fallback_used);
    assert!(result.asset_record.references.iter().any(|reference| {
        reference.reference_kind == AssetReferenceKind::Scene
            && reference.reference_id == scene.key
            && reference.slot == format!("beat_audio:{}:narration", beat.id)
    }));
    let job = result.job_record.expect("job");
    assert_eq!(job.status, JobStatus::Failed);
    assert_eq!(
        job.failure.as_ref().expect("failure").code,
        "tts_provider_error"
    );
    assert!(
        job.failure
            .as_ref()
            .expect("failure")
            .message
            .contains(REDACTED_TRACE_SECRET)
    );
}

#[test]
fn tts_pipeline_for_project_writes_audio_asset_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("starter-project");
    create_starter_project(&project_path);
    let project = plotforge_storage::load_project(&project_path).expect("load project");
    let scene = &project.scenes[0];
    let beat = &scene.beats[0];
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(373));
    let pipeline = TtsPipeline::new(FakeTtsProvider::success());

    let result = pipeline
        .synthesize_for_project(
            &project_path,
            TtsRequest::beat_narration(scene.key.clone(), beat, "civic narrator"),
            &mut registry,
            &mut jobs,
        )
        .expect("project tts generation");

    let asset_path = project_path.join(&result.asset_record.project_path);
    let bytes = fs::read(&asset_path).expect("written audio bytes");
    assert!(asset_path.is_file());
    assert!(String::from_utf8_lossy(&bytes).contains("plotforge-fake-wav"));
    assert_eq!(result.asset_record.byte_length, bytes.len() as u64);
    assert_eq!(
        result.media_reference.project_path,
        result.asset_record.project_path
    );
    assert_eq!(result.media_reference.slot, "narration");
    assert_eq!(result.job_record.expect("job").status, JobStatus::Succeeded);
}

#[test]
fn tts_pipeline_reuses_matching_cached_asset_without_new_job() {
    let project = agent_test_project();
    let scene = &project.scenes[0];
    let beat = &scene.beats[0];
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(375));
    let provider = FakeTtsProvider::success();
    let pipeline = TtsPipeline::new(provider.clone());
    let request = TtsRequest::beat_narration(scene.key.clone(), beat, "civic narrator");

    let first = pipeline
        .synthesize(request.clone(), &mut registry, &mut jobs)
        .expect("first voice");
    let second = pipeline
        .synthesize(request, &mut registry, &mut jobs)
        .expect("cached voice");

    assert_eq!(provider.call_count(), 1);
    assert_eq!(registry.len(), 1);
    assert_eq!(first.asset_record.id, second.asset_record.id);
    assert!(second.job_record.is_none());
}

#[test]
fn tts_pipeline_does_not_cache_silent_fallback_as_success() {
    let project = agent_test_project();
    let scene = &project.scenes[0];
    let beat = &scene.beats[0];
    let mut registry = AssetRegistry::new();
    let mut jobs = JobQueue::new(FakeClock::new(380));
    let failing_pipeline = TtsPipeline::new(FakeTtsProvider::provider_error());
    let success_provider = FakeTtsProvider::success();
    let success_pipeline = TtsPipeline::new(success_provider.clone());
    let request = TtsRequest::beat_narration(scene.key.clone(), beat, "civic narrator");

    let fallback = failing_pipeline
        .synthesize(request.clone(), &mut registry, &mut jobs)
        .expect("silent fallback");
    let generated = success_pipeline
        .synthesize(request, &mut registry, &mut jobs)
        .expect("generated retry");

    assert_eq!(fallback.asset_record.source, AssetSourceKind::Placeholder);
    assert_eq!(generated.asset_record.source, AssetSourceKind::Generated);
    assert_eq!(success_provider.call_count(), 1);
    assert_eq!(jobs.records().count(), 2);
}

#[test]
fn image_provider_agent_pipeline_generates_scene_background_asset() {
    let project = agent_test_project();
    let planner = ImageProviderAgentPipeline::new(
        FakeTextModelProvider::success(),
        FakeImageProvider::success(),
        FakeClock::new(400),
    );

    let plan = planner
        .plan_next_scene(provider_request(&project))
        .expect("image-aware plan");

    assert_eq!(plan.scene.key, "provider-scene-001");
    assert_eq!(
        plan.scene.background_asset,
        "assets/generated/provider-scene-001.png"
    );
    assert!(!plan.fallback_used);
    assert!(plan.error.is_none());
    assert_eq!(planner.asset_records().len(), 1);
    assert_eq!(planner.job_records()[0].status, JobStatus::Succeeded);
}

#[test]
fn image_provider_agent_pipeline_marks_image_fallback_visible() {
    let project = agent_test_project();
    let planner = ImageProviderAgentPipeline::new(
        FakeTextModelProvider::success(),
        FakeImageProvider::timeout(),
        FakeClock::new(500),
    );

    let plan = planner
        .plan_next_scene(provider_request(&project))
        .expect("image fallback plan");

    assert_eq!(plan.scene.key, "provider-scene-001");
    assert!(plan.fallback_used);
    assert_eq!(
        plan.error.as_ref().expect("image error").code,
        "image_provider_timeout"
    );
    assert_eq!(
        planner.asset_records()[0].source,
        AssetSourceKind::Placeholder
    );
    assert_eq!(planner.job_records()[0].status, JobStatus::Failed);
}

fn sample_scene_plan_output_proposal() -> AgentOutputProposal {
    AgentOutputProposal {
        id: "scene-plan-proposal-001".into(),
        agent: AgentRole::ScenePlanner,
        output: AgentProposalPayload::ScenePlan(Box::new(sample_scene_plan_proposal())),
    }
}

fn sample_beat_drafts_output_proposal() -> AgentOutputProposal {
    AgentOutputProposal {
        id: "beat-drafts-proposal-001".into(),
        agent: AgentRole::BeatWriter,
        output: AgentProposalPayload::BeatDrafts(Box::new(sample_beat_drafts_proposal())),
    }
}

fn sample_review_output_proposal() -> AgentOutputProposal {
    AgentOutputProposal {
        id: "review-proposal-001".into(),
        agent: AgentRole::PlotDoctor,
        output: AgentProposalPayload::Review(Box::new(sample_review_proposal())),
    }
}

fn sample_scene_plan_proposal() -> ScenePlanProposal {
    ScenePlanProposal {
        scene_key: "civic-crisis-002".into(),
        title: "Tax Resistance Memorials".into(),
        location: "Civic Hall".into(),
        scene_summary: "The levy creates immediate provincial resistance.".into(),
        dramatic_purpose: "Show the cost of emergency revenue.".into(),
        hook: "Three memorials arrive with broken tax seals.".into(),
        emotional_goal: Some("consequence".into()),
        cast: vec!["city-treasurer".into()],
        entry_beat_id: "civic-crisis-002-beat-001".into(),
        background_asset: Some("assets/generated/civic-crisis-002.png".into()),
    }
}

fn sample_beat_drafts_proposal() -> BeatDraftsProposal {
    BeatDraftsProposal {
        scene_key: "civic-crisis-002".into(),
        beats: vec![sample_beat_draft_proposal()],
    }
}

fn sample_beat_draft_proposal() -> BeatDraftProposal {
    BeatDraftProposal {
        id: "civic-crisis-002-beat-001".into(),
        scene_key: "civic-crisis-002".into(),
        text: "The council reads three provincial reports in silence.".into(),
        choices: vec![Choice {
            id: "inspect-corruption".into(),
            label: "Investigate the collectors".into(),
            action_type: "inspect_corruption".into(),
            input_terms: vec!["inspect".into(), "corruption".into()],
            dramatic_purpose: "Trade civic stability for cleaner revenue.".into(),
            change_scene: true,
        }],
        narrative_function: NarrativeFunction::Hook,
    }
}

fn sample_review_proposal() -> ReviewProposal {
    ReviewProposal {
        scene_key: "civic-crisis-002".into(),
        review: NarrativeReview {
            scene_key: "civic-crisis-002".into(),
            score: 95,
            hook_score: 95,
            pacing_score: 95,
            character_consistency_score: 100,
            payoff_score: 90,
            choice_meaningfulness_score: 95,
            ai_slop_risk: 5,
            issues: Vec::new(),
        },
        notes: vec![plotforge_schema::NarrativeReviewNote {
            id: "proposal-review-note".into(),
            scene_key: Some("civic-crisis-002".into()),
            severity: Severity::Info,
            message: "Proposal advances tax disorder visibly.".into(),
            resolved: true,
        }],
    }
}

fn provider_request<'a>(project: &'a plotforge_schema::ProjectData) -> ScenePlanRequest<'a> {
    ScenePlanRequest {
        project,
        story_state: &project.story_state,
        world_state: &project.world_state,
        player_input: "Raise the levy",
        action_type: "raise_tax",
    }
}

fn text_provider_config() -> TextProviderConfig {
    TextProviderConfig::openai_compatible(
        "openai-compatible",
        "gpt-plotforge-test",
        "https://provider.example.test/v1/chat/completions",
        "PLOTFORGE_TEXT_PROVIDER_TOKEN",
    )
}

/// Regression for the Low finding (L4): an `endpoint_url` carrying a
/// credential in its query string must be rejected by
/// `TextProviderConfig::validate` so it is never persisted to the registry
/// or used as the request URL. The secret-marker scanner only flags `sk-`
/// as a token-prefix, so a `?key=sk-realkey` query string can slip through
/// unless `validate` checks the query string explicitly.
#[test]
fn text_provider_config_rejects_credential_in_endpoint_query_string() {
    let mut config = TextProviderConfig::openai_compatible(
        "openai",
        "gpt-test",
        "https://host/v1?key=sk-realkey",
        "OPENAI_API_KEY",
    );
    let error = config
        .validate()
        .expect_err("query-string credential rejected");
    let _ = error;
    // A plain URL with no credential query param must still validate.
    config.endpoint_url = Some("https://host/v1".into());
    config.validate().expect("plain endpoint validates");
}

/// `url_has_query_credential` must catch the common credential-bearing query
/// keys and ignore non-credential query params.
#[test]
fn url_has_query_credential_detects_credential_keys() {
    use crate::providers_text::url_has_query_credential;
    assert!(url_has_query_credential("https://host/v1?key=sk-x"));
    assert!(url_has_query_credential("https://host/v1?api_key=x"));
    assert!(url_has_query_credential("https://host/v1?access_token=t"));
    assert!(url_has_query_credential("https://host/v1?a=1&token=secret"));
    // Non-credential query params are allowed.
    assert!(!url_has_query_credential("https://host/v1?model=gpt"));
    assert!(!url_has_query_credential("https://host/v1"));
}

fn scene_image_request() -> SceneImageRequest {
    SceneImageRequest {
        scene_key: "scene-one".into(),
        prompt: "paint a tense civic hearing".into(),
        output_path: "assets/generated/scene-one.png".into(),
    }
}

fn world_generation_request() -> WorldGenerationRequest {
    WorldGenerationRequest {
        expansion_goal: "Expand the civic crisis into factions, canon, and forbidden facts.".into(),
        document: WorldEditDocument {
            world_bible_markdown: "# Existing World\n\nA civic crisis strains the treasury.".into(),
            canon_markdown: "# Existing Canon\n\nVisible choices must have consequences.".into(),
            forbidden_facts: vec!["Do not solve the famine with prophecy.".into()],
        },
    }
}

fn story_craft_generation_request() -> StoryCraftGenerationRequest {
    StoryCraftGenerationRequest {
        concept: "A ruler must survive an escalating fiscal and legitimacy crisis.".into(),
        world_bible_markdown: "# World Bible\n\nThe city council is divided by emergency revenue."
            .into(),
        canon_markdown: "# Canon Rules\n\nNo crisis solution is free.".into(),
        forbidden_facts: vec!["No hidden prophecy rescue.".into()],
        document: StoryCraftEditDocument {
            story_bible_markdown: "# Draft Story Bible\n\nInitial premise only.".into(),
            style_guide_markdown: "# Draft Style\n\nGrounded and specific.".into(),
            story_craft: plotforge_schema::StoryCraftState::default(),
        },
        characters: Vec::new(),
    }
}

fn character_generation_request() -> CharacterGenerationRequest {
    CharacterGenerationRequest {
        concept: "A diplomatic envoy pressures the council with concrete tradeoffs.".into(),
        role_hint: "Envoy".into(),
        world_bible_markdown: "# World Bible\n\nFactions trade legitimacy for resources.".into(),
        story_bible_markdown: "# Story Bible\n\nEvery ally has a visible cost.".into(),
        existing_characters: Vec::new(),
    }
}
