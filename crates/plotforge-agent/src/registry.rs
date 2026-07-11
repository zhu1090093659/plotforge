//! User-global provider registry: load `~/.plotforge/providers.json`, build
//! HTTP clients for each registered `ProviderEntry`, and map a
//! `AgentSessionConfig.model_id` to a `TextModelProvider`.
//!
//! The registry is the single source of truth for real provider routing.
//! Credentials are never serialized into the registry; each entry carries
//! only the `credential_env_var` *name*, and the resolver reads the value
//! from the environment at call time. Missing or empty credentials map to
//! explicit errors — never a silent degraded run.
//!
//! For local no-auth endpoints (e.g. an Ollama server without a token),
//! `OptionalEnvCredentialResolver` returns an empty credential string
//! instead of erroring, and the HTTP clients omit the auth header when the
//! credential is empty.

use std::path::PathBuf;

use plotforge_job::ThrottleConfig;
use plotforge_schema::{
    ImageProviderEntry, ProviderEntry, ProviderKind, ProviderRegistry, RemoteModelInfo,
    RemoteModelList, TtsProviderEntry, redact_trace_text,
};

use crate::providers_http::{
    AnthropicMessagesClient, OpenAiCompatibleClient, OpenAiResponsesClient,
};
use crate::providers_image::{ImageProvider, OpenAiImageClient};
use crate::providers_text::{
    ConfiguredTextModelProvider, EnvCredentialResolver, FakeTextModelProvider,
    ProviderCredentialError, ProviderCredentialResolver, TextModelClient, TextModelProvider,
    TextProviderConfig,
};
use crate::providers_tts::OpenAiTtsClient;

/// The reserved model id for the offline local pi-Agent mock. Real providers
/// are opt-in: a fresh install resolves every model id to this default until
/// the user registers a provider.
pub const LOCAL_PI_MODEL_ID: &str = "local-pi";

/// The user-global PlotForge config directory. Resolved from
/// `dirs::config_dir()` with a `HOME` fallback so the registry works on
/// macOS (`~/Library/Application Support/plotforge`), Linux
/// (`~/.config/plotforge`), and a minimal shell that only sets `HOME`.
pub fn user_config_dir() -> Option<PathBuf> {
    if let Some(dir) = dirs::config_dir() {
        return Some(dir.join("plotforge"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".plotforge"))
}

/// The absolute path to `~/.plotforge/providers.json` (or the platform
/// equivalent under the config dir). Returns `None` only when neither
/// `dirs::config_dir()` nor `HOME` is available.
pub fn provider_registry_path() -> Option<PathBuf> {
    user_config_dir().map(|dir| dir.join("providers.json"))
}

/// Loads the user-global provider registry. A missing file returns an empty
/// registry (not an error) so a fresh install keeps working with the
/// `local-pi` mock default. A corrupt or unreadable file returns an error so
/// the user can fix it; we never silently fall back to an empty registry
/// from a file that exists.
pub fn load_provider_registry() -> Result<ProviderRegistry, ProviderRegistryError> {
    let Some(path) = provider_registry_path() else {
        return Ok(ProviderRegistry::default());
    };
    load_provider_registry_from(&path)
}

/// Same as `load_provider_registry` but reads from an explicit `path`. Used
/// by tests to keep the user library hermetic and to exercise the real
/// parse/IO error paths.
pub fn load_provider_registry_from(
    path: &std::path::Path,
) -> Result<ProviderRegistry, ProviderRegistryError> {
    if !path.exists() {
        return Ok(ProviderRegistry::default());
    }
    let content =
        std::fs::read_to_string(path).map_err(|error| ProviderRegistryError::ReadFailed {
            path: path.display().to_string(),
            source: error,
        })?;
    let registry: ProviderRegistry =
        serde_json::from_str(&content).map_err(|error| ProviderRegistryError::ParseFailed {
            path: path.display().to_string(),
            source: error,
        })?;
    Ok(registry)
}

/// Writes the user-global registry to `~/.plotforge/providers.json`,
/// creating the config directory if needed.
pub fn write_provider_registry(registry: &ProviderRegistry) -> Result<(), ProviderRegistryError> {
    let Some(path) = provider_registry_path() else {
        return Err(ProviderRegistryError::NoConfigDir);
    };
    write_provider_registry_to(&path, registry)
}

/// Same as `write_provider_registry` but writes to an explicit `path`. Used
/// by tests to keep the user library hermetic.
pub fn write_provider_registry_to(
    path: &std::path::Path,
    registry: &ProviderRegistry,
) -> Result<(), ProviderRegistryError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| ProviderRegistryError::WriteFailed {
            path: parent.display().to_string(),
            source: error,
        })?;
    }
    let content = serde_json::to_string_pretty(registry)
        .map_err(|error| ProviderRegistryError::SerializeFailed { source: error })?;
    std::fs::write(path, content).map_err(|error| ProviderRegistryError::WriteFailed {
        path: path.display().to_string(),
        source: error,
    })?;
    Ok(())
}

/// Builds a `TextModelClient` for a single registered provider entry. The
/// returned client is wrapped by `ConfiguredTextModelProvider` (which owns
/// config validation, credential resolution, and redaction) in
/// `build_text_provider`. This function only picks the HTTP client shape
/// per `ProviderKind`.
pub fn build_provider_client(
    entry: &ProviderEntry,
) -> Result<Box<dyn TextModelClient>, ProviderBuildError> {
    match entry.kind {
        ProviderKind::OpenAiCompatible => Ok(Box::new(OpenAiCompatibleClient::new().map_err(
            |error| ProviderBuildError::ClientConstruction {
                provider_id: entry.id.clone(),
                message: error.message,
            },
        )?)),
        ProviderKind::OpenAiResponses => {
            Ok(Box::new(OpenAiResponsesClient::new().map_err(|error| {
                ProviderBuildError::ClientConstruction {
                    provider_id: entry.id.clone(),
                    message: error.message,
                }
            })?))
        }
        ProviderKind::AnthropicMessages => Ok(Box::new(AnthropicMessagesClient::new().map_err(
            |error| ProviderBuildError::ClientConstruction {
                provider_id: entry.id.clone(),
                message: error.message,
            },
        )?)),
    }
}

