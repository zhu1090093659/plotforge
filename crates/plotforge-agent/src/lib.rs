use std::{cell::RefCell, collections::BTreeMap};

use plotforge_job::{JobClock, JobQueue, JobQueueError, JobRequest};
use plotforge_media::{AssetRecordInput, AssetRegistry, MediaError};
use plotforge_schema::{
    AgentOutputProposal, AgentProposalPayload, AgentRole, AssetKind, AssetProviderMetadata,
    AssetRecord, AssetReference, AssetReferenceKind, AssetSourceKind, Beat, BeatDraftProposal,
    BeatDraftsProposal, Choice, JobFailure, JobKind, JobRecord, NarrativeFunction, NarrativeReview,
    ProjectData, ReviewProposal, RuntimeError, Scene, ScenePlanProposal, StoryState, WorldState,
    redact_trace_text,
};
use plotforge_storycraft::review_scene;
use sha2::{Digest, Sha256};

const IMAGE_JOB_TIMEOUT_MS: u64 = 60_000;
const IMAGE_JOB_MAX_ATTEMPTS: u32 = 2;
const IMAGE_JOB_ESTIMATED_COST_UNITS: u64 = 1;
const SCENE_BACKGROUND_SLOT: &str = "background_asset";

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
        character_ids: scene_plan.cast.clone(),
        plot_thread_updates: BTreeMap::new(),
        beats: beat_drafts
            .beats
            .iter()
            .map(|beat| Beat {
                id: beat.id.clone(),
                text: beat.text.clone(),
                choices: beat.choices.clone(),
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
    fn complete(
        &self,
        request: &TextModelRequest,
    ) -> Result<TextModelResponse, TextModelProviderError>;
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
                let asset_id = registry.insert_bytes(
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
                let asset_id = registry.insert_bytes(
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
                fallback_used,
                error,
            });
        }

        match provider_scene_plan(&self.text_provider, &request, &scene_key) {
            Ok((mut scene, review)) => {
                let image_request = SceneImageRequest::background(&scene);
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
                scene.background_asset = image_result.asset_record.export_path.clone();

                Ok(ScenePlan {
                    scene,
                    review,
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
    let digest = Sha256::digest(prompt.as_bytes());
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
                fallback_used,
                error,
            });
        }

        match self.provider_scene_plan(&request, &scene_key) {
            Ok((scene, review)) => Ok(ScenePlan {
                scene,
                review,
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
    ) -> Result<(Scene, NarrativeReview), ProviderPipelineError> {
        provider_scene_plan(&self.provider, request, scene_key)
    }
}

fn provider_scene_plan<P>(
    provider: &P,
    request: &ScenePlanRequest<'_>,
    scene_key: &str,
) -> Result<(Scene, NarrativeReview), ProviderPipelineError>
where
    P: TextModelProvider,
{
    let scene_plan = match complete_agent_output(
        provider,
        AgentRole::ScenePlanner,
        request,
        scene_key,
    )?
    .output
    {
        AgentProposalPayload::ScenePlan(scene_plan) => scene_plan,
        output => {
            return Err(ProviderPipelineError::Validation {
                agent: AgentRole::ScenePlanner,
                message: format!("unexpected payload `{}`", payload_kind(&output)),
            });
        }
    };
    let beat_drafts =
        match complete_agent_output(provider, AgentRole::BeatWriter, request, scene_key)?.output {
            AgentProposalPayload::BeatDrafts(beat_drafts) => beat_drafts,
            output => {
                return Err(ProviderPipelineError::Validation {
                    agent: AgentRole::BeatWriter,
                    message: format!("unexpected payload `{}`", payload_kind(&output)),
                });
            }
        };
    let review =
        match complete_agent_output(provider, AgentRole::PlotDoctor, request, scene_key)?.output {
            AgentProposalPayload::Review(review) => review,
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

    Ok((scene, review.review))
}

fn complete_agent_output<P>(
    provider: &P,
    agent: AgentRole,
    request: &ScenePlanRequest<'_>,
    scene_key: &str,
) -> Result<AgentOutputProposal, ProviderPipelineError>
where
    P: TextModelProvider,
{
    let model_request = TextModelRequest {
        call_id: format!("{}-{}", scene_key, payload_call_suffix(&agent)),
        agent: agent.clone(),
        scene_key: scene_key.to_string(),
        prompt: text_model_prompt(&agent, request, scene_key),
    };
    let response = provider
        .complete(&model_request)
        .map_err(ProviderPipelineError::Provider)?;
    let proposal =
        serde_json::from_str::<AgentOutputProposal>(&response.raw_json).map_err(|error| {
            ProviderPipelineError::InvalidJson {
                agent: agent.clone(),
                message: error.to_string(),
            }
        })?;
    validate_agent_output_proposal(&proposal).map_err(|error| {
        ProviderPipelineError::Validation {
            agent,
            message: format!("{error:?}"),
        }
    })?;

    Ok(proposal)
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

    fn with_failure(agent: AgentRole, kind: FakeTextModelFailureKind) -> Self {
        Self {
            failure: Some(FakeTextModelFailure { agent, kind }),
        }
    }
}

impl TextModelProvider for FakeTextModelProvider {
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
}

fn fake_success_response(
    request: &TextModelRequest,
) -> Result<TextModelResponse, TextModelProviderError> {
    let proposal = match request.agent {
        AgentRole::ScenePlanner => AgentOutputProposal {
            id: format!("{}-scene-plan", request.call_id),
            agent: AgentRole::ScenePlanner,
            output: AgentProposalPayload::ScenePlan(ScenePlanProposal {
                scene_key: request.scene_key.clone(),
                title: "Provider Planned Scene".into(),
                location: "Qianqing Palace".into(),
                scene_summary: "A fake text model proposes the next court crisis.".into(),
                dramatic_purpose:
                    "Exercise the provider-backed scene pipeline without external network calls."
                        .into(),
                hook: "The fake provider returns a schema-checked scene plan.".into(),
                emotional_goal: Some("controlled generation".into()),
                cast: vec!["grand-secretary".into(), "eunuch-director".into()],
                entry_beat_id: format!("{}-beat-001", request.scene_key),
                background_asset: Some(format!("assets/generated/{}.png", request.scene_key)),
            }),
        },
        AgentRole::BeatWriter => AgentOutputProposal {
            id: format!("{}-beats", request.call_id),
            agent: AgentRole::BeatWriter,
            output: AgentProposalPayload::BeatDrafts(BeatDraftsProposal {
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
                        dramatic_purpose: "Let the engine continue from provider output.".into(),
                        change_scene: false,
                    }],
                    narrative_function: NarrativeFunction::Hook,
                }],
            }),
        },
        AgentRole::PlotDoctor => AgentOutputProposal {
            id: format!("{}-review", request.call_id),
            agent: AgentRole::PlotDoctor,
            output: AgentProposalPayload::Review(ReviewProposal {
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
            }),
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

    encode_fake_response(&proposal)
}

fn fake_invalid_schema_response(
    request: &TextModelRequest,
) -> Result<TextModelResponse, TextModelProviderError> {
    let proposal = match request.agent {
        AgentRole::ScenePlanner => AgentOutputProposal {
            id: format!("{}-invalid-scene-plan", request.call_id),
            agent: AgentRole::ScenePlanner,
            output: AgentProposalPayload::ScenePlan(ScenePlanProposal {
                scene_key: String::new(),
                title: "Invalid Scene Plan".into(),
                location: "Qianqing Palace".into(),
                scene_summary: "This payload is structurally valid JSON but fails validation."
                    .into(),
                dramatic_purpose: "Test schema validation failure.".into(),
                hook: "A missing scene key should fail.".into(),
                emotional_goal: None,
                cast: Vec::new(),
                entry_beat_id: "missing-entry".into(),
                background_asset: None,
            }),
        },
        AgentRole::BeatWriter => AgentOutputProposal {
            id: format!("{}-invalid-beats", request.call_id),
            agent: AgentRole::BeatWriter,
            output: AgentProposalPayload::BeatDrafts(BeatDraftsProposal {
                scene_key: request.scene_key.clone(),
                beats: Vec::new(),
            }),
        },
        AgentRole::PlotDoctor => AgentOutputProposal {
            id: format!("{}-invalid-review", request.call_id),
            agent: AgentRole::PlotDoctor,
            output: AgentProposalPayload::Review(ReviewProposal {
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
            }),
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

    encode_fake_response(&proposal)
}

fn encode_fake_response(
    proposal: &AgentOutputProposal,
) -> Result<TextModelResponse, TextModelProviderError> {
    serde_json::to_string(proposal)
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
                fallback_used,
                error,
            });
        }

        let scene = dynasty_scene(next_turn, &request);
        let review = review_scene(
            &scene,
            &request.project.story_craft,
            &request.project.characters,
        );
        Ok(ScenePlan {
            scene,
            review,
            fallback_used: false,
            error: None,
        })
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
    use plotforge_job::{JobClock, JobQueue};
    use plotforge_media::AssetRegistry;
    use plotforge_schema::{
        AgentOutputProposal, AgentProposalPayload, AgentRole, AssetReferenceKind, AssetSourceKind,
        BeatDraftProposal, BeatDraftsProposal, Choice, GameProject, JobStatus, NarrativeFunction,
        NarrativeReview, REDACTED_TRACE_SECRET, ReviewProposal, ScenePlanProposal, Severity,
        StoryState, WorldState,
    };

    use super::{
        AgentProposalValidationError, FakeImageProvider, FakeTextModelProvider,
        ImageProviderAgentPipeline, MockAgentPipeline, ProviderAgentPipeline, SceneImagePipeline,
        SceneImageRequest, ScenePlanRequest, ScenePlanner, scene_from_proposals,
        validate_agent_output_proposal,
    };

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

        let plan = MockAgentPipeline
            .plan_next_scene(ScenePlanRequest {
                project: &project,
                story_state: &project.story_state,
                world_state: &project.world_state,
                player_input: "Raise the levy",
                action_type: "raise_tax",
            })
            .expect("plan");

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
        let mut project = plotforge_storage::dynasty_embers_project();
        project.story_state.turn = 1;
        let pipeline = MockAgentPipeline;

        for (action_type, expected_thread) in [
            ("raise_tax", "tax-disorder"),
            ("inspect_corruption", "court-insider"),
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

        assert_eq!(scene.key, "court-crisis-002");
        assert_eq!(scene.beats[0].id, "court-crisis-002-beat-001");
        assert_eq!(scene.character_ids, vec!["grand-secretary"]);
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
        let project = plotforge_storage::dynasty_embers_project();
        let pipeline = ProviderAgentPipeline::new(FakeTextModelProvider::success());

        let plan = pipeline
            .plan_next_scene(provider_request(&project))
            .expect("provider plan");

        assert_eq!(plan.scene.key, "provider-scene-001");
        assert_eq!(plan.review.scene_key, "provider-scene-001");
        assert_eq!(plan.review.score, 96);
        assert!(!plan.fallback_used);
        assert!(plan.error.is_none());
        assert!(plan.scene.plot_thread_updates.is_empty());
    }

    #[test]
    fn fake_text_provider_pipeline_falls_back_on_provider_error() {
        let project = plotforge_storage::dynasty_embers_project();
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
        let project = plotforge_storage::dynasty_embers_project();
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
        let project = plotforge_storage::dynasty_embers_project();
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
    fn fake_text_provider_pipeline_falls_back_on_invalid_json() {
        let project = plotforge_storage::dynasty_embers_project();
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
    fn image_provider_agent_pipeline_generates_scene_background_asset() {
        let project = plotforge_storage::dynasty_embers_project();
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
        let project = plotforge_storage::dynasty_embers_project();
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
            output: AgentProposalPayload::ScenePlan(sample_scene_plan_proposal()),
        }
    }

    fn sample_beat_drafts_output_proposal() -> AgentOutputProposal {
        AgentOutputProposal {
            id: "beat-drafts-proposal-001".into(),
            agent: AgentRole::BeatWriter,
            output: AgentProposalPayload::BeatDrafts(sample_beat_drafts_proposal()),
        }
    }

    fn sample_review_output_proposal() -> AgentOutputProposal {
        AgentOutputProposal {
            id: "review-proposal-001".into(),
            agent: AgentRole::PlotDoctor,
            output: AgentProposalPayload::Review(sample_review_proposal()),
        }
    }

    fn sample_scene_plan_proposal() -> ScenePlanProposal {
        ScenePlanProposal {
            scene_key: "court-crisis-002".into(),
            title: "Tax Resistance Memorials".into(),
            location: "Qianqing Palace".into(),
            scene_summary: "The levy creates immediate provincial resistance.".into(),
            dramatic_purpose: "Show the cost of emergency revenue.".into(),
            hook: "Three memorials arrive with broken tax seals.".into(),
            emotional_goal: Some("consequence".into()),
            cast: vec!["grand-secretary".into()],
            entry_beat_id: "court-crisis-002-beat-001".into(),
            background_asset: Some("assets/generated/court-crisis-002.png".into()),
        }
    }

    fn sample_beat_drafts_proposal() -> BeatDraftsProposal {
        BeatDraftsProposal {
            scene_key: "court-crisis-002".into(),
            beats: vec![sample_beat_draft_proposal()],
        }
    }

    fn sample_beat_draft_proposal() -> BeatDraftProposal {
        BeatDraftProposal {
            id: "court-crisis-002-beat-001".into(),
            scene_key: "court-crisis-002".into(),
            text: "The court reads three provincial reports in silence.".into(),
            choices: vec![Choice {
                id: "inspect-corruption".into(),
                label: "Investigate the collectors".into(),
                action_type: "inspect_corruption".into(),
                dramatic_purpose: "Trade court stability for cleaner revenue.".into(),
                change_scene: true,
            }],
            narrative_function: NarrativeFunction::Hook,
        }
    }

    fn sample_review_proposal() -> ReviewProposal {
        ReviewProposal {
            scene_key: "court-crisis-002".into(),
            review: NarrativeReview {
                scene_key: "court-crisis-002".into(),
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
                scene_key: Some("court-crisis-002".into()),
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

    fn scene_image_request() -> SceneImageRequest {
        SceneImageRequest {
            scene_key: "scene-one".into(),
            prompt: "paint a tense court hearing".into(),
            output_path: "assets/generated/scene-one.png".into(),
        }
    }
}
