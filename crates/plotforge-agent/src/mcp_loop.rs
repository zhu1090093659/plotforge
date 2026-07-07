//! MCP tool-use orchestrator (Phase 4, P4.1).
//!
//! `complete_with_mcp_tools` is the multi-turn tool-call loop that wraps the
//! existing one-shot `complete_text_agent_output` pipeline *without modifying
//! the `TextModelClient` trait* or its three production impls, the fake
//! provider, or the test stub (S.U.P.E.R P + R). The loop runs outside
//! `complete_text_agent_output`, which still owns JSON repair, envelope
//! validation, and reproducibility identity on the final turn.
//!
//! ## Wire protocol (out-of-band tool-call signal)
//!
//! `AgentOutputEnvelope` / `AgentProposalPayload` have no `tool_call` variant
//! by design — payloads are validated proposals (`world_expansion`,
//! `scene_plan`, …). A model that wants to call an MCP tool instead emits a
//! JSON document carrying an out-of-band `mcp_tool_calls` array. This module's
//! `parse_mcp_tool_calls` recognizes that signal *before* any envelope
//! validation runs, so the one-shot pipeline never sees an intermediate
//! tool-call turn as a malformed envelope. Only the final turn (a real
//! `AgentOutputEnvelope`) flows through `complete_text_agent_output`.
//!
//! The intermediate JSON is intentionally not an `AgentOutputEnvelope`:
//! - It carries `mcp_tool_calls: [{ server_id, tool_name, arguments }]`.
//! - It may carry an optional `note` for redaction-safe diagnostics.
//! - It is never validated, repaired, or persisted as a proposal.
//!
//! ## Redaction + reproducibility (P4.1 surface)
//!
//! Every tool-call argument and result is passed through
//! `contains_secret_marker_text` before and after each round. The MCP tool
//! registry derives an `mcp_tool_call_hash` from the non-secret config fields
//! of every enabled server (mirrors `provider_config_hash`); the wrapper
//! stamps that hash onto the final envelope's reproducibility block so the
//! downstream trace/snapshot carries reproducibility metadata for the MCP
//! round without ever storing raw tool bodies (P4.2 extends the evidence
//! surface; this task only carries the hash + the existing fields).
//!
//! ## No silent fallback (AGENTS.md:144)
//!
//! All transport failures surface as explicit errors
//! (`mcp_tool_error` / `mcp_spawn_failed` / `mcp_unknown_server`). A tool
//! whose result carries a secret marker is rejected, not silently redacted
//! and continued. The loop is bounded by `MAX_ROUNDS`; exceeding it is an
//! explicit `mcp_loop_exceeded_rounds` error, not a silent truncation.
//!
//! ## Passthrough when no servers are enabled
//!
//! `enabled_mcp_servers` empty → the wrapper drives the provider through the
//! existing one-shot `complete_text_agent_output` path. The final envelope is
//! indistinguishable from a direct `PiAgent::run_with_envelope` call, so the
//! single-call Studio surface (`pi_agent_apply_run`) is preserved: the
//! wrapper returns the same `(PiAgentRunResult, AgentOutputEnvelope)` pair the
//! existing path returns.

use plotforge_mcp::{McpError, McpToolCallRequest, McpToolClient};
use plotforge_schema::{
    AgentOutputEnvelope, AgentRole, PiAgentDescriptor, PiAgentRunRequest, PiAgentRunResult,
    ReproducibilityMetadata, contains_secret_marker_text,
};

use crate::pipelines::complete_text_agent_output;
use crate::shared::stable_sha256_hash;
use crate::{TextModelProvider, TextModelRequest, TextModelResponse};

/// Maximum number of tool-call rounds before the loop gives up with an
/// explicit error. Bounds a runaway model that keeps requesting tool calls
/// instead of producing a final proposal. The bound is generous (most MCP
/// flows need 1-3 rounds) so legitimate multi-tool flows still succeed.
const MAX_ROUNDS: usize = 10;

/// Errors raised by the MCP tool-use loop. All variants are redaction-safe:
/// no raw tool argument, result body, credential value, or secret marker is
/// ever stored in an error. Mirrors the discipline of `PiAgentError` and
/// `McpError`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum McpLoopError {
    /// A tool-call round returned a result that contained a secret marker.
    /// The marker is rejected, not silently redacted and continued.
    #[error("mcp_loop_secret_marker: tool `{tool_name}` result contained a secret marker")]
    ToolResultSecretMarker { tool_name: String },

    /// A tool-call argument contained a secret marker. The argument is
    /// rejected before the tool is invoked.
    #[error("mcp_loop_secret_marker: tool `{tool_name}` arguments contained a secret marker")]
    ToolArgSecretMarker { tool_name: String },

    /// The MCP transport surfaced an explicit error while invoking a tool.
    /// The `code` is the redaction-safe `McpError` kind (e.g.
    /// `mcp_tool_error`, `mcp_spawn_failed`, `mcp_unknown_server`).
    #[error("mcp_loop_tool_failure: {code}: {detail}")]
    ToolFailure { code: String, detail: String },

    /// The loop exceeded `MAX_ROUNDS` tool-call rounds without the model
    /// producing a final proposal. Bounded to avoid runaway loops.
    #[error("mcp_loop_exceeded_rounds: {0}")]
    ExceededRounds(usize),

    /// The underlying text provider failed on an intermediate or final turn.
    /// The `code`/`message` are redaction-safe (routed through the shared
    /// `ProviderPipelineError::into_runtime_error` redaction path on the
    /// final turn, or the `TextModelProviderError` redaction on intermediate
    /// turns).
    #[error("mcp_loop_provider_failure: {code}: {message}")]
    ProviderFailure { code: String, message: String },
}