/// Builds the full `TextModelProvider` for one provider entry, combining the
/// HTTP client with a `TextProviderConfig`, a credential resolver, and the
/// redaction/validation adapter. This is the value `PiAgent::new` consumes as
/// its provider.
///
/// Credential resolution: when `credential_env_var` is non-empty, the strict
/// `EnvCredentialResolver` is used so a missing or empty env var surfaces the
/// documented `pi_agent_missing_credential` failure mode (the friendly "set
/// your API key" hint). When `credential_env_var` is empty (a local no-auth
/// endpoint like Ollama), `OptionalEnvCredentialResolver` returns an empty
/// credential and the HTTP client omits the auth header.
pub fn build_text_provider(
    entry: &ProviderEntry,
) -> Result<Box<dyn TextModelProvider>, ProviderBuildError> {
    let client = build_provider_client(entry)?;
    // JSON Schema output constraint is enabled for providers that support
    // provider-native structured output: OpenAI Responses (json_schema
    // response_format) and Anthropic Messages (forced tool_use). OpenAI
    // Compatible is left off by default because many compatible endpoints
    // (DeepSeek, GLM, Ollama, vLLM) do not implement the json_schema
    // response_format and would reject the request.
    let supports_json_schema = matches!(
        entry.kind,
        ProviderKind::OpenAiResponses | ProviderKind::AnthropicMessages
    );
    let config = TextProviderConfig {
        enabled: entry.enabled,
        provider: entry.id.clone(),
        model: entry.model.clone(),
        endpoint_url: Some(entry.endpoint_url.clone()),
        credential_env_var: entry.credential_env_var.clone(),
        max_output_tokens: entry.max_output_tokens,
        supports_json_schema,
    };
    let throttle = build_throttle(ThrottleConfig {
        provider_id: entry.id.clone(),
        max_concurrency: entry.max_concurrency,
        requests_per_minute: entry.requests_per_minute,
        daily_token_budget: entry.daily_token_budget,
    })?;
    if entry.credential_env_var.trim().is_empty() {
        Ok(Box::new(
            ConfiguredTextModelProvider::new(config, client, OptionalEnvCredentialResolver)
                .with_throttle(throttle),
        ))
    } else {
        Ok(Box::new(
            ConfiguredTextModelProvider::new(config, client, EnvCredentialResolver)
                .with_throttle(throttle),
        ))
    }
}

// ---------------------------------------------------------------------------
// Image provider registry + construction.
//
// `build_image_provider` constructs an `OpenAiImageClient` from an
// `ImageProviderEntry`, dispatching by endpoint shape. Today every image
// entry routes to the OpenAI Images API; the dispatch is structured so a
// future non-OpenAI image provider can add a branch without touching callers.
// `resolve_image_provider` picks the first enabled image entry from a loaded
// registry so the pi-Agent apply flow can decide whether to attempt image
// generation without owning the registry's selection policy.
//
// Credentials are resolved through the same strict/optional resolver split as
// text providers: a non-empty `credential_env_var` uses `EnvCredentialResolver`
// (a missing/empty env var surfaces an explicit error), and an empty
// `credential_env_var` uses `OptionalEnvCredentialResolver` (local no-auth
// endpoint). The credential value never enters the returned client struct,
// traces, or the persisted registry.
// ---------------------------------------------------------------------------

/// Builds the `ImageProvider` for a single image provider entry. Dispatches to
/// `OpenAiImageClient` for OpenAI-compatible image endpoints. Credential
/// resolution follows the same strict/optional split as `build_text_provider`.
pub fn build_image_provider(
    entry: &ImageProviderEntry,
) -> Result<Box<dyn ImageProvider>, ProviderBuildError> {
    reject_media_daily_token_budget(&entry.id, entry.daily_token_budget)?;
    let throttle = build_throttle(ThrottleConfig {
        provider_id: entry.id.clone(),
        max_concurrency: entry.max_concurrency,
        requests_per_minute: entry.requests_per_minute,
        daily_token_budget: None,
    })?;
    let client = OpenAiImageClient::new(entry, EnvCredentialResolver)
        .map_err(|error| ProviderBuildError::ClientConstruction {
            provider_id: entry.id.clone(),
            message: error.message,
        })?
        .with_throttle(throttle);
    Ok(Box::new(client))
}

/// Returns the first enabled image provider entry in `registry`, or `None`
/// when no image provider is configured. The pi-Agent apply flow uses this to
/// decide whether to attempt scene image generation: when `None`, image
/// generation is skipped (the turn succeeds without a background image, never
/// fails). A disabled entry is treated the same as absent (no silent degraded
/// run from a provider the user explicitly turned off).
pub fn resolve_image_provider(registry: &ProviderRegistry) -> Option<&ImageProviderEntry> {
    registry.image_providers.iter().find(|entry| entry.enabled)
}

// ---------------------------------------------------------------------------
// TTS provider construction.
//
// `build_tts_provider` constructs an `OpenAiTtsClient` from a registered
// `TtsProviderEntry`. Credentials are resolved at call-time inside the
// client's `synthesize` method via `EnvCredentialResolver`; the credential
// value never enters the client struct, traces, or the persisted registry.
// ---------------------------------------------------------------------------

/// Constructs an `OpenAiTtsClient` from a registered `TtsProviderEntry`.
/// Uses the strict `EnvCredentialResolver` for auth-required providers and
/// `OptionalEnvCredentialResolver` for local no-auth endpoints (empty
/// `credential_env_var`), mirroring the text provider pattern.
pub fn build_tts_provider(
    entry: &TtsProviderEntry,
) -> Result<OpenAiTtsClient<EnvCredentialResolver>, ProviderBuildError> {
    reject_media_daily_token_budget(&entry.id, entry.daily_token_budget)?;
    let throttle = build_throttle(ThrottleConfig {
        provider_id: entry.id.clone(),
        max_concurrency: entry.max_concurrency,
        requests_per_minute: entry.requests_per_minute,
        daily_token_budget: None,
    })?;
    OpenAiTtsClient::from_entry(entry, EnvCredentialResolver)
        .map(|client| client.with_throttle(throttle))
        .map_err(|error| ProviderBuildError::ClientConstruction {
            provider_id: entry.id.clone(),
            message: error.message,
        })
}

fn build_throttle(
    config: ThrottleConfig,
) -> Result<Option<crate::throttle::ProviderThrottle>, ProviderBuildError> {
    let provider_id = config.provider_id.clone();
    crate::throttle::ProviderThrottle::shared_from_config(config).map_err(|error| {
        ProviderBuildError::InvalidThrottle {
            provider_id,
            message: error.to_string(),
        }
    })
}

fn reject_media_daily_token_budget(
    provider_id: &str,
    daily_token_budget: Option<u64>,
) -> Result<(), ProviderBuildError> {
    if daily_token_budget.is_some() {
        return Err(ProviderBuildError::UnsupportedQuota {
            provider_id: provider_id.to_string(),
            message: "daily_token_budget is supported only by text providers because image and TTS usage does not report output tokens".into(),
        });
    }
    Ok(())
}

