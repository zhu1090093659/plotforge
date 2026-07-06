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

use plotforge_schema::{ProviderEntry, ProviderKind, ProviderRegistry};

use crate::providers_http::{
    AnthropicMessagesClient, OpenAiCompatibleClient, OpenAiResponsesClient,
};
use crate::providers_text::{
    ConfiguredTextModelProvider, EnvCredentialResolver, FakeTextModelProvider,
    ProviderCredentialError, ProviderCredentialResolver, TextModelClient, TextModelProvider,
    TextProviderConfig,
};

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
    let config = TextProviderConfig {
        enabled: entry.enabled,
        provider: entry.label.clone(),
        model: entry.model.clone(),
        endpoint_url: Some(entry.endpoint_url.clone()),
        credential_env_var: entry.credential_env_var.clone(),
    };
    if entry.credential_env_var.trim().is_empty() {
        Ok(Box::new(ConfiguredTextModelProvider::new(
            config,
            client,
            OptionalEnvCredentialResolver,
        )))
    } else {
        Ok(Box::new(ConfiguredTextModelProvider::new(
            config,
            client,
            EnvCredentialResolver,
        )))
    }
}

/// Resolves a `model_id` to the registered `ProviderEntry` that should serve
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
            }],
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
                },
                ProviderEntry {
                    id: "claude".into(),
                    kind: ProviderKind::AnthropicMessages,
                    label: "Claude".into(),
                    endpoint_url: "https://api.anthropic.com".into(),
                    model: "claude-sonnet-4".into(),
                    credential_env_var: "ANTHROPIC_API_KEY".into(),
                    enabled: false,
                },
            ],
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
}
