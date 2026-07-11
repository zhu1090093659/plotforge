//! Provider registry schema contracts.
//!
//! These types describe the user-global, local-only provider registry that
//! backs real model provider routing. The registry lives at
//! `~/.plotforge/providers.json` and never enters project source, contracts,
//! traces, or export packages. Credentials are referenced indirectly by
//! environment-variable name (`credential_env_var`); the registry never
//! stores credential values, endpoint secrets, or raw provider responses.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The HTTP API format a provider speaks. Drives which HTTP client the agent
/// crate constructs; never carried as an endpoint URL or credential. The
/// `OpenAi*` variants use explicit `rename` attributes so the contract spells
/// them `openai_*` (matching the OpenAI brand) rather than the default
/// snake_case `open_ai_*`.
#[derive(Clone, Copy, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[serde(rename = "openai_compatible")]
    #[default]
    OpenAiCompatible,
    #[serde(rename = "openai_responses")]
    OpenAiResponses,
    AnthropicMessages,
}

/// A single registered provider entry. `credential_env_var` names the shell
/// environment variable that holds the credential; the value itself is never
/// serialized here, in traces, or in any project source.
///
/// `max_output_tokens` is an optional override for the provider's output token
/// cap. When `None`, each HTTP client uses its own default (4096 for OpenAI
/// variants, 8192 for Anthropic Messages). When `Some`, the value is
/// propagated into the request body so a user can tune output length per
/// provider without editing code. The field is `#[serde(default)]` so existing
/// registries without it still deserialize. The three quota fields are
/// optional pre-flight throttle inputs owned by `plotforge-job`; `None`
/// preserves the legacy unthrottled behaviour.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProviderEntry {
    pub id: String,
    pub kind: ProviderKind,
    pub label: String,
    pub endpoint_url: String,
    pub model: String,
    pub credential_env_var: String,
    pub enabled: bool,
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub max_concurrency: Option<u32>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub requests_per_minute: Option<u32>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub daily_token_budget: Option<u64>,
}

/// A single registered image provider entry. Like `ProviderEntry`, this is
/// a routing record only — `credential_env_var` names the shell environment
/// variable that holds the credential; the value itself is never serialized
/// here, in traces, or in any project source.
///
/// `model` selects the upstream image model (e.g. `gpt-image-1`,
/// `gpt-image-2`). `default_size` is the requested image dimensions
/// (e.g. `1024x1024`) and `default_quality` is the generation quality tier
/// (e.g. `medium`). All three are forwarded into the OpenAI Images API
/// request body. Both `default_size` and `default_quality` are
/// `#[serde(default)]` so existing registries without them still deserialize.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImageProviderEntry {
    pub id: String,
    pub endpoint_url: String,
    pub model: String,
    pub credential_env_var: String,
    pub enabled: bool,
    #[serde(default = "default_image_size")]
    pub default_size: String,
    #[serde(default = "default_image_quality")]
    pub default_quality: String,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub max_concurrency: Option<u32>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub requests_per_minute: Option<u32>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub daily_token_budget: Option<u64>,
}

fn default_image_size() -> String {
    "1024x1024".into()
}

fn default_image_quality() -> String {
    "medium".into()
}

/// A registered TTS (text-to-speech) provider entry. Mirrors the
/// `ImageProviderEntry` shape for audio providers. `credential_env_var` names
/// the shell environment variable holding the credential; the value is never
/// serialized here, in traces, or in any project source. `voice` is the
/// default voice for synthesis (the request may override); `format` is the
/// audio output format (mp3, wav, opus, aac, flac, pcm). Both `voice` and
/// `format` are `#[serde(default)]` so existing registries without them still
/// deserialize.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TtsProviderEntry {
    pub id: String,
    pub endpoint_url: String,
    pub model: String,
    pub credential_env_var: String,
    pub enabled: bool,
    #[serde(default = "default_tts_voice")]
    pub voice: String,
    #[serde(default = "default_tts_format")]
    pub format: String,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub max_concurrency: Option<u32>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub requests_per_minute: Option<u32>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub daily_token_budget: Option<u64>,
}

/// A registered moderation provider entry. This is a routing record only:
/// `credential_env_var` names the shell environment variable holding the
/// credential, while the credential value itself is never serialized here,
/// in traces, or in project source. The optional quota fields are pre-flight
/// throttle inputs owned by `plotforge-job`; absent fields preserve the legacy
/// unthrottled behaviour.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModerationProviderEntry {
    pub id: String,
    pub endpoint_url: String,
    pub model: String,
    pub credential_env_var: String,
    pub enabled: bool,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub max_concurrency: Option<u32>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub requests_per_minute: Option<u32>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub daily_token_budget: Option<u64>,
}