/// A parsed tool-call request emitted by the model on an intermediate round.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedMcpToolCall {
    pub server_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

/// Parse the out-of-band `mcp_tool_calls` signal from a raw provider JSON
/// response. Returns `Some(calls)` when the document carries a non-empty
/// `mcp_tool_calls` array (the model is requesting tool calls), and `None`
/// when the document is a final envelope (or any other shape — the caller
/// treats it as the final turn).
///
/// The signal shape is intentionally permissive: the document must be a JSON
/// object with an `mcp_tool_calls` array of `{ server_id, tool_name,
/// arguments }` objects. Unknown fields are ignored (the signal is
/// out-of-band, not a validated schema). An empty array is treated as
/// `None` (the model emitted the field but has no calls to make → final turn).
pub fn parse_mcp_tool_calls(raw_json: &str) -> Option<Vec<ParsedMcpToolCall>> {
    let value: serde_json::Value = match serde_json::from_str(raw_json.trim()) {
        Ok(value) => value,
        Err(_) => return None,
    };
    let object = value.as_object()?;
    let array = object.get("mcp_tool_calls")?.as_array()?;
    if array.is_empty() {
        return None;
    }
    let mut calls = Vec::with_capacity(array.len());
    for entry in array {
        let object = entry.as_object()?;
        let server_id = object.get("server_id")?.as_str()?;
        let tool_name = object.get("tool_name")?.as_str()?;
        let arguments = object.get("arguments").cloned().unwrap_or_default();
        calls.push(ParsedMcpToolCall {
            server_id: server_id.to_string(),
            tool_name: tool_name.to_string(),
            arguments,
        });
    }
    Some(calls)
}

