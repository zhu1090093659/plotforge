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
}

/// The persisted registry file (`~/.plotforge/providers.json`). An empty
/// registry is the default for fresh installs; provider wiring is opt-in.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct ProviderRegistry {
    pub version: String,
    pub providers: Vec<ProviderEntry>,
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
    fn provider_entry_roundtrips_json() {
        let entry = sample_entry();
        let encoded = serde_json::to_string_pretty(&entry).expect("serialize entry");
        let decoded: ProviderEntry = serde_json::from_str(&encoded).expect("deserialize entry");
        assert_eq!(decoded, entry);
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
                },
            ],
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
    }

    #[test]
    fn provider_registry_rejects_unknown_fields() {
        let mut value = serde_json::to_value(ProviderRegistry::default()).expect("registry value");
        value["api_keys"] = serde_json::json!([{"secret": "sk-test-secret-marker"}]);
        let error = serde_json::from_value::<ProviderRegistry>(value)
            .expect_err("unknown registry field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }
}
