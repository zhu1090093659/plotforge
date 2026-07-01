//! Image provider port, deterministic fake, and scene image pipeline.
//!
//! Orchestrates scene background image generation through an `ImageProvider`,
//! a `JobQueue`, and the `AssetRegistry`. Fallbacks register placeholder
//! assets as fallback metadata (never as successful generated-cache hits) and
//! stay trace-visible.

use std::path::Path;

use plotforge_job::{JobClock, JobQueue, JobQueueError, JobRequest};
use plotforge_media::{AssetRegistry, MediaError};
use plotforge_schema::{
    AssetKind, AssetProviderMetadata, AssetRecord, AssetReference, AssetReferenceKind,
    AssetSourceKind, JobFailure, JobKind, JobRecord, RuntimeError, Scene, redact_trace_text,
};

use crate::shared::{SCENE_BACKGROUND_SLOT, insert_media_bytes, stable_prompt_hash};

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
            timeout_ms: crate::shared::IMAGE_JOB_TIMEOUT_MS,
            max_attempts: crate::shared::IMAGE_JOB_MAX_ATTEMPTS,
            estimated_cost_units: crate::shared::IMAGE_JOB_ESTIMATED_COST_UNITS,
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
                    plotforge_media::AssetRecordInput {
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
                    plotforge_media::AssetRecordInput {
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