/// Returns the first enabled TTS provider entry in `registry`, or `None`
/// when no TTS provider is configured. The TTS pipeline uses this to decide
/// whether to attempt audio synthesis: when `None`, TTS is skipped (the
/// pipeline falls back to `FakeTtsProvider` or produces no audio, never
/// fails the turn).
pub fn resolve_tts_provider(registry: &ProviderRegistry) -> Option<&TtsProviderEntry> {
    registry.tts_providers.iter().find(|entry| entry.enabled)
}

// ---------------------------------------------------------------------------
// Dynamic model discovery.
//
// `fetch_provider_models` queries a provider's upstream `/models` (or
// equivalent) endpoint to enumerate the models it serves, caches the list
// locally under `~/.plotforge/cache/models/{provider_id}.json` with a 1-hour
// TTL, and returns redaction-safe `RemoteModelInfo` entries. Credentials are
// resolved through the same `EnvCredentialResolver` used for completions; a
// missing credential is an explicit `ModelDiscoveryError::MissingCredential`,
// never a silent empty list. No raw response body is persisted — only the
// parsed `RemoteModelInfo` metadata.
// ---------------------------------------------------------------------------

/// The model-discovery cache TTL: a cached list is fresh for one hour. After
/// that the next call refetches from the upstream endpoint.
const MODEL_DISCOVERY_TTL_SECS: u64 = 3600;

/// Errors raised by `fetch_provider_models`. All messages are redaction-safe:
/// no credential values, no raw upstream response bodies.
#[derive(Debug, thiserror::Error)]
pub enum ModelDiscoveryError {
    #[error("missing provider credential for model discovery: env var `{env_var}` is not set")]
    MissingCredential { env_var: String },
    #[error("model discovery HTTP request failed (status {code}): {message}")]
    Http { code: String, message: String },
    #[error("model discovery cache error: {message}")]
    Cache { message: String },
}

/// Fetches the list of models a provider serves from its upstream `/models`
/// (or Anthropic `/v1/models`) endpoint, with a 1-hour local cache.
///
/// Cache layout: `~/.plotforge/cache/models/{provider_id}.json` holding a
/// `RemoteModelList`. A fresh cache (age < 3600s) is returned as-is; a stale
/// or missing cache triggers a fresh fetch. The cache directory is created
/// with `create_dir_all` if missing.
///
/// Credentials are resolved via the strict `EnvCredentialResolver` so a
/// missing/empty env var surfaces `MissingCredential` (no silent empty list).
/// The credential value never enters the cache, traces, or error messages
/// (`redact_trace_text` is applied to all error text).
pub fn fetch_provider_models(
    entry: &ProviderEntry,
) -> Result<Vec<RemoteModelInfo>, ModelDiscoveryError> {
    let Some(home_dir) = dirs::home_dir() else {
        return Err(ModelDiscoveryError::Cache {
            message: redact_trace_text("could not resolve user home directory for cache path"),
        });
    };
    let cache_dir = home_dir.join(".plotforge").join("cache").join("models");
    let cache_path = cache_dir.join(format!("{}.json", entry.id));
    fetch_provider_models_to(entry, &cache_path)
}

/// Same as `fetch_provider_models` but reads/writes an explicit cache `path`.
/// Used by tests to keep the cache hermetic and to exercise the real TTL/IO
/// paths without touching the user's `~/.plotforge` directory.
pub fn fetch_provider_models_to(
    entry: &ProviderEntry,
    cache_path: &std::path::Path,
) -> Result<Vec<RemoteModelInfo>, ModelDiscoveryError> {
    let now = unix_now();
    // Fresh cache: return as-is without any network call.
    if let Some(cached) = read_cached_models(cache_path, now)? {
        return Ok(cached.models);
    }
    // Stale or missing: fetch fresh, then persist.
    let models = fetch_models_from_upstream(entry)?;
    let list = RemoteModelList {
        models,
        fetched_at: now,
    };
    write_cached_models(cache_path, &list)?;
    Ok(list.models)
}

/// Reads a cached `RemoteModelList` from `path`. Returns `None` when the file
/// is missing or stale (age >= TTL); returns the list when fresh. A corrupt or
/// unreadable file surfaces an explicit `Cache` error (no silent fallback to
/// refetch, so the user can diagnose a permissions/parse issue).
fn read_cached_models(
    path: &std::path::Path,
    now: u64,
) -> Result<Option<RemoteModelList>, ModelDiscoveryError> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path).map_err(|error| ModelDiscoveryError::Cache {
        message: redact_trace_text(&format!("failed to read model cache: {error}")),
    })?;
    let list: RemoteModelList =
        serde_json::from_str(&content).map_err(|error| ModelDiscoveryError::Cache {
            message: redact_trace_text(&format!("failed to parse model cache: {error}")),
        })?;
    if now >= list.fetched_at && now - list.fetched_at < MODEL_DISCOVERY_TTL_SECS {
        Ok(Some(list))
    } else {
        Ok(None)
    }
}

/// Writes a `RemoteModelList` to `path`, creating the parent directory if
/// missing.
fn write_cached_models(
    path: &std::path::Path,
    list: &RemoteModelList,
) -> Result<(), ModelDiscoveryError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| ModelDiscoveryError::Cache {
            message: redact_trace_text(&format!("failed to create model cache dir: {error}")),
        })?;
    }
    let content =
        serde_json::to_string_pretty(list).map_err(|error| ModelDiscoveryError::Cache {
            message: redact_trace_text(&format!("failed to serialize model cache: {error}")),
        })?;
    std::fs::write(path, content).map_err(|error| ModelDiscoveryError::Cache {
        message: redact_trace_text(&format!("failed to write model cache: {error}")),
    })?;
    Ok(())
}

/// Performs the live upstream `/models` fetch for one provider entry. Resolves
/// the credential through the strict `EnvCredentialResolver`, dispatches by
/// `ProviderKind` to the right URL + headers, and parses the OpenAI/Anthropic
/// model-list response shape into `RemoteModelInfo` entries.
fn fetch_models_from_upstream(
    entry: &ProviderEntry,
) -> Result<Vec<RemoteModelInfo>, ModelDiscoveryError> {
    let credential = EnvCredentialResolver
        .resolve(&entry.credential_env_var)
        .map_err(|_| ModelDiscoveryError::MissingCredential {
            env_var: entry.credential_env_var.clone(),
        })?;
    let client = crate::providers_http::shared_blocking_client().map_err(|error| {
        ModelDiscoveryError::Http {
            code: "client_construction".into(),
            message: redact_trace_text(&error.message),
        }
    })?;
    let (url, auth) = models_endpoint(entry);
    let response = dispatch_models_request(&client, &url, &auth, &credential, entry.kind)?;
    parse_models_response(&response, entry.kind)
}