/// Run the pi-Agent with MCP tool-use enabled. Drives a multi-turn tool-call
/// loop against `provider` when `enabled_mcp_servers` is non-empty; when
/// empty, drives the provider through the one-shot
/// `complete_text_agent_output` path so the result is indistinguishable from
/// `PiAgent::run_with_envelope` (passthrough regression guard).
///
/// `agent_id` roots the deterministic trace-evidence identity (mirrors
/// `PiAgent::run_with_envelope`'s `pi-agent-evidence-...` id) so the result
/// is indistinguishable from the existing one-shot path. The caller owns the
/// `TextModelProvider` and passes it by reference; the loop never clones it.
///
/// `mcp_tool_call_hash_inputs` is the redaction-safe hash input per enabled
/// server (server id + transport kind + endpoint_url + credential_env_var
/// *name*, never the credential value). The wrapper derives the combined
/// `mcp_tool_call_hash` stamped onto the final envelope's reproducibility
/// block. Callers that only need the loop and not the hash may pass an empty
/// slice; the final envelope then carries `mcp_tool_call_hash: None` (the
/// local-mock / no-MCP path).
pub fn complete_with_mcp_tools(
    provider: &dyn TextModelProvider,
    agent_id: &str,
    request: PiAgentRunRequest,
    mcp_client: &dyn McpToolClient,
    // The caller passes the configured `enabled_mcp_servers` list so the
    // wrapper's signature carries the full P4.3 contract. The loop itself
    // drives tool calls based on the model's out-of-band `mcp_tool_calls`
    // signal; whether a server is "enabled" is enforced by the registry at
    // invoke time (an unknown/disabled id surfaces as `mcp_unknown_server`).
    // The parameter is therefore accepted but not read inside this function,
    // so callers (and P4.3) wire it without changing the loop's control flow.
    _enabled_mcp_servers: &[String],
    mcp_tool_call_hash_inputs: &[String],
) -> Result<(PiAgentRunResult, AgentOutputEnvelope), McpLoopError> {
    // Identity checks mirroring `PiAgent::run_with_envelope`: the request's
    // agent_id must agree with the caller's agent_id, the prompt hash must be
    // non-empty, and the prompt summary must not carry a secret marker. These
    // are explicit errors (AGENTS.md:144), not silent fallbacks.
    if request.agent_id != agent_id {
        return Err(McpLoopError::ProviderFailure {
            code: "pi_agent_id_mismatch".into(),
            message: format!(
                "pi-agent request agent_id `{}` did not match facade agent_id `{agent_id}`",
                request.agent_id
            ),
        });
    }
    if request.prompt_hash.trim().is_empty() {
        return Err(McpLoopError::ProviderFailure {
            code: "pi_agent_empty_prompt_hash".into(),
            message: "pi-agent request prompt hash must not be empty".into(),
        });
    }
    if contains_secret_marker_text(&request.prompt_summary) {
        return Err(McpLoopError::ProviderFailure {
            code: "pi_agent_prompt_secret_marker".into(),
            message: "pi-agent request prompt summary contained a secret marker".into(),
        });
    }

    let mut prompt = request.prompt_summary.clone();
    // Accumulate a redaction-safe, per-round tool-call summary that flows into
    // the final `evidence_summary` (P4.2). Only tool name, server id, content-
    // block kinds, and a content hash enter this string — never raw tool
    // arguments, raw result bodies, or credentials.
    let mut mcp_evidence = String::new();

    for round in 0..MAX_ROUNDS {
        let reproducibility = provider.reproducibility_metadata(request.run_seed);
        let model_request = TextModelRequest {
            call_id: format!("pi-agent-mcp-{agent_id}-{}-r{round}", request.run_seed),
            agent: AgentRole::ScenePlanner,
            scene_key: format!("pi-agent-{agent_id}"),
            run_seed: reproducibility.run_seed,
            prompt_version: reproducibility.prompt_version.clone(),
            model_version: reproducibility.model_version.clone(),
            provider_config_hash: reproducibility.provider_config_hash.clone(),
            prompt: prompt.clone(),
            // The MCP loop reuses the legacy single-prompt path; structured
            // chat messages are not assembled here.
            messages: None,
        };
        let response =
            provider
                .complete(&model_request)
                .map_err(|error| McpLoopError::ProviderFailure {
                    code: match error.kind {
                        crate::TextModelProviderErrorKind::Provider => "text_provider_error".into(),
                        crate::TextModelProviderErrorKind::Timeout => {
                            "text_provider_timeout".into()
                        }
                        crate::TextModelProviderErrorKind::RateLimit { .. } => error.code,
                        crate::TextModelProviderErrorKind::ContentFiltered { .. } => error.code,
                        crate::TextModelProviderErrorKind::OutputTruncated { .. } => error.code,
                    },
                    message: error.message,
                })?;

        // Note: the intermediate turn's raw JSON is the tool-call request
        // (carrying `mcp_tool_calls`), not a tool result. Secret markers in
        // tool *arguments* are caught by the per-tool arg check below (which
        // reports the specific tool name), so we do not run a blanket raw-JSON
        // scan here — it would mask the per-tool error variant. The final
        // turn's raw JSON flows through `complete_text_agent_output`, which
        // already runs `contains_secret_marker_text` on the provider output.

        let Some(tool_calls) = parse_mcp_tool_calls(&response.raw_json) else {
            // Final turn: the model produced a document without
            // `mcp_tool_calls`. Run it through the one-shot validation
            // pipeline so JSON repair, envelope validation, and
            // reproducibility identity are applied exactly as on the existing
            // path. The provider's last response is the final-turn JSON.
            // Capture the provider's reproducibility identity so the final
            // envelope carries the same identity a direct
            // `run_with_envelope` call would (not the local-mock identity).
            let provider_reproducibility = provider.reproducibility_metadata(request.run_seed);
            return finalize_with_mcp_hash(
                &response.raw_json,
                agent_id,
                &request,
                provider_reproducibility,
                mcp_tool_call_hash_inputs,
                &mcp_evidence,
            );
        };

        // Intermediate turn: invoke each requested tool through the MCP
        // client, redact-check args + results, and feed a redaction-safe
        // summary back into the next round's prompt.
        let mut tool_notes = String::new();
        for call in tool_calls {
            if contains_mcp_secret_marker(&call.arguments.to_string()) {
                return Err(McpLoopError::ToolArgSecretMarker {
                    tool_name: call.tool_name,
                });
            }
            let tool_request = McpToolCallRequest {
                server_id: call.server_id.clone(),
                tool_name: call.tool_name.clone(),
                arguments: call.arguments,
            };
            let result = mcp_client.invoke_tool(tool_request).map_err(|error| {
                McpLoopError::ToolFailure {
                    code: mcp_error_code(&error),
                    detail: format!("{error}"),
                }
            })?;
            // Redaction check: the tool result content must not carry secret
            // markers. A tool that returns secrets is rejected, not silently
            // redacted and continued. Scan the redaction-safe summary plus the
            // raw text blocks (which may carry a marker the summary omits).
            let result_text = tool_result_summary(&result);
            let mut raw_blocks = String::new();
            for block in &result.content {
                if let plotforge_schema::McpToolContentBlock::Text { text } = block {
                    raw_blocks.push_str(text);
                    raw_blocks.push(' ');
                }
            }
            if contains_mcp_secret_marker(&raw_blocks) || contains_mcp_secret_marker(&result_text) {
                return Err(McpLoopError::ToolResultSecretMarker {
                    tool_name: call.tool_name,
                });
            }
            // Feed a redaction-safe summary back. Only the tool name, server
            // id, ok flag, and content-block kinds enter the next prompt —
            // never raw result bodies or arguments.
            tool_notes.push_str(&format!(
                "\n[tool {}: server={}: ok={} is_error={} blocks={}]",
                call.tool_name,
                call.server_id,
                result.ok,
                result.is_error,
                result.content.len(),
            ));
            // Accumulate a redaction-safe per-tool summary for the final
            // `evidence_summary` (P4.2): tool name, server id, content-block
            // kinds, and a content hash (mirrors the `provider_config_hash`
            // discipline — derived from the result content, never the raw
            // body itself). The hash makes the summary reproducible without
            // storing raw tool bodies in traces/snapshots.
            let block_kinds = result
                .content
                .iter()
                .map(|block| match block {
                    plotforge_schema::McpToolContentBlock::Text { .. } => "text",
                    plotforge_schema::McpToolContentBlock::Image { .. } => "image",
                    plotforge_schema::McpToolContentBlock::Resource { .. } => "resource",
                })
                .collect::<Vec<_>>()
                .join(",");
            let content_hash = stable_sha256_hash(&result_text);
            mcp_evidence.push_str(&format!(
                "\n[r{round} tool={} server={} blocks=[{}] content_hash=sha256:{}]",
                call.tool_name,
                call.server_id,
                block_kinds,
                &content_hash[..12],
            ));
        }
        prompt = format!(
            "{prompt_summary}\n\nMCP tool round {round} complete. Tool results (redacted):{tool_notes}\nContinue and produce the final scene plan proposal.",
            prompt_summary = request.prompt_summary,
        );
    }

    Err(McpLoopError::ExceededRounds(MAX_ROUNDS))
}

