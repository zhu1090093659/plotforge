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

use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use plotforge_job::ThrottleConfig;
use plotforge_schema::{
    ImageProviderEntry, ModerationProviderEntry, ProviderEntry, ProviderKind, ProviderRegistry,
    RemoteModelInfo, RemoteModelList, TtsProviderEntry, redact_trace_text,
};

use crate::providers_http::{
    AnthropicMessagesClient, OpenAiCompatibleClient, OpenAiResponsesClient,
};
use crate::providers_image::{ImageProvider, OpenAiImageClient, validate_image_provider_entry};
use crate::providers_moderation::{
    ModerationProvider, OpenAiModerationClient, validate_moderation_provider_entry,
};
use crate::providers_text::{
    ConfiguredTextModelProvider, EnvCredentialResolver, FakeTextModelProvider,
    ProviderCredentialError, ProviderCredentialResolver, TextModelClient, TextModelProvider,
    TextProviderConfig,
};
use crate::providers_tts::{OpenAiTtsClient, validate_tts_provider_entry};
use crate::throttle::ProviderThrottleScope;

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
    write_provider_registry_atomic(path, registry, |_| Ok(()))
}

/// Runs one provider-registry mutation under a cross-process lock covering
/// the complete load-modify-write transaction.
pub fn mutate_provider_registry<T, E>(
    mutator: impl FnOnce(&mut ProviderRegistry) -> Result<T, E>,
) -> Result<T, ProviderRegistryMutationError<E>> {
    let path = provider_registry_path();
    mutate_provider_registry_with_resolved_path(path.as_deref(), mutator)
}

/// Applies the same missing-path semantics as the user-global transaction
/// without requiring tests to mutate HOME or platform config environment.
/// A mutation error wins over the later persistence failure, matching the
/// legacy load-default-then-write flow; a successful in-memory mutation cannot
/// be persisted and therefore returns `NoConfigDir`.
fn mutate_provider_registry_with_resolved_path<T, E>(
    path: Option<&Path>,
    mutator: impl FnOnce(&mut ProviderRegistry) -> Result<T, E>,
) -> Result<T, ProviderRegistryMutationError<E>> {
    let Some(path) = path else {
        let mut registry = ProviderRegistry::default();
        let _ = mutator(&mut registry).map_err(ProviderRegistryMutationError::Mutation)?;
        return Err(ProviderRegistryMutationError::Registry(
            ProviderRegistryError::NoConfigDir,
        ));
    };
    mutate_provider_registry_at(path, mutator)
}

/// Path-injected counterpart for adapters and hermetic concurrency tests.
pub fn mutate_provider_registry_at<T, E>(
    path: &Path,
    mutator: impl FnOnce(&mut ProviderRegistry) -> Result<T, E>,
) -> Result<T, ProviderRegistryMutationError<E>> {
    let parent = registry_parent(path);
    std::fs::create_dir_all(parent).map_err(|source| {
        ProviderRegistryMutationError::Registry(ProviderRegistryError::WriteFailed {
            path: parent.display().to_string(),
            source,
        })
    })?;
    let lock_path = provider_registry_lock_path(path);
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|source| {
            ProviderRegistryMutationError::Registry(ProviderRegistryError::LockFailed {
                path: lock_path.display().to_string(),
                source,
            })
        })?;
    lock_file.lock().map_err(|source| {
        ProviderRegistryMutationError::Registry(ProviderRegistryError::LockFailed {
            path: lock_path.display().to_string(),
            source,
        })
    })?;

    let mut registry =
        load_provider_registry_from(path).map_err(ProviderRegistryMutationError::Registry)?;
    let output = mutator(&mut registry).map_err(ProviderRegistryMutationError::Mutation)?;
    write_provider_registry_to(path, &registry).map_err(ProviderRegistryMutationError::Registry)?;
    Ok(output)
}