/// Builds the upstream `/models` URL and the auth-shape for a provider kind.
/// OpenAI-compatible and Responses both use `GET {endpoint}/models` with a
/// Bearer header; Anthropic uses `GET {endpoint}/v1/models` with `x-api-key`.
fn models_endpoint(entry: &ProviderEntry) -> (String, ModelsAuth) {
    match entry.kind {
        ProviderKind::AnthropicMessages => {
            let url = join_models_endpoint(&entry.endpoint_url, "v1/models");
            (url, ModelsAuth::ApiKey)
        }
        ProviderKind::OpenAiCompatible | ProviderKind::OpenAiResponses => {
            let url = join_models_endpoint(&entry.endpoint_url, "models");
            (url, ModelsAuth::Bearer)
        }
    }
}

/// The auth header shape a `/models` request uses.
enum ModelsAuth {
    Bearer,
    ApiKey,
}

/// Joins an endpoint base URL with a relative path, normalising slashes.
/// Mirrors `providers_http::join_endpoint` so model discovery stays
/// self-contained without reaching into the (private) HTTP helper.
fn join_models_endpoint(base_url: &str, relative: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    format!("{trimmed}/{relative}")
}

/// Dispatches the GET `/models` request with the right headers and returns
/// the raw response body text. Non-2xx surfaces an `Http` error carrying the
/// status code; transport failures surface as `Http` with a redacted message.
fn dispatch_models_request(
    client: &reqwest::blocking::Client,
    url: &str,
    auth: &ModelsAuth,
    credential: &str,
    kind: ProviderKind,
) -> Result<String, ModelDiscoveryError> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    if kind == ProviderKind::AnthropicMessages {
        headers.insert(
            "anthropic-version",
            reqwest::header::HeaderValue::from_static(crate::providers_http::ANTHROPIC_VERSION),
        );
    }
    match auth {
        ModelsAuth::Bearer => {
            if !credential.trim().is_empty() {
                let value = reqwest::header::HeaderValue::from_str(&format!("Bearer {credential}"))
                    .map_err(|_| ModelDiscoveryError::Http {
                        code: "credential_header".into(),
                        message: redact_trace_text(
                            "provider credential contains bytes illegal in an HTTP header value",
                        ),
                    })?;
                headers.insert("Authorization", value);
            }
        }
        ModelsAuth::ApiKey => {
            if !credential.trim().is_empty() {
                let value = reqwest::header::HeaderValue::from_str(credential).map_err(|_| {
                    ModelDiscoveryError::Http {
                        code: "credential_header".into(),
                        message: redact_trace_text(
                            "provider credential contains bytes illegal in an HTTP header value",
                        ),
                    }
                })?;
                headers.insert("x-api-key", value);
            }
        }
    }
    let response =
        client
            .get(url)
            .headers(headers)
            .send()
            .map_err(|error| ModelDiscoveryError::Http {
                code: "transport".into(),
                message: redact_trace_text(&error.to_string()),
            })?;
    let status = response.status();
    if !status.is_success() {
        return Err(ModelDiscoveryError::Http {
            code: status.as_u16().to_string(),
            message: redact_trace_text(&format!("provider returned HTTP {status}")),
        });
    }
    response.text().map_err(|error| ModelDiscoveryError::Http {
        code: "body".into(),
        message: redact_trace_text(&error.to_string()),
    })
}

/// Parses an upstream `/models` response body into `RemoteModelInfo` entries.
/// Both OpenAI and Anthropic wrap the list in `{ "data": [ ... ] }`; the two
/// differ in which optional fields each entry carries. The parser accepts the
/// union of both shapes so a provider that returns a superset still decodes.
fn parse_models_response(
    body: &str,
    kind: ProviderKind,
) -> Result<Vec<RemoteModelInfo>, ModelDiscoveryError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|error| ModelDiscoveryError::Http {
            code: "decode".into(),
            message: redact_trace_text(&format!("model list response was not JSON: {error}")),
        })?;
    let data = value
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| ModelDiscoveryError::Http {
            code: "decode".into(),
            message: redact_trace_text(&format!(
                "{kind:?} model list response missing `data` array"
            )),
        })?;
    let mut models = Vec::with_capacity(data.len());
    for item in data {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModelDiscoveryError::Http {
                code: "decode".into(),
                message: redact_trace_text("model list entry missing `id`"),
            })?
            .to_string();
        let owned_by = item
            .get("owned_by")
            .and_then(|v| v.as_str())
            .map(String::from);
        let created = item.get("created").and_then(|v| v.as_u64());
        let max_input_tokens = item
            .get("max_input_tokens")
            .and_then(|v| v.as_u64())
            .and_then(|n| u32::try_from(n).ok());
        let max_output_tokens = item
            .get("max_output_tokens")
            .and_then(|v| v.as_u64())
            .and_then(|n| u32::try_from(n).ok());
        models.push(RemoteModelInfo {
            id,
            owned_by,
            created,
            max_input_tokens,
            max_output_tokens,
        });
    }
    Ok(models)
}

/// Returns the current Unix timestamp in seconds. Kept as a helper so tests
/// can reason about TTL arithmetic without touching `SystemTime` directly.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// it. `local-pi` returns `None` so the caller routes to
/// `FakeTextModelProvider::local_pi()`; an unknown model id also returns
/// `None` so the caller surfaces an explicit "no provider for model id"
/// error rather than silently picking a random provider.
pub fn resolve_provider_for_model<'a>(
    model_id: &str,
    registry: &'a ProviderRegistry,
) -> Option<&'a ProviderEntry> {
    if model_id == LOCAL_PI_MODEL_ID {
        return None;
    }
    registry
        .providers
        .iter()
        .find(|entry| entry.id == model_id && entry.enabled)
}

/// A credential resolver that treats a missing or empty env var as an empty
/// credential instead of an error. Used by `build_text_provider` so local
/// no-auth endpoints (Ollama, vLLM without a token) work without forcing the
/// user to set a dummy env var. Providers that *require* auth will fail at
/// the HTTP layer (401/403) with an explicit error, never silently.
#[derive(Clone, Debug, Default)]
pub struct OptionalEnvCredentialResolver;

impl ProviderCredentialResolver for OptionalEnvCredentialResolver {
    fn resolve(&self, env_var: &str) -> Result<String, ProviderCredentialError> {
        // An empty env-var name means "no credential expected". Return empty.
        if env_var.trim().is_empty() {
            return Ok(String::new());
        }
        match std::env::var(env_var) {
            Ok(value) => Ok(value),
            Err(_) => Ok(String::new()),
        }
    }
}