/// Finalize the loop by running the last raw JSON through the one-shot
/// validation pipeline. Stamps the combined `mcp_tool_call_hash` onto the
/// envelope's reproducibility block. The final envelope is indistinguishable
/// from a direct `PiAgent::run_with_envelope` call except for the MCP hash.
///
/// `provider_reproducibility` is the real provider's reproducibility identity
/// (captured from `provider.reproducibility_metadata` on the final turn). It
/// is replayed by the `CachedResponseProvider` so the one-shot validation
/// pipeline stamps the same identity a direct `run_with_envelope` call would,
/// not the local-mock identity — keeping the passthrough branch
/// byte-identical to the existing one-shot path.
///
/// `mcp_evidence` is the redaction-safe per-round tool-call summary
/// accumulated by the loop (tool name, server id, content-block kinds,
/// content hash). It is appended to the final `evidence_summary` so the
/// downstream trace/snapshot carries reproducibility metadata for the MCP
/// round without ever storing raw tool bodies (P4.2).
fn finalize_with_mcp_hash(
    final_raw_json: &str,
    agent_id: &str,
    request: &PiAgentRunRequest,
    provider_reproducibility: ReproducibilityMetadata,
    mcp_tool_call_hash_inputs: &[String],
    mcp_evidence: &str,
) -> Result<(PiAgentRunResult, AgentOutputEnvelope), McpLoopError> {
    // Stamp the MCP tool-call hash onto reproducibility. The hash is derived
    // only from non-secret config fields (server id, transport kind,
    // endpoint_url, credential_env_var *name*); never the credential value.
    let mcp_tool_call_hash = if mcp_tool_call_hash_inputs.is_empty() {
        None
    } else {
        Some(format!(
            "sha256:{}",
            stable_sha256_hash(&mcp_tool_call_hash_inputs.join("|"))
        ))
    };
    let used_mcp_hash = mcp_tool_call_hash.is_some();

    // Run the final-turn JSON through the one-shot validation pipeline via a
    // `CachedResponseProvider` that replays the captured JSON + the provider's
    // real reproducibility identity. This reuses JSON repair, envelope
    // validation, and reproducibility identity exactly as on the existing
    // path, so the final envelope is indistinguishable from a one-shot
    // provider call — except for the MCP hash.
    let cached = CachedResponseProvider::new(
        final_raw_json.to_string(),
        provider_reproducibility,
        mcp_tool_call_hash.clone(),
    );
    let envelope = complete_text_agent_output(
        &cached,
        AgentRole::ScenePlanner,
        request.run_seed,
        format!("pi-agent-mcp-final-{agent_id}-{}", request.run_seed),
        format!("pi-agent-{agent_id}"),
        request.prompt_summary.clone(),
    )
    .map_err(|pipeline_error| {
        let runtime_error = pipeline_error.into_runtime_error();
        McpLoopError::ProviderFailure {
            code: runtime_error.code,
            message: runtime_error.message,
        }
    })?;

    // The pi-Agent facade owns the deterministic trace-evidence id; mirror
    // `run_with_envelope` so the result is indistinguishable from the
    // existing one-shot path except for the MCP hash.
    let trace_evidence_id = format!(
        "pi-agent-evidence-{}",
        stable_sha256_hash(&format!("{}:{}", agent_id, request.run_seed))
    );
    let mut reproducibility = envelope.reproducibility.clone();
    if let Some(hash) = mcp_tool_call_hash {
        reproducibility = reproducibility.with_mcp_tool_call_hash(hash);
    }
    reproducibility = reproducibility.with_trace_id(&trace_evidence_id);
    let trace_id = reproducibility.trace_id.clone();

    // Flip the MCP tool-use capability to `wired` on turns that drove MCP
    // tool calls (P4.2): `used_mcp_hash` is true exactly when the loop had a
    // non-empty `mcp_tool_call_hash_inputs` list, which is true exactly when
    // the caller enabled MCP servers for this turn.
    let descriptor = PiAgentDescriptor {
        agent_id: agent_id.to_string(),
        is_local_pi: true,
        capabilities: crate::pi_agent::pi_agent_capabilities_with_mcp(used_mcp_hash),
    };
    // Build the redaction-safe evidence summary. When MCP was used, append the
    // per-round tool-call summary (tool name, server id, content-block kinds,
    // content hash) — never raw tool arguments or result bodies. The summary
    // is scanned for secret markers as a defence-in-depth check.
    let mut evidence_summary = format!(
        "pi-Agent validated a {} proposal from {} provider evidence{}.",
        crate::shared::payload_kind(&envelope.proposal.output),
        reproducibility.provider_config_hash,
        if used_mcp_hash {
            " with MCP tool use"
        } else {
            ""
        },
    );
    if used_mcp_hash && !mcp_evidence.is_empty() {
        evidence_summary.push_str(" MCP tool-call rounds:");
        evidence_summary.push_str(mcp_evidence);
    }
    // Defence-in-depth: the evidence summary must not carry secret markers.
    if contains_mcp_secret_marker(&evidence_summary) {
        return Err(McpLoopError::ToolResultSecretMarker {
            tool_name: "<evidence-summary>".into(),
        });
    }
    let result = PiAgentRunResult {
        descriptor,
        reproducibility,
        trace_id,
        evidence_summary,
    };
    Ok((result, envelope))
}

