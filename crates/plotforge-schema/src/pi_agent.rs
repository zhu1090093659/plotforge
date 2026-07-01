//! pi-Agent schema contracts.
//!
//! These types describe the pi-Agent runtime surface: a local-first agent
//! facade that wraps a `TextModelProvider` and emits redaction-safe
//! reproducibility evidence. The descriptor and capability list are
//! capability/descriptive surfaces only — they describe what is wired and
//! what is not, and never carry provider credentials or raw provider
//! responses.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ReproducibilityMetadata;

/// A single pi-Agent capability.
///
/// `status` is a descriptive label ("wired" or "not-implemented") rather than
/// a typed enum so the contract stays forward-compatible with new capability
/// states introduced by future phases without a schema breaking change.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PiAgentCapability {
    pub id: String,
    pub label: String,
    pub status: String,
    pub source: String,
    pub evidence: String,
}

/// Describes the pi-Agent runtime: its local identity and the capability list
/// it currently exposes. This is a descriptive surface only; it does not
/// promise external agent execution, network model calls, or platform
/// outcomes.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PiAgentDescriptor {
    pub agent_id: String,
    pub is_local_pi: bool,
    pub capabilities: Vec<PiAgentCapability>,
}

/// A redaction-safe pi-Agent run request.
///
/// The prompt is never carried verbatim; callers pass a short summary plus a
/// stable hash so traces and results stay free of raw prompt content and
/// secrets.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PiAgentRunRequest {
    pub agent_id: String,
    pub run_seed: u64,
    pub prompt_summary: String,
    pub prompt_hash: String,
}

/// The redaction-safe result of a pi-Agent run.
///
/// Reuses the shared `ReproducibilityMetadata` type so trace/snapshot
/// evidence stays consistent across the runtime. The descriptor echoes the
/// agent identity and capabilities used for the run. `evidence_summary` is a
/// short, redacted summary; it must not contain raw provider responses,
/// credentials, or secret markers.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PiAgentRunResult {
    pub descriptor: PiAgentDescriptor,
    pub reproducibility: ReproducibilityMetadata,
    pub trace_id: Option<String>,
    pub evidence_summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pi_agent_capability_roundtrips_json() {
        let capability = PiAgentCapability {
            id: "pi-agent.text-generation".into(),
            label: "Text generation".into(),
            status: "wired".into(),
            source: "local-mock-text-provider".into(),
            evidence: "FakeTextModelProvider envelope".into(),
        };

        let encoded = serde_json::to_string_pretty(&capability).expect("serialize capability");
        let decoded: PiAgentCapability =
            serde_json::from_str(&encoded).expect("deserialize capability");
        assert_eq!(decoded, capability);
    }

    #[test]
    fn pi_agent_capability_rejects_unknown_fields() {
        let capability = PiAgentCapability {
            id: "pi-agent.text-generation".into(),
            label: "Text generation".into(),
            status: "wired".into(),
            source: "local-mock-text-provider".into(),
            evidence: "FakeTextModelProvider envelope".into(),
        };
        let mut value = serde_json::to_value(&capability).expect("capability value");
        value["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<PiAgentCapability>(value)
            .expect_err("unknown field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn pi_agent_descriptor_roundtrips_json() {
        let descriptor = PiAgentDescriptor {
            agent_id: "pi-agent-local".into(),
            is_local_pi: true,
            capabilities: vec![
                PiAgentCapability {
                    id: "pi-agent.text-generation".into(),
                    label: "Text generation".into(),
                    status: "wired".into(),
                    source: "local-mock-text-provider".into(),
                    evidence: "FakeTextModelProvider envelope".into(),
                },
                PiAgentCapability {
                    id: "pi-agent.image-generation".into(),
                    label: "Image generation".into(),
                    status: "not-implemented".into(),
                    source: "deferred".into(),
                    evidence: "Deferred to a later phase.".into(),
                },
            ],
        };

        let encoded = serde_json::to_string_pretty(&descriptor).expect("serialize descriptor");
        let decoded: PiAgentDescriptor =
            serde_json::from_str(&encoded).expect("deserialize descriptor");
        assert_eq!(decoded, descriptor);
        assert!(decoded.is_local_pi);
        assert_eq!(decoded.capabilities.len(), 2);
    }

    #[test]
    fn pi_agent_run_request_roundtrips_json() {
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 7,
            prompt_summary: "Generate a scene plan proposal.".into(),
            prompt_hash: "sha256:abc".into(),
        };

        let encoded = serde_json::to_string_pretty(&request).expect("serialize request");
        let decoded: PiAgentRunRequest =
            serde_json::from_str(&encoded).expect("deserialize request");
        assert_eq!(decoded, request);
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("request value");
        assert!(value.get("prompt").is_none());
    }

    #[test]
    fn pi_agent_run_request_rejects_raw_prompt_fields() {
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 7,
            prompt_summary: "Generate a scene plan proposal.".into(),
            prompt_hash: "sha256:abc".into(),
        };
        let mut value = serde_json::to_value(&request).expect("request value");
        value["prompt"] = serde_json::json!("raw prompt with sk-test-secret-marker");
        let error = serde_json::from_value::<PiAgentRunRequest>(value)
            .expect_err("raw prompt field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn pi_agent_run_result_roundtrips_with_local_mock_reproducibility() {
        let descriptor = PiAgentDescriptor {
            agent_id: "pi-agent-local".into(),
            is_local_pi: true,
            capabilities: vec![PiAgentCapability {
                id: "pi-agent.text-generation".into(),
                label: "Text generation".into(),
                status: "wired".into(),
                source: "local-mock-text-provider".into(),
                evidence: "FakeTextModelProvider envelope".into(),
            }],
        };
        let result = PiAgentRunResult {
            descriptor: descriptor.clone(),
            reproducibility: ReproducibilityMetadata::local_mock(7)
                .with_trace_id("pi-agent-trace-001"),
            trace_id: Some("pi-agent-trace-001".into()),
            evidence_summary: "pi-Agent produced a validated scene plan proposal.".into(),
        };

        let encoded = serde_json::to_string_pretty(&result).expect("serialize result");
        let decoded: PiAgentRunResult = serde_json::from_str(&encoded).expect("deserialize result");
        assert_eq!(decoded, result);
        assert_eq!(decoded.descriptor, descriptor);
        assert_eq!(decoded.reproducibility.run_seed, 7);
        assert_eq!(
            decoded.reproducibility.trace_id.as_deref(),
            Some("pi-agent-trace-001")
        );
        assert_eq!(decoded.trace_id.as_deref(), Some("pi-agent-trace-001"));

        let value: serde_json::Value = serde_json::from_str(&encoded).expect("result value");
        assert!(value.get("raw_provider_response").is_none());
    }

    #[test]
    fn pi_agent_run_result_rejects_raw_provider_fields() {
        let result = PiAgentRunResult {
            descriptor: PiAgentDescriptor {
                agent_id: "pi-agent-local".into(),
                is_local_pi: true,
                capabilities: Vec::new(),
            },
            reproducibility: ReproducibilityMetadata::local_mock(7),
            trace_id: None,
            evidence_summary: "redacted".into(),
        };
        let mut value = serde_json::to_value(&result).expect("result value");
        value["raw_provider_response"] =
            serde_json::json!("raw provider body with sk-test-secret-marker");
        let error = serde_json::from_value::<PiAgentRunResult>(value)
            .expect_err("raw provider response should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }
}