fn write_provider_registry_atomic(
    path: &Path,
    registry: &ProviderRegistry,
    before_rename: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), ProviderRegistryError> {
    let parent = registry_parent(path);
    std::fs::create_dir_all(parent).map_err(|source| ProviderRegistryError::WriteFailed {
        path: parent.display().to_string(),
        source,
    })?;
    let content = serde_json::to_string_pretty(registry)
        .map_err(|error| ProviderRegistryError::SerializeFailed { source: error })?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|source| {
        ProviderRegistryError::WriteFailed {
            path: parent.display().to_string(),
            source,
        }
    })?;
    let temp_path = temp.path().to_path_buf();
    temp.write_all(content.as_bytes())
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|source| ProviderRegistryError::WriteFailed {
            path: temp_path.display().to_string(),
            source,
        })?;
    before_rename(&temp_path).map_err(|source| ProviderRegistryError::WriteFailed {
        path: temp_path.display().to_string(),
        source,
    })?;
    temp.persist(path)
        .map_err(|error| ProviderRegistryError::WriteFailed {
            path: path.display().to_string(),
            source: error.error,
        })?;
    sync_registry_parent(parent)?;
    Ok(())
}

fn registry_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn provider_registry_lock_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("providers.json");
    path.with_file_name(format!(".{file_name}.lock"))
}

#[cfg(unix)]
fn sync_registry_parent(parent: &Path) -> Result<(), ProviderRegistryError> {
    std::fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| ProviderRegistryError::WriteFailed {
            path: parent.display().to_string(),
            source,
        })
}

/// Rust's standard library has no portable directory-sync primitive on
/// non-Unix targets. `NamedTempFile::persist` still supplies atomic replace;
/// the extra crash-durability sync is applied wherever the platform exposes it.
#[cfg(not(unix))]
fn sync_registry_parent(_parent: &Path) -> Result<(), ProviderRegistryError> {
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
    let throttle = build_throttle(
        ProviderThrottleScope::Text,
        throttle_config_hash(&[
            provider_kind_key(entry.kind),
            &entry.endpoint_url,
            &entry.model,
            &entry.credential_env_var,
        ]),
        ThrottleConfig {
            provider_id: entry.id.clone(),
            max_concurrency: entry.max_concurrency,
            requests_per_minute: entry.requests_per_minute,
            daily_token_budget: entry.daily_token_budget,
        },
    )?;
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
    validate_image_provider_entry(entry).map_err(|error| {
        ProviderBuildError::InvalidConfiguration {
            message: error.message,
        }
    })?;
    reject_media_daily_token_budget(&entry.id, entry.daily_token_budget)?;
    let throttle = build_throttle(
        ProviderThrottleScope::Image,
        throttle_config_hash(&[&entry.endpoint_url, &entry.model, &entry.credential_env_var]),
        ThrottleConfig {
            provider_id: entry.id.clone(),
            max_concurrency: entry.max_concurrency,
            requests_per_minute: entry.requests_per_minute,
            daily_token_budget: None,
        },
    )?;
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
    validate_tts_provider_entry(entry).map_err(|error| {
        ProviderBuildError::InvalidConfiguration {
            message: error.message,
        }
    })?;
    reject_media_daily_token_budget(&entry.id, entry.daily_token_budget)?;
    let throttle = build_throttle(
        ProviderThrottleScope::Tts,
        throttle_config_hash(&[&entry.endpoint_url, &entry.model, &entry.credential_env_var]),
        ThrottleConfig {
            provider_id: entry.id.clone(),
            max_concurrency: entry.max_concurrency,
            requests_per_minute: entry.requests_per_minute,
            daily_token_budget: None,
        },
    )?;
    OpenAiTtsClient::from_entry(entry, EnvCredentialResolver)
        .map(|client| client.with_throttle(throttle))
        .map_err(|error| ProviderBuildError::ClientConstruction {
            provider_id: entry.id.clone(),
            message: error.message,
        })
}