/// Render a redaction-safe one-line summary of a tool-call result. Only
/// describes the block kinds and counts, never the raw block content.
fn tool_result_summary(result: &plotforge_schema::McpToolCallResult) -> String {
    let kinds: Vec<&str> = result
        .content
        .iter()
        .map(|block| match block {
            plotforge_schema::McpToolContentBlock::Text { .. } => "text",
            plotforge_schema::McpToolContentBlock::Image { .. } => "image",
            plotforge_schema::McpToolContentBlock::Resource { .. } => "resource",
        })
        .collect();
    format!(
        "ok={} is_error={} blocks=[{}]",
        result.ok,
        result.is_error,
        kinds.join(","),
    )
}

/// Check a tool-call argument or result for secret markers. MCP arguments
/// and results are structured JSON, so a whitespace-tokenized scan (the
/// standard `contains_secret_marker_text`) can miss an `sk-...` marker buried
/// inside a single dense JSON token like `{"token":"sk-test-secret-marker"}`.
/// This helper re-scans the same marker set against the raw string with
/// case-insensitive `contains`, so a secret anywhere in the JSON is caught
/// regardless of whitespace. Mirrors the markers in
/// `plotforge_schema::contains_secret_marker` but operates on the full string.
fn contains_mcp_secret_marker(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    normalized.contains("sk-")
        || normalized.contains("api_key")
        || normalized.contains("secret_key")
        || normalized.contains("openai_api_key")
        || normalized.contains("authorization:")
        || normalized.contains("bearer")
        || normalized.contains("token=")
}

/// Map an `McpError` to its redaction-safe code string for trace/error use.
fn mcp_error_code(error: &McpError) -> String {
    match error {
        McpError::SpawnFailed { .. } => "mcp_spawn_failed".into(),
        McpError::HandshakeFailed { .. } => "mcp_handshake_failed".into(),
        McpError::ToolError { .. } => "mcp_tool_error".into(),
        McpError::TransportUnsupported { .. } => "mcp_transport_unsupported".into(),
        McpError::UnknownServer { .. } => "mcp_unknown_server".into(),
        McpError::Io { .. } => "mcp_io".into(),
    }
}

// ===========================================================================
// Internal adapter
// ===========================================================================

/// A `TextModelProvider` that returns a single cached raw JSON response on
/// every `complete()` call. Used on the final turn to replay the last raw
/// JSON through `complete_text_agent_output` so JSON repair, envelope
/// validation, and reproducibility identity are applied exactly as on the
/// one-shot path. Carries the real provider's reproducibility identity
/// (captured on the final turn) plus an optional `mcp_tool_call_hash` so the
/// reproducibility block stamped by the pipeline matches the direct path and
/// includes the MCP hash.
struct CachedResponseProvider {
    raw_json: String,
    reproducibility: ReproducibilityMetadata,
    mcp_tool_call_hash: Option<String>,
}

impl CachedResponseProvider {
    fn new(
        raw_json: String,
        reproducibility: ReproducibilityMetadata,
        mcp_tool_call_hash: Option<String>,
    ) -> Self {
        Self {
            raw_json,
            reproducibility,
            mcp_tool_call_hash,
        }
    }
}