/// Errors raised by the registry loader. All variants carry a redaction-safe
/// message: no credential values, no raw file contents, no secrets.
#[derive(Debug, thiserror::Error)]
pub enum ProviderRegistryError {
    #[error("could not resolve a user config directory (no HOME and no platform config dir)")]
    NoConfigDir,
    #[error("failed to read provider registry at {path}: {source}")]
    ReadFailed {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse provider registry at {path}: {source}")]
    ParseFailed {
        path: String,
        source: serde_json::Error,
    },
    #[error("failed to write provider registry at {path}: {source}")]
    WriteFailed {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to serialize provider registry: {source}")]
    SerializeFailed { source: serde_json::Error },
}

/// Errors raised by `build_provider_client` / `build_text_provider`.
#[derive(Debug, thiserror::Error)]
pub enum ProviderBuildError {
    #[error("could not construct HTTP client for provider `{provider_id}`: {message}")]
    ClientConstruction {
        provider_id: String,
        message: String,
    },
    #[error("invalid throttle configuration for provider `{provider_id}`: {message}")]
    InvalidThrottle {
        provider_id: String,
        message: String,
    },
    #[error("unsupported quota configuration for provider `{provider_id}`: {message}")]
    UnsupportedQuota {
        provider_id: String,
        message: String,
    },
}

/// Convenience used by tests and the local-pi default path: returns the
/// boxed `FakeTextModelProvider::local_pi()` as a `TextModelProvider`.
pub fn local_pi_provider() -> FakeTextModelProvider {
    FakeTextModelProvider::local_pi()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn missing_registry_file_returns_empty_default() {
        // Exercise the real public path-injected loader so the missing-file
        // branch, parse failure, and roundtrip are all guarded hermetically
        // (the old test re-implemented the branch inline).
        let dir = TempDir::new().expect("temp dir");
        let missing_path = dir.path().join("definitely-missing").join("providers.json");
        assert!(!missing_path.exists());
        let registry = load_provider_registry_from(&missing_path).expect("missing file ok");
        assert!(registry.providers.is_empty());
    }

    #[test]
    fn corrupt_registry_returns_parse_error() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("providers.json");
        std::fs::write(&path, "{ not valid json").expect("write corrupt");
        let error = load_provider_registry_from(&path).expect_err("corrupt must error");
        assert!(
            matches!(error, ProviderRegistryError::ParseFailed { .. }),
            "expected ParseFailed, got {error:?}"
        );
    }

    #[test]
    fn write_and_load_registry_roundtrips() {
        let dir = TempDir::new().expect("temp dir");
        let registry_path = dir.path().join("providers.json");
        let registry = ProviderRegistry {
            version: "1".into(),
            providers: vec![ProviderEntry {
                id: "glm".into(),
                kind: ProviderKind::OpenAiCompatible,
                label: "GLM 4.6".into(),
                endpoint_url: "https://open.bigmodels.cn/api/paas/v4".into(),
                model: "glm-4.6".into(),
                credential_env_var: "ZAI_API_KEY".into(),
                enabled: true,
                max_output_tokens: None,
                max_concurrency: None,
                requests_per_minute: None,
                daily_token_budget: None,
            }],
            image_providers: Vec::new(),
            tts_providers: Vec::new(),
        };
        write_provider_registry_to(&registry_path, &registry).expect("write via public API");
        let loaded = load_provider_registry_from(&registry_path).expect("read via public API");
        assert_eq!(loaded, registry);
        let raw = std::fs::read_to_string(&registry_path).expect("read raw");
        // Redaction safety: the persisted file must not carry credentials.
        assert!(!raw.contains("api_key"));
        assert!(!raw.contains("sk-"));
    }

    #[test]
    fn resolve_provider_for_model_returns_entry_for_known_id() {
        let registry = ProviderRegistry {
            version: "1".into(),
            providers: vec![
                ProviderEntry {
                    id: "glm".into(),
                    kind: ProviderKind::OpenAiCompatible,
                    label: "GLM".into(),
                    endpoint_url: "https://x".into(),
                    model: "glm-4.6".into(),
                    credential_env_var: "ZAI_API_KEY".into(),
                    enabled: true,
                    max_output_tokens: None,
                    max_concurrency: None,
                    requests_per_minute: None,
                    daily_token_budget: None,
                },
                ProviderEntry {
                    id: "claude".into(),
                    kind: ProviderKind::AnthropicMessages,
                    label: "Claude".into(),
                    endpoint_url: "https://api.anthropic.com".into(),
                    model: "claude-sonnet-4".into(),
                    credential_env_var: "ANTHROPIC_API_KEY".into(),
                    enabled: false,
                    max_output_tokens: None,
                    max_concurrency: None,
                    requests_per_minute: None,
                    daily_token_budget: None,
                },
            ],
            image_providers: Vec::new(),
            tts_providers: Vec::new(),
        };
        let resolved = resolve_provider_for_model("glm", &registry).expect("glm entry");
        assert_eq!(resolved.id, "glm");
        // Disabled providers are not returned.
        assert!(resolve_provider_for_model("claude", &registry).is_none());
        // Unknown model ids return None (no silent fallback).
        assert!(resolve_provider_for_model("unknown", &registry).is_none());
        // local-pi returns None so the caller routes to the mock.
        assert!(resolve_provider_for_model(LOCAL_PI_MODEL_ID, &registry).is_none());
    }

    #[test]
    fn build_provider_client_dispatches_by_kind() {
        let entry = ProviderEntry {
            id: "glm".into(),
            kind: ProviderKind::OpenAiCompatible,
            label: "GLM".into(),
            endpoint_url: "https://x".into(),
            model: "glm-4.6".into(),
            credential_env_var: "ZAI_API_KEY".into(),
            enabled: true,
            max_output_tokens: None,
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        };
        let _client = build_provider_client(&entry).expect("openai client");
        let anthropic_entry = ProviderEntry {
            id: "claude".into(),
            kind: ProviderKind::AnthropicMessages,
            label: "Claude".into(),
            endpoint_url: "https://api.anthropic.com".into(),
            model: "claude-sonnet-4".into(),
            credential_env_var: "ANTHROPIC_API_KEY".into(),
            enabled: true,
            max_output_tokens: None,
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        };
        let _client = build_provider_client(&anthropic_entry).expect("anthropic client");
        let responses_entry = ProviderEntry {
            id: "openai".into(),
            kind: ProviderKind::OpenAiResponses,
            label: "OpenAI".into(),
            endpoint_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o".into(),
            credential_env_var: "OPENAI_API_KEY".into(),
            enabled: true,
            max_output_tokens: None,
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        };
        let _client = build_provider_client(&responses_entry).expect("responses client");
    }

    #[test]
    fn optional_resolver_returns_empty_for_missing_env_var() {
        let resolver = OptionalEnvCredentialResolver;
        let value = resolver
            .resolve("PLOTFORGE_DEFINITELY_NOT_SET_VAR_12345")
            .expect("missing env var resolves to empty");
        assert!(value.is_empty());
    }

    #[test]
    fn optional_resolver_returns_empty_for_empty_env_var_name() {
        let resolver = OptionalEnvCredentialResolver;
        let value = resolver.resolve("").expect("empty env var name ok");
        assert!(value.is_empty());
        let value = resolver.resolve("   ").expect("blank env var name ok");
        assert!(value.is_empty());
    }

    /// Regression for the High finding (H2): a registered provider with a
    /// non-empty `credential_env_var` whose env var is unset must surface the
    /// documented `pi_agent_missing_credential` failure mode (the friendly
    /// "set your API key" hint), not a silent empty credential → HTTP 401.
    /// The registry must use the strict `EnvCredentialResolver` for
    /// auth-required providers.
    #[test]
    fn build_text_provider_surfaces_missing_credential_for_auth_required_provider() {
        let entry = ProviderEntry {
            id: "glm".into(),
            kind: ProviderKind::OpenAiCompatible,
            label: "GLM".into(),
            endpoint_url: "https://example.invalid/v1".into(),
            model: "glm-4.6".into(),
            credential_env_var: "PLOTFORGE_H2_TEST_UNSET_VAR".into(),
            enabled: true,
            max_output_tokens: None,
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        };
        let provider = build_text_provider(&entry).expect("provider builds");
        // The env var is not set in the test process, so the strict resolver
        // must error with `ProviderCredentialError::Missing` and the adapter
        // must map it to `text_provider_missing_credential`.
        let reproducibility = provider.reproducibility_metadata(1);
        let request = crate::TextModelRequest {
            call_id: "h2-test".into(),
            agent: plotforge_schema::AgentRole::ScenePlanner,
            scene_key: "scene-1".into(),
            run_seed: reproducibility.run_seed,
            prompt_version: reproducibility.prompt_version.clone(),
            model_version: reproducibility.model_version.clone(),
            provider_config_hash: reproducibility.provider_config_hash.clone(),
            prompt: "{}".into(),
            messages: None,
        };
        let error = provider
            .complete(&request)
            .expect_err("missing credential must error");
        assert_eq!(error.code, "text_provider_missing_credential");
        assert!(
            error.message.contains("PLOTFORGE_H2_TEST_UNSET_VAR"),
            "error must name the missing env var, got: {error}"
        );
        // Redaction: the message must not contain a credential value.
        assert!(!error.message.contains("sk-"));
    }

    /// H3 regression: a local no-auth endpoint (empty `credential_env_var`)
    /// must be usable, not rejected. The optional resolver returns an empty
    /// credential, `validate()` accepts an empty env-var name, and the HTTP
    /// client omits the auth header. The call then either succeeds (a running
    /// Ollama) or fails with an explicit transport error — never a config
    /// rejection. Previously `validate()` rejected the empty name and the
    /// no-auth path was unreachable.
    #[test]
    fn build_text_provider_uses_optional_resolver_for_no_auth_endpoint() {
        let entry = ProviderEntry {
            id: "ollama".into(),
            kind: ProviderKind::OpenAiCompatible,
            label: "Ollama".into(),
            endpoint_url: "http://localhost:11434/v1".into(),
            model: "llama3".into(),
            credential_env_var: String::new(),
            enabled: true,
            max_output_tokens: None,
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        };
        let provider = build_text_provider(&entry).expect("no-auth provider builds");
        let reproducibility = provider.reproducibility_metadata(1);
        let request = crate::TextModelRequest {
            call_id: "noauth-test".into(),
            agent: plotforge_schema::AgentRole::ScenePlanner,
            scene_key: "scene-1".into(),
            run_seed: reproducibility.run_seed,
            prompt_version: reproducibility.prompt_version.clone(),
            model_version: reproducibility.model_version.clone(),
            provider_config_hash: reproducibility.provider_config_hash.clone(),
            prompt: "{}".into(),
            messages: None,
        };
        // No Ollama is running in the test process, so the call must fail —
        // but with an explicit HTTP transport error, NOT a config rejection.
        // This is the contract: the no-auth path reaches the HTTP client;
        // it does not die at `validate()`.
        let error = provider
            .complete(&request)
            .expect_err("no Ollama running; transport error expected");
        assert!(
            !error.code.contains("text_provider_config"),
            "no-auth endpoint must pass validate(), got config error: {error}"
        );
        assert!(
            error.code.contains("text_provider_http")
                || error.code.contains("text_provider_timeout"),
            "expected an HTTP transport error for the unreachable no-auth endpoint, got: {error}"
        );
    }

    #[test]
    fn build_text_provider_uses_stable_registry_id_for_usage_identity() {
        let entry = ProviderEntry {
            id: "stable-provider-id".into(),
            kind: ProviderKind::OpenAiCompatible,
            label: "Editable display label".into(),
            endpoint_url: "http://localhost:11434/v1".into(),
            model: "model-a".into(),
            credential_env_var: String::new(),
            enabled: true,
            max_output_tokens: None,
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        };

        let provider = build_text_provider(&entry).expect("provider builds");
        let identity = provider
            .usage_identity()
            .expect("registered provider identity");
        assert_eq!(identity.provider_id, entry.id);
        assert_ne!(identity.provider_id, entry.label);
    }

    #[test]
    fn rebuilding_provider_throttle_reuses_process_state() {
        let config = ThrottleConfig {
            provider_id: "registry-rebuild-throttle-test".into(),
            requests_per_minute: Some(1),
            ..ThrottleConfig::default()
        };
        let first = build_throttle(config.clone())
            .expect("first provider build")
            .expect("quota creates throttle");
        let second = build_throttle(config)
            .expect("second provider build")
            .expect("quota creates throttle");

        drop(first.acquire().expect("first build consumes token"));
        let failure = second
            .acquire()
            .expect_err("rebuilt provider must observe the same bucket")
            .into_failure();
        assert!(matches!(
            failure,
            crate::throttle::ThrottleFailure::RateLimit { .. }
        ));
    }

    // -----------------------------------------------------------------------
    // T1.2: Dynamic model discovery (`fetch_provider_models`).
    //
    // These tests exercise the live `/models` fetch against an in-process TCP
    // server (OpenAI- and Anthropic-shaped responses), the local cache TTL,
    // the missing-credential explicit error, and the cache create/read path.
    // They use `fetch_provider_models_to` with an explicit temp cache path so
    // the user's `~/.plotforge` directory is never touched. A unique env-var
    // sentinel per test keeps the credential resolution hermetic; the env var
    // is set in-process for the success cases and deliberately left unset for
    // the missing-credential case.
    // -----------------------------------------------------------------------

    fn discovery_entry(
        id: &str,
        kind: ProviderKind,
        endpoint: &str,
        env_var: &str,
    ) -> ProviderEntry {
        ProviderEntry {
            id: id.into(),
            kind,
            label: id.into(),
            endpoint_url: endpoint.into(),
            model: "test-model".into(),
            credential_env_var: env_var.into(),
            enabled: true,
            max_output_tokens: None,
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        }
    }

    /// Spawns a one-shot in-process TCP server that reads the request line and
    /// replies with `body` preceded by a 200 status line. Returns the bound
    /// address. The server accepts a single connection then exits; the join
    /// handle is returned so the test can wait for clean shutdown.
    fn models_tcp_server(body: String) -> (std::net::SocketAddr, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf); // drain the request line/headers
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        (addr, handle)
    }

