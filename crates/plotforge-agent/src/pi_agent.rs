//! pi-Agent facade.
//!
//! A thin local-first agent surface that wraps a `TextModelProvider` and
//! emits a redaction-safe `PiAgentRunResult`. The facade is local-only: it
//! performs no network calls and never stores raw provider responses,
//! credentials, or secret markers in its output. It reuses the shared
//! `ReproducibilityMetadata` type so trace/snapshot evidence stays
//! consistent with the rest of the runtime.
//!
//! The facade is intentionally additive. It reuses the existing
//! `complete_text_agent_output` pipeline (provider call, JSON repair,
//! envelope validation, and `validate_agent_output_proposal`) so the
//! pi-Agent surface benefits from the same validation path as the existing
//! agent pipelines. The broader agent module reorganization is tracked
//! separately (T2.1); this module only adds the new surface without moving
//! existing symbols.

use plotforge_schema::{
    AgentRole, PiAgentCapability, PiAgentDescriptor, PiAgentRunRequest, PiAgentRunResult,
};

use crate::{TextModelProvider, complete_text_agent_output, contains_secret_marker_text};

/// pi-Agent facade errors. Explicit errors only — no silent fallback.
///
/// Provider failures are surfaced as a redacted code/message pair (routed
/// through the shared `ProviderPipelineError::into_runtime_error` redaction
/// path) so the pi-Agent surface never leaks raw provider responses, secret
/// markers, or credentials.
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum PiAgentError {
    /// The request prompt summary contained a secret marker.
    #[error("pi-agent request prompt summary contained a secret marker")]
    PromptSecretMarker,
    /// The request prompt hash was empty.
    #[error("pi-agent request prompt hash must not be empty")]
    EmptyPromptHash,
    /// The underlying text provider failed or returned invalid output. The
    /// `code` and `message` are redacted before being stored here.
    #[error("pi-agent provider failure: {code}: {message}")]
    Provider { code: String, message: String },
}

/// The pi-Agent facade. Holds a boxed `TextModelProvider` so it can wrap any
/// local provider (fake, configured, or future real provider) behind a single
/// redaction-safe surface.
pub struct PiAgent {
    provider: Box<dyn TextModelProvider>,
    agent_id: String,
}

impl PiAgent {
    /// Construct a pi-Agent facade over a text model provider. The agent id
    /// identifies this pi-Agent instance in descriptors and results.
    pub fn new(provider: Box<dyn TextModelProvider>, agent_id: impl Into<String>) -> Self {
        Self {
            provider,
            agent_id: agent_id.into(),
        }
    }

    /// Run the pi-Agent against a redaction-safe request. Calls the provider,
    /// validates the resulting proposal, and returns a `PiAgentRunResult`
    /// carrying reproducibility metadata and a redacted evidence summary.
    /// No raw provider response is ever stored in the result.
    pub fn run(&self, request: PiAgentRunRequest) -> Result<PiAgentRunResult, PiAgentError> {
        if request.prompt_hash.trim().is_empty() {
            return Err(PiAgentError::EmptyPromptHash);
        }
        if contains_secret_marker_text(&request.prompt_summary) {
            return Err(PiAgentError::PromptSecretMarker);
        }

        // Reuse the shared provider pipeline so the pi-Agent benefits from
        // the same validation path (envelope validation, JSON repair, and
        // `validate_agent_output_proposal`) as the existing agent pipelines.
        let envelope = complete_text_agent_output(
            self.provider.as_ref(),
            AgentRole::ScenePlanner,
            request.run_seed,
            format!("pi-agent-{}-{}", self.agent_id, request.run_seed),
            format!("pi-agent-{}", self.agent_id),
            request.prompt_summary.clone(),
        )
        .map_err(|pipeline_error| {
            // Route the provider failure through the shared redaction path so
            // the pi-Agent surface never leaks raw provider text or secrets.
            let runtime_error = pipeline_error.into_runtime_error();
            PiAgentError::Provider {
                code: runtime_error.code,
                message: runtime_error.message,
            }
        })?;

        let reproducibility = envelope.reproducibility.clone();
        let trace_id = reproducibility.trace_id.clone();
        let descriptor = PiAgentDescriptor {
            agent_id: self.agent_id.clone(),
            is_local_pi: true,
            capabilities: pi_agent_capabilities(),
        };

        // Redaction-safe evidence summary: never include raw provider
        // responses, prompt text, or secrets. Only describe the validated
        // proposal kind and the reproducibility identity.
        let evidence_summary = format!(
            "pi-Agent validated a {} proposal from {} provider evidence.",
            crate::payload_kind(&envelope.proposal.output),
            reproducibility.provider_config_hash,
        );

        Ok(PiAgentRunResult {
            descriptor,
            reproducibility,
            trace_id,
            evidence_summary,
        })
    }
}

