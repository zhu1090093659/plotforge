//! Image provider port, deterministic fake, and scene image pipeline.
//!
//! Orchestrates scene background image generation through an `ImageProvider`,
//! a `JobQueue`, and the `AssetRegistry`. Fallbacks register placeholder
//! assets as fallback metadata (never as successful generated-cache hits) and
//! stay trace-visible.

use std::path::Path;

use base64::Engine;
use plotforge_job::{JobClock, JobQueue, JobQueueError, JobRequest};
use plotforge_media::{AssetRegistry, MediaError};
use plotforge_schema::{
    AssetKind, AssetProviderMetadata, AssetRecord, AssetReference, AssetReferenceKind,
    AssetSourceKind, ImageProviderEntry, JobFailure, JobKind, JobRecord, RuntimeError, Scene,
    redact_trace_text,
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
    /// Upstream returned a 429 (Too Many Requests). `retry_after_ms` carries
    /// the server-advised delay parsed from the `Retry-After` header, in
    /// milliseconds, when present. It is `None` if the header was absent or
    /// unparseable — the caller then falls back to its own backoff.
    RateLimit {
        retry_after_ms: Option<u64>,
    },
    /// Upstream flagged the image request as content-policy-filtered. Carries
    /// a short provider-reported reason token (not user content, so it is
    /// trace-safe). Non-retryable: retrying with the same prompt reproduces
    /// the filter.
    ContentFiltered {
        reason: String,
    },
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

    /// 429 / rate-limit error. `retry_after_ms` is the parsed `Retry-After`
    /// header value (in ms) when the server supplied one.
    pub fn rate_limit(retry_after_ms: Option<u64>, message: impl Into<String>) -> Self {
        Self {
            kind: ImageProviderErrorKind::RateLimit { retry_after_ms },
            code: "image_provider_rate_limit".into(),
            message: message.into(),
        }
    }

    /// Content-policy-filtered image response. Non-retryable. `reason` is the
    /// short provider-reported stop reason (e.g. `"content_filter"`); it is a
    /// fixed enumerated token, not user content, so it is trace-safe.
    pub fn content_filtered(reason: impl Into<String>) -> Self {
        Self {
            kind: ImageProviderErrorKind::ContentFiltered {
                reason: reason.into(),
            },
            code: "image_provider_content_filtered".into(),
            message: "content policy triggered; modify prompt".into(),
        }
    }

    fn retryable(&self) -> bool {
        matches!(
            self.kind,
            ImageProviderErrorKind::Provider
                | ImageProviderErrorKind::Timeout
                | ImageProviderErrorKind::RateLimit { .. }
        )
    }

    fn into_runtime_error(self) -> RuntimeError {
        match self.kind {
            ImageProviderErrorKind::Provider => RuntimeError::redacted(self.code, self.message),
            ImageProviderErrorKind::Timeout => RuntimeError::redacted(
                "image_provider_timeout",
                format!("image provider timed out: {}", self.message),
            ),
            ImageProviderErrorKind::RateLimit { .. } => {
                RuntimeError::redacted(self.code, self.message)
            }
            ImageProviderErrorKind::ContentFiltered { .. } => {
                RuntimeError::redacted(self.code, self.message)
            }
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

/// Blanket impl so a `Box<dyn ImageProvider>` can be used wherever an
/// `ImageProvider` is expected (e.g. `SceneImagePipeline<Box<dyn
/// ImageProvider>>`). This lets the registry's `build_image_provider` return
/// a boxed trait object and the studio layer feed it straight into the
/// pipeline without an extra adapter layer.
impl ImageProvider for Box<dyn ImageProvider> {
    fn generate(
        &self,
        request: &ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse, ImageProviderError> {
        (**self).generate(request)
    }
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

// ---------------------------------------------------------------------------
// OpenAI Images API client (`POST /v1/images/generations`).
//
// `OpenAiImageClient` implements the `ImageProvider` trait against the
// OpenAI-compatible Images API. It is configured from an
// `ImageProviderEntry` (model, default_size, default_quality,
// credential_env_var, endpoint_url) and resolves the credential at call time
// via the strict `EnvCredentialResolver` — the value never lives as a struct
// field, never enters traces, and never enters the persisted registry. A
// missing/empty credential surfaces an explicit error (no silent degraded
// run); a local no-auth image endpoint may use an empty
// `credential_env_var`, in which case the optional resolver yields an empty
// credential and the auth header is omitted.
//
// Image generation is slower than text, so the client uses a dedicated
// `reqwest::blocking::Client` with a 120s timeout (vs. the 60s shared text
// client). The response carries either `data[0].b64_json` (base64-encoded
// image bytes, decoded here) or `data[0].url` (fetched via the shared
// blocking client). A 429 surfaces `RateLimit` (with the parsed `Retry-After`
// in ms); a content-filter `data[0].content_filter` field or a non-2xx with a
// content_filter reason surfaces `ContentFiltered`. No raw response body
// enters traces — only redaction-safe codes and messages.
// ---------------------------------------------------------------------------

/// The maximum time a single image generation HTTP call may take before it is
/// treated as a timeout. Image generation is materially slower than text
/// completion (diffusion models), so this is double the text-client timeout.
const IMAGE_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
/// A short connect-only timeout so unreachable image endpoints fail fast
/// instead of holding a worker for the full 120s.
const IMAGE_HTTP_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// A `reqwest::blocking::Client` tuned for image generation (120s timeout).
/// Built once per `OpenAiImageClient` and reused across calls. Redirects are
/// disabled for the same credential-leak reason as the text client (H4).
fn image_blocking_client() -> Result<reqwest::blocking::Client, ImageProviderError> {
    reqwest::blocking::Client::builder()
        .timeout(IMAGE_HTTP_TIMEOUT)
        .connect_timeout(IMAGE_HTTP_CONNECT_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| {
            ImageProviderError::provider(
                "image_provider_http_client",
                redact_trace_text(&error.to_string()),
            )
        })
}

/// The decoded shape of an OpenAI Images API response `data[0]` entry. Only
/// the fields the client reads are parsed; `content_filter` is checked before
/// `b64_json`/`url` so a filtered response surfaces as `ContentFiltered`
/// rather than a confusing missing-image decode error.
#[derive(Clone, Debug, serde::Deserialize)]
struct OpenAiImageDataEntry {
    #[serde(default)]
    b64_json: Option<String>,
    #[serde(default)]
    url: Option<String>,
    /// OpenAI surfaces content-policy rejections as a boolean
    /// `content_filter` field on the data entry. When `true` the response has
    /// no image payload.
    #[serde(default)]
    content_filter: Option<bool>,
}

/// The top-level OpenAI Images API response shape: `{ "data": [ { ... } ] }`.
/// A content-filter rejection may also carry a top-level `error` object; that
/// path is handled by the non-2xx branch in `execute_image` before this
/// struct is parsed.
#[derive(Clone, Debug, serde::Deserialize)]
struct OpenAiImageResponse {
    data: Vec<OpenAiImageDataEntry>,
}

/// A real `ImageProvider` backed by the OpenAI Images API
/// (`POST {endpoint}/images/generations`).
///
/// Constructed from an `ImageProviderEntry` plus a credential resolver. The
/// credential resolver is injected (strict `EnvCredentialResolver` for
/// auth-required providers, `OptionalEnvCredentialResolver` for local no-auth
/// endpoints) so resolution runs at call time and the value never lives in
/// the struct. The request body is
/// `{ "model", "prompt", "size", "quality", "output_format": "png", "n": 1 }`;
/// the response is parsed for `data[0].b64_json` (decoded) or `data[0].url`
/// (fetched).
#[derive(Clone)]
pub struct OpenAiImageClient<R> {
    endpoint_url: String,
    model: String,
    default_size: String,
    default_quality: String,
    credential_env_var: String,
    credential_resolver: R,
    client: reqwest::blocking::Client,
}

impl<R> OpenAiImageClient<R>
where
    R: crate::providers_text::ProviderCredentialResolver,
{
    /// Builds a client from an `ImageProviderEntry` and a credential resolver.
    /// The HTTP client is constructed eagerly so a build failure (rare)
    /// surfaces before the first image call rather than mid-turn. The
    /// credential value is NOT resolved here — only at `generate()` time.
    pub fn new(
        entry: &ImageProviderEntry,
        credential_resolver: R,
    ) -> Result<Self, ImageProviderError> {
        let client = image_blocking_client()?;
        Ok(Self {
            endpoint_url: entry.endpoint_url.clone(),
            model: entry.model.clone(),
            default_size: entry.default_size.clone(),
            default_quality: entry.default_quality.clone(),
            credential_env_var: entry.credential_env_var.clone(),
            credential_resolver,
            client,
        })
    }

    /// The endpoint-relative path for the OpenAI Images API generations
    /// endpoint. Joined onto `endpoint_url` (normalising trailing slashes).
    fn generations_url(&self) -> String {
        let trimmed = self.endpoint_url.trim_end_matches('/');
        format!("{trimmed}/images/generations")
    }
}

impl<R> ImageProvider for OpenAiImageClient<R>
where
    R: crate::providers_text::ProviderCredentialResolver,
{
    fn generate(
        &self,
        request: &ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse, ImageProviderError> {
        // Resolve the credential at call time. A missing/empty credential for
        // an auth-required provider surfaces an explicit error (no silent
        // degraded run); the optional resolver yields an empty credential for
        // local no-auth endpoints.
        let credential = self
            .credential_resolver
            .resolve(&self.credential_env_var)
            .map_err(|_| ImageProviderError::provider(
                "image_provider_missing_credential",
                format!(
                    "image provider credential env var `{}` is missing or empty; set it before generating images",
                    self.credential_env_var
                ),
            ))?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        if !credential.trim().is_empty() {
            let value = reqwest::header::HeaderValue::from_str(&format!("Bearer {credential}"))
                .map_err(|_| {
                    ImageProviderError::provider(
                        "image_provider_credential_header",
                        "provider credential contains bytes illegal in an HTTP header value",
                    )
                })?;
            headers.insert(reqwest::header::AUTHORIZATION, value);
        }

        let body = serde_json::json!({
            "model": self.model,
            "prompt": request.prompt,
            "size": self.default_size,
            "quality": self.default_quality,
            "output_format": "png",
            "n": 1,
        });

        let builder = self
            .client
            .post(self.generations_url())
            .headers(headers)
            .json(&body);
        let text = execute_image(builder)?;
        let parsed: OpenAiImageResponse = serde_json::from_str(&text).map_err(|error| {
            ImageProviderError::provider(
                "image_provider_http_decode",
                redact_trace_text(&format!("images response was not JSON: {error}")),
            )
        })?;
        let entry = parsed.data.into_iter().next().ok_or_else(|| {
            ImageProviderError::provider(
                "image_provider_http_decode",
                "images response missing data[0]",
            )
        })?;
        // Content filter: check before attempting to extract bytes so a
        // filtered response surfaces a clear, actionable error.
        if entry.content_filter.unwrap_or(false) {
            return Err(ImageProviderError::content_filtered("content_filter"));
        }
        // Prefer b64_json (inline); fall back to fetching data[0].url via the
        // shared blocking client.
        let bytes = if let Some(b64) = entry.b64_json.as_deref() {
            decode_b64_image(b64)?
        } else if let Some(url) = entry.url.as_deref() {
            fetch_image_url(url)?
        } else {
            return Err(ImageProviderError::provider(
                "image_provider_http_decode",
                "images response data[0] has neither b64_json nor url",
            ));
        };

        Ok(ImageGenerationResponse::png(
            bytes,
            "openai-image",
            Some(self.model.clone()),
            None,
            1,
        ))
    }
}

/// Executes an image request, mapping transport/status failures into
/// redaction-safe `ImageProviderError`s. A 429 surfaces `RateLimit` with the
/// parsed `Retry-After` (ms); other non-2xx surface as `Provider` with the
/// status code in the message. Timeouts surface as `Timeout`. No raw response
/// body enters the error text.
fn execute_image(request: reqwest::blocking::RequestBuilder) -> Result<String, ImageProviderError> {
    let response = request.send().map_err(|error| {
        if error.is_timeout() {
            ImageProviderError::timeout(redact_trace_text(&error.to_string()))
        } else {
            ImageProviderError::provider(
                "image_provider_http_send",
                redact_trace_text(&error.to_string()),
            )
        }
    })?;
    let status = response.status();
    if !status.is_success() {
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after_ms =
                parse_image_retry_after(response.headers().get(reqwest::header::RETRY_AFTER));
            return Err(ImageProviderError::rate_limit(
                retry_after_ms,
                format!("provider returned HTTP {status}"),
            ));
        }
        return Err(ImageProviderError::provider(
            "image_provider_http_status",
            format!("provider returned HTTP {status}"),
        ));
    }
    response.text().map_err(|error| {
        ImageProviderError::provider(
            "image_provider_http_body",
            redact_trace_text(&error.to_string()),
        )
    })
}

/// Parses an HTTP `Retry-After` header value into milliseconds. Mirrors the
/// text-client parser: supports delta-seconds and HTTP-date forms. Returns
/// `None` when the header is absent or unparseable.
fn parse_image_retry_after(header: Option<&reqwest::header::HeaderValue>) -> Option<u64> {
    let value = header?;
    let value = value.to_str().ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Some(seconds.saturating_mul(1000));
    }
    // HTTP-date form: delegate to the shared text-client parser via a
    // re-implementation here to keep the image module self-contained (the
    // text parser is private to providers_http).
    image_httpdate_to_system_time(trimmed).and_then(|date| {
        let now = std::time::SystemTime::now();
        match date.duration_since(now) {
            Ok(duration) => Some(duration.as_millis().try_into().ok()?),
            Err(_) => Some(0),
        }
    })
}

/// Parses an RFC 7231 HTTP-date into a `SystemTime`. Accepts the IMF-fixdate
/// form providers send (`Wed, 21 Oct 2026 07:28:00 GMT`).
fn image_httpdate_to_system_time(value: &str) -> Option<std::time::SystemTime> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() != 6 {
        return None;
    }
    let day: u32 = parts[1].parse().ok()?;
    let month = image_month_index(parts[2])?;
    let year: i32 = parts[3].parse().ok()?;
    let time_parts: Vec<&str> = parts[4].split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour: u32 = time_parts[0].parse().ok()?;
    let minute: u32 = time_parts[1].parse().ok()?;
    let second: u32 = time_parts[2].parse().ok()?;
    if parts[5] != "GMT" {
        return None;
    }
    let epoch_seconds = image_days_from_civil(year, month, day)? as i64 * 86_400
        + (hour as i64 * 3600)
        + (minute as i64 * 60)
        + second as i64;
    let duration = std::time::Duration::from_secs(epoch_seconds.max(0) as u64);
    Some(std::time::SystemTime::UNIX_EPOCH + duration)
}

fn image_month_index(name: &str) -> Option<u32> {
    match name {
        "Jan" => Some(1),
        "Feb" => Some(2),
        "Mar" => Some(3),
        "Apr" => Some(4),
        "May" => Some(5),
        "Jun" => Some(6),
        "Jul" => Some(7),
        "Aug" => Some(8),
        "Sep" => Some(9),
        "Oct" => Some(10),
        "Nov" => Some(11),
        "Dec" => Some(12),
        _ => None,
    }
}

/// Howard Hinnant's days-from-civil algorithm (mirrors the text client).
fn image_days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m = month as i32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe as i64 * 365 + yoe as i64 / 4 - yoe as i64 / 100 + doy as i64;
    Some(era as i64 * 146_097 + doe - 719_468)
}

/// Decodes a base64-encoded image body into raw bytes. Surfaces an explicit
/// error (never a silent empty result) when the payload is malformed.
fn decode_b64_image(b64: &str) -> Result<Vec<u8>, ImageProviderError> {
    base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|error| {
            ImageProviderError::provider(
                "image_provider_b64_decode",
                redact_trace_text(&format!("failed to decode b64_json: {error}")),
            )
        })
}

/// Fetches the image bytes at a `data[0].url` using the shared blocking
/// client. The URL is provider-issued and short-lived; the bytes are returned
/// directly to the caller (no disk write here). Transport failures surface as
/// redaction-safe `Provider` errors.
fn fetch_image_url(url: &str) -> Result<Vec<u8>, ImageProviderError> {
    let client = crate::providers_http::shared_blocking_client().map_err(|error| {
        ImageProviderError::provider(
            "image_provider_http_client",
            redact_trace_text(&error.message),
        )
    })?;
    let response = client.get(url).send().map_err(|error| {
        if error.is_timeout() {
            ImageProviderError::timeout(redact_trace_text(&error.to_string()))
        } else {
            ImageProviderError::provider(
                "image_provider_url_fetch",
                redact_trace_text(&error.to_string()),
            )
        }
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(ImageProviderError::provider(
            "image_provider_url_fetch",
            format!("image url returned HTTP {status}"),
        ));
    }
    let bytes = response.bytes().map_err(|error| {
        ImageProviderError::provider(
            "image_provider_url_fetch",
            redact_trace_text(&error.to_string()),
        )
    })?;
    if bytes.is_empty() {
        return Err(ImageProviderError::provider(
            "image_provider_url_fetch",
            "image url returned empty body",
        ));
    }
    Ok(bytes.to_vec())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers_text::EnvCredentialResolver;
    use crate::registry::OptionalEnvCredentialResolver;
    use plotforge_schema::ImageProviderEntry;

    /// A capturing in-process TCP server for image-client tests. Mirrors the
    /// `capturing_server` helper in `providers_http.rs`: it reads the full
    /// request into a shared buffer (so tests can assert on the request body)
    /// and replies with `reply_body` as the 200 response. Returns the bound
    /// address, the join handle, and the captured request buffer.
    fn image_capturing_server(
        reply_body: String,
    ) -> (
        std::net::SocketAddr,
        std::thread::JoinHandle<()>,
        std::sync::Arc<std::sync::Mutex<String>>,
    ) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let captured_clone = captured.clone();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = vec![0u8; 65536];
            let read = stream.read(&mut buf).expect("read request");
            let request = String::from_utf8_lossy(&buf[..read]).to_string();
            *captured_clone.lock().expect("capture lock") = request;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                reply_body.len(),
                reply_body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        (addr, handle, captured)
    }

    /// A one-shot TCP server that replies with a fixed status line + body
    /// (used for non-2xx error tests). Returns the address + join handle.
    fn image_status_server(
        status_line: String,
        body: String,
        extra_headers: &str,
    ) -> (std::net::SocketAddr, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let extra = extra_headers.to_string();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let response = format!(
                "{status_line}Content-Length: {}\r\n{extra}\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        (addr, handle)
    }

    fn sample_image_entry(endpoint: &str, env_var: &str) -> ImageProviderEntry {
        ImageProviderEntry {
            id: "openai-image".into(),
            endpoint_url: endpoint.into(),
            model: "gpt-image-1".into(),
            credential_env_var: env_var.into(),
            enabled: true,
            default_size: "1024x1024".into(),
            default_quality: "medium".into(),
        }
    }

    fn sample_image_request() -> ImageGenerationRequest {
        ImageGenerationRequest {
            scene_key: "scene-1".into(),
            prompt: "a misty courtroom at dawn".into(),
            output_path: "assets/generated/scene-1.png".into(),
        }
    }

    #[test]
    fn openai_image_client_decodes_b64_json_response() {
        let png_bytes = b"\x89PNG\r\n\x1a\nfake-image-bytes".to_vec();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
        let body = serde_json::json!({
            "data": [{ "b64_json": b64 }]
        })
        .to_string();
        let (addr, handle, captured) = image_capturing_server(body);
        let env_var = "PLOTFORGE_T31_IMAGE_B64_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let entry = sample_image_entry(&format!("http://{addr}"), env_var);
        let client = OpenAiImageClient::new(&entry, EnvCredentialResolver).expect("client builds");
        let response = client.generate(&sample_image_request()).expect("generate");
        handle.join().expect("server thread clean");
        assert_eq!(response.bytes, png_bytes);
        assert_eq!(response.provider, "openai-image");
        assert_eq!(response.model.as_deref(), Some("gpt-image-1"));
        // Redaction: the request body must carry the model/prompt/size/quality
        // fields and the Authorization header must be present (not the raw
        // credential echoed back).
        let request = captured.lock().expect("capture lock").clone();
        assert!(
            request.contains("\"model\":\"gpt-image-1\""),
            "model in body: {request}"
        );
        assert!(
            request.contains("\"size\":\"1024x1024\""),
            "size in body: {request}"
        );
        assert!(
            request.contains("\"quality\":\"medium\""),
            "quality in body: {request}"
        );
        assert!(
            request.contains("\"output_format\":\"png\""),
            "output_format in body: {request}"
        );
        assert!(request.contains("\"n\":1"), "n=1 in body: {request}");
        // reqwest canonicalises header names to lowercase; assert the bearer
        // auth header is present (case-insensitive) and the raw credential is
        // not echoed in the request body.
        let request_lower = request.to_ascii_lowercase();
        assert!(
            request_lower.contains("authorization: bearer "),
            "auth header present (case-insensitive), got: {request}"
        );
        assert!(
            !request.contains("test-credential\r\n\r\n"),
            "credential not in body"
        );
        unsafe { std::env::remove_var(env_var) };
    }

    #[test]
    fn openai_image_client_fetches_url_when_no_b64_json() {
        // When data[0] carries only a url (no b64_json), the client fetches
        // the image bytes from that url via the shared blocking client. We
        // serve the image bytes from a second one-shot server.
        use std::io::{Read, Write};
        let png_bytes = b"\x89PNG\r\n\x1a\nurl-fetched-bytes".to_vec();
        let image_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind image");
        let image_addr = image_listener.local_addr().expect("image addr");
        let image_bytes = png_bytes.clone();
        let image_handle = std::thread::spawn(move || {
            let (mut stream, _) = image_listener.accept().expect("accept image");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\n\r\n",
                image_bytes.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(&image_bytes);
            let _ = stream.flush();
        });
        let body = serde_json::json!({
            "data": [{ "url": format!("http://{image_addr}/img.png") }]
        })
        .to_string();
        let (addr, handle, _captured) = image_capturing_server(body);
        let env_var = "PLOTFORGE_T31_IMAGE_URL_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let entry = sample_image_entry(&format!("http://{addr}"), env_var);
        let client = OpenAiImageClient::new(&entry, EnvCredentialResolver).expect("client builds");
        let response = client.generate(&sample_image_request()).expect("generate");
        handle.join().expect("generations server clean");
        image_handle.join().expect("image server clean");
        assert_eq!(response.bytes, png_bytes);
        unsafe { std::env::remove_var(env_var) };
    }

    #[test]
    fn openai_image_client_429_surfaces_rate_limit() {
        let (addr, handle) = image_status_server(
            "HTTP/1.1 429 Too Many Requests\r\n".into(),
            String::new(),
            "Retry-After: 5\r\n",
        );
        let env_var = "PLOTFORGE_T31_IMAGE_429_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let entry = sample_image_entry(&format!("http://{addr}"), env_var);
        let client = OpenAiImageClient::new(&entry, EnvCredentialResolver).expect("client builds");
        let error = client
            .generate(&sample_image_request())
            .expect_err("429 must error");
        handle.join().expect("server thread clean");
        assert!(
            matches!(
                error.kind,
                ImageProviderErrorKind::RateLimit {
                    retry_after_ms: Some(5000)
                }
            ),
            "expected RateLimit{{retry_after_ms:Some(5000)}}, got {error:?}"
        );
        assert_eq!(error.code, "image_provider_rate_limit");
        unsafe { std::env::remove_var(env_var) };
    }

    #[test]
    fn openai_image_client_429_without_retry_after_is_none() {
        let (addr, handle) = image_status_server(
            "HTTP/1.1 429 Too Many Requests\r\n".into(),
            String::new(),
            "",
        );
        let env_var = "PLOTFORGE_T31_IMAGE_429_NO_HEADER_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let entry = sample_image_entry(&format!("http://{addr}"), env_var);
        let client = OpenAiImageClient::new(&entry, EnvCredentialResolver).expect("client builds");
        let error = client
            .generate(&sample_image_request())
            .expect_err("429 must error");
        handle.join().expect("server thread clean");
        assert!(
            matches!(
                error.kind,
                ImageProviderErrorKind::RateLimit {
                    retry_after_ms: None
                }
            ),
            "expected RateLimit{{None}} when header absent, got {error:?}"
        );
        unsafe { std::env::remove_var(env_var) };
    }

    #[test]
    fn openai_image_client_content_filter_surfaces_content_filtered() {
        let body = serde_json::json!({
            "data": [{ "b64_json": null, "url": null, "content_filter": true }]
        })
        .to_string();
        let (addr, handle, _captured) = image_capturing_server(body);
        let env_var = "PLOTFORGE_T31_IMAGE_FILTER_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let entry = sample_image_entry(&format!("http://{addr}"), env_var);
        let client = OpenAiImageClient::new(&entry, EnvCredentialResolver).expect("client builds");
        let error = client
            .generate(&sample_image_request())
            .expect_err("content_filter must error");
        handle.join().expect("server thread clean");
        assert_eq!(
            error.kind,
            ImageProviderErrorKind::ContentFiltered {
                reason: "content_filter".into()
            }
        );
        assert_eq!(error.code, "image_provider_content_filtered");
        // Non-retryable: content filter reproduces on retry.
        assert!(!error.retryable());
        unsafe { std::env::remove_var(env_var) };
    }

    #[test]
    fn openai_image_client_missing_credential_is_explicit_error() {
        // A provider with a credential_env_var that is unset must surface an
        // explicit error, never a silent 401. No server is contacted.
        let env_var = "PLOTFORGE_T31_IMAGE_UNSET_CRED";
        unsafe { std::env::remove_var(env_var) };
        let entry = sample_image_entry("http://127.0.0.1:1", env_var);
        let client = OpenAiImageClient::new(&entry, EnvCredentialResolver).expect("client builds");
        let error = client
            .generate(&sample_image_request())
            .expect_err("missing credential must error");
        assert_eq!(error.code, "image_provider_missing_credential");
        assert!(
            error.message.contains(env_var),
            "error must name the missing env var, got: {error}"
        );
        assert!(
            !error.message.contains("sk-"),
            "no credential value in message"
        );
    }

    #[test]
    fn openai_image_client_non_2xx_surfaces_http_status_error() {
        let (addr, handle) = image_status_server(
            "HTTP/1.1 500 Internal Server Error\r\n".into(),
            String::new(),
            "",
        );
        let env_var = "PLOTFORGE_T31_IMAGE_500_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let entry = sample_image_entry(&format!("http://{addr}"), env_var);
        let client = OpenAiImageClient::new(&entry, EnvCredentialResolver).expect("client builds");
        let error = client
            .generate(&sample_image_request())
            .expect_err("500 must error");
        handle.join().expect("server thread clean");
        assert_eq!(error.kind, ImageProviderErrorKind::Provider);
        assert!(error.code.contains("image_provider_http_status"));
        assert!(error.message.contains("500"));
        // 5xx is retryable; content filter is not.
        assert!(error.retryable());
        unsafe { std::env::remove_var(env_var) };
    }

    #[test]
    fn openai_image_client_omits_auth_header_for_empty_credential_env_var() {
        // A local no-auth image endpoint (empty credential_env_var) must omit
        // the Authorization header, mirroring the text client's no-auth path.
        let png_bytes = b"local-noauth-image".to_vec();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
        let body = serde_json::json!({ "data": [{ "b64_json": b64 }] }).to_string();
        let (addr, handle, captured) = image_capturing_server(body);
        let entry = sample_image_entry(&format!("http://{addr}"), "");
        let client =
            OpenAiImageClient::new(&entry, OptionalEnvCredentialResolver).expect("client builds");
        let response = client.generate(&sample_image_request()).expect("generate");
        handle.join().expect("server thread clean");
        assert_eq!(response.bytes, png_bytes);
        let request = captured.lock().expect("capture lock").clone();
        assert!(
            !request.to_ascii_lowercase().contains("authorization"),
            "no-auth endpoint must omit Authorization header, got: {request}"
        );
    }

    #[test]
    fn openai_image_client_missing_data_array_is_decode_error() {
        let body = serde_json::json!({ "data": [] }).to_string();
        let (addr, handle, _captured) = image_capturing_server(body);
        let env_var = "PLOTFORGE_T31_IMAGE_EMPTY_DATA_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let entry = sample_image_entry(&format!("http://{addr}"), env_var);
        let client = OpenAiImageClient::new(&entry, EnvCredentialResolver).expect("client builds");
        let error = client
            .generate(&sample_image_request())
            .expect_err("empty data must error");
        handle.join().expect("server thread clean");
        assert!(error.code.contains("image_provider_http_decode"));
        assert!(error.message.contains("data[0]"));
        unsafe { std::env::remove_var(env_var) };
    }
}