    #[test]
    fn fetch_provider_models_decodes_openai_shape() {
        let body = serde_json::json!({
            "data": [
                { "id": "gpt-4o", "owned_by": "openai", "created": 1700000000_u64 },
                { "id": "gpt-4o-mini", "owned_by": "openai", "created": 1700000001_u64 }
            ]
        })
        .to_string();
        let (addr, handle) = models_tcp_server(body);
        let env_var = "PLOTFORGE_T12_OPENAI_TEST_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let entry = discovery_entry(
            "openai-test",
            ProviderKind::OpenAiCompatible,
            &format!("http://{addr}"),
            env_var,
        );
        let dir = TempDir::new().expect("temp dir");
        let cache = dir.path().join("openai-test.json");
        let models = fetch_provider_models_to(&entry, &cache).expect("fetch openai models");
        handle.join().expect("server thread clean");
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "gpt-4o");
        assert_eq!(models[0].owned_by.as_deref(), Some("openai"));
        assert_eq!(models[0].created, Some(1_700_000_000));
        // OpenAI shape does not carry token caps.
        assert_eq!(models[0].max_input_tokens, None);
        assert_eq!(models[0].max_output_tokens, None);
        // Cache file must be created and round-trip.
        assert!(cache.exists());
        let raw = std::fs::read_to_string(&cache).expect("read cache");
        assert!(raw.contains("gpt-4o"));
        unsafe { std::env::remove_var(env_var) };
    }

    #[test]
    fn fetch_provider_models_decodes_anthropic_shape() {
        let body = serde_json::json!({
            "data": [
                {
                    "id": "claude-3-5-sonnet",
                    "max_input_tokens": 200000_u32,
                    "max_output_tokens": 8192_u32
                }
            ]
        })
        .to_string();
        let (addr, handle) = models_tcp_server(body);
        let env_var = "PLOTFORGE_T12_ANTHROPIC_TEST_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let entry = discovery_entry(
            "anthropic-test",
            ProviderKind::AnthropicMessages,
            &format!("http://{addr}"),
            env_var,
        );
        let dir = TempDir::new().expect("temp dir");
        let cache = dir.path().join("anthropic-test.json");
        let models = fetch_provider_models_to(&entry, &cache).expect("fetch anthropic models");
        handle.join().expect("server thread clean");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "claude-3-5-sonnet");
        assert_eq!(models[0].max_input_tokens, Some(200_000));
        assert_eq!(models[0].max_output_tokens, Some(8192));
        // Anthropic shape does not carry owned_by/created.
        assert_eq!(models[0].owned_by, None);
        assert_eq!(models[0].created, None);
        unsafe { std::env::remove_var(env_var) };
    }

    #[test]
    fn fetch_provider_models_cache_returns_fresh_within_ttl() {
        // Pre-seed a cache file with a fresh fetched_at; the call must return
        // the cached list WITHOUT touching the network (no server is spawned,
        // so any HTTP call would fail).
        let dir = TempDir::new().expect("temp dir");
        let cache = dir.path().join("cached.json");
        let now = unix_now();
        let seeded = RemoteModelList {
            models: vec![RemoteModelInfo {
                id: "cached-model".into(),
                owned_by: None,
                created: None,
                max_input_tokens: None,
                max_output_tokens: None,
            }],
            fetched_at: now,
        };
        write_cached_models(&cache, &seeded).expect("seed cache");
        let entry = discovery_entry(
            "cached",
            ProviderKind::OpenAiCompatible,
            "http://127.0.0.1:1", // unreachable; must never be hit
            "PLOTFORGE_T12_UNSET_CACHE_VAR",
        );
        let models = fetch_provider_models_to(&entry, &cache).expect("fresh cache returns cached");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "cached-model");
    }

    #[test]
    fn fetch_provider_models_stale_cache_triggers_refetch() {
        // Seed a stale cache (fetched_at well in the past) and a live server;
        // the call must refetch and overwrite the cache with the fresh list.
        let body = serde_json::json!({
            "data": [ { "id": "fresh-model", "owned_by": "openai" } ]
        })
        .to_string();
        let (addr, handle) = models_tcp_server(body);
        let env_var = "PLOTFORGE_T12_STALE_TEST_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let dir = TempDir::new().expect("temp dir");
        let cache = dir.path().join("stale.json");
        let stale = RemoteModelList {
            models: vec![RemoteModelInfo {
                id: "stale-model".into(),
                owned_by: None,
                created: None,
                max_input_tokens: None,
                max_output_tokens: None,
            }],
            fetched_at: 0, // epoch: definitively stale
        };
        write_cached_models(&cache, &stale).expect("seed stale cache");
        let entry = discovery_entry(
            "stale",
            ProviderKind::OpenAiCompatible,
            &format!("http://{addr}"),
            env_var,
        );
        let models = fetch_provider_models_to(&entry, &cache).expect("stale cache refetches");
        handle.join().expect("server thread clean");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "fresh-model");
        // The cache must now hold the fresh list.
        let raw = std::fs::read_to_string(&cache).expect("read cache");
        assert!(raw.contains("fresh-model"));
        assert!(!raw.contains("stale-model"));
        unsafe { std::env::remove_var(env_var) };
    }

    #[test]
    fn fetch_provider_models_missing_credential_is_explicit_error() {
        // A provider with a credential_env_var that is not set must surface
        // `MissingCredential`, never a silent empty list or a 401.
        let env_var = "PLOTFORGE_T12_MISSING_CRED_UNSET_VAR";
        unsafe { std::env::remove_var(env_var) };
        let entry = discovery_entry(
            "no-cred",
            ProviderKind::OpenAiCompatible,
            "http://127.0.0.1:1",
            env_var,
        );
        let dir = TempDir::new().expect("temp dir");
        let cache = dir.path().join("no-cred.json");
        let error =
            fetch_provider_models_to(&entry, &cache).expect_err("missing credential must error");
        assert!(matches!(
            error,
            ModelDiscoveryError::MissingCredential { ref env_var } if env_var == "PLOTFORGE_T12_MISSING_CRED_UNSET_VAR"
        ));
        // No cache file must be written for a credential failure.
        assert!(!cache.exists());
    }

    #[test]
    fn fetch_provider_models_cache_file_created_and_read() {
        // A successful fetch creates a cache file whose contents round-trip
        // back through `read_cached_models` while fresh.
        let body = serde_json::json!({ "data": [ { "id": "m1" } ] }).to_string();
        let (addr, handle) = models_tcp_server(body);
        let env_var = "PLOTFORGE_T12_CACHE_IO_TEST_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let entry = discovery_entry(
            "cache-io",
            ProviderKind::OpenAiCompatible,
            &format!("http://{addr}"),
            env_var,
        );
        let dir = TempDir::new().expect("temp dir");
        let cache = dir.path().join("cache-io.json");
        let models = fetch_provider_models_to(&entry, &cache).expect("fetch");
        handle.join().expect("server thread clean");
        assert_eq!(models.len(), 1);
        assert!(cache.exists(), "cache file must be created");
        let now = unix_now();
        let read_back = read_cached_models(&cache, now)
            .expect("read cache")
            .expect("fresh cache present");
        assert_eq!(read_back.models.len(), 1);
        assert_eq!(read_back.models[0].id, "m1");
        // fetched_at is within a few seconds of now.
        assert!(read_back.fetched_at <= now);
        unsafe { std::env::remove_var(env_var) };
    }

    #[test]
    fn fetch_provider_models_non_2xx_surfaces_http_error() {
        // A server returning 401 surfaces an Http error carrying the status
        // code, never a silent empty list.
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n");
            let _ = stream.flush();
        });
        let env_var = "PLOTFORGE_T12_401_TEST_KEY";
        unsafe { std::env::set_var(env_var, "test-credential") };
        let entry = discovery_entry(
            "auth-fail",
            ProviderKind::OpenAiCompatible,
            &format!("http://{addr}"),
            env_var,
        );
        let dir = TempDir::new().expect("temp dir");
        let cache = dir.path().join("auth-fail.json");
        let error = fetch_provider_models_to(&entry, &cache).expect_err("401 must error");
        handle.join().expect("server thread clean");
        assert!(matches!(error, ModelDiscoveryError::Http { .. }));
        assert!(error.to_string().contains("401"));
        unsafe { std::env::remove_var(env_var) };
    }

    // -----------------------------------------------------------------------
    // T3.1: image provider registry + construction.
    // -----------------------------------------------------------------------

    fn sample_image_entry(id: &str, env_var: &str, enabled: bool) -> ImageProviderEntry {
        ImageProviderEntry {
            id: id.into(),
            endpoint_url: "https://api.openai.com/v1".into(),
            model: "gpt-image-1".into(),
            credential_env_var: env_var.into(),
            enabled,
            default_size: "1024x1024".into(),
            default_quality: "medium".into(),
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        }
    }

    #[test]
    fn media_daily_token_budget_is_rejected_as_unsupported() {
        let error = reject_media_daily_token_budget("image-a", Some(1))
            .expect_err("media daily tokens unsupported");
        assert!(matches!(error, ProviderBuildError::UnsupportedQuota { .. }));
        assert!(
            error
                .to_string()
                .contains("supported only by text providers")
        );
        reject_media_daily_token_budget("image-a", None).expect("no budget remains supported");
    }

    #[test]
    fn build_image_provider_dispatches_to_openai_image_client() {
        // build_image_provider must construct an OpenAiImageClient for an
        // image entry. The client is returned as Box<dyn ImageProvider>; we
        // exercise it with a Fake-free call against an unreachable endpoint
        // to confirm it is a real HTTP client (it errors with a transport
        // error, not a panic).
        let entry = sample_image_entry("openai-image", "PLOTFORGE_T31_BUILD_KEY", true);
        let provider = build_image_provider(&entry).expect("image provider builds");
        let request = crate::providers_image::ImageGenerationRequest {
            scene_key: "scene-1".into(),
            prompt: "test".into(),
            output_path: "assets/generated/scene-1.png".into(),
        };
        let error = provider
            .generate(&request)
            .expect_err("unreachable endpoint must error");
        // The error must be an explicit HTTP transport/timeout error, never a
        // silent success or a build failure.
        assert!(
            error.code.contains("image_provider_http")
                || error.code.contains("image_provider_missing_credential"),
            "expected an http error, got: {error}"
        );
    }

    #[test]
    fn resolve_image_provider_returns_first_enabled_entry() {
        let registry = ProviderRegistry {
            version: "1".into(),
            providers: Vec::new(),
            tts_providers: Vec::new(),
            image_providers: vec![
                sample_image_entry("disabled-image", "OFF_KEY", false),
                sample_image_entry("enabled-image", "ON_KEY", true),
                sample_image_entry("second-enabled", "ON2_KEY", true),
            ],
        };
        let resolved = resolve_image_provider(&registry).expect("an enabled entry exists");
        assert_eq!(resolved.id, "enabled-image");
        assert_eq!(resolved.model, "gpt-image-1");
    }

    #[test]
    fn resolve_image_provider_returns_none_when_all_disabled() {
        let registry = ProviderRegistry {
            version: "1".into(),
            providers: Vec::new(),
            tts_providers: Vec::new(),
            image_providers: vec![sample_image_entry("off", "OFF_KEY", false)],
        };
        assert!(resolve_image_provider(&registry).is_none());
    }

    #[test]
    fn resolve_image_provider_returns_none_when_empty() {
        let registry = ProviderRegistry::default();
        assert!(resolve_image_provider(&registry).is_none());
    }

    #[test]
    fn write_and_load_registry_roundtrips_image_providers() {
        let dir = TempDir::new().expect("temp dir");
        let registry_path = dir.path().join("providers.json");
        let registry = ProviderRegistry {
            version: "1".into(),
            providers: vec![ProviderEntry {
                id: "glm".into(),
                kind: ProviderKind::OpenAiCompatible,
                label: "GLM 4.6".into(),
                endpoint_url: "https://open.bigmodels.cn/api/paas/v4".into(),
                model: "glm-4.6".into(),
                credential_env_var: "ZAI_API_KEY".into(),
                enabled: true,
                max_output_tokens: None,
                max_concurrency: None,
                requests_per_minute: None,
                daily_token_budget: None,
            }],
            tts_providers: Vec::new(),
            image_providers: vec![sample_image_entry("openai-image", "OPENAI_API_KEY", true)],
        };
        write_provider_registry_to(&registry_path, &registry).expect("write via public API");
        let loaded = load_provider_registry_from(&registry_path).expect("read via public API");
        assert_eq!(loaded, registry);
        assert_eq!(loaded.image_providers.len(), 1);
        assert_eq!(loaded.image_providers[0].model, "gpt-image-1");
        // Redaction safety: the persisted file must not carry credentials.
        let raw = std::fs::read_to_string(&registry_path).expect("read raw");
        assert!(!raw.contains("api_key"));
        assert!(!raw.contains("sk-"));
    }
}
