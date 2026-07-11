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
    /// Upstream returned a 429 (Too Many Requests). `retry_after_ms` carries
    /// the server-advised delay parsed from the `Retry-After` header, in
    /// milliseconds, when present. Mirrors the image/text provider pattern.
    RateLimit {
        retry_after_ms: Option<u64>,
    },
    /// Upstream flagged the TTS request as content-policy-filtered.
    /// Non-retryable: retrying with the same text reproduces the filter.
    ContentFiltered {
        reason: String,
    },
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

    /// 429 / rate-limit error. `retry_after_ms` is the parsed `Retry-After`
    /// header value (in ms) when the server supplied one.
    pub fn rate_limit(retry_after_ms: Option<u64>, message: impl Into<String>) -> Self {
        Self {
            kind: TtsProviderErrorKind::RateLimit { retry_after_ms },
            code: "tts_provider_rate_limit".into(),
            message: message.into(),
        }
    }

    /// Content-policy-filtered TTS response. Non-retryable. `reason` is the
    /// short provider-reported stop reason; it is a fixed enumerated token,
    /// not user content, so it is trace-safe.
    pub fn content_filtered(reason: impl Into<String>) -> Self {
        Self {
            kind: TtsProviderErrorKind::ContentFiltered {
                reason: reason.into(),
            },
            code: "tts_provider_content_filtered".into(),
            message: "content policy triggered; modify prompt".into(),
        }
    }

    fn retryable(&self) -> bool {
        matches!(
            self.kind,
            TtsProviderErrorKind::Provider
                | TtsProviderErrorKind::Timeout
                | TtsProviderErrorKind::RateLimit { .. }
        )
    }

    fn into_runtime_error(self) -> RuntimeError {
        match self.kind {
            TtsProviderErrorKind::Provider => RuntimeError::redacted(self.code, self.message),
            TtsProviderErrorKind::Timeout => RuntimeError::redacted(
                "tts_provider_timeout",
                format!("tts provider timed out: {}", self.message),
            ),
            TtsProviderErrorKind::RateLimit { .. } => {
                RuntimeError::redacted(self.code, self.message)
            }
            TtsProviderErrorKind::ContentFiltered { .. } => {
                RuntimeError::redacted(self.code, self.message)
            }
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

// ---------------------------------------------------------------------------
// OpenAI TTS client.
//
// A real `TtsProvider` backed by the OpenAI Audio Speech API
// (`POST /v1/audio/speech`). Sends `Authorization: Bearer <credential>` and
// receives raw audio bytes as the response body (not JSON). Credentials are
// resolved at call-time via an injected `ProviderCredentialResolver` (strict
// `EnvCredentialResolver` for auth-required providers,
// `OptionalEnvCredentialResolver` for local no-auth endpoints) — the value
// never lives in a struct field, trace, or the persisted registry. A 429
// surfaces `RateLimit` (with the parsed `Retry-After` in ms); an empty audio
// body surfaces an explicit error rather than a silent zero-byte asset.
// ---------------------------------------------------------------------------

/// The maximum time a single TTS HTTP call may take before it is treated as a
/// timeout. TTS synthesis is typically faster than image generation.
const TTS_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// A `TtsProvider` that talks to the OpenAI Audio Speech API
/// (`/v1/audio/speech`). The response body is raw audio bytes — NOT JSON.
/// Generic over the credential resolver so tests can inject a fake resolver
/// instead of mutating process-global env state.
#[derive(Clone, Debug)]
pub struct OpenAiTtsClient<R> {
    endpoint_url: String,
    model: String,
    default_voice: String,
    format: String,
    credential_env_var: String,
    credential_resolver: R,
    client: reqwest::blocking::Client,
}

impl<R> OpenAiTtsClient<R>
where
    R: crate::providers_text::ProviderCredentialResolver,
{
    /// Constructs a new TTS client from a registry entry and a credential
    /// resolver. The credential value is NOT resolved here — only at
    /// `synthesize()` time.
    pub fn from_entry(
        entry: &plotforge_schema::TtsProviderEntry,
        credential_resolver: R,
    ) -> Result<Self, TtsProviderError> {
        Self::new(
            &entry.endpoint_url,
            &entry.model,
            &entry.voice,
            &entry.format,
            &entry.credential_env_var,
            credential_resolver,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        endpoint_url: &str,
        model: &str,
        default_voice: &str,
        format: &str,
        credential_env_var: &str,
        credential_resolver: R,
    ) -> Result<Self, TtsProviderError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(TTS_HTTP_TIMEOUT)
            .connect_timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| {
                TtsProviderError::provider(
                    "tts_provider_http_client",
                    redact_trace_text(&error.to_string()),
                )
            })?;
        Ok(Self {
            endpoint_url: endpoint_url.into(),
            model: model.into(),
            default_voice: default_voice.into(),
            format: format.into(),
            credential_env_var: credential_env_var.into(),
            credential_resolver,
            client,
        })
    }
}

impl<R> TtsProvider for OpenAiTtsClient<R>
where
    R: crate::providers_text::ProviderCredentialResolver,
{
    fn synthesize(&self, request: &TtsRequest) -> Result<TtsProviderOutput, TtsProviderError> {
        // Resolve the credential at call time. A missing/empty credential for
        // an auth-required provider surfaces an explicit error (no silent
        // degraded run); the optional resolver yields an empty credential for
        // local no-auth endpoints.
        let credential = self
            .credential_resolver
            .resolve(&self.credential_env_var)
            .map_err(|_| {
                TtsProviderError::provider(
                    "tts_provider_missing_credential",
                    format!(
                        "tts provider credential env var `{}` is missing or empty; set it before synthesizing audio",
                        self.credential_env_var
                    ),
                )
            })?;
        let speech_url = format!("{}/audio/speech", self.endpoint_url.trim_end_matches('/'));
        let voice = if request.voice.is_empty() {
            self.default_voice.as_str()
        } else {
            request.voice.as_str()
        };
        let body = serde_json::json!({
            "model": self.model,
            "input": request.text,
            "voice": voice,
            "response_format": self.format,
        });
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        if !credential.trim().is_empty() {
            let value = reqwest::header::HeaderValue::from_str(&format!("Bearer {credential}"))
                .map_err(|_| {
                    TtsProviderError::provider(
                        "tts_provider_credential_header",
                        "provider credential contains bytes illegal in an HTTP header value",
                    )
                })?;
            headers.insert("Authorization", value);
        }
        let response = self
            .client
            .post(&speech_url)
            .headers(headers)
            .json(&body)
            .send()
            .map_err(|error| {
                if error.is_timeout() {
                    TtsProviderError::timeout(redact_trace_text(&error.to_string()))
                } else {
                    TtsProviderError::provider(
                        "tts_provider_http_send",
                        redact_trace_text(&error.to_string()),
                    )
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            // A 429 surfaces as `RateLimit` with the parsed `Retry-After` (ms)
            // so callers can honour server-advised backoff. Other non-2xx
            // statuses surface as generic `Provider` errors. No raw response
            // body enters the error text.
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let retry_after_ms = crate::shared::parse_retry_after(
                    response.headers().get(reqwest::header::RETRY_AFTER),
                );
                return Err(TtsProviderError::rate_limit(
                    retry_after_ms,
                    format!("tts provider returned HTTP {status}"),
                ));
            }
            return Err(TtsProviderError::provider(
                "tts_provider_http_status",
                format!("tts provider returned HTTP {status}"),
            ));
        }
        let bytes = response.bytes().map_err(|error| {
            TtsProviderError::provider(
                "tts_provider_http_body",
                redact_trace_text(&error.to_string()),
            )
        })?;
        // Guard against an empty audio body: a 200 OK with zero bytes is a
        // silent failure (a degenerate audio asset), not a successful
        // synthesis. Surface it explicitly so the pipeline's fallback path
        // registers a placeholder rather than a zero-byte file.
        if bytes.is_empty() {
            return Err(TtsProviderError::provider(
                "tts_provider_empty_body",
                "tts provider returned an empty audio body",
            ));
        }
        Ok(TtsProviderOutput::audio(
            bytes.to_vec(),
            "openai_tts",
            Some(self.model.clone()),
            None,
            1,
        ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn tts_capturing_server(
        reply_body: Vec<u8>,
    ) -> (
        std::net::SocketAddr,
        thread::JoinHandle<()>,
        std::sync::Arc<std::sync::Mutex<String>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let captured_clone = captured.clone();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = vec![0u8; 65536];
            let read = stream.read(&mut buf).expect("read request");
            let request = String::from_utf8_lossy(&buf[..read]).to_string();
            *captured_clone.lock().expect("capture lock") = request;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nContent-Length: {}\r\n\r\n",
                reply_body.len(),
            );
            stream
                .write_all(response.as_bytes())
                .expect("write headers");
            stream.write_all(&reply_body).expect("write body");
            let _ = stream.flush();
        });
        (addr, handle, captured)
    }

    fn sample_tts_request() -> TtsRequest {
        TtsRequest {
            target: TtsTarget::Scene {
                scene_key: "scene-1".into(),
            },
            text: "Hello world".into(),
            voice: "coral".into(),
            output_path: "assets/generated/audio/test.wav".into(),
            asset_kind: AssetKind::Audio,
        }
    }

    #[test]
    fn openai_tts_client_synthesizes_audio_bytes() {
        use crate::registry::OptionalEnvCredentialResolver;
        let fake_audio = b"fake-mp3-audio-bytes".to_vec();
        let (addr, handle, captured) = tts_capturing_server(fake_audio.clone());
        // Use a no-auth client so we don't need to set env vars
        let client = OpenAiTtsClient::new(
            &format!("http://{addr}"),
            "gpt-4o-mini-tts",
            "coral",
            "mp3",
            "", // empty credential_env_var = no auth
            OptionalEnvCredentialResolver,
        )
        .expect("construct client");
        let request = sample_tts_request();
        let output = client.synthesize(&request).expect("synthesize");
        assert_eq!(output.bytes, fake_audio);
        assert_eq!(output.provider, "openai_tts");
        assert_eq!(output.model, Some("gpt-4o-mini-tts".into()));
        // Verify the request body contained the right model, input, and voice
        let body = captured.lock().expect("capture lock").clone();
        assert!(
            body.contains("\"model\":\"gpt-4o-mini-tts\""),
            "body must contain model, got: {body}"
        );
        assert!(
            body.contains("\"input\":\"Hello world\""),
            "body must contain input text, got: {body}"
        );
        assert!(
            body.contains("\"voice\":\"coral\""),
            "body must contain voice, got: {body}"
        );
        handle.join().expect("server thread clean");
    }

    #[test]
    fn openai_tts_client_surfaces_missing_credential_explicitly() {
        use crate::providers_text::EnvCredentialResolver;
        let client = OpenAiTtsClient::new(
            "https://example.invalid",
            "gpt-4o-mini-tts",
            "coral",
            "mp3",
            "PLOTFORGE_TTS_TEST_MISSING_KEY",
            EnvCredentialResolver,
        )
        .expect("construct client");
        let request = sample_tts_request();
        let error = client.synthesize(&request).expect_err("missing credential");
        assert!(
            error.code.contains("missing_credential"),
            "error code must mention missing_credential, got: {}",
            error.code
        );
    }

    #[test]
    fn openai_tts_client_surfaces_http_error_status() {
        use crate::registry::OptionalEnvCredentialResolver;
        // A server that returns 500 Internal Server Error.
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = vec![0u8; 1024];
            let _ = stream.read(&mut buf);
            let response = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n";
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        let client = OpenAiTtsClient::new(
            &format!("http://{addr}"),
            "gpt-4o-mini-tts",
            "coral",
            "mp3",
            "",
            OptionalEnvCredentialResolver,
        )
        .expect("construct client");
        let request = sample_tts_request();
        let error = client.synthesize(&request).expect_err("http error");
        assert!(
            error.code.contains("http_status"),
            "error code must mention http_status, got: {}",
            error.code
        );
        handle.join().expect("server thread clean");
    }

    #[test]
    fn openai_tts_client_surfaces_rate_limit_with_retry_after() {
        use crate::registry::OptionalEnvCredentialResolver;
        // A server that returns 429 with a Retry-After header.
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = vec![0u8; 1024];
            let _ = stream.read(&mut buf);
            let response =
                "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 5\r\nContent-Length: 0\r\n\r\n";
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        let client = OpenAiTtsClient::new(
            &format!("http://{addr}"),
            "gpt-4o-mini-tts",
            "coral",
            "mp3",
            "",
            OptionalEnvCredentialResolver,
        )
        .expect("construct client");
        let request = sample_tts_request();
        let error = client.synthesize(&request).expect_err("rate limit");
        assert!(
            error.code.contains("rate_limit"),
            "error code must mention rate_limit, got: {}",
            error.code
        );
        assert_eq!(
            error.kind,
            TtsProviderErrorKind::RateLimit {
                retry_after_ms: Some(5000)
            },
            "429 must surface RateLimit with parsed retry_after_ms"
        );
        handle.join().expect("server thread clean");
    }

    #[test]
    fn openai_tts_client_rejects_empty_audio_body() {
        use crate::registry::OptionalEnvCredentialResolver;
        // A server that returns 200 OK with an empty body.
        let (addr, handle, _captured) = tts_capturing_server(Vec::new());
        let client = OpenAiTtsClient::new(
            &format!("http://{addr}"),
            "gpt-4o-mini-tts",
            "coral",
            "mp3",
            "",
            OptionalEnvCredentialResolver,
        )
        .expect("construct client");
        let request = sample_tts_request();
        let error = client.synthesize(&request).expect_err("empty body");
        assert!(
            error.code.contains("empty_body"),
            "error code must mention empty_body, got: {}",
            error.code
        );
        handle.join().expect("server thread clean");
    }
}