/// Builds an OpenAI-compatible moderation provider. Empty credential env-var
/// names select the explicit no-auth resolver; non-empty names stay strict.
pub fn build_moderation_provider(
    entry: &ModerationProviderEntry,
) -> Result<Box<dyn ModerationProvider>, ProviderBuildError> {
    validate_moderation_provider_entry(entry).map_err(|error| {
        ProviderBuildError::InvalidConfiguration {
            message: error.message,
        }
    })?;
    reject_non_text_daily_token_budget(&entry.id, "moderation", entry.daily_token_budget)?;
    let throttle = build_throttle(
        ProviderThrottleScope::Moderation,
        moderation_throttle_config_hash(entry)?,
        ThrottleConfig {
            provider_id: entry.id.clone(),
            max_concurrency: entry.max_concurrency,
            requests_per_minute: entry.requests_per_minute,
            daily_token_budget: None,
        },
    )?;
    if entry.credential_env_var.trim().is_empty() {
        OpenAiModerationClient::new(entry, OptionalEnvCredentialResolver)
            .map(|client| Box::new(client.with_throttle(throttle)) as Box<dyn ModerationProvider>)
            .map_err(|error| ProviderBuildError::ClientConstruction {
                provider_id: entry.id.clone(),
                message: error.message,
            })
    } else {
        OpenAiModerationClient::new(entry, EnvCredentialResolver)
            .map(|client| Box::new(client.with_throttle(throttle)) as Box<dyn ModerationProvider>)
            .map_err(|error| ProviderBuildError::ClientConstruction {
                provider_id: entry.id.clone(),
                message: error.message,
            })
    }
}

/// Returns the first enabled moderation entry, preserving registry order.
pub fn resolve_moderation_provider(
    registry: &ProviderRegistry,
) -> Option<&ModerationProviderEntry> {
    registry
        .moderation_providers
        .iter()
        .find(|entry| entry.enabled)
}

/// Canonical moderation reproducibility identity. Quota fields are excluded;
/// the throttle registry uses its own normalized upstream identity below.
pub fn moderation_config_hash(entry: &ModerationProviderEntry) -> String {
    format!(
        "sha256:{}",
        crate::shared::stable_sha256_hash(&format!(
            "id={}\nendpoint_url={}\nmodel={}\ncredential_env_var={}\nenabled={}\n",
            entry.id, entry.endpoint_url, entry.model, entry.credential_env_var, entry.enabled,
        ))
    )
}

/// Stable upstream identity for the process-shared moderation throttle gate.
/// The provider id is already a separate `ProviderThrottleKey` field, while
/// enabled/quota edits must reconfigure the same gate without erasing debt.
fn moderation_throttle_config_hash(
    entry: &ModerationProviderEntry,
) -> Result<String, ProviderBuildError> {
    let request_endpoint =
        crate::shared::join_provider_endpoint(&entry.endpoint_url, "moderations")
            .map_err(|message| ProviderBuildError::InvalidConfiguration { message })?;
    Ok(throttle_config_hash(&[
        "openai_moderations",
        &request_endpoint,
        &entry.model,
        &entry.credential_env_var,
    ]))
}

fn build_throttle(
    scope: ProviderThrottleScope,
    provider_config_hash: String,
    config: ThrottleConfig,
) -> Result<Option<crate::throttle::ProviderThrottle>, ProviderBuildError> {
    let provider_id = config.provider_id.clone();
    crate::throttle::ProviderThrottle::shared_from_config(scope, provider_config_hash, config)
        .map_err(|error| ProviderBuildError::InvalidThrottle {
            provider_id,
            message: error.to_string(),
        })
}

fn throttle_config_hash(parts: &[&str]) -> String {
    format!(
        "sha256:{}",
        crate::shared::stable_sha256_hash(&parts.join("\n"))
    )
}