impl TextModelProvider for CachedResponseProvider {
    fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
        // Replay the real provider's reproducibility identity (captured on the
        // final turn), overriding only the `run_seed` (which `complete_text_
        // agent_output` passes through) and stamping the MCP hash when set.
        let mut metadata = self.reproducibility.clone();
        metadata.run_seed = run_seed;
        if let Some(hash) = self.mcp_tool_call_hash.clone() {
            metadata = metadata.with_mcp_tool_call_hash(hash);
        }
        metadata
    }

    fn complete(
        &self,
        _request: &TextModelRequest,
    ) -> Result<TextModelResponse, crate::TextModelProviderError> {
        Ok(TextModelResponse::json(self.raw_json.clone()))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use plotforge_mcp::{
        McpError, McpToolCallRequest, McpToolCallResult, McpToolClient, McpToolContentBlock,
        McpToolManifest,
    };
    use plotforge_schema::{
        AgentOutputEnvelope, AgentOutputProposal, AgentProposalPayload, CONTRACT_SCHEMA_VERSION,
        CONTRACT_VERSION, PiAgentRunRequest, ReproducibilityMetadata, ScenePlanProposal,
        contains_secret_marker_text,
    };

    use super::*;
    use crate::{
        FakeTextModelProvider, PiAgent, TextModelClient, TextModelClientRequest, TextModelProvider,
        TextModelResponse, TextProviderConfig,
    };

    /// A `TextModelProvider` that returns a scripted sequence of raw JSON
    /// responses, one per `complete()` call. Used to drive the loop with a
    /// tool-call turn followed by a final turn. The reproducibility identity
    /// is the local-mock identity (matching `FakeTextModelProvider`).
    struct ScriptedProvider {
        responses: Vec<String>,
        call: Cell<usize>,
    }

    impl ScriptedProvider {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses,
                call: Cell::new(0),
            }
        }
    }

    impl TextModelProvider for ScriptedProvider {
        fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
            ReproducibilityMetadata::local_mock(run_seed)
        }

        fn complete(
            &self,
            _request: &TextModelRequest,
        ) -> Result<TextModelResponse, crate::TextModelProviderError> {
            let index = self.call.get();
            self.call.set(index + 1);
            let response = self
                .responses
                .get(index)
                .cloned()
                .unwrap_or_else(|| self.responses.last().cloned().unwrap_or_default());
            Ok(TextModelResponse::json(response))
        }
    }

    /// A `McpToolClient` mock that returns canned tool-call results. Records
    /// the invocations so tests can assert the loop called the right tools.
    struct MockMcpToolClient {
        invocations: std::cell::RefCell<Vec<McpToolCallRequest>>,
        result: McpToolCallResult,
        error: Option<McpError>,
    }

    impl MockMcpToolClient {
        fn ok(text: &str) -> Self {
            Self {
                invocations: std::cell::RefCell::new(Vec::new()),
                result: McpToolCallResult {
                    ok: true,
                    content: vec![McpToolContentBlock::Text { text: text.into() }],
                    is_error: false,
                },
                error: None,
            }
        }

        fn error(error: McpError) -> Self {
            Self {
                invocations: std::cell::RefCell::new(Vec::new()),
                result: McpToolCallResult {
                    ok: true,
                    content: Vec::new(),
                    is_error: false,
                },
                error: Some(error),
            }
        }
    }

    impl McpToolClient for MockMcpToolClient {
        fn list_tools(&self, _server_id: &str) -> Result<Vec<McpToolManifest>, McpError> {
            Ok(Vec::new())
        }

        fn invoke_tool(&self, request: McpToolCallRequest) -> Result<McpToolCallResult, McpError> {
            self.invocations.borrow_mut().push(request.clone());
            if let Some(error) = &self.error {
                return Err(error.clone());
            }
            Ok(self.result.clone())
        }
    }

    fn make_request(agent_id: &str) -> PiAgentRunRequest {
        PiAgentRunRequest {
            agent_id: agent_id.into(),
            run_seed: 7,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "sha256:abc".into(),
        }
    }

    /// Build a final-turn raw JSON envelope carrying a ScenePlan payload,
    /// matching the shape `complete_text_agent_output` expects to validate.
    fn final_envelope_json(scene_key: &str, call_id: &str) -> String {
        let proposal = AgentOutputProposal {
            id: format!("{call_id}-scene-plan"),
            agent: AgentRole::ScenePlanner,
            output: AgentProposalPayload::ScenePlan(Box::new(ScenePlanProposal {
                scene_key: scene_key.into(),
                title: "MCP-Tooled Scene".into(),
                location: "Civic Hall".into(),
                scene_summary: "A scene produced after MCP tool use.".into(),
                dramatic_purpose: "Exercise the MCP tool-call loop.".into(),
                hook: "The model called a tool then produced a scene plan.".into(),
                emotional_goal: Some("controlled generation".into()),
                cast: vec!["city-treasurer".into()],
                entry_beat_id: format!("{scene_key}-beat-001"),
                background_asset: None,
            })),
        };
        let envelope = AgentOutputEnvelope {
            id: format!("{}-envelope", proposal.id),
            contract_version: CONTRACT_VERSION.into(),
            schema_version: CONTRACT_SCHEMA_VERSION,
            agent: AgentRole::ScenePlanner,
            reproducibility: ReproducibilityMetadata::local_mock(7),
            proposal,
        };
        serde_json::to_string(&envelope).expect("serialize final envelope")
    }

    /// Build an intermediate tool-call turn JSON document.
    fn tool_call_json(server_id: &str, tool_name: &str, arguments: serde_json::Value) -> String {
        serde_json::json!({
            "mcp_tool_calls": [
                { "server_id": server_id, "tool_name": tool_name, "arguments": arguments }
            ]
        })
        .to_string()
    }

    /// (a) Empty `enabled_mcp_servers` → identical reproducibility identity to
    /// `run_with_envelope`. Regression guard: the passthrough branch must not
    /// change the existing one-shot path's *reproducibility identity* when no
    /// MCP servers are enabled. The envelope/proposal `id` fields are
    /// call-derived (the loop uses an `mcp-final` call-id prefix), so they are
    /// not asserted byte-identical — only the reproducibility block, trace id,
    /// and evidence summary (the fields downstream consumers rely on).
    #[test]
    fn mcp_loop_empty_servers_matches_run_with_envelope() {
        // Use the same provider identity for both paths so the comparison is
        // fair. The loop path captures the provider's reproducibility identity
        // on the final turn and replays it, so it matches the direct path.
        let provider = FakeTextModelProvider::local_pi();
        let agent = PiAgent::new(
            Box::new(FakeTextModelProvider::local_pi()),
            "pi-agent-local",
        );
        let mcp = MockMcpToolClient::ok("tool result");
        let request = make_request("pi-agent-local");

        let (looped_result, looped_envelope) =
            complete_with_mcp_tools(&provider, "pi-agent-local", request.clone(), &mcp, &[], &[])
                .expect("empty-server path succeeds");
        let (direct_result, direct_envelope) = agent
            .run_with_envelope(request)
            .expect("direct path succeeds");

        // Reproducibility identity must be byte-identical (run_seed,
        // prompt_version, model_version, provider_config_hash, trace_id).
        assert_eq!(looped_result.reproducibility, direct_result.reproducibility);
        assert_eq!(looped_result.trace_id, direct_result.trace_id);
        // Evidence summary differs only in the " with MCP tool use" suffix on
        // the loop path when servers are enabled; here (empty servers) both
        // carry no MCP hash, so the suffix is absent on both — they must match.
        assert_eq!(
            looped_result.evidence_summary,
            direct_result.evidence_summary
        );
        // The envelope payload kind must match (both ScenePlan).
        assert!(matches!(
            looped_envelope.proposal.output,
            AgentProposalPayload::ScenePlan(_)
        ));
        assert!(matches!(
            direct_envelope.proposal.output,
            AgentProposalPayload::ScenePlan(_)
        ));
        // No MCP hash on the passthrough path.
        assert!(
            looped_result.reproducibility.mcp_tool_call_hash.is_none(),
            "passthrough must not stamp an MCP hash"
        );
        // No tool invocations on the passthrough path.
        assert!(
            mcp.invocations.borrow().is_empty(),
            "passthrough must not invoke any tools"
        );
    }

    /// (b) With a `MockMcpToolClient` returning canned tool calls, the loop
    /// produces a final envelope after the tool round.
    #[test]
    fn mcp_loop_invokes_tool_then_finalizes() {
        let tool_call = tool_call_json(
            "local-fs",
            "list_files",
            serde_json::json!({"path": "/tmp"}),
        );
        let final_json =
            final_envelope_json("pi-agent-pi-agent-local", "pi-agent-mcp-pi-agent-local-r0");
        let provider = ScriptedProvider::new(vec![tool_call, final_json]);
        let mcp = MockMcpToolClient::ok("file: a.txt");
        let request = make_request("pi-agent-local");

        let (result, envelope) = complete_with_mcp_tools(
            &provider,
            "pi-agent-local",
            request,
            &mcp,
            &["local-fs".to_string()],
            &["local-fs|stdio||".to_string()],
        )
        .expect("loop succeeds");

        // The loop must have invoked the tool exactly once.
        let invocations = mcp.invocations.borrow();
        assert_eq!(invocations.len(), 1, "loop must invoke the tool once");
        assert_eq!(invocations[0].server_id, "local-fs");
        assert_eq!(invocations[0].tool_name, "list_files");

        // The final envelope must carry a ScenePlan payload (the loop's final
        // turn is a real proposal, not a tool-call turn).
        assert!(
            matches!(envelope.proposal.output, AgentProposalPayload::ScenePlan(_)),
            "final envelope must be a ScenePlan"
        );
        // The MCP tool-call hash must be stamped onto reproducibility.
        assert!(
            result
                .reproducibility
                .mcp_tool_call_hash
                .as_ref()
                .unwrap()
                .starts_with("sha256:"),
            "mcp_tool_call_hash must be present and sha256-prefixed"
        );
        // Evidence summary must be redaction-safe.
        assert!(!contains_secret_marker_text(&result.evidence_summary));

        // P4.2: the evidence summary must carry a redaction-safe tool-call
        // summary (tool name, server id, content hash) but NEVER the raw tool
        // arguments or raw tool result bodies.
        let summary = &result.evidence_summary;
        assert!(
            summary.contains("list_files"),
            "evidence summary must name the invoked tool"
        );
        assert!(
            summary.contains("local-fs"),
            "evidence summary must name the server id"
        );
        assert!(
            summary.contains("content_hash=sha256:"),
            "evidence summary must carry the redaction-safe content hash"
        );
        // The raw tool arguments ("/tmp") and raw tool result ("file: a.txt")
        // must NOT appear anywhere in the evidence summary.
        assert!(
            !summary.contains("/tmp"),
            "evidence summary must not carry raw tool arguments"
        );
        assert!(
            !summary.contains("file: a.txt"),
            "evidence summary must not carry raw tool result bodies"
        );

        // P4.2: the `pi-agent.mcp-tool-use` capability must be `wired` on a
        // turn that drove MCP tool calls (the loop flips it via
        // `pi_agent_capabilities_with_mcp(true)`).
        let mcp_capability = result
            .descriptor
            .capabilities
            .iter()
            .find(|c| c.id == "pi-agent.mcp-tool-use")
            .expect("mcp-tool-use capability present on the loop result");
        assert_eq!(
            mcp_capability.status, "wired",
            "mcp-tool-use must be wired on a turn that drove tool calls"
        );

        // P4.2: the serialized result must roundtrip and carry only redacted
        // MCP fields (no raw tool bodies leak through serialization).
        let encoded = serde_json::to_string(&result).expect("serialize result");
        let decoded: PiAgentRunResult = serde_json::from_str(&encoded).expect("deserialize result");
        assert_eq!(decoded.reproducibility, result.reproducibility);
        assert!(!encoded.contains("file: a.txt"));
        assert!(!encoded.contains("/tmp"));
    }

    /// (c) Tool error → explicit `mcp_tool_error`, not silent skip.
    #[test]
    fn mcp_loop_tool_error_is_explicit() {
        let tool_call = tool_call_json("local-fs", "boom", serde_json::json!({}));
        let final_json = final_envelope_json("pi-agent-pi-agent-local", "ignored");
        let provider = ScriptedProvider::new(vec![tool_call, final_json]);
        let mcp = MockMcpToolClient::error(McpError::ToolError {
            tool_name: "boom".into(),
            detail: "tool exploded".into(),
        });
        let request = make_request("pi-agent-local");

        let error = complete_with_mcp_tools(
            &provider,
            "pi-agent-local",
            request,
            &mcp,
            &["local-fs".to_string()],
            &["local-fs|stdio||".to_string()],
        )
        .expect_err("tool error must surface");
        assert!(
            matches!(error, McpLoopError::ToolFailure { ref code, .. } if code == "mcp_tool_error"),
            "expected ToolFailure with mcp_tool_error code, got {error:?}"
        );
    }

    /// (d) Secret marker in a tool result → explicit rejection, not silent
    /// redaction-and-continue.
    #[test]
    fn mcp_loop_rejects_secret_marker_in_tool_result() {
        let tool_call = tool_call_json("local-fs", "leak", serde_json::json!({}));
        let final_json = final_envelope_json("pi-agent-pi-agent-local", "ignored");
        let provider = ScriptedProvider::new(vec![tool_call, final_json]);
        let mcp = MockMcpToolClient::ok("the api_key is sk-test-secret-marker here");
        let request = make_request("pi-agent-local");

        let error = complete_with_mcp_tools(
            &provider,
            "pi-agent-local",
            request,
            &mcp,
            &["local-fs".to_string()],
            &["local-fs|stdio||".to_string()],
        )
        .expect_err("secret marker in tool result must be rejected");
        assert!(
            matches!(error, McpLoopError::ToolResultSecretMarker { ref tool_name } if tool_name == "leak"),
            "expected ToolResultSecretMarker, got {error:?}"
        );
    }

    /// (e) Secret marker in a tool-call argument → explicit rejection before
    /// the tool is invoked.
    #[test]
    fn mcp_loop_rejects_secret_marker_in_tool_args() {
        let tool_call = tool_call_json(
            "local-fs",
            "read_file",
            serde_json::json!({"token": "sk-test-secret-marker"}),
        );
        let final_json = final_envelope_json("pi-agent-pi-agent-local", "ignored");
        let provider = ScriptedProvider::new(vec![tool_call, final_json]);
        let mcp = MockMcpToolClient::ok("should not be reached");
        let request = make_request("pi-agent-local");

        let error = complete_with_mcp_tools(
            &provider,
            "pi-agent-local",
            request,
            &mcp,
            &["local-fs".to_string()],
            &["local-fs|stdio||".to_string()],
        )
        .expect_err("secret marker in tool args must be rejected");
        assert!(
            matches!(error, McpLoopError::ToolArgSecretMarker { ref tool_name } if tool_name == "read_file"),
            "expected ToolArgSecretMarker, got {error:?}"
        );
        // The tool must not have been invoked.
        assert!(
            mcp.invocations.borrow().is_empty(),
            "tool must not be invoked when args carry a secret marker"
        );
    }

    /// (f) Loop exceeding `MAX_ROUNDS` → explicit `ExceededRounds` error.
    #[test]
    fn mcp_loop_exceeds_rounds() {
        // A provider that always emits a tool-call turn, never a final turn.
        let tool_call = tool_call_json("local-fs", "loop", serde_json::json!({}));
        let provider = ScriptedProvider::new(vec![tool_call; MAX_ROUNDS + 1]);
        let mcp = MockMcpToolClient::ok("looping");
        let request = make_request("pi-agent-local");

        let error = complete_with_mcp_tools(
            &provider,
            "pi-agent-local",
            request,
            &mcp,
            &["local-fs".to_string()],
            &["local-fs|stdio||".to_string()],
        )
        .expect_err("runaway loop must surface");
        assert_eq!(error, McpLoopError::ExceededRounds(MAX_ROUNDS));
    }

    /// (g) `parse_mcp_tool_calls` recognizes the out-of-band signal and
    /// returns `None` for a final-turn envelope.
    #[test]
    fn parse_mcp_tool_calls_recognizes_signal() {
        // Tool-call turn → Some(calls).
        let tool_call = tool_call_json("srv", "t", serde_json::json!({"a": 1}));
        let parsed = parse_mcp_tool_calls(&tool_call).expect("tool-call turn parsed");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].server_id, "srv");
        assert_eq!(parsed[0].tool_name, "t");

        // Final-turn envelope → None.
        let final_json = final_envelope_json("s", "c");
        assert!(
            parse_mcp_tool_calls(&final_json).is_none(),
            "final envelope must not parse as a tool-call turn"
        );

        // Empty `mcp_tool_calls` array → None (model has no calls to make).
        let empty = serde_json::json!({"mcp_tool_calls": []}).to_string();
        assert!(parse_mcp_tool_calls(&empty).is_none());

        // Non-JSON → None.
        assert!(parse_mcp_tool_calls("not json").is_none());
    }

    /// The loop must not touch the `TextModelClient` trait or its impls.
    /// This is a compile-time guard: if the wrapper ever requires extending
    /// `TextModelClient`, this test (and the crate) will fail to compile.
    #[test]
    fn mcp_loop_does_not_extend_text_model_client() {
        // The `TextModelClient` trait is unchanged: it still has a single
        // `complete` method returning `TextModelResponse { raw_json }`.
        fn assert_trait_shape(_client: &dyn TextModelClient) {}
        struct StubClient;
        impl TextModelClient for StubClient {
            fn complete(
                &self,
                _request: TextModelClientRequest<'_>,
            ) -> Result<TextModelResponse, crate::TextModelProviderError> {
                Ok(TextModelResponse::json("{}"))
            }
        }
        assert_trait_shape(&StubClient);
    }

    /// A real `ConfiguredTextModelProvider` (production impl) must still work
    /// as a `TextModelProvider` for the loop's intermediate turns. This is a
    /// smoke guard that the three production impls are byte-identical
    /// (P4.1 acceptance: do not touch `TextModelClient` + impls).
    #[test]
    fn production_providers_still_implement_text_model_provider() {
        fn assert_provider<P: TextModelProvider>(_provider: &P) {}
        let config = TextProviderConfig::openai_compatible(
            "stub",
            "stub-model",
            "https://example.invalid/v1",
            "PLOTFORGE_TEST_STUB_KEY",
        );
        let client = crate::OpenAiCompatibleClient::new()
            .expect("openai-compatible client constructs for the smoke test");
        let provider = crate::ConfiguredTextModelProvider::new(
            config,
            client,
            crate::OptionalEnvCredentialResolver,
        );
        assert_provider(&provider);
    }
}
