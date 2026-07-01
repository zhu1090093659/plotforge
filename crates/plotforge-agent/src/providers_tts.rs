//! TTS provider port, deterministic fake, and TTS pipeline.
//!
//! Orchestrates scene/beat/character voice synthesis through a `TtsProvider`,
//! a `JobQueue`, and the `AssetRegistry`. Fallbacks register silent-audio
//! placeholder assets as fallback metadata and stay trace-visible.

use std::path::Path;

use plotforge_job::{JobClock, JobQueue, JobQueueError, JobRequest};
use plotforge_media::{AssetRegistry, MediaError};
use plotforge_schema::{
    AssetKind, AssetProviderMetadata, AssetRecord, AssetReference, AssetReferenceKind,
    AssetSourceKind, Beat, Character, JobFailure, JobKind, JobRecord, MediaAssetReference,
    RuntimeError, Scene, redact_trace_text,
};

use crate::shared::{insert_media_bytes, stable_prompt_hash};

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
            timeout_ms: crate::shared::TTS_JOB_TIMEOUT_MS,
            max_attempts: crate::shared::TTS_JOB_MAX_ATTEMPTS,
            estimated_cost_units: crate::shared::TTS_JOB_ESTIMATED_COST_UNITS,
        })?;
        jobs.start(&job.id)?;
        jobs.report_progress(&job.id, 0, 1, Some(tts_progress_message(&request)))?;

        match self.provider.synthesize(&request) {
            Ok(output) => {
                let asset_id = insert_media_bytes(
                    registry,
                    project_root,
                    plotforge_media::AssetRecordInput {
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
                    plotforge_media::AssetRecordInput {
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
