use std::{cell::RefCell, collections::BTreeMap, env, path::Path};

use plotforge_job::{JobClock, JobQueue, JobQueueError, JobRequest};
use plotforge_media::{AssetRecordInput, AssetRegistry, MediaError};
use plotforge_schema::{
    AgentOutputEnvelope, AgentOutputProposal, AgentProposalPayload, AgentRole, AssetKind,
    AssetProviderMetadata, AssetRecord, AssetReference, AssetReferenceKind, AssetSourceKind, Beat,
    BeatDraftProposal, BeatDraftsProposal, BeatNext, CONTRACT_SCHEMA_VERSION, CONTRACT_VERSION,
    Character, CharacterGenerationReport, CharacterGenerationRequest, CharacterPortraitRequest,
    CharacterProposal, Choice, EmotionalArcPoint, GenerationEvidence, GenerationStatus, JobFailure,
    JobKind, JobRecord, MediaAssetReference, NarrativeFunction, NarrativeReview, PlotThread,
    PlotThreadStatus, PlotThreadType, ProjectData, ReproducibilityMetadata, ReviewProposal,
    RuntimeError, Scene, ScenePlanProposal, StoryCraftGenerationReport,
    StoryCraftGenerationRequest, StoryCraftPlanProposal, StoryState, WorldEditDocument,
    WorldExpansionProposal, WorldGenerationReport, WorldGenerationRequest, WorldState,
    contains_secret_marker_text, redact_trace_text,
};
use plotforge_storycraft::review_scene;
use sha2::{Digest, Sha256};

const IMAGE_JOB_TIMEOUT_MS: u64 = 60_000;
const IMAGE_JOB_MAX_ATTEMPTS: u32 = 2;
const IMAGE_JOB_ESTIMATED_COST_UNITS: u64 = 1;
const TTS_JOB_TIMEOUT_MS: u64 = 30_000;
const TTS_JOB_MAX_ATTEMPTS: u32 = 2;
const TTS_JOB_ESTIMATED_COST_UNITS: u64 = 1;
const SCENE_BACKGROUND_SLOT: &str = "background_asset";
const TEXT_PROMPT_VERSION: &str = "plotforge-agent-text-prompt-v1";
const FAKE_TEXT_MODEL_VERSION: &str = "fake-text-model-v1";
const FAKE_TEXT_PROVIDER_CONFIG_HASH: &str = "sha256:fake-text-provider-config-v1";

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentProposalValidationError {
    MissingBeatDrafts,
    EmptyField(&'static str),
    AgentPayloadMismatch {
        agent: AgentRole,
        payload_kind: &'static str,
    },
    DuplicateBeatId(String),
    EntryBeatMissing(String),
    BeatDraftsSceneMismatch {
        expected_scene_key: String,
        actual_scene_key: String,
    },
    BeatSceneMismatch {
        beat_id: String,
        expected_scene_key: String,
        actual_scene_key: String,
    },
    ReviewSceneMismatch {
        expected_scene_key: String,
        actual_scene_key: String,
    },
}

pub fn validate_agent_output_proposal(
    proposal: &AgentOutputProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&proposal.id, "proposal.id")?;

    match (&proposal.agent, &proposal.output) {
        (AgentRole::StoryArchitect, AgentProposalPayload::WorldExpansion(world_expansion)) => {
            validate_world_expansion_proposal(world_expansion)
        }
        (AgentRole::StoryCraftPlanner, AgentProposalPayload::StoryCraftPlan(story_craft_plan)) => {
            validate_story_craft_plan_proposal(story_craft_plan)
        }
        (AgentRole::CharacterDesigner, AgentProposalPayload::CharacterProfile(character)) => {
            validate_character_proposal(character)
        }
        (AgentRole::ScenePlanner, AgentProposalPayload::ScenePlan(scene_plan)) => {
            validate_scene_plan_proposal(scene_plan)
        }
        (AgentRole::BeatWriter, AgentProposalPayload::BeatDrafts(beat_drafts)) => {
            validate_beat_drafts_proposal(beat_drafts)
        }
        (
            AgentRole::PlotDoctor | AgentRole::ConsistencyChecker,
            AgentProposalPayload::Review(review),
        ) => validate_review_proposal(review),
        (agent, output) => Err(AgentProposalValidationError::AgentPayloadMismatch {
            agent: agent.clone(),
            payload_kind: payload_kind(output),
        }),
    }
}

pub fn validate_world_expansion_proposal(
    world_expansion: &WorldExpansionProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(
        &world_expansion.world_bible_markdown,
        "world_expansion.world_bible_markdown",
    )?;
    require_non_empty(
        &world_expansion.canon_markdown,
        "world_expansion.canon_markdown",
    )?;

    Ok(())
}

pub fn validate_story_craft_plan_proposal(
    story_craft_plan: &StoryCraftPlanProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(
        &story_craft_plan.story_bible_markdown,
        "story_craft_plan.story_bible_markdown",
    )?;
    require_non_empty(
        &story_craft_plan.style_guide_markdown,
        "story_craft_plan.style_guide_markdown",
    )?;
    require_non_empty(
        &story_craft_plan.story_craft.bible.genre_promise,
        "story_craft_plan.story_craft.bible.genre_promise",
    )?;
    require_non_empty(
        &story_craft_plan.story_craft.bible.central_question,
        "story_craft_plan.story_craft.bible.central_question",
    )?;

    Ok(())
}

pub fn validate_character_proposal(
    character: &CharacterProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&character.character.id, "character.id")?;
    require_non_empty(&character.character.name, "character.name")?;
    require_non_empty(&character.character.role, "character.role")?;
    require_non_empty(&character.character.visual_card, "character.visual_card")?;
    require_non_empty(&character.character.voice_card, "character.voice_card")?;

    Ok(())
}

pub fn validate_scene_plan_proposal(
    scene_plan: &ScenePlanProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&scene_plan.scene_key, "scene_plan.scene_key")?;
    require_non_empty(&scene_plan.title, "scene_plan.title")?;
    require_non_empty(&scene_plan.location, "scene_plan.location")?;
    require_non_empty(&scene_plan.scene_summary, "scene_plan.scene_summary")?;
    require_non_empty(&scene_plan.dramatic_purpose, "scene_plan.dramatic_purpose")?;
    require_non_empty(&scene_plan.hook, "scene_plan.hook")?;
    require_non_empty(&scene_plan.entry_beat_id, "scene_plan.entry_beat_id")?;

    Ok(())
}

pub fn validate_beat_drafts_proposal(
    beat_drafts: &BeatDraftsProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&beat_drafts.scene_key, "beat_drafts.scene_key")?;
    if beat_drafts.beats.is_empty() {
        return Err(AgentProposalValidationError::MissingBeatDrafts);
    }

    let mut beat_ids = std::collections::BTreeSet::new();
    for beat in &beat_drafts.beats {
        validate_beat_draft_proposal(beat, &beat_drafts.scene_key)?;
        if !beat_ids.insert(beat.id.as_str()) {
            return Err(AgentProposalValidationError::DuplicateBeatId(
                beat.id.clone(),
            ));
        }
    }

    Ok(())
}

pub fn validate_review_proposal(
    review: &ReviewProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&review.scene_key, "review.scene_key")?;
    require_non_empty(&review.review.scene_key, "review.review.scene_key")?;
    if review.review.scene_key != review.scene_key {
        return Err(AgentProposalValidationError::ReviewSceneMismatch {
            expected_scene_key: review.scene_key.clone(),
            actual_scene_key: review.review.scene_key.clone(),
        });
    }

    Ok(())
}

pub fn validate_scene_proposal_parts(
    scene_plan: &ScenePlanProposal,
    beat_drafts: &BeatDraftsProposal,
    review: Option<&ReviewProposal>,
) -> Result<(), AgentProposalValidationError> {
    validate_scene_plan_proposal(scene_plan)?;
    validate_beat_drafts_proposal(beat_drafts)?;

    if beat_drafts.scene_key != scene_plan.scene_key {
        return Err(AgentProposalValidationError::BeatDraftsSceneMismatch {
            expected_scene_key: scene_plan.scene_key.clone(),
            actual_scene_key: beat_drafts.scene_key.clone(),
        });
    }

    let beat_ids = beat_drafts
        .beats
        .iter()
        .map(|beat| beat.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if !beat_ids.contains(scene_plan.entry_beat_id.as_str()) {
        return Err(AgentProposalValidationError::EntryBeatMissing(
            scene_plan.entry_beat_id.clone(),
        ));
    }

    if let Some(review) = review {
        validate_review_proposal(review)?;
        if review.scene_key != scene_plan.scene_key {
            return Err(AgentProposalValidationError::ReviewSceneMismatch {
                expected_scene_key: scene_plan.scene_key.clone(),
                actual_scene_key: review.scene_key.clone(),
            });
        }
    }

    Ok(())
}

pub fn scene_from_proposals(
    scene_plan: &ScenePlanProposal,
    beat_drafts: &BeatDraftsProposal,
    review: Option<&ReviewProposal>,
) -> Result<Scene, AgentProposalValidationError> {
    validate_scene_proposal_parts(scene_plan, beat_drafts, review)?;
    Ok(Scene {
        key: scene_plan.scene_key.clone(),
        title: scene_plan.title.clone(),
        location: scene_plan.location.clone(),
        dramatic_purpose: scene_plan.dramatic_purpose.clone(),
        hook: scene_plan.hook.clone(),
        background_asset: scene_plan
            .background_asset
            .clone()
            .unwrap_or_else(|| format!("assets/generated/{}.png", scene_plan.scene_key)),
        audio_refs: Vec::new(),
        character_ids: scene_plan.cast.clone(),
        plot_thread_updates: BTreeMap::new(),
        entry_beat_id: Some(scene_plan.entry_beat_id.clone()),
        beats: beat_drafts
            .beats
            .iter()
            .enumerate()
            .map(|(index, beat)| Beat {
                id: beat.id.clone(),
                text: beat.text.clone(),
                speaker: None,
                line_delivery: None,
                audio_refs: Vec::new(),
                choices: beat.choices.clone(),
                next: beat_drafts
                    .beats
                    .get(index + 1)
                    .map(|next| BeatNext::Beat(next.id.clone()))
                    .unwrap_or(BeatNext::Scene),
            })
            .collect(),
    })
}

fn validate_beat_draft_proposal(
    beat: &BeatDraftProposal,
    expected_scene_key: &str,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&beat.id, "beat_drafts.id")?;
    require_non_empty(&beat.scene_key, "beat_drafts.scene_key")?;
    require_non_empty(&beat.text, "beat_drafts.text")?;
    if beat.scene_key != expected_scene_key {
        return Err(AgentProposalValidationError::BeatSceneMismatch {
            beat_id: beat.id.clone(),
            expected_scene_key: expected_scene_key.to_string(),
            actual_scene_key: beat.scene_key.clone(),
        });
    }
    if beat.choices.is_empty() {
        return Err(AgentProposalValidationError::EmptyField(
            "beat_drafts.choices",
        ));
    }
    for choice in &beat.choices {
        require_non_empty(&choice.id, "choice.id")?;
        require_non_empty(&choice.label, "choice.label")?;
        require_non_empty(&choice.action_type, "choice.action_type")?;
        require_non_empty(&choice.dramatic_purpose, "choice.dramatic_purpose")?;
    }

    Ok(())
}