fn provider_kind_key(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::OpenAiCompatible => "openai_compatible",
        ProviderKind::OpenAiResponses => "openai_responses",
        ProviderKind::AnthropicMessages => "anthropic_messages",
    }
}

fn reject_media_daily_token_budget(
    provider_id: &str,
    daily_token_budget: Option<u64>,
) -> Result<(), ProviderBuildError> {
    reject_non_text_daily_token_budget(provider_id, "image and TTS", daily_token_budget)
}

fn reject_non_text_daily_token_budget(
    provider_id: &str,
    provider_kind: &str,
    daily_token_budget: Option<u64>,
) -> Result<(), ProviderBuildError> {
    if daily_token_budget.is_some() {
        return Err(ProviderBuildError::UnsupportedQuota {
            provider_id: provider_id.to_string(),
            message: format!(
                "daily_token_budget is supported only by text providers because {provider_kind} usage does not report output tokens"
            ),
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
    #[error("failed to lock provider registry at {path}: {source}")]
    LockFailed {
        path: String,
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub enum ProviderRegistryMutationError<E> {
    Registry(ProviderRegistryError),
    Mutation(E),
}

impl<E> std::fmt::Display for ProviderRegistryMutationError<E>
where
    E: std::fmt::Display,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Registry(error) => write!(formatter, "{error}"),
            Self::Mutation(error) => write!(formatter, "{error}"),
        }
    }
}

impl<E> std::error::Error for ProviderRegistryMutationError<E> where
    E: std::fmt::Debug + std::fmt::Display
{
}

/// Errors raised by `build_provider_client` / `build_text_provider`.
#[derive(Debug, thiserror::Error)]
pub enum ProviderBuildError {
    #[error("invalid provider configuration: {message}")]
    InvalidConfiguration { message: String },
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
    use std::sync::{Arc, Barrier};
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
    fn atomic_registry_write_replaces_existing_file() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("providers.json");
        let first = ProviderRegistry {
            version: "first".into(),
            ..ProviderRegistry::default()
        };
        let second = ProviderRegistry {
            version: "second".into(),
            ..ProviderRegistry::default()
        };
        write_provider_registry_to(&path, &first).expect("first write");
        write_provider_registry_to(&path, &second).expect("atomic replacement");
        assert_eq!(
            load_provider_registry_from(&path).expect("load replaced file"),
            second
        );
    }

    #[test]
    fn failed_atomic_registry_write_preserves_previous_json() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("providers.json");
        let previous = ProviderRegistry {
            version: "previous".into(),
            ..ProviderRegistry::default()
        };
        let replacement = ProviderRegistry {
            version: "replacement".into(),
            ..ProviderRegistry::default()
        };
        write_provider_registry_to(&path, &previous).expect("baseline write");
        let error = write_provider_registry_atomic(&path, &replacement, |_| {
            Err(std::io::Error::other("injected before persist"))
        })
        .expect_err("injected failure");
        assert!(matches!(error, ProviderRegistryError::WriteFailed { .. }));
        assert_eq!(
            load_provider_registry_from(&path).expect("previous JSON remains valid"),
            previous
        );
        assert_eq!(
            std::fs::read_dir(dir.path())
                .expect("read temp dir")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains("tmp"))
                .count(),
            0,
            "failed atomic write must clean its temporary file"
        );
    }

    #[test]
    fn missing_registry_path_runs_successful_mutation_then_returns_no_config_dir() {
        let error = mutate_provider_registry_with_resolved_path(None, |registry| {
            registry
                .moderation_providers
                .push(sample_moderation_entry("no-config", false));
            Ok::<_, &'static str>(())
        })
        .expect_err("successful mutation cannot be persisted without a config dir");

        assert!(matches!(
            error,
            ProviderRegistryMutationError::Registry(ProviderRegistryError::NoConfigDir)
        ));
    }

    #[test]
    fn missing_registry_path_preserves_mutation_error_before_no_config_dir() {
        let error = mutate_provider_registry_with_resolved_path(None, |registry| {
            assert!(registry.providers.is_empty());
            Err::<(), _>("provider_not_found")
        })
        .expect_err("mutation error wins");

        assert!(matches!(
            error,
            ProviderRegistryMutationError::Mutation("provider_not_found")
        ));
    }

    #[test]
    fn concurrent_registry_mutations_do_not_lose_updates() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("providers.json");
        let barrier = Arc::new(Barrier::new(3));
        let handles = ["one", "two"].map(|id| {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                mutate_provider_registry_at(&path, |registry| {
                    std::thread::sleep(std::time::Duration::from_millis(25));
                    registry
                        .moderation_providers
                        .push(sample_moderation_entry(id, false));
                    Ok::<_, String>(())
                })
                .expect("locked mutation");
            })
        });
        barrier.wait();
        for handle in handles {
            handle.join().expect("mutation thread");
        }
        let registry = load_provider_registry_from(&path).expect("load mutations");
        let mut ids = registry
            .moderation_providers
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(ids, vec!["one", "two"]);
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
            moderation_providers: Vec::new(),
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
            moderation_providers: Vec::new(),
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
        let first_config = ThrottleConfig {
            provider_id: "registry-rebuild-throttle-test".into(),
            max_concurrency: Some(1),
            requests_per_minute: Some(2),
            ..ThrottleConfig::default()
        };
        let first = build_throttle(
            ProviderThrottleScope::Text,
            "sha256:registry-rebuild".into(),
            first_config,
        )
        .expect("first provider build")
        .expect("quota creates throttle");
        let permit = first.acquire().expect("first build consumes capacity");

        let second = build_throttle(
            ProviderThrottleScope::Text,
            "sha256:registry-rebuild".into(),
            ThrottleConfig {
                provider_id: "registry-rebuild-throttle-test".into(),
                max_concurrency: Some(1),
                requests_per_minute: Some(1),
                ..ThrottleConfig::default()
            },
        )
        .expect("second provider build")
        .expect("quota creates throttle");
        let failure = second
            .acquire()
            .expect_err("rebuilt provider must observe the in-flight permit")
            .into_failure();
        assert!(matches!(
            failure,
            crate::throttle::ThrottleFailure::RateLimit { .. }
        ));
        drop(permit);
        let failure = second
            .acquire()
            .expect_err("quota update must not refill the consumed bucket")
            .into_failure();
        assert!(matches!(
            failure,
            crate::throttle::ThrottleFailure::RateLimit { .. }
        ));
    }

    #[test]
    fn provider_kinds_with_the_same_id_do_not_share_throttle_state() {
        let config = ThrottleConfig {
            provider_id: "cross-kind-throttle-test".into(),
            requests_per_minute: Some(1),
            ..ThrottleConfig::default()
        };
        let text = build_throttle(
            ProviderThrottleScope::Text,
            "sha256:cross-kind".into(),
            config.clone(),
        )
        .expect("text build")
        .expect("text throttle");
        let image = build_throttle(
            ProviderThrottleScope::Image,
            "sha256:cross-kind".into(),
            config,
        )
        .expect("image build")
        .expect("image throttle");

        drop(text.acquire().expect("text token"));
        drop(image.acquire().expect("independent image token"));
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
    fn persisted_unsafe_media_provider_ids_are_rejected_before_client_use() {
        let image = sample_image_entry("sk-image-secret", "", true);
        let image_error = match build_image_provider(&image) {
            Ok(_) => panic!("unsafe image provider id must not build"),
            Err(error) => error,
        };
        assert!(matches!(
            image_error,
            ProviderBuildError::InvalidConfiguration { .. }
        ));
        assert!(!image_error.to_string().contains("sk-image-secret"));

        let tts = TtsProviderEntry {
            id: "sk-tts-secret".into(),
            endpoint_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini-tts".into(),
            credential_env_var: String::new(),
            enabled: true,
            voice: "coral".into(),
            format: "mp3".into(),
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        };
        let tts_error = match build_tts_provider(&tts) {
            Ok(_) => panic!("unsafe TTS provider id must not build"),
            Err(error) => error,
        };
        assert!(matches!(
            tts_error,
            ProviderBuildError::InvalidConfiguration { .. }
        ));
        assert!(!tts_error.to_string().contains("sk-tts-secret"));
    }

    #[test]
    fn resolve_image_provider_returns_first_enabled_entry() {
        let registry = ProviderRegistry {
            version: "1".into(),
            providers: Vec::new(),
            tts_providers: Vec::new(),
            moderation_providers: Vec::new(),
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
            moderation_providers: Vec::new(),
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
            moderation_providers: Vec::new(),
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

    fn sample_moderation_entry(id: &str, enabled: bool) -> ModerationProviderEntry {
        ModerationProviderEntry {
            id: id.into(),
            endpoint_url: "https://api.openai.com/v1".into(),
            model: "omni-moderation-latest".into(),
            credential_env_var: "OPENAI_API_KEY".into(),
            enabled,
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        }
    }

    #[test]
    fn resolve_moderation_provider_returns_first_enabled() {
        let registry = ProviderRegistry {
            moderation_providers: vec![
                sample_moderation_entry("disabled", false),
                sample_moderation_entry("first", true),
                sample_moderation_entry("second", true),
            ],
            ..ProviderRegistry::default()
        };
        assert_eq!(
            resolve_moderation_provider(&registry).map(|entry| entry.id.as_str()),
            Some("first")
        );
    }

    #[test]
    fn resolve_moderation_provider_none_when_empty_or_disabled() {
        assert!(resolve_moderation_provider(&ProviderRegistry::default()).is_none());
        let registry = ProviderRegistry {
            moderation_providers: vec![sample_moderation_entry("disabled", false)],
            ..ProviderRegistry::default()
        };
        assert!(resolve_moderation_provider(&registry).is_none());
    }

    #[test]
    fn build_moderation_provider_constructs_no_auth_client() {
        let mut entry = sample_moderation_entry("local", true);
        entry.endpoint_url = "http://127.0.0.1:9/v1".into();
        entry.credential_env_var.clear();
        build_moderation_provider(&entry).expect("moderation provider builds");
    }

    #[test]
    fn build_moderation_provider_rejects_daily_token_budget() {
        let mut entry = sample_moderation_entry("budgeted", true);
        entry.daily_token_budget = Some(100);
        let error = match build_moderation_provider(&entry) {
            Ok(_) => panic!("daily budget must be rejected"),
            Err(error) => error,
        };
        assert!(matches!(error, ProviderBuildError::UnsupportedQuota { .. }));
    }

    #[test]
    fn persisted_unsafe_moderation_entry_is_rejected_before_network() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("providers.json");
        let mut entry = sample_moderation_entry("manual", true);
        entry.credential_env_var = "sk-secret-literal".into();
        let registry = ProviderRegistry {
            moderation_providers: vec![entry],
            ..ProviderRegistry::default()
        };
        write_provider_registry_to(&path, &registry).expect("simulate manual registry");
        let loaded = load_provider_registry_from(&path).expect("load manual registry");
        let error = match build_moderation_provider(&loaded.moderation_providers[0]) {
            Ok(_) => panic!("unsafe persisted entry must not build"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            ProviderBuildError::InvalidConfiguration { .. }
        ));
        assert!(!error.to_string().contains("sk-secret-literal"));
    }

    #[test]
    fn moderation_config_hash_is_stable_and_excludes_quota() {
        let entry = sample_moderation_entry("moderation", true);
        let mut quota_edit = entry.clone();
        quota_edit.max_concurrency = Some(2);
        quota_edit.requests_per_minute = Some(30);
        assert_eq!(
            moderation_config_hash(&entry),
            moderation_config_hash(&entry)
        );
        assert_eq!(
            moderation_config_hash(&entry),
            moderation_config_hash(&quota_edit),
            "quota edits must reconfigure the same shared gate"
        );

        let mut model_edit = entry.clone();
        model_edit.model = "text-moderation-latest".into();
        assert_ne!(
            moderation_config_hash(&entry),
            moderation_config_hash(&model_edit)
        );
        assert!(!plotforge_schema::contains_secret_marker_text(
            &moderation_config_hash(&entry)
        ));

        for edit in [
            |entry: &mut ModerationProviderEntry| entry.id.push_str("-other"),
            |entry: &mut ModerationProviderEntry| entry.endpoint_url.push_str("/other"),
            |entry: &mut ModerationProviderEntry| entry.credential_env_var.push_str("_OTHER"),
            |entry: &mut ModerationProviderEntry| entry.enabled = !entry.enabled,
        ] {
            let mut changed = entry.clone();
            edit(&mut changed);
            assert_ne!(
                moderation_config_hash(&entry),
                moderation_config_hash(&changed)
            );
        }
    }

    #[test]
    fn moderation_enabled_toggle_preserves_shared_rate_debt() {
        let mut entry = sample_moderation_entry("moderation-enabled-toggle", false);
        entry.requests_per_minute = Some(1);
        let first = build_throttle(
            ProviderThrottleScope::Moderation,
            moderation_throttle_config_hash(&entry).expect("throttle identity"),
            ThrottleConfig {
                provider_id: entry.id.clone(),
                requests_per_minute: entry.requests_per_minute,
                ..ThrottleConfig::default()
            },
        )
        .expect("first throttle")
        .expect("configured gate");
        drop(first.acquire().expect("first request consumes capacity"));

        entry.enabled = true;
        let rebuilt = build_throttle(
            ProviderThrottleScope::Moderation,
            moderation_throttle_config_hash(&entry).expect("throttle identity"),
            ThrottleConfig {
                provider_id: entry.id.clone(),
                requests_per_minute: entry.requests_per_minute,
                ..ThrottleConfig::default()
            },
        )
        .expect("rebuilt throttle")
        .expect("configured gate");
        let failure = rebuilt
            .acquire()
            .expect_err("enabled toggle must not reset RPM debt")
            .into_failure();
        assert!(matches!(
            failure,
            crate::throttle::ThrottleFailure::RateLimit { .. }
        ));
    }

    #[test]
    fn moderation_canonical_endpoint_preserves_shared_rate_debt() {
        let mut entry = sample_moderation_entry("moderation-canonical-endpoint", true);
        entry.endpoint_url = "https://api.openai.com:443/v1/".into();
        entry.requests_per_minute = Some(1);
        let first = build_throttle(
            ProviderThrottleScope::Moderation,
            moderation_throttle_config_hash(&entry).expect("first identity"),
            ThrottleConfig {
                provider_id: entry.id.clone(),
                requests_per_minute: entry.requests_per_minute,
                ..ThrottleConfig::default()
            },
        )
        .expect("first throttle")
        .expect("configured gate");
        drop(first.acquire().expect("consume capacity"));

        entry.endpoint_url = "https://api.openai.com/v1".into();
        let rebuilt = build_throttle(
            ProviderThrottleScope::Moderation,
            moderation_throttle_config_hash(&entry).expect("canonical identity"),
            ThrottleConfig {
                provider_id: entry.id.clone(),
                requests_per_minute: entry.requests_per_minute,
                ..ThrottleConfig::default()
            },
        )
        .expect("rebuilt throttle")
        .expect("configured gate");
        assert!(matches!(
            rebuilt
                .acquire()
                .expect_err("equivalent endpoint must retain RPM debt")
                .into_failure(),
            crate::throttle::ThrottleFailure::RateLimit { .. }
        ));
    }
}