fn default_tts_voice() -> String {
    "coral".into()
}

fn default_tts_format() -> String {
    "mp3".into()
}

/// The persisted registry file (`~/.plotforge/providers.json`). An empty
/// registry is the default for fresh installs; provider wiring is opt-in.
///
/// `image_providers`, `tts_providers`, and `moderation_providers` are
/// `#[serde(default)]` so registries written before those provider families
/// were introduced still deserialize. Each provider family is an independent
/// list with its own resolver in the agent adapter.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct ProviderRegistry {
    pub version: String,
    pub providers: Vec<ProviderEntry>,
    #[serde(default)]
    pub image_providers: Vec<ImageProviderEntry>,
    #[serde(default)]
    pub tts_providers: Vec<TtsProviderEntry>,
    #[serde(default)]
    pub moderation_providers: Vec<ModerationProviderEntry>,
}

/// A single model discovered from a provider's upstream `/models` (or
/// equivalent) endpoint. The fields mirror the union of the OpenAI
/// (`{ id, owned_by, created }`) and Anthropic
/// (`{ id, max_input_tokens, max_output_tokens }`) model-list response shapes;
/// every field except `id` is optional because neither provider returns the
/// full set. No credential, endpoint, or raw response body is ever stored
/// here — only the descriptive model metadata.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RemoteModelInfo {
    pub id: String,
    #[serde(default)]
    pub owned_by: Option<String>,
    #[serde(default)]
    pub created: Option<u64>,
    #[serde(default)]
    pub max_input_tokens: Option<u32>,
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
}

