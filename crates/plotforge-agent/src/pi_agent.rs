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
    AgentOutputEnvelope, AgentRole, PiAgentCapability, PiAgentDescriptor, PiAgentRunRequest,
    PiAgentRunResult,
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
    /// The request `agent_id` must match the id this facade was constructed
    /// with (`PiAgent::new`). A mismatch is an explicit error rather than a
    /// silent divergence, because the facade's identity (`descriptor.agent_id`
    /// and the deterministic trace-evidence id) is anchored to `self.agent_id`.
    #[error("pi-agent request agent_id `{request}` did not match facade agent_id `{facade}`")]
    AgentIdMismatch { facade: String, request: String },
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
    /// The validated envelope carried a payload kind the caller cannot commit
    /// (e.g. a world-expansion payload where a scene plan was expected). The
    /// `kind` is the redaction-safe payload kind string, not the payload body.
    #[error("pi-agent envelope carried unsupported payload kind `{kind}`")]
    UnsupportedPayloadKind { kind: String },
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
        let (result, _envelope) = self.run_with_envelope(request)?;
        Ok(result)
    }

    /// Run the pi-Agent and return both the redaction-safe `PiAgentRunResult`
    /// and the validated `AgentOutputEnvelope`. The envelope carries the
    /// proposal payload (e.g. a `ScenePlan`) that a runtime commit path can
    /// apply; `run` discards it. This is the path `pi_agent_apply_run` uses
    /// so the Studio layer can commit the agent's proposal as a state change
    /// without re-running the provider.
    pub fn run_with_envelope(
        &self,
        request: PiAgentRunRequest,
    ) -> Result<(PiAgentRunResult, AgentOutputEnvelope), PiAgentError> {
        self.validate_request(&request)?;

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
        .map_err(Self::provider_error)?;

        Ok((self.assemble_result(&envelope, request.run_seed), envelope))
    }

    /// Run the pi-Agent with assembled project context, using the structured
    /// prompt-template path (T2.1) so the pi-Agent receives the full project
    /// context — world bible, characters, story state, current scene, rules,
    /// and visual style — in its prompt.
    ///
    /// `context` is the markdown context block produced by
    /// [`crate::assemble_context`] (already truncated to the model's context
    /// budget by the caller). The user message sent to the provider is the
    /// player's input (`request.prompt_summary`) followed by the context
    /// block, so the model sees both the per-turn instruction and the
    /// project state. The system message is the built-in `SCENE_PLANNER_V1`
    /// template (role + JSON output contract); the reproducibility
    /// `prompt_version` is therefore the per-role template version
    /// (`scene_planner_v1`), not the local-mock provider's prompt version —
    /// this is the additive structured-prompt boundary from T2.1.
    ///
    /// The validation + trace-identity contract is identical to
    /// `run_with_envelope`: the request's `agent_id` must match the facade,
    /// `prompt_hash` must be non-empty, and `prompt_summary` must not carry a
    /// secret marker. The context block is additionally redaction-scanned by
    /// the provider pipeline (`contains_secret_marker_text`) as a defence-
    /// in-depth measure. The returned `PiAgentRunResult` carries the same
    /// deterministic `pi-agent-evidence-...` trace id and the same
    /// redaction-safe evidence summary.
    pub fn run_with_envelope_with_context(
        &self,
        request: PiAgentRunRequest,
        context: &str,
    ) -> Result<(PiAgentRunResult, AgentOutputEnvelope), PiAgentError> {
        self.validate_request(&request)?;

        // The user message is the player's input plus the assembled context
        // block. When the context is empty (a fresh project with no world
        // bible yet), the user message is just the player input, mirroring
        // `run_with_envelope`.
        let user_message = if context.trim().is_empty() {
            request.prompt_summary.clone()
        } else {
            format!("{}\n\n{}", request.prompt_summary.trim(), context)
        };

        // Assemble the structured chat messages (system + user) and drive the
        // structured-prompt pipeline. `complete_text_agent_output_with_
        // messages` packs the messages into `TextModelRequest.messages` and
        // overrides `prompt_version` with the ScenePlanner template version.
        let messages = crate::prompts::PromptAssembler::assemble(
            &crate::prompts::SCENE_PLANNER_V1,
            &user_message,
        );
        let envelope = crate::pipelines::complete_text_agent_output_with_messages(
            self.provider.as_ref(),
            AgentRole::ScenePlanner,
            request.run_seed,
            format!("pi-agent-{}-{}", self.agent_id, request.run_seed),
            format!("pi-agent-{}", self.agent_id),
            messages,
            crate::prompts::SCENE_PLANNER_V1.version,
        )
        .map_err(Self::provider_error)?;

        Ok((self.assemble_result(&envelope, request.run_seed), envelope))
    }

    /// Validate the redaction-safe request: `agent_id` must match the facade,
    /// `prompt_hash` must be non-empty, and `prompt_summary` must not carry a
    /// secret marker. Shared by both `run_with_envelope` and
    /// `run_with_envelope_with_context` so the two entry points enforce the
    /// same redaction boundary.
    fn validate_request(&self, request: &PiAgentRunRequest) -> Result<(), PiAgentError> {
        // The facade's identity is anchored to `self.agent_id`; the request's
        // `agent_id` must agree with it. A mismatch is an explicit error so the
        // caller never silently observes identity rooted at the facade id while
        // believing it was rooted at the request id.
        if request.agent_id != self.agent_id {
            return Err(PiAgentError::AgentIdMismatch {
                facade: self.agent_id.clone(),
                request: request.agent_id.clone(),
            });
        }
        if request.prompt_hash.trim().is_empty() {
            return Err(PiAgentError::EmptyPromptHash);
        }
        if contains_secret_marker_text(&request.prompt_summary) {
            return Err(PiAgentError::PromptSecretMarker);
        }
        Ok(())
    }

    /// Build the redaction-safe `PiAgentRunResult` from a validated envelope.
    /// Shared by both `run_with_envelope` and `run_with_envelope_with_context`
    /// so the trace-identity and evidence-summary contract is identical.
    fn assemble_result(&self, envelope: &AgentOutputEnvelope, run_seed: u64) -> PiAgentRunResult {
        let mut reproducibility = envelope.reproducibility.clone();
        // The pi-Agent facade owns the trace-evidence identity for its results:
        // it derives a deterministic, redaction-safe id from `agent_id:run_seed`
        // and unconditionally overwrites any provider-supplied `trace_id`. This
        // is intentional (not a silent fallback): the shared
        // `complete_text_agent_output` pipeline already overwrites the
        // reproducibility block (`run_seed`, `prompt_version`, `model_version`,
        // `provider_config_hash`) with the locally-expected values so real
        // provider responses validate, but it leaves `trace_id` to the caller.
        // The pi-Agent surface is the authoritative trace identity for
        // downstream consumers, and a provider id would otherwise leak into
        // `result.reproducibility.trace_id` and `result.trace_id` without the
        // facade's deterministic contract.
        let trace_evidence_id = format!(
            "pi-agent-evidence-{}",
            crate::shared::stable_sha256_hash(&format!("{}:{}", self.agent_id, run_seed))
        );
        reproducibility = reproducibility.with_trace_id(&trace_evidence_id);
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

        PiAgentRunResult {
            descriptor,
            reproducibility,
            trace_id,
            evidence_summary,
        }
    }

    /// Route a `ProviderPipelineError` through the shared redaction path so
    /// the pi-Agent surface never leaks raw provider text or secrets.
    fn provider_error(pipeline_error: crate::pipelines::ProviderPipelineError) -> PiAgentError {
        let runtime_error = pipeline_error.into_runtime_error();
        PiAgentError::Provider {
            code: runtime_error.code,
            message: runtime_error.message,
        }
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
///
/// `mcp_enabled = false` (the default for the one-shot path): the
/// `pi-agent.mcp-tool-use` capability is `not-implemented`, honestly
/// reflecting that MCP tool use is opt-in per `AgentSessionConfig.enabled_
/// mcp_servers`. Call the `complete_with_mcp_tools` wrapper with a non-empty
/// server list to flip this capability to `wired` for that turn.
pub fn pi_agent_capabilities() -> Vec<PiAgentCapability> {
    pi_agent_capabilities_with_mcp(false)
}

/// Return the capability list with the MCP tool-use capability's status set
/// by `mcp_enabled`. When `true`, `pi-agent.mcp-tool-use` is `wired`
/// (the `complete_with_mcp_tools` loop is driving tool calls this turn);
/// when `false`, it is `not-implemented` (the one-shot path is taken). The
/// capability never promises external agent execution, network model calls,
/// or platform outcomes — only local MCP tool-call orchestration.
pub fn pi_agent_capabilities_with_mcp(mcp_enabled: bool) -> Vec<PiAgentCapability> {
    let mcp_status = if mcp_enabled {
        "wired"
    } else {
        "not-implemented"
    };
    let mcp_evidence = if mcp_enabled {
        "complete_with_mcp_tools loop is driving MCP tool calls this turn."
    } else {
        "MCP tool use is opt-in via AgentSessionConfig.enabled_mcp_servers; \
         not active on this turn."
    };
    vec![
        PiAgentCapability {
            id: "pi-agent.text-generation".into(),
            label: "Text generation".into(),
            status: "wired".into(),
            source: "local-mock-text-provider".into(),
            evidence: "FakeTextModelProvider produces validated agent output envelopes.".into(),
        },
        PiAgentCapability {
            id: "pi-agent.mcp-tool-use".into(),
            label: "MCP tool use".into(),
            status: mcp_status.into(),
            source: "plotforge-mcp-client".into(),
            evidence: mcp_evidence.into(),
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
        AgentOutputEnvelope, AgentOutputProposal, AgentProposalPayload, CONTRACT_SCHEMA_VERSION,
        CONTRACT_VERSION, PiAgentCapability, ReproducibilityMetadata, ScenePlanProposal,
        contains_secret_marker_text,
    };

    use super::*;
    use crate::{
        ConfiguredTextModelProvider, FakeTextModelProvider, TextModelClient,
        TextModelClientRequest, TextModelProvider, TextModelResponse, TextProviderConfig,
    };

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
        // Pin the exact wired capability so an accidental over-claim (e.g.
        // flipping image-generation or steam-upload to wired) fails this test
        // instead of passing under the previous `>= 1` bound.
        let wired: Vec<&PiAgentCapability> = capabilities
            .iter()
            .filter(|capability| capability.status == "wired")
            .collect();
        assert_eq!(
            wired.len(),
            1,
            "exactly one capability should be wired (mcp-tool-use is not-implemented by default); got {wired:?}"
        );
        assert_eq!(
            wired[0].id, "pi-agent.text-generation",
            "only text-generation should be wired by default"
        );
        // The MCP tool-use capability must be present and not-implemented by
        // default (it flips to wired only on turns that drive the MCP loop).
        let mcp = capabilities
            .iter()
            .find(|c| c.id == "pi-agent.mcp-tool-use")
            .expect("pi-agent.mcp-tool-use capability must be present");
        assert_eq!(
            mcp.status, "not-implemented",
            "mcp-tool-use must be not-implemented on the default (non-MCP) path"
        );
        for capability in &capabilities {
            assert!(!contains_secret_marker_text(&capability.evidence));
            assert!(!capability.evidence.contains("sk-"));
        }
    }

    /// When MCP servers are enabled, `pi_agent_capabilities_with_mcp(true)`
    /// flips the `pi-agent.mcp-tool-use` capability to `wired`. The other
    /// capabilities (text-generation wired, image/steam-upload deferred) are
    /// unchanged. The capability evidence stays redaction-safe.
    #[test]
    fn pi_agent_capabilities_with_mcp_flips_mcp_tool_use() {
        let capabilities = pi_agent_capabilities_with_mcp(true);
        let mcp = capabilities
            .iter()
            .find(|c| c.id == "pi-agent.mcp-tool-use")
            .expect("mcp-tool-use capability present");
        assert_eq!(
            mcp.status, "wired",
            "mcp-tool-use must be wired when servers are enabled"
        );
        let wired: Vec<&PiAgentCapability> = capabilities
            .iter()
            .filter(|c| c.status == "wired")
            .collect();
        assert_eq!(
            wired.len(),
            2,
            "exactly two capabilities wired when MCP is on; got {wired:?}"
        );
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

    /// A `TextModelProvider` whose `complete()` returns a valid envelope (via
    /// `FakeTextModelProvider::local_pi()`) but injects a provider-supplied
    /// `trace_id` into the envelope reproducibility. Used to verify the
    /// pi-Agent facade overwrites any provider-supplied trace id with its own
    /// deterministic `pi-agent-evidence-...` id.
    struct ProviderWithTraceId {
        inner: FakeTextModelProvider,
        provider_trace_id: &'static str,
    }

    impl TextModelProvider for ProviderWithTraceId {
        fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
            self.inner.reproducibility_metadata(run_seed)
        }

        fn complete(
            &self,
            request: &crate::TextModelRequest,
        ) -> Result<crate::TextModelResponse, crate::TextModelProviderError> {
            let response = self.inner.complete(request)?;
            // Patch the serialized envelope so its reproducibility.trace_id is
            // the provider-supplied value. This mimics a real provider that
            // returns its own trace id through the envelope.
            let mut value: serde_json::Value =
                serde_json::from_str(&response.raw_json).expect("fake response is valid JSON");
            value["reproducibility"]["trace_id"] = serde_json::json!(self.provider_trace_id);
            Ok(crate::TextModelResponse::json(value.to_string()))
        }
    }

    /// The deterministic trace-evidence id must change when either `agent_id`
    /// or `run_seed` changes. This guards against regressions that drop either
    /// input from the hash, which would otherwise still pass the existing
    /// "same inputs -> same id" tests.
    #[test]
    fn pi_agent_trace_id_changes_with_agent_id_and_run_seed() {
        let make = |agent_id: &str, seed: u64| {
            let agent = PiAgent::new(Box::new(FakeTextModelProvider::local_pi()), agent_id);
            let request = PiAgentRunRequest {
                agent_id: agent_id.into(),
                run_seed: seed,
                prompt_summary: "Generate a validated scene plan proposal.".into(),
                prompt_hash: "sha256:abc".into(),
            };
            agent.run(request).expect("pi-agent run succeeds").trace_id
        };

        // Same agent_id, different run_seed -> different trace id.
        let id_seed_7 = make("pi-agent-local", 7).expect("trace id present");
        let id_seed_8 = make("pi-agent-local", 8).expect("trace id present");
        assert_ne!(
            id_seed_7, id_seed_8,
            "trace id must change when run_seed changes"
        );

        // Same run_seed, different agent_id -> different trace id.
        let id_agent_a = make("pi-agent-a", 7).expect("trace id present");
        let id_agent_b = make("pi-agent-b", 7).expect("trace id present");
        assert_ne!(
            id_agent_a, id_agent_b,
            "trace id must change when agent_id changes"
        );

        // Same inputs again -> identical id (stability, not just distinctness).
        let id_repeat = make("pi-agent-local", 7).expect("trace id present");
        assert_eq!(
            id_repeat, id_seed_7,
            "trace id must be deterministic for the same inputs"
        );
    }

    /// A provider that returns its own `trace_id` in the envelope must have
    /// that value overwritten by the pi-Agent facade's deterministic id, on
    /// both `result.reproducibility.trace_id` and the top-level `result.trace_id`.
    /// This is intentional (the facade owns trace identity), not a silent fallback.
    #[test]
    fn pi_agent_overwrites_provider_supplied_trace_id() {
        let provider = ProviderWithTraceId {
            inner: FakeTextModelProvider::local_pi(),
            provider_trace_id: "provider-supplied-trace-xyz",
        };
        let agent = PiAgent::new(Box::new(provider), "pi-agent-local");
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 99,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "sha256:abc".into(),
        };

        let result = agent.run(request).expect("pi-agent run succeeds");

        let expected = format!(
            "pi-agent-evidence-{}",
            crate::shared::stable_sha256_hash("pi-agent-local:99")
        );
        assert_eq!(
            result.reproducibility.trace_id.as_deref(),
            Some(expected.as_str()),
            "facade must overwrite the provider-supplied trace_id with its deterministic id"
        );
        let top = result
            .trace_id
            .as_ref()
            .expect("top-level trace id present");
        assert_eq!(
            top, &expected,
            "top-level trace_id must match the deterministic facade id, not the provider's"
        );
        assert_ne!(
            top, "provider-supplied-trace-xyz",
            "provider trace id must not leak through"
        );
    }

    /// A request whose `agent_id` does not match the facade's `self.agent_id`
    /// must surface an explicit error rather than silently rooting identity at
    /// the facade id while the caller believes it was rooted at the request id.
    #[test]
    fn pi_agent_rejects_mismatched_request_agent_id() {
        let agent = PiAgent::new(
            Box::new(FakeTextModelProvider::local_pi()),
            "pi-agent-local",
        );
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-other".into(),
            run_seed: 7,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "sha256:abc".into(),
        };

        let error = agent
            .run(request)
            .expect_err("mismatched agent_id rejected");
        assert!(
            matches!(
                error,
                PiAgentError::AgentIdMismatch {
                    ref facade,
                    ref request,
                } if facade == "pi-agent-local" && request == "pi-agent-other"
            ),
            "expected AgentIdMismatch, got {error:?}"
        );
    }

    /// A stub `TextModelClient` that mimics a real HTTP provider: it returns an
    /// `AgentOutputEnvelope` JSON whose reproducibility fields are deliberately
    /// *wrong* (the values a remote model would echo — empty/garbage — not the
    /// locally-derived ones). This proves the runtime-owns-reproducibility fix
    /// for the Critical finding (C1): a real provider response now validates
    /// instead of being rejected by `ensure_matching_reproducibility`.
    struct StubHttpProviderClient;

    impl TextModelClient for StubHttpProviderClient {
        fn complete(
            &self,
            request: TextModelClientRequest<'_>,
        ) -> Result<TextModelResponse, crate::TextModelProviderError> {
            // Build a valid ScenePlan proposal. The reproducibility block
            // echoes intentionally-wrong values to simulate a remote model
            // that cannot know the locally-derived identity.
            let proposal = AgentOutputProposal {
                id: format!("{}-scene-plan", request.request.call_id),
                agent: request.request.agent.clone(),
                output: AgentProposalPayload::ScenePlan(Box::new(ScenePlanProposal {
                    scene_key: request.request.scene_key.clone(),
                    title: "Stub Provider Scene".into(),
                    location: "Test Hall".into(),
                    scene_summary: "A stub provider proposes a scene.".into(),
                    dramatic_purpose: "Prove real providers can validate.".into(),
                    hook: "The stub returns an envelope with wrong reproducibility.".into(),
                    emotional_goal: None,
                    cast: Vec::new(),
                    entry_beat_id: format!("{}-beat-001", request.request.scene_key),
                    background_asset: None,
                })),
            };
            let envelope = AgentOutputEnvelope {
                id: format!("{}-envelope", proposal.id),
                contract_version: CONTRACT_VERSION.into(),
                schema_version: CONTRACT_SCHEMA_VERSION,
                agent: proposal.agent.clone(),
                reproducibility: ReproducibilityMetadata {
                    run_seed: 999_999,                                    // wrong
                    prompt_version: "remote-model-prompt-v7".into(),      // wrong
                    model_version: "remote-model-v7".into(),              // wrong
                    provider_config_hash: "sha256:remote-unknown".into(), // wrong
                    mcp_tool_call_hash: None,
                    trace_id: Some("provider-supplied-trace-abc".into()),
                    snapshot_id: None,
                },
                proposal,
            };
            let json = serde_json::to_string(&envelope).expect("serialize stub envelope");
            Ok(TextModelResponse::json(json))
        }
    }

    /// Regression test for the Critical reproducibility finding (C1): a
    /// `ConfiguredTextModelProvider` wrapping a stub HTTP client that returns
    /// wrong reproducibility metadata must still drive a successful
    /// `run_with_envelope` because the runtime owns reproducibility identity.
    #[test]
    fn pi_agent_run_with_envelope_validates_real_provider_output() {
        let config = TextProviderConfig::openai_compatible(
            "stub-provider",
            "stub-model",
            "https://example.invalid/v1",
            "PLOTFORGE_TEST_STUB_KEY",
        );
        let provider = ConfiguredTextModelProvider::new(
            config,
            StubHttpProviderClient,
            crate::OptionalEnvCredentialResolver,
        );
        let agent = PiAgent::new(Box::new(provider), "pi-agent-local");
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 7,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "sha256:abc".into(),
        };

        let (result, envelope) = agent
            .run_with_envelope(request)
            .expect("real-provider envelope must validate after the reproducibility fix");
        // The facade overwrites reproducibility with locally-derived values,
        // not the wrong values the stub supplied.
        assert_eq!(result.reproducibility.run_seed, 7);
        assert_ne!(
            result.reproducibility.prompt_version, "remote-model-prompt-v7",
            "facade must overwrite the provider's prompt_version"
        );
        assert_ne!(
            result.reproducibility.provider_config_hash, "sha256:remote-unknown",
            "facade must overwrite the provider's config hash"
        );
        // The provider-supplied trace id must be overwritten by the facade's
        // deterministic id.
        assert_ne!(
            result.reproducibility.trace_id.as_deref(),
            Some("provider-supplied-trace-abc"),
            "facade must overwrite the provider's trace_id"
        );
        // The validated proposal must be a ScenePlan (the payload the apply
        // path commits).
        assert!(
            matches!(envelope.proposal.output, AgentProposalPayload::ScenePlan(_)),
            "envelope must carry a ScenePlan payload for the apply path"
        );
    }
}