fn payload_kind(output: &AgentProposalPayload) -> &'static str {
    match output {
        AgentProposalPayload::WorldExpansion(_) => "world_expansion",
        AgentProposalPayload::StoryCraftPlan(_) => "story_craft_plan",
        AgentProposalPayload::CharacterProfile(_) => "character_profile",
        AgentProposalPayload::ScenePlan(_) => "scene_plan",
        AgentProposalPayload::BeatDrafts(_) => "beat_drafts",
        AgentProposalPayload::Review(_) => "review",
    }
}

fn require_non_empty(value: &str, field: &'static str) -> Result<(), AgentProposalValidationError> {
    if value.trim().is_empty() {
        Err(AgentProposalValidationError::EmptyField(field))
    } else {
        Ok(())
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextModelRequest {
    pub call_id: String,
    pub agent: AgentRole,
    pub scene_key: String,
    pub run_seed: u64,
    pub prompt_version: String,
    pub model_version: String,
    pub provider_config_hash: String,
    pub prompt: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextModelResponse {
    pub raw_json: String,
}

impl TextModelResponse {
    pub fn json(raw_json: impl Into<String>) -> Self {
        Self {
            raw_json: raw_json.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextModelProviderErrorKind {
    Provider,
    Timeout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextModelProviderError {
    pub kind: TextModelProviderErrorKind,
    pub code: String,
    pub message: String,
}

impl TextModelProviderError {
    pub fn provider(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: TextModelProviderErrorKind::Provider,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            kind: TextModelProviderErrorKind::Timeout,
            code: "text_provider_timeout".into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TextModelProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for TextModelProviderError {}

pub trait TextModelProvider {
    fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
        ReproducibilityMetadata::local_mock(run_seed)
    }

    fn complete(
        &self,
        request: &TextModelRequest,
    ) -> Result<TextModelResponse, TextModelProviderError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextProviderConfig {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub endpoint_url: Option<String>,
    pub credential_env_var: String,
}

impl TextProviderConfig {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            provider: "disabled".into(),
            model: "disabled".into(),
            endpoint_url: None,
            credential_env_var: "PLOTFORGE_TEXT_PROVIDER_TOKEN".into(),
        }
    }

    pub fn openai_compatible(
        provider: impl Into<String>,
        model: impl Into<String>,
        endpoint_url: impl Into<String>,
        credential_env_var: impl Into<String>,
    ) -> Self {
        Self {
            enabled: true,
            provider: provider.into(),
            model: model.into(),
            endpoint_url: Some(endpoint_url.into()),
            credential_env_var: credential_env_var.into(),
        }
    }

    pub fn validate(&self) -> Result<(), TextProviderConfigError> {
        require_safe_config_field("provider", &self.provider)?;
        require_safe_config_field("model", &self.model)?;
        if let Some(endpoint_url) = self.endpoint_url.as_ref() {
            require_safe_config_field("endpoint_url", endpoint_url)?;
            if !endpoint_url.starts_with("https://") && !endpoint_url.starts_with("http://") {
                return Err(TextProviderConfigError::InvalidField {
                    field: "endpoint_url",
                    reason: "must start with http:// or https://".into(),
                });
            }
        }
        if self.credential_env_var.trim().is_empty() {
            return Err(TextProviderConfigError::InvalidField {
                field: "credential_env_var",
                reason: "must not be empty".into(),
            });
        }
        if !self
            .credential_env_var
            .chars()
            .all(|value| value.is_ascii_uppercase() || value.is_ascii_digit() || value == '_')
        {
            return Err(TextProviderConfigError::InvalidField {
                field: "credential_env_var",
                reason: "must contain only uppercase ASCII letters, digits, or underscores".into(),
            });
        }

        Ok(())
    }

    pub fn provider_config_hash(&self) -> String {
        let endpoint_url = self.endpoint_url.as_deref().unwrap_or("");
        format!(
            "sha256:{}",
            stable_sha256_hash(&format!(
                "enabled={}\nprovider={}\nmodel={}\nendpoint_url={}\ncredential_env_var={}\n",
                self.enabled, self.provider, self.model, endpoint_url, self.credential_env_var
            ))
        )
    }

    pub fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
        ReproducibilityMetadata {
            run_seed,
            prompt_version: TEXT_PROMPT_VERSION.into(),
            model_version: self.model.clone(),
            provider_config_hash: self.provider_config_hash(),
            trace_id: None,
            snapshot_id: None,
        }
    }
}

fn require_safe_config_field(
    field: &'static str,
    value: &str,
) -> Result<(), TextProviderConfigError> {
    if value.trim().is_empty() {
        return Err(TextProviderConfigError::InvalidField {
            field,
            reason: "must not be empty".into(),
        });
    }
    if contains_secret_marker_text(value) {
        return Err(TextProviderConfigError::InvalidField {
            field,
            reason: "must not contain secret markers".into(),
        });
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextProviderConfigError {
    InvalidField { field: &'static str, reason: String },
}

impl std::fmt::Display for TextProviderConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidField { field, reason } => {
                write!(
                    formatter,
                    "invalid text provider config field {field}: {reason}"
                )
            }
        }
    }
}

fn redact_text_provider_error(error: TextModelProviderError) -> TextModelProviderError {
    TextModelProviderError {
        kind: error.kind,
        code: redact_trace_text(&error.code),
        message: redact_trace_text(&error.message),
    }
}

impl std::error::Error for TextProviderConfigError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderCredentialError {
    Missing { env_var: String },
    Empty { env_var: String },
}

impl std::fmt::Display for ProviderCredentialError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { env_var } => {
                write!(
                    formatter,
                    "missing provider credential in env var `{env_var}`"
                )
            }
            Self::Empty { env_var } => {
                write!(
                    formatter,
                    "provider credential env var `{env_var}` is empty"
                )
            }
        }
    }
}

impl std::error::Error for ProviderCredentialError {}

pub trait ProviderCredentialResolver {
    fn resolve(&self, env_var: &str) -> Result<String, ProviderCredentialError>;
}

#[derive(Clone, Debug, Default)]
pub struct EnvCredentialResolver;

impl ProviderCredentialResolver for EnvCredentialResolver {
    fn resolve(&self, env_var: &str) -> Result<String, ProviderCredentialError> {
        match env::var(env_var) {
            Ok(value) if !value.trim().is_empty() => Ok(value),
            Ok(_) => Err(ProviderCredentialError::Empty {
                env_var: env_var.into(),
            }),
            Err(_) => Err(ProviderCredentialError::Missing {
                env_var: env_var.into(),
            }),
        }
    }
}

pub struct TextModelClientRequest<'a> {
    pub config: &'a TextProviderConfig,
    pub credential: &'a str,
    pub request: &'a TextModelRequest,
}

pub trait TextModelClient {
    fn complete(
        &self,
        request: TextModelClientRequest<'_>,
    ) -> Result<TextModelResponse, TextModelProviderError>;
}

#[derive(Clone, Debug)]
pub struct ConfiguredTextModelProvider<C, R> {
    config: TextProviderConfig,
    client: C,
    credential_resolver: R,
}

impl<C, R> ConfiguredTextModelProvider<C, R> {
    pub fn new(config: TextProviderConfig, client: C, credential_resolver: R) -> Self {
        Self {
            config,
            client,
            credential_resolver,
        }
    }

    pub fn config(&self) -> &TextProviderConfig {
        &self.config
    }
}

impl<C, R> TextModelProvider for ConfiguredTextModelProvider<C, R>
where
    C: TextModelClient,
    R: ProviderCredentialResolver,
{
    fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
        self.config.reproducibility_metadata(run_seed)
    }

    fn complete(
        &self,
        request: &TextModelRequest,
    ) -> Result<TextModelResponse, TextModelProviderError> {
        self.config.validate().map_err(|error| {
            TextModelProviderError::provider("text_provider_config", error.to_string())
        })?;
        if !self.config.enabled {
            return Err(TextModelProviderError::provider(
                "text_provider_disabled",
                "text provider is disabled by local configuration",
            ));
        }
        if contains_secret_marker_text(&request.prompt) {
            return Err(TextModelProviderError::provider(
                "text_provider_prompt_secret",
                "text model prompt contained a secret marker",
            ));
        }
        let credential = self
            .credential_resolver
            .resolve(&self.config.credential_env_var)
            .map_err(|error| {
                TextModelProviderError::provider(
                    "text_provider_missing_credential",
                    error.to_string(),
                )
            })?;
        let response = self
            .client
            .complete(TextModelClientRequest {
                config: &self.config,
                credential: &credential,
                request,
            })
            .map_err(redact_text_provider_error)?;

        Ok(response)
    }
}

pub fn generate_world_expansion(
    request: WorldGenerationRequest,
    run_seed: u64,
) -> WorldGenerationReport {
    let provider = FakeTextModelProvider::success();
    generate_world_expansion_with_provider(&provider, request, run_seed)
}

pub fn generate_world_expansion_with_provider<P>(
    provider: &P,
    request: WorldGenerationRequest,
    run_seed: u64,
) -> WorldGenerationReport
where
    P: TextModelProvider,
{
    match complete_text_agent_output(
        provider,
        AgentRole::StoryArchitect,
        run_seed,
        "world-expansion".into(),
        "world-generation".into(),
        world_generation_prompt(&request),
    ) {
        Ok(envelope) => match envelope.proposal.output.clone() {
            AgentProposalPayload::WorldExpansion(proposal) => {
                let proposal = *proposal;
                WorldGenerationReport {
                    document: WorldEditDocument {
                        world_bible_markdown: proposal.world_bible_markdown,
                        canon_markdown: proposal.canon_markdown,
                        forbidden_facts: proposal.forbidden_facts,
                    },
                    evidence: generation_evidence(
                        GenerationStatus::Succeeded,
                        false,
                        None,
                        envelope.reproducibility.clone(),
                        vec![envelope],
                    ),
                }
            }
            output => generation_world_fallback(
                request.document,
                run_seed,
                ProviderPipelineError::Validation {
                    agent: AgentRole::StoryArchitect,
                    message: format!("unexpected payload `{}`", payload_kind(&output)),
                },
            ),
        },
        Err(error) => generation_world_fallback(request.document, run_seed, error),
    }
}

pub fn generate_story_craft(
    request: StoryCraftGenerationRequest,
    run_seed: u64,
) -> StoryCraftGenerationReport {
    let provider = FakeTextModelProvider::success();
    generate_story_craft_with_provider(&provider, request, run_seed)
}

pub fn generate_story_craft_with_provider<P>(
    provider: &P,
    request: StoryCraftGenerationRequest,
    run_seed: u64,
) -> StoryCraftGenerationReport
where
    P: TextModelProvider,
{
    match complete_text_agent_output(
        provider,
        AgentRole::StoryCraftPlanner,
        run_seed,
        "story-craft-generation".into(),
        "story-craft-generation".into(),
        story_craft_generation_prompt(&request),
    ) {
        Ok(envelope) => match envelope.proposal.output.clone() {
            AgentProposalPayload::StoryCraftPlan(proposal) => {
                let proposal = *proposal;
                StoryCraftGenerationReport {
                    document: plotforge_schema::StoryCraftEditDocument {
                        story_bible_markdown: proposal.story_bible_markdown,
                        style_guide_markdown: proposal.style_guide_markdown,
                        story_craft: proposal.story_craft,
                    },
                    evidence: generation_evidence(
                        GenerationStatus::Succeeded,
                        false,
                        None,
                        envelope.reproducibility.clone(),
                        vec![envelope],
                    ),
                }
            }
            output => generation_story_craft_fallback(
                request.document,
                run_seed,
                ProviderPipelineError::Validation {
                    agent: AgentRole::StoryCraftPlanner,
                    message: format!("unexpected payload `{}`", payload_kind(&output)),
                },
            ),
        },
        Err(error) => generation_story_craft_fallback(request.document, run_seed, error),
    }
}

pub fn generate_character(
    request: CharacterGenerationRequest,
    run_seed: u64,
) -> CharacterGenerationReport {
    let provider = FakeTextModelProvider::success();
    generate_character_with_provider(&provider, request, run_seed)
}

pub fn generate_character_with_provider<P>(
    provider: &P,
    request: CharacterGenerationRequest,
    run_seed: u64,
) -> CharacterGenerationReport
where
    P: TextModelProvider,
{
    match complete_text_agent_output(
        provider,
        AgentRole::CharacterDesigner,
        run_seed,
        "character-generation".into(),
        "character-generation".into(),
        character_generation_prompt(&request),
    ) {
        Ok(envelope) => match envelope.proposal.output.clone() {
            AgentProposalPayload::CharacterProfile(proposal) => {
                let proposal = *proposal;
                CharacterGenerationReport {
                    character: proposal.character,
                    evidence: generation_evidence(
                        GenerationStatus::Succeeded,
                        false,
                        None,
                        envelope.reproducibility.clone(),
                        vec![envelope],
                    ),
                }
            }
            output => generation_character_fallback(
                request,
                run_seed,
                ProviderPipelineError::Validation {
                    agent: AgentRole::CharacterDesigner,
                    message: format!("unexpected payload `{}`", payload_kind(&output)),
                },
            ),
        },
        Err(error) => generation_character_fallback(request, run_seed, error),
    }
}

fn complete_text_agent_output<P>(
    provider: &P,
    agent: AgentRole,
    run_seed: u64,
    call_id: String,
    scene_key: String,
    prompt: String,
) -> Result<AgentOutputEnvelope, ProviderPipelineError>
where
    P: TextModelProvider,
{
    if contains_secret_marker_text(&prompt) {
        return Err(ProviderPipelineError::Validation {
            agent,
            message: "text generation prompt contained a secret marker".into(),
        });
    }

    let reproducibility = provider.reproducibility_metadata(run_seed);
    let model_request = TextModelRequest {
        call_id,
        agent: agent.clone(),
        scene_key,
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
    let envelope =
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
    ensure_matching_reproducibility(&reproducibility, &envelope.reproducibility, agent.clone())?;
    validate_agent_output_proposal(&envelope.proposal).map_err(|error| {
        ProviderPipelineError::Validation {
            agent,
            message: format!("{error:?}"),
        }
    })?;

    Ok(envelope)
}

fn generation_world_fallback(
    document: WorldEditDocument,
    run_seed: u64,
    error: ProviderPipelineError,
) -> WorldGenerationReport {
    let runtime_error = error.into_runtime_error();
    WorldGenerationReport {
        document,
        evidence: generation_evidence(
            GenerationStatus::Fallback,
            true,
            Some(runtime_error),
            ReproducibilityMetadata::local_mock(run_seed),
            Vec::new(),
        ),
    }
}

fn generation_story_craft_fallback(
    document: plotforge_schema::StoryCraftEditDocument,
    run_seed: u64,
    error: ProviderPipelineError,
) -> StoryCraftGenerationReport {
    let runtime_error = error.into_runtime_error();
    StoryCraftGenerationReport {
        document,
        evidence: generation_evidence(
            GenerationStatus::Fallback,
            true,
            Some(runtime_error),
            ReproducibilityMetadata::local_mock(run_seed),
            Vec::new(),
        ),
    }
}

fn generation_character_fallback(
    request: CharacterGenerationRequest,
    run_seed: u64,
    error: ProviderPipelineError,
) -> CharacterGenerationReport {
    let runtime_error = error.into_runtime_error();
    CharacterGenerationReport {
        character: fallback_character(&request, run_seed),
        evidence: generation_evidence(
            GenerationStatus::Fallback,
            true,
            Some(runtime_error),
            ReproducibilityMetadata::local_mock(run_seed),
            Vec::new(),
        ),
    }
}

fn generation_evidence(
    status: GenerationStatus,
    fallback_used: bool,
    error: Option<RuntimeError>,
    reproducibility: ReproducibilityMetadata,
    envelopes: Vec<AgentOutputEnvelope>,
) -> GenerationEvidence {
    GenerationEvidence {
        status,
        fallback_used,
        error,
        reproducibility,
        envelopes,
    }
}

fn world_generation_prompt(request: &WorldGenerationRequest) -> String {
    format!(
        "agent=StoryArchitect; expansion_goal={}; world_bible_chars={}; canon_chars={}; forbidden_fact_count={}",
        request.expansion_goal,
        request.document.world_bible_markdown.len(),
        request.document.canon_markdown.len(),
        request.document.forbidden_facts.len()
    )
}

fn story_craft_generation_prompt(request: &StoryCraftGenerationRequest) -> String {
    format!(
        "agent=StoryCraftPlanner; concept={}; world_bible_chars={}; canon_chars={}; forbidden_fact_count={}; character_count={}",
        request.concept,
        request.world_bible_markdown.len(),
        request.canon_markdown.len(),
        request.forbidden_facts.len(),
        request.characters.len()
    )
}

fn character_generation_prompt(request: &CharacterGenerationRequest) -> String {
    format!(
        "agent=CharacterDesigner; concept={}; role_hint={}; existing_character_count={}; world_bible_chars={}; story_bible_chars={}",
        request.concept,
        request.role_hint,
        request.existing_characters.len(),
        request.world_bible_markdown.len(),
        request.story_bible_markdown.len()
    )
}

fn generated_story_craft_state() -> plotforge_schema::StoryCraftState {
    let mut story_craft = plotforge_storycraft::sample_story_craft_state();
    story_craft.bible.genre_promise = "A pressure-driven interactive civic drama.".into();
    story_craft.bible.central_question =
        "Can the player preserve legitimacy while every survival choice has a cost?".into();
    story_craft.emotional_arc = vec![
        EmotionalArcPoint {
            scene_key: "opening-pressure".into(),
            target_emotion: "urgent responsibility".into(),
            intensity: 72,
        },
        EmotionalArcPoint {
            scene_key: "first-reversal".into(),
            target_emotion: "earned consequence".into(),
            intensity: 84,
        },
        EmotionalArcPoint {
            scene_key: "public-reckoning".into(),
            target_emotion: "costly agency".into(),
            intensity: 92,
        },
    ];
    story_craft.plot_threads = vec![
        PlotThread {
            id: "resource-legitimacy".into(),
            title: "Resource legitimacy".into(),
            promise: "Every emergency resource move changes public trust.".into(),
            thread_type: PlotThreadType::Political,
            status: PlotThreadStatus::Open,
            introduced_at: "opening-pressure".into(),
            expected_payoff: Some(
                "A later scene forces a choice between reserves and legitimacy.".into(),
            ),
            related_characters: Vec::new(),
            related_world_flags: vec!["public_trust_tested".into()],
            last_update: "Generated from StoryCraft planner.".into(),
        },
        PlotThread {
            id: "hidden-civic-cost".into(),
            title: "Hidden civic cost".into(),
            promise: "A useful ally demands a future compromise.".into(),
            thread_type: PlotThreadType::Mystery,
            status: PlotThreadStatus::Open,
            introduced_at: "opening-pressure".into(),
            expected_payoff: Some(
                "The ally's price becomes visible after the first success.".into(),
            ),
            related_characters: Vec::new(),
            related_world_flags: vec!["ally_price_unpaid".into()],
            last_update: "Generated from StoryCraft planner.".into(),
        },
        PlotThread {
            id: "player-style-mirror".into(),
            title: "Player style mirror".into(),
            promise: "Repeated choices teach factions what the ruler values.".into(),
            thread_type: PlotThreadType::Relationship,
            status: PlotThreadStatus::Open,
            introduced_at: "opening-pressure".into(),
            expected_payoff: Some("A faction copies or punishes the player's pattern.".into()),
            related_characters: Vec::new(),
            related_world_flags: vec!["ruling_pattern_noticed".into()],
            last_update: "Generated from StoryCraft planner.".into(),
        },
    ];
    story_craft.character_arcs = Vec::new();
    story_craft
}

fn generated_character(prompt: &str, provider_config_hash: &str) -> Character {
    let prompt_hash = format!("sha256:{}", stable_sha256_hash(prompt));
    Character {
        id: "generated-character".into(),
        name: "Generated Envoy".into(),
        role: "Pressure-bearing story catalyst".into(),
        traits: vec!["observant".into(), "cost-aware".into(), "direct".into()],
        visual_card: "Historically grounded portrait, restrained clothing, alert posture.".into(),
        voice_card: "Concise, tactful, and specific about consequences.".into(),
        portrait_request: Some(CharacterPortraitRequest {
            prompt_summary: "Portrait for a pressure-bearing story catalyst.".into(),
            style: "grounded historical character card".into(),
            target_asset_slot: "portrait".into(),
            prompt_hash,
            provider_config_hash: provider_config_hash.into(),
            reference_asset_ids: Vec::new(),
            fallback_allowed: true,
        }),
    }
}

fn fallback_character(request: &CharacterGenerationRequest, run_seed: u64) -> Character {
    let role = if request.role_hint.trim().is_empty() {
        "Fallback story catalyst"
    } else {
        request.role_hint.trim()
    };
    let prompt_hash = format!(
        "sha256:{}",
        stable_sha256_hash(&format!(
            "{}\n{}\n{}",
            request.concept, request.role_hint, run_seed
        ))
    );

    Character {
        id: "fallback-character".into(),
        name: "Fallback Character".into(),
        role: role.into(),
        traits: vec!["visible fallback".into(), "editable".into()],
        visual_card: "Fallback character card; edit before production use.".into(),
        voice_card: "Plain fallback voice; edit before production use.".into(),
        portrait_request: Some(CharacterPortraitRequest {
            prompt_summary: format!("Fallback portrait request for {role}."),
            style: "local placeholder character card".into(),
            target_asset_slot: "portrait".into(),
            prompt_hash,
            provider_config_hash: ReproducibilityMetadata::local_mock(run_seed)
                .provider_config_hash,
            reference_asset_ids: Vec::new(),
            fallback_allowed: true,
        }),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageGenerationRequest {
    pub scene_key: String,
    pub prompt: String,
    pub output_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageGenerationResponse {
    pub bytes: Vec<u8>,
    pub provider: String,
    pub model: Option<String>,
    pub request_id: Option<String>,
    pub spent_cost_units: u64,
}

impl ImageGenerationResponse {
    pub fn png(
        bytes: impl Into<Vec<u8>>,
        provider: impl Into<String>,
        model: Option<String>,
        request_id: Option<String>,
        spent_cost_units: u64,
    ) -> Self {
        Self {
            bytes: bytes.into(),
            provider: provider.into(),
            model,
            request_id,
            spent_cost_units,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageProviderErrorKind {
    Provider,
    Timeout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageProviderError {
    pub kind: ImageProviderErrorKind,
    pub code: String,
    pub message: String,
}

impl ImageProviderError {
    pub fn provider(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: ImageProviderErrorKind::Provider,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            kind: ImageProviderErrorKind::Timeout,
            code: "image_provider_timeout".into(),
            message: message.into(),
        }
    }

    fn retryable(&self) -> bool {
        matches!(
            self.kind,
            ImageProviderErrorKind::Provider | ImageProviderErrorKind::Timeout
        )
    }

    fn into_runtime_error(self) -> RuntimeError {
        match self.kind {
            ImageProviderErrorKind::Provider => RuntimeError::redacted(self.code, self.message),
            ImageProviderErrorKind::Timeout => RuntimeError::redacted(
                "image_provider_timeout",
                format!("image provider timed out: {}", self.message),
            ),
        }
    }
}

impl std::fmt::Display for ImageProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ImageProviderError {}

pub trait ImageProvider {
    fn generate(
        &self,
        request: &ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse, ImageProviderError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneImageRequest {
    pub scene_key: String,
    pub prompt: String,
    pub output_path: String,
}

impl SceneImageRequest {
    pub fn background(scene: &Scene) -> Self {
        Self {
            scene_key: scene.key.clone(),
            prompt: scene_image_prompt(scene),
            output_path: format!("assets/generated/{}.png", scene.key),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneImageResult {
    pub asset_record: AssetRecord,
    pub job_record: Option<JobRecord>,
    pub fallback_used: bool,
    pub error: Option<RuntimeError>,
}

#[derive(Debug)]
pub enum SceneImagePipelineError {
    Job(JobQueueError),
    Media(MediaError),
    MissingAssetRecord(String),
}

impl std::fmt::Display for SceneImagePipelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Job(error) => write!(formatter, "{error}"),
            Self::Media(error) => write!(formatter, "{error}"),
            Self::MissingAssetRecord(id) => {
                write!(
                    formatter,
                    "asset registry did not return inserted record {id}"
                )
            }
        }
    }
}

impl std::error::Error for SceneImagePipelineError {}

impl From<JobQueueError> for SceneImagePipelineError {
    fn from(error: JobQueueError) -> Self {
        Self::Job(error)
    }
}

impl From<MediaError> for SceneImagePipelineError {
    fn from(error: MediaError) -> Self {
        Self::Media(error)
    }
}

#[derive(Clone, Debug)]
pub struct SceneImagePipeline<P> {
    provider: P,
}

impl<P> SceneImagePipeline<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P> SceneImagePipeline<P>
where
    P: ImageProvider,
{
    pub fn generate_scene_background<C>(
        &self,
        request: SceneImageRequest,
        registry: &mut AssetRegistry,
        jobs: &mut JobQueue<C>,
    ) -> Result<SceneImageResult, SceneImagePipelineError>
    where
        C: JobClock,
    {
        self.generate_scene_background_with_project_root(request, None, registry, jobs)
    }

    pub fn generate_scene_background_for_project<C>(
        &self,
        project_root: impl AsRef<Path>,
        request: SceneImageRequest,
        registry: &mut AssetRegistry,
        jobs: &mut JobQueue<C>,
    ) -> Result<SceneImageResult, SceneImagePipelineError>
    where
        C: JobClock,
    {
        self.generate_scene_background_with_project_root(
            request,
            Some(project_root.as_ref()),
            registry,
            jobs,
        )
    }

    fn generate_scene_background_with_project_root<C>(
        &self,
        request: SceneImageRequest,
        project_root: Option<&Path>,
        registry: &mut AssetRegistry,
        jobs: &mut JobQueue<C>,
    ) -> Result<SceneImageResult, SceneImagePipelineError>
    where
        C: JobClock,
    {
        let prompt_hash = stable_prompt_hash(&request.prompt);
        if let Some(asset_record) =
            cached_scene_background(registry, &request.scene_key, &prompt_hash)
        {
            return Ok(SceneImageResult {
                asset_record,
                job_record: None,
                fallback_used: false,
                error: None,
            });
        }

        let job = jobs.enqueue(JobRequest {
            kind: JobKind::ImageGeneration,
            timeout_ms: IMAGE_JOB_TIMEOUT_MS,
            max_attempts: IMAGE_JOB_MAX_ATTEMPTS,
            estimated_cost_units: IMAGE_JOB_ESTIMATED_COST_UNITS,
        })?;
        jobs.start(&job.id)?;
        jobs.report_progress(
            &job.id,
            0,
            1,
            Some(format!(
                "generating background image for {}",
                request.scene_key
            )),
        )?;

        let provider_request = ImageGenerationRequest {
            scene_key: request.scene_key.clone(),
            prompt: request.prompt.clone(),
            output_path: request.output_path.clone(),
        };
        match self.provider.generate(&provider_request) {
            Ok(response) => {
                let asset_id = insert_media_bytes(
                    registry,
                    project_root,
                    AssetRecordInput {
                        kind: AssetKind::Image,
                        source: AssetSourceKind::Generated,
                        project_path: request.output_path.clone(),
                        export_path: Some(request.output_path),
                        provider_metadata: Some(AssetProviderMetadata {
                            provider: response.provider,
                            model: response.model,
                            request_id: response.request_id,
                            prompt_hash: Some(prompt_hash),
                            fallback_used: false,
                        }),
                        references: vec![scene_background_reference(&request.scene_key)],
                    },
                    &response.bytes,
                )?;
                jobs.report_progress(&job.id, 1, 1, None)?;
                let job_record = jobs.succeed(&job.id, response.spent_cost_units)?;
                let asset_record = registry
                    .get(&asset_id)
                    .cloned()
                    .ok_or(SceneImagePipelineError::MissingAssetRecord(asset_id))?;

                Ok(SceneImageResult {
                    asset_record,
                    job_record: Some(job_record),
                    fallback_used: false,
                    error: None,
                })
            }
            Err(error) => {
                let retryable = error.retryable();
                let runtime_error = error.into_runtime_error();
                let job_record = jobs.fail(
                    &job.id,
                    JobFailure {
                        code: runtime_error.code.clone(),
                        message: redact_trace_text(&runtime_error.message),
                        retryable,
                    },
                )?;
                let placeholder_bytes = placeholder_image_bytes(&request.scene_key, &prompt_hash);
                let asset_id = insert_media_bytes(
                    registry,
                    project_root,
                    AssetRecordInput {
                        kind: AssetKind::Image,
                        source: AssetSourceKind::Placeholder,
                        project_path: request.output_path.clone(),
                        export_path: Some(request.output_path),
                        provider_metadata: Some(AssetProviderMetadata {
                            provider: "plotforge-placeholder".into(),
                            model: Some("placeholder-image-v1".into()),
                            request_id: None,
                            prompt_hash: Some(prompt_hash),
                            fallback_used: true,
                        }),
                        references: vec![scene_background_reference(&request.scene_key)],
                    },
                    &placeholder_bytes,
                )?;
                let asset_record = registry
                    .get(&asset_id)
                    .cloned()
                    .ok_or(SceneImagePipelineError::MissingAssetRecord(asset_id))?;

                Ok(SceneImageResult {
                    asset_record,
                    job_record: Some(job_record),
                    fallback_used: true,
                    error: Some(runtime_error),
                })
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FakeImageProvider {
    failure: Option<FakeImageFailureKind>,
    calls: std::rc::Rc<std::cell::Cell<u32>>,
}

impl FakeImageProvider {
    pub fn success() -> Self {
        Self::default()
    }

    pub fn provider_error() -> Self {
        Self::with_failure(FakeImageFailureKind::ProviderError)
    }

    pub fn timeout() -> Self {
        Self::with_failure(FakeImageFailureKind::Timeout)
    }

    pub fn call_count(&self) -> u32 {
        self.calls.get()
    }

    fn with_failure(kind: FakeImageFailureKind) -> Self {
        Self {
            failure: Some(kind),
            calls: std::rc::Rc::new(std::cell::Cell::new(0)),
        }
    }
}

impl ImageProvider for FakeImageProvider {
    fn generate(
        &self,
        request: &ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse, ImageProviderError> {
        self.calls.set(self.calls.get() + 1);
        if let Some(failure) = &self.failure {
            return match failure {
                FakeImageFailureKind::ProviderError => Err(ImageProviderError::provider(
                    "image_provider_error",
                    "fake image provider failed OPENAI_API_KEY=sk-image-secret",
                )),
                FakeImageFailureKind::Timeout => Err(ImageProviderError::timeout(
                    "fake image provider timeout token=image-secret",
                )),
            };
        }

        Ok(ImageGenerationResponse::png(
            fake_image_bytes(request),
            "fake-image",
            Some("placeholder-v1".into()),
            Some(format!("fake-image-{}", request.scene_key)),
            1,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FakeImageFailureKind {
    ProviderError,
    Timeout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TtsTarget {
    Scene { scene_key: String },
    Beat { scene_key: String, beat_id: String },
    Character { character_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TtsRequest {
    pub target: TtsTarget,
    pub text: String,
    pub voice: String,
    pub output_path: String,
    pub asset_kind: AssetKind,
}

impl TtsRequest {
    pub fn scene_narration(
        scene: &Scene,
        text: impl Into<String>,
        voice: impl Into<String>,
    ) -> Self {
        Self {
            target: TtsTarget::Scene {
                scene_key: scene.key.clone(),
            },
            text: text.into(),
            voice: voice.into(),
            output_path: format!("assets/generated/audio/{}-scene.wav", scene.key),
            asset_kind: AssetKind::Audio,
        }
    }

    pub fn beat_narration(
        scene_key: impl Into<String>,
        beat: &Beat,
        voice: impl Into<String>,
    ) -> Self {
        let scene_key = scene_key.into();
        Self {
            target: TtsTarget::Beat {
                scene_key: scene_key.clone(),
                beat_id: beat.id.clone(),
            },
            text: beat.text.clone(),
            voice: voice.into(),
            output_path: format!("assets/generated/audio/{}-{}.wav", scene_key, beat.id),
            asset_kind: AssetKind::Audio,
        }
    }

    pub fn character_voice(character: &Character, sample_text: impl Into<String>) -> Self {
        Self {
            target: TtsTarget::Character {
                character_id: character.id.clone(),
            },
            text: sample_text.into(),
            voice: character.voice_card.clone(),
            output_path: format!("assets/generated/voices/{}.wav", character.id),
            asset_kind: AssetKind::Voice,
        }
    }

    fn prompt_hash(&self) -> String {
        stable_prompt_hash(&format!(
            "target={:?}; voice={}; text={}",
            self.target, self.voice, self.text
        ))
    }

    fn reference(&self) -> AssetReference {
        match &self.target {
            TtsTarget::Scene { scene_key } => AssetReference {
                reference_kind: AssetReferenceKind::Scene,
                reference_id: scene_key.clone(),
                slot: "scene_audio".into(),
            },
            TtsTarget::Beat { scene_key, beat_id } => AssetReference {
                reference_kind: AssetReferenceKind::Scene,
                reference_id: scene_key.clone(),
                slot: format!("beat_audio:{beat_id}:narration"),
            },
            TtsTarget::Character { character_id } => AssetReference {
                reference_kind: AssetReferenceKind::Character,
                reference_id: character_id.clone(),
                slot: "voice".into(),
            },
        }
    }

    fn media_reference(&self, asset_record: &AssetRecord) -> MediaAssetReference {
        MediaAssetReference {
            asset_id: Some(asset_record.id.clone()),
            kind: asset_record.kind.clone(),
            source: asset_record.source.clone(),
            project_path: asset_record.project_path.clone(),
            export_path: asset_record.export_path.clone(),
            slot: match &self.target {
                TtsTarget::Scene { .. } => "scene_audio".into(),
                TtsTarget::Beat { .. } => "narration".into(),
                TtsTarget::Character { .. } => "voice".into(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TtsProviderOutput {
    pub bytes: Vec<u8>,
    pub provider: String,
    pub model: Option<String>,
    pub request_id: Option<String>,
    pub spent_cost_units: u64,
}

impl TtsProviderOutput {
    pub fn audio(
        bytes: impl Into<Vec<u8>>,
        provider: impl Into<String>,
        model: Option<String>,
        request_id: Option<String>,
        spent_cost_units: u64,
    ) -> Self {
        Self {
            bytes: bytes.into(),
            provider: provider.into(),
            model,
            request_id,
            spent_cost_units,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TtsProviderErrorKind {
    Provider,
    Timeout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TtsProviderError {
    pub kind: TtsProviderErrorKind,
    pub code: String,
    pub message: String,
}

impl TtsProviderError {
    pub fn provider(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: TtsProviderErrorKind::Provider,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            kind: TtsProviderErrorKind::Timeout,
            code: "tts_provider_timeout".into(),
            message: message.into(),
        }
    }

    fn retryable(&self) -> bool {
        matches!(
            self.kind,
            TtsProviderErrorKind::Provider | TtsProviderErrorKind::Timeout
        )
    }

    fn into_runtime_error(self) -> RuntimeError {
        match self.kind {
            TtsProviderErrorKind::Provider => RuntimeError::redacted(self.code, self.message),
            TtsProviderErrorKind::Timeout => RuntimeError::redacted(
                "tts_provider_timeout",
                format!("tts provider timed out: {}", self.message),
            ),
        }
    }
}

impl std::fmt::Display for TtsProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for TtsProviderError {}

pub trait TtsProvider {
    fn synthesize(&self, request: &TtsRequest) -> Result<TtsProviderOutput, TtsProviderError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TtsPipelineResult {
    pub asset_record: AssetRecord,
    pub media_reference: MediaAssetReference,
    pub job_record: Option<JobRecord>,
    pub fallback_used: bool,
    pub error: Option<RuntimeError>,
}

#[derive(Debug)]
pub enum TtsPipelineError {
    Job(JobQueueError),
    Media(MediaError),
    MissingAssetRecord(String),
}

impl std::fmt::Display for TtsPipelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Job(error) => write!(formatter, "{error}"),
            Self::Media(error) => write!(formatter, "{error}"),
            Self::MissingAssetRecord(id) => {
                write!(
                    formatter,
                    "asset registry did not return inserted record {id}"
                )
            }
        }
    }
}

impl std::error::Error for TtsPipelineError {}

impl From<JobQueueError> for TtsPipelineError {
    fn from(error: JobQueueError) -> Self {
        Self::Job(error)
    }
}

impl From<MediaError> for TtsPipelineError {
    fn from(error: MediaError) -> Self {
        Self::Media(error)
    }
}

#[derive(Clone, Debug)]
pub struct TtsPipeline<P> {
    provider: P,
}

impl<P> TtsPipeline<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P> TtsPipeline<P>
where
    P: TtsProvider,
{
    pub fn synthesize<C>(
        &self,
        request: TtsRequest,
        registry: &mut AssetRegistry,
        jobs: &mut JobQueue<C>,
    ) -> Result<TtsPipelineResult, TtsPipelineError>
    where
        C: JobClock,
    {
        self.synthesize_with_project_root(request, None, registry, jobs)
    }

    pub fn synthesize_for_project<C>(
        &self,
        project_root: impl AsRef<Path>,
        request: TtsRequest,
        registry: &mut AssetRegistry,
        jobs: &mut JobQueue<C>,
    ) -> Result<TtsPipelineResult, TtsPipelineError>
    where
        C: JobClock,
    {
        self.synthesize_with_project_root(request, Some(project_root.as_ref()), registry, jobs)
    }

    fn synthesize_with_project_root<C>(
        &self,
        request: TtsRequest,
        project_root: Option<&Path>,
        registry: &mut AssetRegistry,
        jobs: &mut JobQueue<C>,
    ) -> Result<TtsPipelineResult, TtsPipelineError>
    where
        C: JobClock,
    {
        let prompt_hash = request.prompt_hash();
        let reference = request.reference();
        if let Some(asset_record) =
            cached_tts_asset(registry, &request.asset_kind, &reference, &prompt_hash)
        {
            let media_reference = request.media_reference(&asset_record);
            return Ok(TtsPipelineResult {
                asset_record,
                media_reference,
                job_record: None,
                fallback_used: false,
                error: None,
            });
        }

        let job = jobs.enqueue(JobRequest {
            kind: JobKind::TtsGeneration,
            timeout_ms: TTS_JOB_TIMEOUT_MS,
            max_attempts: TTS_JOB_MAX_ATTEMPTS,
            estimated_cost_units: TTS_JOB_ESTIMATED_COST_UNITS,
        })?;
        jobs.start(&job.id)?;
        jobs.report_progress(&job.id, 0, 1, Some(tts_progress_message(&request)))?;

        match self.provider.synthesize(&request) {
            Ok(output) => {
                let asset_id = insert_media_bytes(
                    registry,
                    project_root,
                    AssetRecordInput {
                        kind: request.asset_kind.clone(),
                        source: AssetSourceKind::Generated,
                        project_path: request.output_path.clone(),
                        export_path: Some(request.output_path.clone()),
                        provider_metadata: Some(AssetProviderMetadata {
                            provider: output.provider,
                            model: output.model,
                            request_id: output.request_id,
                            prompt_hash: Some(prompt_hash),
                            fallback_used: false,
                        }),
                        references: vec![reference],
                    },
                    &output.bytes,
                )?;
                jobs.report_progress(&job.id, 1, 1, None)?;
                let job_record = jobs.succeed(&job.id, output.spent_cost_units)?;
                let asset_record = registry
                    .get(&asset_id)
                    .cloned()
                    .ok_or(TtsPipelineError::MissingAssetRecord(asset_id))?;
                let media_reference = request.media_reference(&asset_record);

                Ok(TtsPipelineResult {
                    asset_record,
                    media_reference,
                    job_record: Some(job_record),
                    fallback_used: false,
                    error: None,
                })
            }
            Err(error) => {
                let retryable = error.retryable();
                let runtime_error = error.into_runtime_error();
                let job_record = jobs.fail(
                    &job.id,
                    JobFailure {
                        code: runtime_error.code.clone(),
                        message: redact_trace_text(&runtime_error.message),
                        retryable,
                    },
                )?;
                let fallback_bytes = silent_audio_bytes(&request, &prompt_hash);
                let asset_id = insert_media_bytes(
                    registry,
                    project_root,
                    AssetRecordInput {
                        kind: request.asset_kind.clone(),
                        source: AssetSourceKind::Placeholder,
                        project_path: request.output_path.clone(),
                        export_path: Some(request.output_path.clone()),
                        provider_metadata: Some(AssetProviderMetadata {
                            provider: "plotforge-silent-fallback".into(),
                            model: Some("silent-audio-v1".into()),
                            request_id: None,
                            prompt_hash: Some(prompt_hash),
                            fallback_used: true,
                        }),
                        references: vec![reference],
                    },
                    &fallback_bytes,
                )?;
                let asset_record = registry
                    .get(&asset_id)
                    .cloned()
                    .ok_or(TtsPipelineError::MissingAssetRecord(asset_id))?;
                let media_reference = request.media_reference(&asset_record);

                Ok(TtsPipelineResult {
                    asset_record,
                    media_reference,
                    job_record: Some(job_record),
                    fallback_used: true,
                    error: Some(runtime_error),
                })
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FakeTtsProvider {
    failure: Option<FakeTtsFailureKind>,
    calls: std::rc::Rc<std::cell::Cell<u32>>,
}

impl FakeTtsProvider {
    pub fn success() -> Self {
        Self::default()
    }

    pub fn provider_error() -> Self {
        Self::with_failure(FakeTtsFailureKind::ProviderError)
    }

    pub fn timeout() -> Self {
        Self::with_failure(FakeTtsFailureKind::Timeout)
    }

    pub fn call_count(&self) -> u32 {
        self.calls.get()
    }

    fn with_failure(kind: FakeTtsFailureKind) -> Self {
        Self {
            failure: Some(kind),
            calls: std::rc::Rc::new(std::cell::Cell::new(0)),
        }
    }
}

impl TtsProvider for FakeTtsProvider {
    fn synthesize(&self, request: &TtsRequest) -> Result<TtsProviderOutput, TtsProviderError> {
        self.calls.set(self.calls.get() + 1);
        if let Some(failure) = &self.failure {
            return match failure {
                FakeTtsFailureKind::ProviderError => Err(TtsProviderError::provider(
                    "tts_provider_error",
                    "fake tts provider failed OPENAI_API_KEY=sk-tts-secret",
                )),
                FakeTtsFailureKind::Timeout => Err(TtsProviderError::timeout(
                    "fake tts provider timeout token=tts-secret",
                )),
            };
        }

        Ok(TtsProviderOutput::audio(
            fake_tts_bytes(request),
            "fake-tts",
            Some("fake-tts-v1".into()),
            Some(format!("fake-tts-{}", tts_request_id_suffix(request))),
            1,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FakeTtsFailureKind {
    ProviderError,
    Timeout,
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

    pub fn asset_records(&self) -> Vec<AssetRecord> {
        self.state
            .borrow()
            .asset_registry
            .records()
            .cloned()
            .collect()
    }

    pub fn job_records(&self) -> Vec<JobRecord> {
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

fn insert_media_bytes(
    registry: &mut AssetRegistry,
    project_root: Option<&Path>,
    input: AssetRecordInput,
    bytes: &[u8],
) -> Result<String, MediaError> {
    match project_root {
        Some(project_root) => registry.insert_project_bytes(project_root, input, bytes),
        None => registry.insert_bytes(input, bytes),
    }
}

fn cached_scene_background(
    registry: &AssetRegistry,
    scene_key: &str,
    prompt_hash: &str,
) -> Option<AssetRecord> {
    registry
        .records_referenced_by(AssetReferenceKind::Scene, scene_key)
        .into_iter()
        .filter(|record| {
            record.kind == AssetKind::Image
                && record.source == AssetSourceKind::Generated
                && record
                    .references
                    .iter()
                    .any(|reference| reference.slot == SCENE_BACKGROUND_SLOT)
        })
        .find(|record| {
            record.provider_metadata.as_ref().is_some_and(|metadata| {
                !metadata.fallback_used && metadata.prompt_hash.as_deref() == Some(prompt_hash)
            })
        })
        .cloned()
}

fn cached_tts_asset(
    registry: &AssetRegistry,
    asset_kind: &AssetKind,
    reference: &AssetReference,
    prompt_hash: &str,
) -> Option<AssetRecord> {
    registry
        .records_referenced_by(reference.reference_kind.clone(), &reference.reference_id)
        .into_iter()
        .filter(|record| {
            record.kind == *asset_kind
                && record.source == AssetSourceKind::Generated
                && record
                    .references
                    .iter()
                    .any(|existing| existing == reference)
        })
        .find(|record| {
            record.provider_metadata.as_ref().is_some_and(|metadata| {
                !metadata.fallback_used && metadata.prompt_hash.as_deref() == Some(prompt_hash)
            })
        })
        .cloned()
}

fn scene_background_reference(scene_key: &str) -> AssetReference {
    AssetReference {
        reference_kind: AssetReferenceKind::Scene,
        reference_id: scene_key.into(),
        slot: SCENE_BACKGROUND_SLOT.into(),
    }
}

fn scene_image_prompt(scene: &Scene) -> String {
    format!(
        "scene_key={}; title={}; location={}; hook={}; dramatic_purpose={}",
        scene.key, scene.title, scene.location, scene.hook, scene.dramatic_purpose
    )
}

fn stable_prompt_hash(prompt: &str) -> String {
    stable_sha256_hash(prompt)
}

fn stable_sha256_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fake_image_bytes(request: &ImageGenerationRequest) -> Vec<u8> {
    format!(
        "plotforge-fake-png\nscene={}\nprompt={}\n",
        request.scene_key, request.prompt
    )
    .into_bytes()
}

fn placeholder_image_bytes(scene_key: &str, prompt_hash: &str) -> Vec<u8> {
    format!("plotforge-placeholder-png\nscene={scene_key}\nprompt_hash={prompt_hash}\n")
        .into_bytes()
}

fn tts_progress_message(request: &TtsRequest) -> String {
    match &request.target {
        TtsTarget::Scene { scene_key } => {
            format!("generating scene narration audio for {scene_key}")
        }
        TtsTarget::Beat { scene_key, beat_id } => {
            format!("generating beat narration audio for {scene_key}/{beat_id}")
        }
        TtsTarget::Character { character_id } => {
            format!("generating character voice audio for {character_id}")
        }
    }
}

fn tts_request_id_suffix(request: &TtsRequest) -> String {
    match &request.target {
        TtsTarget::Scene { scene_key } => format!("scene-{scene_key}"),
        TtsTarget::Beat { scene_key, beat_id } => format!("beat-{scene_key}-{beat_id}"),
        TtsTarget::Character { character_id } => format!("character-{character_id}"),
    }
}

fn fake_tts_bytes(request: &TtsRequest) -> Vec<u8> {
    format!(
        "plotforge-fake-wav\ntarget={:?}\nvoice={}\ntext={}\n",
        request.target, request.voice, request.text
    )
    .into_bytes()
}

fn silent_audio_bytes(request: &TtsRequest, prompt_hash: &str) -> Vec<u8> {
    format!(
        "plotforge-silent-wav\ntarget={:?}\nprompt_hash={prompt_hash}\n",
        request.target
    )
    .into_bytes()
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
    ensure_matching_reproducibility(
        &scene_plan_output.reproducibility,
        &beat_output.reproducibility,
        AgentRole::BeatWriter,
    )?;
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
    ensure_matching_reproducibility(
        &scene_plan_output.reproducibility,
        &review_output.reproducibility,
        AgentRole::PlotDoctor,
    )?;
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
    let envelope =
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
    ensure_matching_reproducibility(&reproducibility, &envelope.reproducibility, agent.clone())?;
    validate_agent_output_proposal(&envelope.proposal).map_err(|error| {
        ProviderPipelineError::Validation {
            agent,
            message: format!("{error:?}"),
        }
    })?;

    Ok(envelope)
}

fn validate_agent_output_envelope(
    envelope: &AgentOutputEnvelope,
    expected_agent: &AgentRole,
) -> Result<(), String> {
    if envelope.contract_version != CONTRACT_VERSION {
        return Err(format!(
            "unsupported contract version `{}`, expected `{}`",
            envelope.contract_version, CONTRACT_VERSION
        ));
    }
    if envelope.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema version `{}`, expected `{}`",
            envelope.schema_version, CONTRACT_SCHEMA_VERSION
        ));
    }
    if &envelope.agent != expected_agent {
        return Err(format!(
            "envelope agent `{:?}` did not match requested agent `{:?}`",
            envelope.agent, expected_agent
        ));
    }
    if envelope.proposal.agent != envelope.agent {
        return Err(format!(
            "proposal agent `{:?}` did not match envelope agent `{:?}`",
            envelope.proposal.agent, envelope.agent
        ));
    }
    if envelope.reproducibility.prompt_version.trim().is_empty() {
        return Err("missing prompt_version".into());
    }
    if envelope.reproducibility.model_version.trim().is_empty() {
        return Err("missing model_version".into());
    }
    if envelope
        .reproducibility
        .provider_config_hash
        .trim()
        .is_empty()
    {
        return Err("missing provider_config_hash".into());
    }

    Ok(())
}

fn repair_json_text(raw_json: &str) -> Result<String, String> {
    let trimmed = raw_json.trim();
    if trimmed.is_empty() {
        return Err("empty provider JSON".into());
    }
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return Ok(trimmed.to_string());
    }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}'))
        && start < end
    {
        let candidate = &trimmed[start..=end];
        if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
            return Ok(candidate.to_string());
        }
    }
    Err("unable to repair provider JSON envelope".into())
}

fn ensure_matching_reproducibility(
    expected: &ReproducibilityMetadata,
    actual: &ReproducibilityMetadata,
    agent: AgentRole,
) -> Result<(), ProviderPipelineError> {
    if expected.run_seed != actual.run_seed
        || expected.prompt_version != actual.prompt_version
        || expected.model_version != actual.model_version
        || expected.provider_config_hash != actual.provider_config_hash
    {
        return Err(ProviderPipelineError::Validation {
            agent,
            message: "provider outputs used inconsistent reproducibility metadata".into(),
        });
    }

    Ok(())
}

fn local_reproducibility(request: &ScenePlanRequest<'_>) -> ReproducibilityMetadata {
    ReproducibilityMetadata::local_mock(request.project.game.run_seed)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ProviderPipelineError {
    Provider(TextModelProviderError),
    InvalidJson { agent: AgentRole, message: String },
    Validation { agent: AgentRole, message: String },
}

impl ProviderPipelineError {
    fn into_runtime_error(self) -> RuntimeError {
        match self {
            Self::Provider(error) => match error.kind {
                TextModelProviderErrorKind::Provider => {
                    RuntimeError::redacted(error.code, error.message)
                }
                TextModelProviderErrorKind::Timeout => RuntimeError::redacted(
                    "text_provider_timeout",
                    format!("text provider timed out: {}", error.message),
                ),
            },
            Self::InvalidJson { agent, message } => RuntimeError::redacted(
                "text_provider_invalid_json",
                format!("{agent:?} returned invalid JSON: {message}"),
            ),
            Self::Validation { agent, message } => RuntimeError::redacted(
                "text_provider_schema_validation",
                format!("{agent:?} returned invalid proposal: {message}"),
            ),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FakeTextModelProvider {
    failure: Option<FakeTextModelFailure>,
}

impl FakeTextModelProvider {
    pub fn success() -> Self {
        Self::default()
    }

    pub fn provider_error(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::ProviderError)
    }

    pub fn timeout(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::Timeout)
    }

    pub fn invalid_json(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::InvalidJson)
    }

    pub fn invalid_schema(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::InvalidSchema)
    }

    pub fn wrapped_json(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::WrappedJson)
    }

    pub fn secret_marker(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::SecretMarker)
    }

    fn with_failure(agent: AgentRole, kind: FakeTextModelFailureKind) -> Self {
        Self {
            failure: Some(FakeTextModelFailure { agent, kind }),
        }
    }
}

impl TextModelProvider for FakeTextModelProvider {
    fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
        ReproducibilityMetadata {
            run_seed,
            prompt_version: TEXT_PROMPT_VERSION.into(),
            model_version: FAKE_TEXT_MODEL_VERSION.into(),
            provider_config_hash: FAKE_TEXT_PROVIDER_CONFIG_HASH.into(),
            trace_id: None,
            snapshot_id: None,
        }
    }

    fn complete(
        &self,
        request: &TextModelRequest,
    ) -> Result<TextModelResponse, TextModelProviderError> {
        if let Some(failure) = &self.failure
            && failure.agent == request.agent
        {
            return match failure.kind {
                FakeTextModelFailureKind::ProviderError => Err(TextModelProviderError::provider(
                    "text_provider_error",
                    format!("{:?} fake provider failure", request.agent),
                )),
                FakeTextModelFailureKind::Timeout => Err(TextModelProviderError::timeout(format!(
                    "{:?} fake provider timeout",
                    request.agent
                ))),
                FakeTextModelFailureKind::InvalidJson => Ok(TextModelResponse::json("{")),
                FakeTextModelFailureKind::InvalidSchema => fake_invalid_schema_response(request),
                FakeTextModelFailureKind::WrappedJson => fake_wrapped_json_response(request),
                FakeTextModelFailureKind::SecretMarker => Ok(TextModelResponse::json(
                    "```json\n{\"api_key\":\"sk-test-secret-marker\"}\n```",
                )),
            };
        }

        fake_success_response(request)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FakeTextModelFailure {
    agent: AgentRole,
    kind: FakeTextModelFailureKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FakeTextModelFailureKind {
    ProviderError,
    Timeout,
    InvalidJson,
    InvalidSchema,
    WrappedJson,
    SecretMarker,
}

fn fake_success_response(
    request: &TextModelRequest,
) -> Result<TextModelResponse, TextModelProviderError> {
    let proposal = match request.agent {
        AgentRole::StoryArchitect => AgentOutputProposal {
            id: format!("{}-world-expansion", request.call_id),
            agent: AgentRole::StoryArchitect,
            output: AgentProposalPayload::WorldExpansion(Box::new(WorldExpansionProposal {
                world_bible_markdown: "# World Bible\n\nThe generated world bible turns the initial premise into concrete factions, resources, and visible consequences.\n\n## Pressure Model\n\nEvery player order should change at least one faction expectation or world resource.\n".into(),
                canon_markdown: "# Canon Rules\n\n- Canon changes must preserve visible consequences.\n- The player can revise strategy, but not erase already revealed costs.\n- Generated facts must not contradict forbidden facts.\n".into(),
                forbidden_facts: vec![
                    "Do not reveal a hidden prophecy that solves the crisis.".into(),
                    "Do not make the player immune to resource consequences.".into(),
                ],
            })),
        },
        AgentRole::StoryCraftPlanner => AgentOutputProposal {
            id: format!("{}-story-craft", request.call_id),
            agent: AgentRole::StoryCraftPlanner,
            output: AgentProposalPayload::StoryCraftPlan(Box::new(StoryCraftPlanProposal {
                story_bible_markdown:
                    "# Story Bible\n\nThe opening arc converts a broad premise into three concrete promises: resource pressure, faction cost, and player reputation.\n"
                        .into(),
                style_guide_markdown:
                    "# Style Guide\n\nUse specific pressure, short sentences, and consequence-first choices. Avoid vague grandeur and convenient prophecy.\n"
                        .into(),
                story_craft: generated_story_craft_state(),
            })),
        },
        AgentRole::CharacterDesigner => AgentOutputProposal {
            id: format!("{}-character", request.call_id),
            agent: AgentRole::CharacterDesigner,
            output: AgentProposalPayload::CharacterProfile(Box::new(CharacterProposal {
                character: generated_character(&request.prompt, &request.provider_config_hash),
            })),
        },
        AgentRole::ScenePlanner => AgentOutputProposal {
            id: format!("{}-scene-plan", request.call_id),
            agent: AgentRole::ScenePlanner,
            output: AgentProposalPayload::ScenePlan(Box::new(ScenePlanProposal {
                scene_key: request.scene_key.clone(),
                title: "Provider Planned Scene".into(),
                location: "Civic Hall".into(),
                scene_summary: "A fake text model proposes the next civic crisis.".into(),
                dramatic_purpose:
                    "Exercise the provider-backed scene pipeline without external network calls."
                        .into(),
                hook: "The fake provider returns a schema-checked scene plan.".into(),
                emotional_goal: Some("controlled generation".into()),
                cast: vec!["city-treasurer".into(), "guild-liaison".into()],
                entry_beat_id: format!("{}-beat-001", request.scene_key),
                background_asset: Some(format!("assets/generated/{}.png", request.scene_key)),
            })),
        },
        AgentRole::BeatWriter => AgentOutputProposal {
            id: format!("{}-beats", request.call_id),
            agent: AgentRole::BeatWriter,
            output: AgentProposalPayload::BeatDrafts(Box::new(BeatDraftsProposal {
                scene_key: request.scene_key.clone(),
                beats: vec![BeatDraftProposal {
                    id: format!("{}-beat-001", request.scene_key),
                    scene_key: request.scene_key.clone(),
                    text: "The fake provider writes a beat that keeps the player in control."
                        .into(),
                    choices: vec![Choice {
                        id: "continue-council".into(),
                        label: "Continue".into(),
                        action_type: "continue".into(),
                        input_terms: choice_input_terms("continue"),
                        dramatic_purpose: "Let the engine continue from provider output.".into(),
                        change_scene: false,
                    }],
                    narrative_function: NarrativeFunction::Hook,
                }],
            })),
        },
        AgentRole::PlotDoctor => AgentOutputProposal {
            id: format!("{}-review", request.call_id),
            agent: AgentRole::PlotDoctor,
            output: AgentProposalPayload::Review(Box::new(ReviewProposal {
                scene_key: request.scene_key.clone(),
                review: NarrativeReview {
                    scene_key: request.scene_key.clone(),
                    score: 96,
                    hook_score: 95,
                    pacing_score: 96,
                    character_consistency_score: 100,
                    payoff_score: 94,
                    choice_meaningfulness_score: 95,
                    ai_slop_risk: 5,
                    issues: Vec::new(),
                },
                notes: Vec::new(),
            })),
        },
        _ => {
            return Err(TextModelProviderError::provider(
                "fake_provider_unsupported_agent",
                format!(
                    "fake provider has no output contract for {:?}",
                    request.agent
                ),
            ));
        }
    };

    encode_fake_response(&proposal, request)
}

fn fake_invalid_schema_response(
    request: &TextModelRequest,
) -> Result<TextModelResponse, TextModelProviderError> {
    let proposal = match request.agent {
        AgentRole::StoryArchitect => AgentOutputProposal {
            id: format!("{}-invalid-world-expansion", request.call_id),
            agent: AgentRole::StoryArchitect,
            output: AgentProposalPayload::WorldExpansion(Box::new(WorldExpansionProposal {
                world_bible_markdown: String::new(),
                canon_markdown: "Invalid canon".into(),
                forbidden_facts: Vec::new(),
            })),
        },
        AgentRole::StoryCraftPlanner => AgentOutputProposal {
            id: format!("{}-invalid-story-craft", request.call_id),
            agent: AgentRole::StoryCraftPlanner,
            output: AgentProposalPayload::StoryCraftPlan(Box::new(StoryCraftPlanProposal {
                story_bible_markdown: String::new(),
                style_guide_markdown: "Invalid style".into(),
                story_craft: generated_story_craft_state(),
            })),
        },
        AgentRole::CharacterDesigner => AgentOutputProposal {
            id: format!("{}-invalid-character", request.call_id),
            agent: AgentRole::CharacterDesigner,
            output: AgentProposalPayload::CharacterProfile(Box::new(CharacterProposal {
                character: Character {
                    id: String::new(),
                    name: "Invalid Character".into(),
                    role: "Invalid".into(),
                    traits: Vec::new(),
                    visual_card: "Invalid visual".into(),
                    voice_card: "Invalid voice".into(),
                    portrait_request: None,
                },
            })),
        },
        AgentRole::ScenePlanner => AgentOutputProposal {
            id: format!("{}-invalid-scene-plan", request.call_id),
            agent: AgentRole::ScenePlanner,
            output: AgentProposalPayload::ScenePlan(Box::new(ScenePlanProposal {
                scene_key: String::new(),
                title: "Invalid Scene Plan".into(),
                location: "Civic Hall".into(),
                scene_summary: "This payload is structurally valid JSON but fails validation."
                    .into(),
                dramatic_purpose: "Test schema validation failure.".into(),
                hook: "A missing scene key should fail.".into(),
                emotional_goal: None,
                cast: Vec::new(),
                entry_beat_id: "missing-entry".into(),
                background_asset: None,
            })),
        },
        AgentRole::BeatWriter => AgentOutputProposal {
            id: format!("{}-invalid-beats", request.call_id),
            agent: AgentRole::BeatWriter,
            output: AgentProposalPayload::BeatDrafts(Box::new(BeatDraftsProposal {
                scene_key: request.scene_key.clone(),
                beats: Vec::new(),
            })),
        },
        AgentRole::PlotDoctor => AgentOutputProposal {
            id: format!("{}-invalid-review", request.call_id),
            agent: AgentRole::PlotDoctor,
            output: AgentProposalPayload::Review(Box::new(ReviewProposal {
                scene_key: request.scene_key.clone(),
                review: NarrativeReview {
                    scene_key: "other-scene".into(),
                    score: 96,
                    hook_score: 95,
                    pacing_score: 96,
                    character_consistency_score: 100,
                    payoff_score: 94,
                    choice_meaningfulness_score: 95,
                    ai_slop_risk: 5,
                    issues: Vec::new(),
                },
                notes: Vec::new(),
            })),
        },
        _ => {
            return Err(TextModelProviderError::provider(
                "fake_provider_unsupported_agent",
                format!(
                    "fake provider has no invalid schema output for {:?}",
                    request.agent
                ),
            ));
        }
    };

    encode_fake_response(&proposal, request)
}

fn fake_wrapped_json_response(
    request: &TextModelRequest,
) -> Result<TextModelResponse, TextModelProviderError> {
    let response = fake_success_response(request)?;
    Ok(TextModelResponse::json(format!(
        "Provider draft:\n```json\n{}\n```\nEnd of draft.",
        response.raw_json
    )))
}

fn encode_fake_response(
    proposal: &AgentOutputProposal,
    request: &TextModelRequest,
) -> Result<TextModelResponse, TextModelProviderError> {
    let envelope = AgentOutputEnvelope {
        id: format!("{}-envelope", proposal.id),
        contract_version: CONTRACT_VERSION.into(),
        schema_version: CONTRACT_SCHEMA_VERSION,
        agent: proposal.agent.clone(),
        reproducibility: ReproducibilityMetadata {
            run_seed: request.run_seed,
            prompt_version: request.prompt_version.clone(),
            model_version: request.model_version.clone(),
            provider_config_hash: request.provider_config_hash.clone(),
            trace_id: None,
            snapshot_id: None,
        },
        proposal: proposal.clone(),
    };
    serde_json::to_string(&envelope)
        .map(TextModelResponse::json)
        .map_err(|error| {
            TextModelProviderError::provider("fake_provider_serialization", error.to_string())
        })
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
                        input_terms: choice_input_terms("raise_tax"),
                        dramatic_purpose: "Gain treasury while risking unrest.".into(),
                        change_scene: true,
                    },
                    Choice {
                        id: "inspect-corruption".into(),
                        label: "Investigate payroll corruption".into(),
                        action_type: "inspect_corruption".into(),
                        input_terms: choice_input_terms("inspect_corruption"),
                        dramatic_purpose: "Seek hidden leakage while destabilizing civic factions."
                            .into(),
                        change_scene: true,
                    },
                    Choice {
                        id: "continue-council".into(),
                        label: "Hear one more minister".into(),
                        action_type: "continue".into(),
                        input_terms: choice_input_terms("continue"),
                        dramatic_purpose:
                            "Stay in the scene to gather more pressure before committing.".into(),
                        change_scene: false,
                    },
                    Choice {
                        id: "pay-army".into(),
                        label: "Pay the border army first".into(),
                        action_type: "pay_army".into(),
                        input_terms: choice_input_terms("pay_army"),
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
                        input_terms: choice_input_terms("raise_tax"),
                        dramatic_purpose: "Gain treasury while risking unrest.".into(),
                        change_scene: true,
                    },
                    Choice {
                        id: "inspect-corruption".into(),
                        label: "Investigate payroll corruption".into(),
                        action_type: "inspect_corruption".into(),
                        input_terms: choice_input_terms("inspect_corruption"),
                        dramatic_purpose: "Seek hidden leakage while destabilizing civic factions."
                            .into(),
                        change_scene: true,
                    },
                    Choice {
                        id: "pay-army".into(),
                        label: "Pay the border army first".into(),
                        action_type: "pay_army".into(),
                        input_terms: choice_input_terms("pay_army"),
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
                    input_terms: choice_input_terms("continue"),
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

fn choice_input_terms(action_type: &str) -> Vec<String> {
    let terms: &[&str] = match action_type {
        "continue" => &["continue", "hear", "minister", "听", "继续", "陈情"],
        "raise_tax" => &["raise", "tax", "levy", "加征", "港税"],
        "inspect_corruption" => &["inspect", "corruption", "严查", "贪墨", "查"],
        "pay_army" => &["pay", "army", "军饷", "拨", "内帑", "边军"],
        _ => &[],
    };
    terms.iter().map(|term| (*term).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, fs, rc::Rc};

    use plotforge_job::{JobClock, JobQueue};
    use plotforge_media::AssetRegistry;
    use plotforge_schema::{
        AgentOutputProposal, AgentProposalPayload, AgentRole, AssetKind, AssetReferenceKind,
        AssetSourceKind, BeatDraftProposal, BeatDraftsProposal, CharacterGenerationRequest, Choice,
        GameProject, GenerationStatus, JobStatus, NarrativeFunction, NarrativeReview,
        ProjectCreationRequest, ProjectTemplateId, REDACTED_TRACE_SECRET, ReviewProposal,
        ScenePlanProposal, Severity, StoryCraftEditDocument, StoryCraftGenerationRequest,
        StoryState, WorldEditDocument, WorldGenerationRequest, WorldState,
    };

    use super::{
        AgentProposalValidationError, ConfiguredTextModelProvider, FakeImageProvider,
        FakeTextModelProvider, FakeTtsProvider, ImageProviderAgentPipeline, MockAgentPipeline,
        ProviderAgentPipeline, ProviderCredentialError, ProviderCredentialResolver,
        SceneImagePipeline, SceneImageRequest, ScenePlanRequest, ScenePlanner, TextModelClient,
        TextModelClientRequest, TextModelProviderError, TextModelResponse, TextProviderConfig,
        TtsPipeline, TtsRequest, fake_success_response, generate_character,
        generate_character_with_provider, generate_story_craft, generate_story_craft_with_provider,
        generate_world_expansion, generate_world_expansion_with_provider, scene_from_proposals,
        validate_agent_output_proposal,
    };

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
        plotforge_storage::create_project_from_request(
            project_path,
            sample_creation_request(),
            false,
        )
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

        let error =
            scene_from_proposals(&scene_plan, &beat_drafts, None).expect_err("missing entry");

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
        assert_eq!(plan.review.scene_key, "provider-scene-001");
        assert_eq!(plan.review.score, 96);
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
        let pipeline = ProviderAgentPipeline::new(FakeTextModelProvider::wrapped_json(
            AgentRole::ScenePlanner,
        ));

        let plan = pipeline
            .plan_next_scene(provider_request(&project))
            .expect("provider plan");

        assert_eq!(plan.scene.key, "provider-scene-001");
        assert_eq!(plan.review.scene_key, "provider-scene-001");
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
        let pipeline = ProviderAgentPipeline::new(FakeTextModelProvider::invalid_schema(
            AgentRole::BeatWriter,
        ));

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
        let report = generate_world_expansion(world_generation_request(), 41);

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
        let report = generate_story_craft(story_craft_generation_request(), 42);

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
        let report = generate_character(character_generation_request(), 43);

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
        let report =
            generate_character_with_provider(&provider, character_generation_request(), 44);

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
        let report =
            generate_world_expansion_with_provider(&provider, world_generation_request(), 45);

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

    fn scene_image_request() -> SceneImageRequest {
        SceneImageRequest {
            scene_key: "scene-one".into(),
            prompt: "paint a tense civic hearing".into(),
            output_path: "assets/generated/scene-one.png".into(),
        }
    }

    fn world_generation_request() -> WorldGenerationRequest {
        WorldGenerationRequest {
            expansion_goal: "Expand the civic crisis into factions, canon, and forbidden facts."
                .into(),
            document: WorldEditDocument {
                world_bible_markdown: "# Existing World\n\nA civic crisis strains the treasury."
                    .into(),
                canon_markdown: "# Existing Canon\n\nVisible choices must have consequences."
                    .into(),
                forbidden_facts: vec!["Do not solve the famine with prophecy.".into()],
            },
        }
    }

    fn story_craft_generation_request() -> StoryCraftGenerationRequest {
        StoryCraftGenerationRequest {
            concept: "A ruler must survive an escalating fiscal and legitimacy crisis.".into(),
            world_bible_markdown:
                "# World Bible\n\nThe city council is divided by emergency revenue.".into(),
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
            world_bible_markdown: "# World Bible\n\nFactions trade legitimacy for resources."
                .into(),
            story_bible_markdown: "# Story Bible\n\nEvery ally has a visible cost.".into(),
            existing_characters: Vec::new(),
        }
    }
}