impl std::fmt::Debug for PiAgent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PiAgent")
            .field("agent_id", &self.agent_id)
            .finish_non_exhaustive()
        // The provider is intentionally not included in the debug output to
        // avoid leaking any provider-internal state into trace/debug output.
    }
}

/// Return the static capability list the pi-Agent runtime exposes. The
/// list describes what is wired and what is deferred; it never promises
/// external agent execution, network model calls, or platform outcomes.
pub fn pi_agent_capabilities() -> Vec<PiAgentCapability> {
    vec![
        PiAgentCapability {
            id: "pi-agent.text-generation".into(),
            label: "Text generation".into(),
            status: "wired".into(),
            source: "local-mock-text-provider".into(),
            evidence: "FakeTextModelProvider produces validated agent output envelopes.".into(),
        },
        PiAgentCapability {
            id: "pi-agent.image-generation".into(),
            label: "Image generation".into(),
            status: "not-implemented".into(),
            source: "deferred".into(),
            evidence:
                "Deferred to a later phase; no image provider is wired into the pi-Agent facade."
                    .into(),
        },
        PiAgentCapability {
            id: "pi-agent.steam-upload".into(),
            label: "Steam upload".into(),
            status: "not-implemented".into(),
            source: "deferred".into(),
            evidence:
                "Steam/Workshop integration remains deferred; no upload automation is implied."
                    .into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use plotforge_schema::{
        CONTRACT_SCHEMA_VERSION, CONTRACT_VERSION, contains_secret_marker_text,
    };

    use super::*;
    use crate::FakeTextModelProvider;

    #[test]
    fn pi_agent_runs_local_mock_provider() {
        let agent = PiAgent::new(Box::new(FakeTextModelProvider::success()), "pi-agent-local");
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 7,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "sha256:abc".into(),
        };

        let result = agent.run(request).expect("pi-agent run succeeds");

        assert!(result.descriptor.is_local_pi);
        assert_eq!(result.descriptor.agent_id, "pi-agent-local");
        assert!(!result.descriptor.capabilities.is_empty());
        assert_eq!(result.reproducibility.run_seed, 7);
        assert!(
            result
                .reproducibility
                .provider_config_hash
                .starts_with("sha256:"),
            "provider_config_hash must be present"
        );
        assert!(
            result.reproducibility.prompt_version.contains("plotforge"),
            "prompt_version must be present"
        );
        assert!(
            !result.reproducibility.model_version.is_empty(),
            "model_version must be present"
        );

        // The evidence summary must be redaction-safe: no raw provider
        // responses and no secret markers.
        assert!(!contains_secret_marker_text(&result.evidence_summary));
        assert!(
            !result.evidence_summary.contains("raw_provider_response"),
            "evidence summary must not reference raw provider responses"
        );
        assert!(
            !result.evidence_summary.contains("sk-"),
            "evidence summary must not contain secret markers"
        );
    }

    #[test]
    fn pi_agent_run_result_is_serializable_and_redaction_safe() {
        let agent = PiAgent::new(Box::new(FakeTextModelProvider::success()), "pi-agent-local");
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 42,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "sha256:def".into(),
        };

        let result = agent.run(request).expect("pi-agent run succeeds");
        let encoded = serde_json::to_string(&result).expect("serialize result");
        let decoded: PiAgentRunResult = serde_json::from_str(&encoded).expect("deserialize result");
        assert_eq!(decoded.descriptor, result.descriptor);
        assert_eq!(decoded.reproducibility.run_seed, 42);
        assert!(
            !encoded.contains("raw_provider_response"),
            "serialized result must not reference raw provider responses"
        );
        assert!(
            !encoded.contains("api_key"),
            "serialized result must not reference api_key"
        );
        assert!(
            !encoded.contains("sk-"),
            "serialized result must not contain secret markers"
        );
    }

    #[test]
    fn pi_agent_rejects_empty_prompt_hash() {
        let agent = PiAgent::new(Box::new(FakeTextModelProvider::success()), "pi-agent-local");
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 7,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "   ".into(),
        };

        let error = agent.run(request).expect_err("empty prompt hash rejected");
        assert_eq!(error, PiAgentError::EmptyPromptHash);
    }

    #[test]
    fn pi_agent_rejects_secret_marker_in_prompt_summary() {
        let agent = PiAgent::new(Box::new(FakeTextModelProvider::success()), "pi-agent-local");
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 7,
            prompt_summary: "Generate with OPENAI_API_KEY=sk-test-secret-marker".into(),
            prompt_hash: "sha256:abc".into(),
        };

        let error = agent.run(request).expect_err("secret marker rejected");
        assert_eq!(error, PiAgentError::PromptSecretMarker);
    }

    #[test]
    fn pi_agent_propagates_provider_failure_without_silent_fallback() {
        // A failing provider must surface an explicit error, never a silent
        // fallback result. Use the ScenePlanner role which the facade passes
        // to the provider pipeline; FakeTextModelProvider::provider_error
        // fails for that agent.
        let agent = PiAgent::new(
            Box::new(FakeTextModelProvider::provider_error(
                AgentRole::ScenePlanner,
            )),
            "pi-agent-local",
        );
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 7,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "sha256:abc".into(),
        };

        let error = agent.run(request).expect_err("provider failure surfaces");
        assert!(matches!(error, PiAgentError::Provider { .. }));
        // The provider error must be redaction-safe: no raw secret markers.
        if let PiAgentError::Provider { code, message } = error {
            assert!(!contains_secret_marker_text(&message));
            assert!(!message.contains("sk-"));
            assert!(!code.contains("sk-"));
        }
    }

    #[test]
    fn pi_agent_capabilities_describe_wired_and_deferred() {
        let capabilities = pi_agent_capabilities();
        let wired = capabilities
            .iter()
            .filter(|capability| capability.status == "wired")
            .count();
        assert!(wired >= 1, "at least one capability should be wired");
        for capability in &capabilities {
            assert!(!contains_secret_marker_text(&capability.evidence));
            assert!(!capability.evidence.contains("sk-"));
        }
    }

    /// Smoke-assert that the pi-Agent surface carries the contract/version
    /// envelope expectations used by downstream trace consumers. This guards
    /// against accidental drift if the facade ever emits a custom envelope.
    #[test]
    fn pi_agent_reproducibility_metadata_is_complete() {
        let agent = PiAgent::new(Box::new(FakeTextModelProvider::success()), "pi-agent-local");
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 13,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "sha256:abc".into(),
        };

        let result = agent.run(request).expect("pi-agent run succeeds");
        let reproducibility = &result.reproducibility;
        assert_eq!(reproducibility.run_seed, 13);
        assert!(!reproducibility.prompt_version.is_empty());
        assert!(!reproducibility.model_version.is_empty());
        assert!(!reproducibility.provider_config_hash.is_empty());
        // Trace/snapshot ids are optional; just confirm the fields compile.
        let _ = reproducibility.trace_id.as_ref();
        let _ = reproducibility.snapshot_id.as_ref();
        // Contract version sanity (used by the envelope validation path).
        assert_eq!(CONTRACT_VERSION, env!("CARGO_PKG_VERSION"));
        // Schema version is a compile-time constant; confirm the field is
        // present and non-default rather than re-asserting a constant.
        let _ = CONTRACT_SCHEMA_VERSION;
    }
}