/// The cached model list for a single provider, persisted at
/// `~/.plotforge/cache/models/{provider_id}.json`. `fetched_at` is a Unix
/// timestamp (seconds); the registry treats a cache entry older than the
/// discovery TTL (1 hour) as stale and refetches.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RemoteModelList {
    pub models: Vec<RemoteModelInfo>,
    pub fetched_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> ProviderEntry {
        ProviderEntry {
            id: "glm".into(),
            kind: ProviderKind::OpenAiCompatible,
            label: "GLM 4.6".into(),
            endpoint_url: "https://open.bigmodels.cn/api/paas/v4".into(),
            model: "glm-4.6".into(),
            credential_env_var: "ZAI_API_KEY".into(),
            enabled: true,
            max_output_tokens: None,
            max_concurrency: Some(2),
            requests_per_minute: Some(30),
            daily_token_budget: Some(100_000),
        }
    }

    #[test]
    fn provider_kind_renames_snake_case() {
        let kinds = [
            (ProviderKind::OpenAiCompatible, "openai_compatible"),
            (ProviderKind::OpenAiResponses, "openai_responses"),
            (ProviderKind::AnthropicMessages, "anthropic_messages"),
        ];
        for (kind, expected) in kinds {
            let encoded = serde_json::to_string(&kind).expect("serialize kind");
            assert_eq!(encoded, format!("\"{expected}\""));
            let decoded: ProviderKind = serde_json::from_str(&encoded).expect("deserialize kind");
            assert_eq!(decoded, kind);
        }
    }

    #[test]
    fn provider_entry_roundtrips_json_with_quota_fields() {
        let entry = sample_entry();
        let encoded = serde_json::to_string_pretty(&entry).expect("serialize entry");
        let decoded: ProviderEntry = serde_json::from_str(&encoded).expect("deserialize entry");
        assert_eq!(decoded, entry);
        assert_eq!(decoded.max_concurrency, Some(2));
        assert_eq!(decoded.requests_per_minute, Some(30));
        assert_eq!(decoded.daily_token_budget, Some(100_000));
    }

    #[test]
    fn provider_quota_contracts_require_positive_values() {
        for schema in [
            serde_json::to_value(schemars::schema_for!(ProviderEntry)).expect("provider schema"),
            serde_json::to_value(schemars::schema_for!(ImageProviderEntry)).expect("image schema"),
            serde_json::to_value(schemars::schema_for!(TtsProviderEntry)).expect("TTS schema"),
            serde_json::to_value(schemars::schema_for!(ModerationProviderEntry))
                .expect("moderation schema"),
        ] {
            for field in [
                "max_concurrency",
                "requests_per_minute",
                "daily_token_budget",
            ] {
                let property = &schema["properties"][field];
                assert!(
                    contains_minimum_one(property),
                    "{field} must carry minimum=1 in {property}"
                );
            }
        }
    }

    fn contains_minimum_one(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(fields) => {
                fields.get("minimum") == Some(&serde_json::json!(1))
                    || fields.values().any(contains_minimum_one)
            }
            serde_json::Value::Array(values) => values.iter().any(contains_minimum_one),
            _ => false,
        }
    }

    #[test]
    fn provider_entry_rejects_secret_fields() {
        let entry = sample_entry();
        let mut value = serde_json::to_value(&entry).expect("entry value");
        value["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<ProviderEntry>(value)
            .expect_err("api_key field should be rejected");
        assert!(error.to_string().contains("unknown field"));

        let mut value = serde_json::to_value(sample_entry()).expect("entry value");
        value["secret_token"] = serde_json::json!("tok-secret-marker");
        let error = serde_json::from_value::<ProviderEntry>(value)
            .expect_err("secret_token field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn provider_registry_roundtrips_json() {
        let registry = ProviderRegistry {
            version: "1".into(),
            providers: vec![
                sample_entry(),
                ProviderEntry {
                    id: "local-ollama".into(),
                    kind: ProviderKind::OpenAiCompatible,
                    label: "Local Ollama".into(),
                    endpoint_url: "http://localhost:11434/v1".into(),
                    model: "qwen2.5".into(),
                    credential_env_var: String::new(),
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
        let encoded = serde_json::to_string_pretty(&registry).expect("serialize registry");
        let decoded: ProviderRegistry =
            serde_json::from_str(&encoded).expect("deserialize registry");
        assert_eq!(decoded, registry);
        assert_eq!(decoded.providers.len(), 2);
    }

    #[test]
    fn provider_registry_default_is_empty() {
        let registry = ProviderRegistry::default();
        assert!(registry.providers.is_empty());
        assert!(registry.version.is_empty());
        assert!(registry.tts_providers.is_empty());
        assert!(registry.moderation_providers.is_empty());
    }

    #[test]
    fn provider_registry_rejects_unknown_fields() {
        let mut value = serde_json::to_value(ProviderRegistry::default()).expect("registry value");
        value["api_keys"] = serde_json::json!([{"secret": "sk-test-secret-marker"}]);
        let error = serde_json::from_value::<ProviderRegistry>(value)
            .expect_err("unknown registry field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn provider_entry_roundtrips_max_output_tokens() {
        let mut entry = sample_entry();
        entry.max_output_tokens = Some(8192);
        let encoded = serde_json::to_string_pretty(&entry).expect("serialize entry");
        assert!(encoded.contains("max_output_tokens"));
        let decoded: ProviderEntry = serde_json::from_str(&encoded).expect("deserialize entry");
        assert_eq!(decoded, entry);
        assert_eq!(decoded.max_output_tokens, Some(8192));
    }

    #[test]
    fn provider_entry_backward_compatible_without_max_output_tokens() {
        // An existing registry serialized before `max_output_tokens` was added
        // must still deserialize with the field defaulting to `None`.
        let legacy = serde_json::json!({
            "id": "glm",
            "kind": "openai_compatible",
            "label": "GLM 4.6",
            "endpoint_url": "https://open.bigmodels.cn/api/paas/v4",
            "model": "glm-4.6",
            "credential_env_var": "ZAI_API_KEY",
            "enabled": true
        });
        let decoded: ProviderEntry = serde_json::from_value(legacy).expect("legacy deserialize");
        assert_eq!(decoded.max_output_tokens, None);
        assert_eq!(decoded.max_concurrency, None);
        assert_eq!(decoded.requests_per_minute, None);
        assert_eq!(decoded.daily_token_budget, None);
    }

    #[test]
    fn provider_entry_backward_compatible_without_quota_fields() {
        let legacy = serde_json::json!({
            "id": "glm",
            "kind": "openai_compatible",
            "label": "GLM 4.6",
            "endpoint_url": "https://open.bigmodels.cn/api/paas/v4",
            "model": "glm-4.6",
            "credential_env_var": "ZAI_API_KEY",
            "enabled": true,
            "max_output_tokens": 8192
        });
        let decoded: ProviderEntry = serde_json::from_value(legacy).expect("legacy deserialize");
        assert_eq!(decoded.max_concurrency, None);
        assert_eq!(decoded.requests_per_minute, None);
        assert_eq!(decoded.daily_token_budget, None);
    }

    #[test]
    fn remote_model_info_roundtrips_openai_shape() {
        let info = RemoteModelInfo {
            id: "gpt-4o".into(),
            owned_by: Some("openai".into()),
            created: Some(1_700_000_000),
            max_input_tokens: None,
            max_output_tokens: None,
        };
        let encoded = serde_json::to_string(&info).expect("serialize");
        let decoded: RemoteModelInfo = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(decoded, info);
    }

    #[test]
    fn remote_model_info_roundtrips_anthropic_shape() {
        let info = RemoteModelInfo {
            id: "claude-3-5-sonnet".into(),
            owned_by: None,
            created: None,
            max_input_tokens: Some(200_000),
            max_output_tokens: Some(8192),
        };
        let encoded = serde_json::to_string(&info).expect("serialize");
        let decoded: RemoteModelInfo = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(decoded, info);
    }

    #[test]
    fn remote_model_info_rejects_unknown_fields() {
        let value = serde_json::json!({
            "id": "gpt-4o",
            "secret": "sk-test-secret-marker"
        });
        let error = serde_json::from_value::<RemoteModelInfo>(value)
            .expect_err("unknown remote model field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn remote_model_list_roundtrips() {
        let list = RemoteModelList {
            models: vec![RemoteModelInfo {
                id: "gpt-4o".into(),
                owned_by: Some("openai".into()),
                created: None,
                max_input_tokens: None,
                max_output_tokens: None,
            }],
            fetched_at: 1_700_000_000,
        };
        let encoded = serde_json::to_string(&list).expect("serialize");
        let decoded: RemoteModelList = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(decoded, list);
        assert_eq!(decoded.fetched_at, 1_700_000_000);
    }

    fn sample_image_entry() -> ImageProviderEntry {
        ImageProviderEntry {
            id: "openai-image".into(),
            endpoint_url: "https://api.openai.com/v1".into(),
            model: "gpt-image-1".into(),
            credential_env_var: "OPENAI_API_KEY".into(),
            enabled: true,
            default_size: "1024x1024".into(),
            default_quality: "medium".into(),
            max_concurrency: Some(1),
            requests_per_minute: Some(12),
            daily_token_budget: Some(50_000),
        }
    }

    #[test]
    fn image_provider_entry_roundtrips_json_with_quota_fields() {
        let entry = sample_image_entry();
        let encoded = serde_json::to_string_pretty(&entry).expect("serialize image entry");
        let decoded: ImageProviderEntry =
            serde_json::from_str(&encoded).expect("deserialize image entry");
        assert_eq!(decoded, entry);
        assert_eq!(decoded.max_concurrency, Some(1));
        assert_eq!(decoded.requests_per_minute, Some(12));
        assert_eq!(decoded.daily_token_budget, Some(50_000));
    }

    #[test]
    fn image_provider_entry_rejects_secret_fields() {
        let entry = sample_image_entry();
        let mut value = serde_json::to_value(&entry).expect("image entry value");
        value["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<ImageProviderEntry>(value)
            .expect_err("api_key field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn image_provider_entry_defaults_size_and_quality_when_absent() {
        // An entry serialized without default_size/default_quality must
        // deserialize with the documented defaults (1024x1024 / medium).
        let legacy = serde_json::json!({
            "id": "openai-image",
            "endpoint_url": "https://api.openai.com/v1",
            "model": "gpt-image-1",
            "credential_env_var": "OPENAI_API_KEY",
            "enabled": true
        });
        let decoded: ImageProviderEntry =
            serde_json::from_value(legacy).expect("legacy image entry deserialize");
        assert_eq!(decoded.default_size, "1024x1024");
        assert_eq!(decoded.default_quality, "medium");
        assert_eq!(decoded.max_concurrency, None);
        assert_eq!(decoded.requests_per_minute, None);
        assert_eq!(decoded.daily_token_budget, None);
    }

    #[test]
    fn image_provider_entry_backward_compatible_without_quota_fields() {
        let legacy = serde_json::json!({
            "id": "openai-image",
            "endpoint_url": "https://api.openai.com/v1",
            "model": "gpt-image-1",
            "credential_env_var": "OPENAI_API_KEY",
            "enabled": true,
            "default_size": "1536x1024",
            "default_quality": "high"
        });
        let decoded: ImageProviderEntry =
            serde_json::from_value(legacy).expect("legacy image entry deserialize");
        assert_eq!(decoded.max_concurrency, None);
        assert_eq!(decoded.requests_per_minute, None);
        assert_eq!(decoded.daily_token_budget, None);
    }

    #[test]
    fn image_provider_entry_preserves_custom_size_and_quality() {
        let entry = ImageProviderEntry {
            id: "openai-image".into(),
            endpoint_url: "https://api.openai.com/v1".into(),
            model: "gpt-image-1".into(),
            credential_env_var: "OPENAI_API_KEY".into(),
            enabled: true,
            default_size: "1536x1024".into(),
            default_quality: "high".into(),
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        };
        let encoded = serde_json::to_string(&entry).expect("serialize");
        let decoded: ImageProviderEntry = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(decoded.default_size, "1536x1024");
        assert_eq!(decoded.default_quality, "high");
    }

    #[test]
    fn provider_registry_with_image_providers_roundtrips() {
        let registry = ProviderRegistry {
            version: "1".into(),
            providers: vec![sample_entry()],
            image_providers: vec![sample_image_entry()],
            tts_providers: Vec::new(),
            moderation_providers: Vec::new(),
        };
        let encoded = serde_json::to_string_pretty(&registry).expect("serialize registry");
        let decoded: ProviderRegistry =
            serde_json::from_str(&encoded).expect("deserialize registry");
        assert_eq!(decoded, registry);
        assert_eq!(decoded.image_providers.len(), 1);
        assert_eq!(decoded.image_providers[0].model, "gpt-image-1");
    }

    #[test]
    fn provider_registry_backward_compatible_without_image_providers() {
        // An existing registry serialized before `image_providers` was added
        // must still deserialize with an empty image_providers list.
        let legacy = serde_json::json!({
            "version": "1",
            "providers": [{
                "id": "glm",
                "kind": "openai_compatible",
                "label": "GLM 4.6",
                "endpoint_url": "https://open.bigmodels.cn/api/paas/v4",
                "model": "glm-4.6",
                "credential_env_var": "ZAI_API_KEY",
                "enabled": true
            }]
        });
        let decoded: ProviderRegistry =
            serde_json::from_value(legacy).expect("legacy registry deserialize");
        assert_eq!(decoded.providers.len(), 1);
        assert!(decoded.image_providers.is_empty());
    }

    fn sample_tts_entry() -> TtsProviderEntry {
        TtsProviderEntry {
            id: "openai-tts".into(),
            endpoint_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini-tts".into(),
            credential_env_var: "OPENAI_API_KEY".into(),
            enabled: true,
            voice: "coral".into(),
            format: "mp3".into(),
            max_concurrency: Some(3),
            requests_per_minute: Some(60),
            daily_token_budget: Some(75_000),
        }
    }

    #[test]
    fn tts_provider_entry_roundtrips_json_with_quota_fields() {
        let entry = sample_tts_entry();
        let encoded = serde_json::to_string_pretty(&entry).expect("serialize entry");
        let decoded: TtsProviderEntry = serde_json::from_str(&encoded).expect("deserialize entry");
        assert_eq!(decoded, entry);
        assert_eq!(decoded.max_concurrency, Some(3));
        assert_eq!(decoded.requests_per_minute, Some(60));
        assert_eq!(decoded.daily_token_budget, Some(75_000));
    }

    #[test]
    fn tts_provider_entry_rejects_unknown_fields() {
        let entry = sample_tts_entry();
        let mut value = serde_json::to_value(&entry).expect("entry value");
        value["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<TtsProviderEntry>(value)
            .expect_err("api_key field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn tts_provider_entry_defaults_voice_and_format_when_absent() {
        let legacy = serde_json::json!({
            "id": "openai-tts",
            "endpoint_url": "https://api.openai.com/v1",
            "model": "gpt-4o-mini-tts",
            "credential_env_var": "OPENAI_API_KEY",
            "enabled": true
        });
        let decoded: TtsProviderEntry =
            serde_json::from_value(legacy).expect("legacy tts entry deserialize");
        assert_eq!(decoded.voice, "coral");
        assert_eq!(decoded.format, "mp3");
        assert_eq!(decoded.max_concurrency, None);
        assert_eq!(decoded.requests_per_minute, None);
        assert_eq!(decoded.daily_token_budget, None);
    }

    #[test]
    fn tts_provider_entry_backward_compatible_without_quota_fields() {
        let legacy = serde_json::json!({
            "id": "openai-tts",
            "endpoint_url": "https://api.openai.com/v1",
            "model": "gpt-4o-mini-tts",
            "credential_env_var": "OPENAI_API_KEY",
            "enabled": true,
            "voice": "coral",
            "format": "mp3"
        });
        let decoded: TtsProviderEntry =
            serde_json::from_value(legacy).expect("legacy tts entry deserialize");
        assert_eq!(decoded.max_concurrency, None);
        assert_eq!(decoded.requests_per_minute, None);
        assert_eq!(decoded.daily_token_budget, None);
    }

    #[test]
    fn provider_registry_with_tts_providers_roundtrips() {
        let registry = ProviderRegistry {
            version: "1".into(),
            providers: vec![sample_entry()],
            image_providers: Vec::new(),
            tts_providers: vec![sample_tts_entry()],
            moderation_providers: Vec::new(),
        };
        let encoded = serde_json::to_string_pretty(&registry).expect("serialize registry");
        let decoded: ProviderRegistry =
            serde_json::from_str(&encoded).expect("deserialize registry");
        assert_eq!(decoded, registry);
        assert_eq!(decoded.tts_providers.len(), 1);
        assert_eq!(decoded.tts_providers[0].model, "gpt-4o-mini-tts");
    }

    #[test]
    fn provider_registry_backward_compatible_without_tts_providers() {
        let legacy = serde_json::json!({
            "version": "1",
            "providers": [],
            "image_providers": []
        });
        let decoded: ProviderRegistry =
            serde_json::from_value(legacy).expect("legacy registry deserialize");
        assert!(decoded.tts_providers.is_empty());
    }

    fn sample_moderation_entry() -> ModerationProviderEntry {
        ModerationProviderEntry {
            id: "openai-moderation".into(),
            endpoint_url: "https://api.openai.com/v1".into(),
            model: "omni-moderation-latest".into(),
            credential_env_var: "OPENAI_API_KEY".into(),
            enabled: true,
            max_concurrency: Some(2),
            requests_per_minute: Some(45),
            daily_token_budget: Some(80_000),
        }
    }

    #[test]
    fn moderation_provider_entry_roundtrips_json() {
        let entry = sample_moderation_entry();
        let encoded = serde_json::to_string_pretty(&entry).expect("serialize moderation entry");
        let decoded: ModerationProviderEntry =
            serde_json::from_str(&encoded).expect("deserialize moderation entry");
        assert_eq!(decoded, entry);
    }

    #[test]
    fn moderation_provider_entry_rejects_secret_fields() {
        for field in ["api_key", "secret_token"] {
            let mut value =
                serde_json::to_value(sample_moderation_entry()).expect("moderation entry value");
            value[field] = serde_json::json!("sk-test-secret-marker");
            let error = serde_json::from_value::<ModerationProviderEntry>(value)
                .expect_err("secret-bearing field should be rejected");
            assert!(error.to_string().contains("unknown field"));
        }
    }

    #[test]
    fn moderation_provider_entry_backward_compatible_without_quota_fields() {
        let legacy = serde_json::json!({
            "id": "openai-moderation",
            "endpoint_url": "https://api.openai.com/v1",
            "model": "omni-moderation-latest",
            "credential_env_var": "OPENAI_API_KEY",
            "enabled": true
        });
        let decoded: ModerationProviderEntry =
            serde_json::from_value(legacy).expect("legacy moderation entry deserialize");
        assert_eq!(decoded.max_concurrency, None);
        assert_eq!(decoded.requests_per_minute, None);
        assert_eq!(decoded.daily_token_budget, None);
    }

    #[test]
    fn provider_registry_with_moderation_providers_roundtrips() {
        let registry = ProviderRegistry {
            version: "1".into(),
            providers: Vec::new(),
            image_providers: Vec::new(),
            tts_providers: Vec::new(),
            moderation_providers: vec![sample_moderation_entry()],
        };
        let encoded = serde_json::to_string_pretty(&registry).expect("serialize registry");
        let decoded: ProviderRegistry =
            serde_json::from_str(&encoded).expect("deserialize registry");
        assert_eq!(decoded, registry);
    }

    #[test]
    fn provider_registry_backward_compatible_without_moderation_providers() {
        let legacy = serde_json::json!({
            "version": "1",
            "providers": [],
            "image_providers": [],
            "tts_providers": []
        });
        let decoded: ProviderRegistry =
            serde_json::from_value(legacy).expect("legacy registry deserialize");
        assert!(decoded.moderation_providers.is_empty());
    }
}
