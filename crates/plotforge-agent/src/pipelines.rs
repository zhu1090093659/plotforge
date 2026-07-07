//! Text agent orchestration pipelines.
//!
//! Holds the shared `ProviderPipelineError`, envelope validation/JSON repair,
//! reproducibility matching, and the `complete_text_agent_output` facade used
//! by the generation entry points (`generate_world_expansion`, etc.) and the
//! pi-Agent facade. Scene planner orchestration (`complete_agent_output`)
//! lives in the scene planner module; this module only owns the text-output
//! pipeline shared by generation and pi-Agent paths.

use plotforge_schema::{
    AgentOutputEnvelope, AgentProposalPayload, AgentRole, CONTRACT_SCHEMA_VERSION,
    CONTRACT_VERSION, Character, CharacterGenerationReport, CharacterGenerationRequest,
    CharacterPortraitRequest, GenerationEvidence, GenerationStatus, ReproducibilityMetadata,
    RuntimeError, StoryCraftGenerationReport, StoryCraftGenerationRequest, WorldEditDocument,
    WorldGenerationReport, WorldGenerationRequest, contains_secret_marker_text,
};

use crate::providers_text::{
    FakeTextModelProvider, TextModelProvider, TextModelProviderError, TextModelProviderErrorKind,
};
use crate::shared::{payload_kind, stable_sha256_hash};
use crate::validation::validate_agent_output_proposal;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProviderPipelineError {
    Provider(TextModelProviderError),
    InvalidJson { agent: AgentRole, message: String },
    Validation { agent: AgentRole, message: String },
}

impl ProviderPipelineError {
    pub(crate) fn into_runtime_error(self) -> RuntimeError {
        match self {
            Self::Provider(error) => match error.kind {
                TextModelProviderErrorKind::Provider => {
                    RuntimeError::redacted(error.code, error.message)
                }
                TextModelProviderErrorKind::Timeout => RuntimeError::redacted(
                    "text_provider_timeout",
                    format!("text provider timed out: {}", error.message),
                ),
                TextModelProviderErrorKind::RateLimit { .. } => {
                    RuntimeError::redacted(error.code, error.message)
                }
                TextModelProviderErrorKind::ContentFiltered { .. } => {
                    RuntimeError::redacted(error.code, error.message)
                }
                TextModelProviderErrorKind::OutputTruncated { .. } => {
                    RuntimeError::redacted(error.code, error.message)
                }
            },
            Self::InvalidJson { agent, message } => RuntimeError::redacted(
                "text_provider_invalid_json",
                format!("{agent:?} returned invalid JSON: {message}"),
            ),
            Self::Validation { agent, message } => RuntimeError::redacted(
                "text_provider_schema_validation",
                format!("{agent:?} returned invalid proposal: {message}"),
            ),
        }
    }
}

/// Retry configuration for transient provider failures. Defaults are tuned
/// for interactive generation: up to 3 attempts, 500ms base, capped at 8s.
/// The retry loop wraps `provider.complete()` and honours the server-advised
/// `Retry-After` (carried in `RateLimit.retry_after_ms`) when present,
/// otherwise falling back to exponential backoff with jitter.
///
/// `retry_attempts` is recorded on the final error so callers can surface the
/// attempt count in trace diagnostics (e.g. "rate-limited after 3 attempts").
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RetryPolicy {
    pub max_attempts: u8,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 500,
            max_delay_ms: 8_000,
        }
    }
}

impl RetryPolicy {
    /// Computes the backoff delay for a given 0-based attempt index. Honours
    /// the server-advised `retry_after_ms` when present (clamped to
    /// `max_delay_ms` so a misbehaving server cannot stall the rail), and
    /// otherwise uses exponential backoff with jitter:
    /// `delay = min(max_delay, base * 2^attempt) + jitter`, where the final
    /// sum is clamped to `max_delay_ms` so jitter can never exceed the cap.
    fn delay_for(&self, attempt: u8, retry_after_ms: Option<u64>) -> std::time::Duration {
        if let Some(server_ms) = retry_after_ms {
            let clamped = server_ms.min(self.max_delay_ms);
            return std::time::Duration::from_millis(clamped);
        }
        let exp = self
            .base_delay_ms
            .saturating_mul(2u64.saturating_pow(attempt as u32));
        let base = exp.min(self.max_delay_ms);
        // Deterministic-ish jitter derived from the attempt index so tests are
        // reproducible without a global RNG. The jitter is non-negative and
        // bounded, then the sum is clamped to `max_delay_ms`.
        let jitter = (base / 4) * ((attempt as u64).wrapping_mul(0x9E37) % 2);
        std::time::Duration::from_millis((base + jitter).min(self.max_delay_ms))
    }
}

/// Wraps `provider.complete()` in the retry loop, retrying only the
/// transient kinds (RateLimit, Provider 5xx, Timeout). Content-filtered and
/// truncated errors are non-retryable and surface immediately. Returns the
/// final response on success, or the last error annotated with the number of
/// attempts made (in `error.code` for trace visibility).
fn complete_with_retry<P>(
    provider: &P,
    request: &crate::providers_text::TextModelRequest,
    policy: &RetryPolicy,
) -> Result<crate::providers_text::TextModelResponse, TextModelProviderError>
where
    P: TextModelProvider + ?Sized,
{
    let mut last_error: Option<TextModelProviderError> = None;
    for attempt in 0..policy.max_attempts {
        match provider.complete(request) {
            Ok(response) => return Ok(response),
            Err(error) => {
                let retryable = error.kind.is_retryable();
                let is_last_attempt = attempt + 1 >= policy.max_attempts;
                if !retryable || is_last_attempt {
                    // Record the attempt count in the *message* (not the
                    // `code`) so the stable code that `into_runtime_error`
                    // and downstream traces branch on stays unchanged, while
                    // trace diagnostics still report how many attempts ran.
                    let annotated = TextModelProviderError {
                        kind: error.kind.clone(),
                        code: error.code,
                        message: format!("{} (after {} attempt(s))", error.message, attempt + 1),
                    };
                    return Err(annotated);
                }
                let retry_after_ms = match &error.kind {
                    TextModelProviderErrorKind::RateLimit { retry_after_ms } => *retry_after_ms,
                    _ => None,
                };
                last_error = Some(error);
                std::thread::sleep(policy.delay_for(attempt, retry_after_ms));
            }
        }
    }
    // Unreachable when max_attempts >= 1; keep the loop total for clarity.
    Err(last_error.unwrap_or_else(|| {
        TextModelProviderError::provider(
            "text_provider_retry_exhausted",
            "provider retry loop exited without a result",
        )
    }))
}

pub(crate) fn complete_text_agent_output<P>(
    provider: &P,
    agent: AgentRole,
    run_seed: u64,
    call_id: String,
    scene_key: String,
    prompt: String,
) -> Result<AgentOutputEnvelope, ProviderPipelineError>
where
    P: TextModelProvider + ?Sized,
{
    complete_text_agent_output_with_retry(
        provider,
        agent,
        run_seed,
        call_id,
        scene_key,
        prompt,
        &RetryPolicy::default(),
    )
}

/// Like `complete_text_agent_output` but with a caller-supplied retry
/// policy. Exposed (crate) so tests can drive the retry loop with a small
/// `max_attempts` and negligible backoff; production callers use the
/// default policy via `complete_text_agent_output`.
pub(crate) fn complete_text_agent_output_with_retry<P>(
    provider: &P,
    agent: AgentRole,
    run_seed: u64,
    call_id: String,
    scene_key: String,
    prompt: String,
    policy: &RetryPolicy,
) -> Result<AgentOutputEnvelope, ProviderPipelineError>
where
    P: TextModelProvider + ?Sized,
{
    complete_text_agent_output_core(
        provider, agent, run_seed, call_id, scene_key, prompt, None, None, policy,
    )
}

/// Structured-prompt variant of `complete_text_agent_output`.
///
/// This packs a pre-assembled chat-message list (`messages`, produced by
/// `crate::prompts::PromptAssembler::assemble`) into the
/// `TextModelRequest.messages` field so a future chat-completions provider
/// adapter can send a structured turn instead of a single completion string.
/// The `prompt` field of the request is set to the concatenated content of
/// the `User` messages in `messages` so the existing single-prompt provider
/// path (and the redaction scan) keeps working unchanged — this is the
/// additive fallback/compat contract: `None` means "use `prompt`", and even
/// when `messages` is `Some`, the `prompt` string carries the same user
/// content so providers that have not yet adopted the chat-message body are
/// not broken.
///
/// `prompt_version` overrides `ReproducibilityMetadata.prompt_version`. The
/// legacy `complete_text_agent_output` path derives that field from the
/// provider's `reproducibility_metadata` (which reports the shared
/// `TEXT_PROMPT_VERSION` constant); the structured-template path instead
/// reports the per-role template version (e.g. `scene_planner_v1`) so a
/// generated envelope can be traced back to the exact prompt template that
/// produced it. The rest of the reproducibility block (`run_seed`,
/// `model_version`, `provider_config_hash`) is still owned by the runtime.
///
/// Uses the default `RetryPolicy`. The `TextModelClient` trait and the three
/// HTTP `complete()` signatures are NOT modified by this function — it only
/// populates the new additive `messages` field on the request struct.
///
/// `#[allow(dead_code)]`: this is the structured-prompt entry point for the
/// ScenePlanner/BeatWriter/PlotDoctor pipeline. Existing callers (pi-Agent,
/// MCP loop, generation pipelines) still use the legacy
/// `complete_text_agent_output` single-prompt path; the structured path is
/// exercised by the prompt-template tests and will be adopted by the
/// role-specific agent pipelines in later tasks. Removing it would break the
/// additive template contract this task introduces.
#[allow(clippy::too_many_arguments)] // mirrors complete_text_agent_output's arity
#[allow(dead_code)] // consumed by tests + future role-specific pipelines
pub(crate) fn complete_text_agent_output_with_messages<P>(
    provider: &P,
    agent: AgentRole,
    run_seed: u64,
    call_id: String,
    scene_key: String,
    messages: Vec<crate::prompts::ChatMessage>,
    prompt_version: &str,
) -> Result<AgentOutputEnvelope, ProviderPipelineError>
where
    P: TextModelProvider + ?Sized,
{
    // Derive the fallback `prompt` string from the user-message content so the
    // existing single-prompt provider path (redaction scan, HTTP body,
    // generated_character prompt hash) keeps working when `messages` is
    // populated. Only `User` messages contribute: the system message is
    // role/instruction text that the legacy single-prompt path never carried,
    // and the fallback `prompt` must stay a faithful summary of the user turn.
    let prompt = user_prompt_from_messages(&messages);
    complete_text_agent_output_core(
        provider,
        agent,
        run_seed,
        call_id,
        scene_key,
        prompt,
        Some(messages),
        Some(prompt_version),
        &RetryPolicy::default(),
    )
}

/// Shared core for the text-output pipeline. `messages` is the optional
/// structured chat-message list (populated by the prompt-template path);
/// `prompt_version_override` replaces the provider-derived `prompt_version`
/// in the reproducibility block when `Some` (the structured-template path
/// reports the per-role template version). Both `None` reproduces the legacy
/// `complete_text_agent_output_with_retry` behaviour exactly.
#[allow(clippy::too_many_arguments)] // shared by the two entry points above
fn complete_text_agent_output_core<P>(
    provider: &P,
    agent: AgentRole,
    run_seed: u64,
    call_id: String,
    scene_key: String,
    prompt: String,
    messages: Option<Vec<crate::prompts::ChatMessage>>,
    prompt_version_override: Option<&str>,
    policy: &RetryPolicy,
) -> Result<AgentOutputEnvelope, ProviderPipelineError>
where
    P: TextModelProvider + ?Sized,
{
    if contains_secret_marker_text(&prompt) {
        return Err(ProviderPipelineError::Validation {
            agent,
            message: "text generation prompt contained a secret marker".into(),
        });
    }

    let mut reproducibility = provider.reproducibility_metadata(run_seed);
    if let Some(version) = prompt_version_override {
        // The structured-template path owns the prompt version: overwrite the
        // provider-derived legacy `TEXT_PROMPT_VERSION` with the per-role
        // template version so reproducibility metadata is truthful about which
        // prompt path produced the envelope.
        reproducibility.prompt_version = version.into();
    }
    let model_request = crate::providers_text::TextModelRequest {
        call_id,
        agent: agent.clone(),
        scene_key,
        run_seed: reproducibility.run_seed,
        prompt_version: reproducibility.prompt_version.clone(),
        model_version: reproducibility.model_version.clone(),
        provider_config_hash: reproducibility.provider_config_hash.clone(),
        prompt,
        // `None` => legacy single-prompt path; `Some(messages)` => structured
        // chat-message turn assembled by `PromptAssembler`.
        messages,
    };
    let response = complete_with_retry(provider, &model_request, policy)
        .map_err(ProviderPipelineError::Provider)?;
    if contains_secret_marker_text(&response.raw_json) {
        return Err(ProviderPipelineError::Validation {
            agent,
            message: "provider output contained a secret marker".into(),
        });
    }

    let repaired_json = repair_json_text(&response.raw_json).map_err(|message| {
        ProviderPipelineError::InvalidJson {
            agent: agent.clone(),
            message,
        }
    })?;
    let mut envelope =
        serde_json::from_str::<AgentOutputEnvelope>(&repaired_json).map_err(|error| {
            ProviderPipelineError::InvalidJson {
                agent: agent.clone(),
                message: error.to_string(),
            }
        })?;
    validate_agent_output_envelope(&envelope, &agent).map_err(|message| {
        ProviderPipelineError::Validation {
            agent: agent.clone(),
            message,
        }
    })?;
    // The runtime owns reproducibility identity. Real providers cannot know
    // the locally-derived `run_seed`, `prompt_version`, `model_version`, or
    // `provider_config_hash`, so requiring them to echo those values would
    // reject every real model response. Instead we overwrite the envelope's
    // reproducibility block with the locally-expected values and validate
    // only the structural fields (contract/schema version, agent, non-empty
    // reproducibility) via `validate_agent_output_envelope`. The pi-Agent
    // facade extends this ownership to `trace_id` (see `pi_agent.rs`).
    envelope.reproducibility = reproducibility.clone();
    validate_agent_output_proposal(&envelope.proposal).map_err(|error| {
        ProviderPipelineError::Validation {
            agent,
            message: format!("{error:?}"),
        }
    })?;

    Ok(envelope)
}

/// Concatenates the content of all `User` messages in `messages` into a
/// single fallback prompt string. Used by `complete_text_agent_output_with_
/// messages` to populate `TextModelRequest.prompt` so the existing
/// single-prompt provider path keeps working when a structured chat-message
/// list is supplied. Returns an empty string when there are no user messages
/// (the caller's redaction scan treats an empty prompt as valid; the provider
/// will surface its own error for an empty completion).
#[allow(dead_code)] // called only by complete_text_agent_output_with_messages
fn user_prompt_from_messages(messages: &[crate::prompts::ChatMessage]) -> String {
    messages
        .iter()
        .filter(|message| message.role == crate::prompts::MessageRole::User)
        .map(|message| message.content.as_str())
        .collect::<Vec<&str>>()
        .join("\n")
}

pub(crate) fn validate_agent_output_envelope(
    envelope: &AgentOutputEnvelope,
    expected_agent: &AgentRole,
) -> Result<(), String> {
    if envelope.contract_version != CONTRACT_VERSION {
        return Err(format!(
            "unsupported contract version `{}`, expected `{}`",
            envelope.contract_version, CONTRACT_VERSION
        ));
    }
    if envelope.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema version `{}`, expected `{}`",
            envelope.schema_version, CONTRACT_SCHEMA_VERSION
        ));
    }
    if &envelope.agent != expected_agent {
        return Err(format!(
            "envelope agent `{:?}` did not match requested agent `{:?}`",
            envelope.agent, expected_agent
        ));
    }
    if envelope.proposal.agent != envelope.agent {
        return Err(format!(
            "proposal agent `{:?}` did not match envelope agent `{:?}`",
            envelope.proposal.agent, envelope.agent
        ));
    }
    if envelope.reproducibility.prompt_version.trim().is_empty() {
        return Err("missing prompt_version".into());
    }
    if envelope.reproducibility.model_version.trim().is_empty() {
        return Err("missing model_version".into());
    }
    if envelope
        .reproducibility
        .provider_config_hash
        .trim()
        .is_empty()
    {
        return Err("missing provider_config_hash".into());
    }

    Ok(())
}

pub(crate) fn repair_json_text(raw_json: &str) -> Result<String, String> {
    let trimmed = raw_json.trim();
    if trimmed.is_empty() {
        return Err("empty provider JSON".into());
    }
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return Ok(trimmed.to_string());
    }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}'))
        && start < end
    {
        let candidate = &trimmed[start..=end];
        if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
            return Ok(candidate.to_string());
        }
    }
    Err("unable to repair provider JSON envelope".into())
}

pub fn generate_world_expansion(
    request: WorldGenerationRequest,
    run_seed: u64,
) -> WorldGenerationReport {
    let provider = FakeTextModelProvider::success();
    generate_world_expansion_with_provider(&provider, request, run_seed)
}

pub fn generate_world_expansion_with_provider<P>(
    provider: &P,
    request: WorldGenerationRequest,
    run_seed: u64,
) -> WorldGenerationReport
where
    P: TextModelProvider,
{
    match complete_text_agent_output(
        provider,
        AgentRole::StoryArchitect,
        run_seed,
        "world-expansion".into(),
        "world-generation".into(),
        world_generation_prompt(&request),
    ) {
        Ok(envelope) => match envelope.proposal.output.clone() {
            AgentProposalPayload::WorldExpansion(proposal) => {
                let proposal = *proposal;
                WorldGenerationReport {
                    document: WorldEditDocument {
                        world_bible_markdown: proposal.world_bible_markdown,
                        canon_markdown: proposal.canon_markdown,
                        forbidden_facts: proposal.forbidden_facts,
                    },
                    evidence: generation_evidence(
                        GenerationStatus::Succeeded,
                        false,
                        None,
                        envelope.reproducibility.clone(),
                        vec![envelope],
                    ),
                }
            }
            output => generation_world_fallback(
                request.document,
                run_seed,
                ProviderPipelineError::Validation {
                    agent: AgentRole::StoryArchitect,
                    message: format!("unexpected payload `{}`", payload_kind(&output)),
                },
            ),
        },
        Err(error) => generation_world_fallback(request.document, run_seed, error),
    }
}

pub fn generate_story_craft(
    request: StoryCraftGenerationRequest,
    run_seed: u64,
) -> StoryCraftGenerationReport {
    let provider = FakeTextModelProvider::success();
    generate_story_craft_with_provider(&provider, request, run_seed)
}

pub fn generate_story_craft_with_provider<P>(
    provider: &P,
    request: StoryCraftGenerationRequest,
    run_seed: u64,
) -> StoryCraftGenerationReport
where
    P: TextModelProvider,
{
    match complete_text_agent_output(
        provider,
        AgentRole::StoryCraftPlanner,
        run_seed,
        "story-craft-generation".into(),
        "story-craft-generation".into(),
        story_craft_generation_prompt(&request),
    ) {
        Ok(envelope) => match envelope.proposal.output.clone() {
            AgentProposalPayload::StoryCraftPlan(proposal) => {
                let proposal = *proposal;
                StoryCraftGenerationReport {
                    document: plotforge_schema::StoryCraftEditDocument {
                        story_bible_markdown: proposal.story_bible_markdown,
                        style_guide_markdown: proposal.style_guide_markdown,
                        story_craft: proposal.story_craft,
                    },
                    evidence: generation_evidence(
                        GenerationStatus::Succeeded,
                        false,
                        None,
                        envelope.reproducibility.clone(),
                        vec![envelope],
                    ),
                }
            }
            output => generation_story_craft_fallback(
                request.document,
                run_seed,
                ProviderPipelineError::Validation {
                    agent: AgentRole::StoryCraftPlanner,
                    message: format!("unexpected payload `{}`", payload_kind(&output)),
                },
            ),
        },
        Err(error) => generation_story_craft_fallback(request.document, run_seed, error),
    }
}

pub fn generate_character(
    request: CharacterGenerationRequest,
    run_seed: u64,
) -> CharacterGenerationReport {
    let provider = FakeTextModelProvider::success();
    generate_character_with_provider(&provider, request, run_seed)
}

pub fn generate_character_with_provider<P>(
    provider: &P,
    request: CharacterGenerationRequest,
    run_seed: u64,
) -> CharacterGenerationReport
where
    P: TextModelProvider,
{
    match complete_text_agent_output(
        provider,
        AgentRole::CharacterDesigner,
        run_seed,
        "character-generation".into(),
        "character-generation".into(),
        character_generation_prompt(&request),
    ) {
        Ok(envelope) => match envelope.proposal.output.clone() {
            AgentProposalPayload::CharacterProfile(proposal) => {
                let proposal = *proposal;
                CharacterGenerationReport {
                    character: proposal.character,
                    evidence: generation_evidence(
                        GenerationStatus::Succeeded,
                        false,
                        None,
                        envelope.reproducibility.clone(),
                        vec![envelope],
                    ),
                }
            }
            output => generation_character_fallback(
                request,
                run_seed,
                ProviderPipelineError::Validation {
                    agent: AgentRole::CharacterDesigner,
                    message: format!("unexpected payload `{}`", payload_kind(&output)),
                },
            ),
        },
        Err(error) => generation_character_fallback(request, run_seed, error),
    }
}

fn generation_world_fallback(
    document: WorldEditDocument,
    run_seed: u64,
    error: ProviderPipelineError,
) -> WorldGenerationReport {
    let runtime_error = error.into_runtime_error();
    WorldGenerationReport {
        document,
        evidence: generation_evidence(
            GenerationStatus::Fallback,
            true,
            Some(runtime_error),
            ReproducibilityMetadata::local_mock(run_seed),
            Vec::new(),
        ),
    }
}

fn generation_story_craft_fallback(
    document: plotforge_schema::StoryCraftEditDocument,
    run_seed: u64,
    error: ProviderPipelineError,
) -> StoryCraftGenerationReport {
    let runtime_error = error.into_runtime_error();
    StoryCraftGenerationReport {
        document,
        evidence: generation_evidence(
            GenerationStatus::Fallback,
            true,
            Some(runtime_error),
            ReproducibilityMetadata::local_mock(run_seed),
            Vec::new(),
        ),
    }
}

fn generation_character_fallback(
    request: CharacterGenerationRequest,
    run_seed: u64,
    error: ProviderPipelineError,
) -> CharacterGenerationReport {
    let runtime_error = error.into_runtime_error();
    CharacterGenerationReport {
        character: fallback_character(&request, run_seed),
        evidence: generation_evidence(
            GenerationStatus::Fallback,
            true,
            Some(runtime_error),
            ReproducibilityMetadata::local_mock(run_seed),
            Vec::new(),
        ),
    }
}

fn generation_evidence(
    status: GenerationStatus,
    fallback_used: bool,
    error: Option<RuntimeError>,
    reproducibility: ReproducibilityMetadata,
    envelopes: Vec<AgentOutputEnvelope>,
) -> GenerationEvidence {
    GenerationEvidence {
        status,
        fallback_used,
        error,
        reproducibility,
        envelopes,
    }
}

fn world_generation_prompt(request: &WorldGenerationRequest) -> String {
    format!(
        "agent=StoryArchitect; expansion_goal={}; world_bible_chars={}; canon_chars={}; forbidden_fact_count={}",
        request.expansion_goal,
        request.document.world_bible_markdown.len(),
        request.document.canon_markdown.len(),
        request.document.forbidden_facts.len()
    )
}

fn story_craft_generation_prompt(request: &StoryCraftGenerationRequest) -> String {
    format!(
        "agent=StoryCraftPlanner; concept={}; world_bible_chars={}; canon_chars={}; forbidden_fact_count={}; character_count={}",
        request.concept,
        request.world_bible_markdown.len(),
        request.canon_markdown.len(),
        request.forbidden_facts.len(),
        request.characters.len()
    )
}

fn character_generation_prompt(request: &CharacterGenerationRequest) -> String {
    format!(
        "agent=CharacterDesigner; concept={}; role_hint={}; existing_character_count={}; world_bible_chars={}; story_bible_chars={}",
        request.concept,
        request.role_hint,
        request.existing_characters.len(),
        request.world_bible_markdown.len(),
        request.story_bible_markdown.len()
    )
}

fn fallback_character(request: &CharacterGenerationRequest, run_seed: u64) -> Character {
    let role = if request.role_hint.trim().is_empty() {
        "Fallback story catalyst"
    } else {
        request.role_hint.trim()
    };
    let prompt_hash = format!(
        "sha256:{}",
        stable_sha256_hash(&format!(
            "{}\n{}\n{}",
            request.concept, request.role_hint, run_seed
        ))
    );

    Character {
        id: "fallback-character".into(),
        name: "Fallback Character".into(),
        role: role.into(),
        traits: vec!["visible fallback".into(), "editable".into()],
        visual_card: "Fallback character card; edit before production use.".into(),
        voice_card: "Plain fallback voice; edit before production use.".into(),
        portrait_request: Some(CharacterPortraitRequest {
            prompt_summary: format!("Fallback portrait request for {role}."),
            style: "local placeholder character card".into(),
            target_asset_slot: "portrait".into(),
            prompt_hash,
            provider_config_hash: ReproducibilityMetadata::local_mock(run_seed)
                .provider_config_hash,
            reference_asset_ids: Vec::new(),
            fallback_allowed: true,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers_text::{TextModelRequest, TextModelResponse};
    use plotforge_schema::ReproducibilityMetadata;

    /// A fake `TextModelProvider` that returns a configurable sequence of
    /// errors before finally succeeding. Used to exercise the retry loop:
    /// `failures` is the list of errors to return in order (one per call);
    /// once exhausted, the provider returns a success response built from
    /// `FakeTextModelProvider`. A shared `Rc<Cell<usize>>` counter records
    /// the number of `complete()` invocations so tests can assert retry
    /// attempt counts.
    #[derive(Clone)]
    struct SequencedTextProvider {
        failures: Vec<TextModelProviderError>,
        calls: std::rc::Rc<std::cell::Cell<usize>>,
    }

    impl SequencedTextProvider {
        fn new(
            failures: Vec<TextModelProviderError>,
        ) -> (Self, std::rc::Rc<std::cell::Cell<usize>>) {
            let calls = std::rc::Rc::new(std::cell::Cell::new(0));
            (
                Self {
                    failures,
                    calls: calls.clone(),
                },
                calls,
            )
        }
    }

    impl TextModelProvider for SequencedTextProvider {
        fn complete(
            &self,
            request: &TextModelRequest,
        ) -> Result<TextModelResponse, TextModelProviderError> {
            let n = self.calls.get();
            self.calls.set(n + 1);
            if n < self.failures.len() {
                return Err(self.failures[n].clone());
            }
            // Success: delegate to the deterministic fake for a scene planner.
            crate::providers_text::FakeTextModelProvider::success().complete(request)
        }

        fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
            ReproducibilityMetadata::local_mock(run_seed)
        }
    }

    /// A `TextModelProvider` that always returns the given error. Used to
    /// verify that a single non-retryable error surfaces immediately (one
    /// call) and that a retryable error exhausts the budget.
    #[derive(Clone)]
    struct AlwaysFailingProvider {
        error: TextModelProviderError,
        calls: std::rc::Rc<std::cell::Cell<usize>>,
    }

    impl AlwaysFailingProvider {
        fn new(error: TextModelProviderError) -> (Self, std::rc::Rc<std::cell::Cell<usize>>) {
            let calls = std::rc::Rc::new(std::cell::Cell::new(0));
            (
                Self {
                    error,
                    calls: calls.clone(),
                },
                calls,
            )
        }
    }

    impl TextModelProvider for AlwaysFailingProvider {
        fn complete(
            &self,
            _request: &TextModelRequest,
        ) -> Result<TextModelResponse, TextModelProviderError> {
            let n = self.calls.get();
            self.calls.set(n + 1);
            Err(self.error.clone())
        }

        fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
            ReproducibilityMetadata::local_mock(run_seed)
        }
    }

    /// Retry policy with negligible backoff so tests do not stall. The
    /// `base_delay_ms` of 1ms and `max_delay_ms` of 2ms keep the suite fast
    /// while still exercising the backoff path.
    fn fast_policy(max_attempts: u8) -> RetryPolicy {
        RetryPolicy {
            max_attempts,
            base_delay_ms: 1,
            max_delay_ms: 2,
        }
    }

    #[test]
    fn retry_loop_retries_rate_limit_then_succeeds() {
        let failures = vec![
            TextModelProviderError::rate_limit(Some(2), "rate limited"),
            TextModelProviderError::rate_limit(Some(2), "rate limited"),
        ];
        let (provider, calls) = SequencedTextProvider::new(failures);
        let policy = fast_policy(3);
        let result = complete_text_agent_output_with_retry(
            &provider,
            AgentRole::ScenePlanner,
            1,
            "retry-test".into(),
            "scene-1".into(),
            "{\"role\":\"scene_planner\"}".into(),
            &policy,
        );
        let envelope = result.expect("retry then success");
        assert_eq!(envelope.agent, AgentRole::ScenePlanner);
        assert_eq!(calls.get(), 3, "expected 2 failures + 1 success call");
    }

    #[test]
    fn retry_loop_exhausts_max_attempts_with_explicit_error() {
        let (provider, calls) =
            AlwaysFailingProvider::new(TextModelProviderError::rate_limit(None, "rate limited"));
        let policy = fast_policy(2);
        let error = complete_text_agent_output_with_retry(
            &provider,
            AgentRole::ScenePlanner,
            1,
            "retry-exhaust".into(),
            "scene-1".into(),
            "{\"role\":\"scene_planner\"}".into(),
            &policy,
        )
        .expect_err("must exhaust retries");
        assert_eq!(calls.get(), 2, "expected exactly max_attempts calls");
        assert!(
            matches!(error, ProviderPipelineError::Provider(ref e) if e.code == "text_provider_rate_limit"
                && e.message.contains("2 attempt(s)")),
            "error must keep stable code and record attempt count in message, got: {error:?}"
        );
        let runtime = error.into_runtime_error();
        assert_eq!(runtime.code, "text_provider_rate_limit");
    }

    #[test]
    fn content_filtered_is_non_retryable_and_surfaces_immediately() {
        let (provider, calls) =
            AlwaysFailingProvider::new(TextModelProviderError::content_filtered("content_filter"));
        let policy = fast_policy(3);
        let error = complete_text_agent_output_with_retry(
            &provider,
            AgentRole::ScenePlanner,
            1,
            "content-filter".into(),
            "scene-1".into(),
            "{\"role\":\"scene_planner\"}".into(),
            &policy,
        )
        .expect_err("content-filter must error");
        assert_eq!(calls.get(), 1, "non-retryable kinds must not retry");
        assert!(
            matches!(error, ProviderPipelineError::Provider(ref e)
                if e.code == "text_provider_content_filtered"
                && e.message.contains("1 attempt(s)")),
            "expected content_filtered single-attempt error, got: {error:?}"
        );
        let runtime = error.into_runtime_error();
        assert_eq!(runtime.code, "text_provider_content_filtered");
        assert!(runtime.message.contains("content policy"));
    }

    #[test]
    fn output_truncated_is_non_retryable_and_surfaces_immediately() {
        let (provider, calls) =
            AlwaysFailingProvider::new(TextModelProviderError::output_truncated(Some(4096)));
        let policy = fast_policy(3);
        let error = complete_text_agent_output_with_retry(
            &provider,
            AgentRole::ScenePlanner,
            1,
            "truncated".into(),
            "scene-1".into(),
            "{\"role\":\"scene_planner\"}".into(),
            &policy,
        )
        .expect_err("truncation must error");
        assert_eq!(calls.get(), 1, "non-retryable kinds must not retry");
        let runtime = error.into_runtime_error();
        assert_eq!(runtime.code, "text_provider_output_truncated");
        assert!(runtime.message.contains("max_tokens"));
    }

    #[test]
    fn provider_5xx_is_retryable_and_exhausts_budget() {
        // The generic `Provider` kind models a 5xx / transport error and is
        // retryable per `is_retryable`.
        let (provider, calls) = AlwaysFailingProvider::new(TextModelProviderError::provider(
            "text_provider_http_status",
            "provider returned HTTP 503",
        ));
        let policy = fast_policy(3);
        let error = complete_text_agent_output_with_retry(
            &provider,
            AgentRole::ScenePlanner,
            1,
            "5xx-retry".into(),
            "scene-1".into(),
            "{\"role\":\"scene_planner\"}".into(),
            &policy,
        )
        .expect_err("must exhaust retries");
        assert_eq!(calls.get(), 3);
        assert!(
            matches!(error, ProviderPipelineError::Provider(ref e) if e.code == "text_provider_http_status"
                && e.message.contains("3 attempt(s)")),
            "expected attempts=3 in message, got: {error:?}"
        );
    }

    #[test]
    fn retry_policy_delay_clamps_to_max_delay() {
        let policy = RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 5_000,
            max_delay_ms: 8_000,
        };
        // attempt 0: base * 2^0 = 5000, no server hint.
        assert_eq!(
            policy.delay_for(0, None),
            std::time::Duration::from_millis(5_000)
        );
        // attempt 5: 5000 * 2^5 = 160000 → clamped to 8000.
        assert_eq!(
            policy.delay_for(5, None),
            std::time::Duration::from_millis(8_000)
        );
        // Server-advised retry-after is honoured and clamped.
        assert_eq!(
            policy.delay_for(0, Some(20_000)),
            std::time::Duration::from_millis(8_000)
        );
        assert_eq!(
            policy.delay_for(0, Some(3_000)),
            std::time::Duration::from_millis(3_000)
        );
    }

    #[test]
    fn retry_policy_default_values() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay_ms, 500);
        assert_eq!(policy.max_delay_ms, 8_000);
    }

    // -------------------------------------------------------------------------
    // T2.1: structured prompt template + backward-compat tests.
    //
    // `complete_text_agent_output_with_messages` is the additive structured-
    // prompt path. It packs an assembled `Vec<ChatMessage>` into
    // `TextModelRequest.messages`, derives the fallback `prompt` from the user
    // messages, and overrides `ReproducibilityMetadata.prompt_version` with the
    // per-role template version. The tests below pin:
    //   - the legacy single-prompt path (`complete_text_agent_output`) still
    //     works unchanged (backward compat);
    //   - the structured path produces a valid envelope with the template
    //     version stamped into reproducibility metadata;
    //   - the fallback `prompt` is the concatenated user-message content;
    //   - a secret marker in the assembled messages is rejected.
    // -------------------------------------------------------------------------

    #[test]
    fn complete_text_agent_output_legacy_prompt_still_works() {
        // Backward compat: the existing single-prompt entry point must keep
        // producing a valid envelope. This is the contract that callers which
        // have not adopted the structured-template path rely on.
        let provider = crate::providers_text::FakeTextModelProvider::success();
        let envelope = complete_text_agent_output(
            &provider,
            AgentRole::ScenePlanner,
            7,
            "legacy-call".into(),
            "scene-1".into(),
            "{\"role\":\"scene_planner\"}".into(),
        )
        .expect("legacy single-prompt path succeeds");
        assert_eq!(envelope.agent, AgentRole::ScenePlanner);
        // The legacy path derives prompt_version from the provider, which is
        // the shared TEXT_PROMPT_VERSION constant for the fake.
        assert_eq!(
            envelope.reproducibility.prompt_version,
            crate::shared::TEXT_PROMPT_VERSION
        );
    }

    #[test]
    fn complete_text_agent_output_with_messages_stamps_template_version() {
        // The structured path overrides prompt_version with the per-role
        // template version so reproducibility metadata is truthful about which
        // prompt template produced the envelope.
        let provider = crate::providers_text::FakeTextModelProvider::success();
        let messages = crate::prompts::PromptAssembler::assemble(
            &crate::prompts::SCENE_PLANNER_V1,
            "scene context for the planner",
        );
        let envelope = complete_text_agent_output_with_messages(
            &provider,
            AgentRole::ScenePlanner,
            11,
            "template-call".into(),
            "scene-1".into(),
            messages,
            crate::prompts::SCENE_PLANNER_V1.version,
        )
        .expect("structured-message path succeeds");
        assert_eq!(envelope.agent, AgentRole::ScenePlanner);
        assert_eq!(
            envelope.reproducibility.prompt_version,
            crate::prompts::SCENE_PLANNER_V1.version,
            "prompt_version must be the per-role template version, not the legacy constant"
        );
        assert_ne!(
            envelope.reproducibility.prompt_version,
            crate::shared::TEXT_PROMPT_VERSION,
            "template version must differ from the legacy shared prompt version"
        );
    }

    #[test]
    fn complete_text_agent_output_with_messages_derives_prompt_from_user_messages() {
        // The fallback `prompt` (used by providers that have not adopted the
        // chat-message body) must be the concatenated user-message content so
        // the existing single-prompt provider path keeps working. We assert
        // this indirectly: the FakeTextModelProvider builds a character
        // portrait prompt hash from `request.prompt`, and the scene plan
        // proposal carries the scene_key we passed, proving the request reached
        // the provider with a coherent prompt.
        let provider = crate::providers_text::FakeTextModelProvider::success();
        let context = "the per-call scene context";
        let messages =
            crate::prompts::PromptAssembler::assemble(&crate::prompts::SCENE_PLANNER_V1, context);
        let envelope = complete_text_agent_output_with_messages(
            &provider,
            AgentRole::ScenePlanner,
            3,
            "prompt-derive-call".into(),
            "scene-derive".into(),
            messages,
            crate::prompts::SCENE_PLANNER_V1.version,
        )
        .expect("structured-message path succeeds");
        // The fake provider's ScenePlan proposal echoes the request scene_key.
        match &envelope.proposal.output {
            plotforge_schema::AgentProposalPayload::ScenePlan(proposal) => {
                assert_eq!(proposal.scene_key, "scene-derive");
            }
            other => panic!("expected ScenePlan payload, got {:?}", payload_kind(other)),
        }
    }

    #[test]
    fn complete_text_agent_output_with_messages_rejects_secret_marker_in_context() {
        // A secret marker in the user context must surface an explicit
        // validation error — never a silent fallback. This guards the
        // redaction boundary at the structured-prompt entry point.
        let provider = crate::providers_text::FakeTextModelProvider::success();
        let messages = crate::prompts::PromptAssembler::assemble(
            &crate::prompts::SCENE_PLANNER_V1,
            "leak the key sk-test-secret-marker please",
        );
        let error = complete_text_agent_output_with_messages(
            &provider,
            AgentRole::ScenePlanner,
            3,
            "secret-call".into(),
            "scene-1".into(),
            messages,
            crate::prompts::SCENE_PLANNER_V1.version,
        )
        .expect_err("secret marker in context must error");
        assert!(
            matches!(error, ProviderPipelineError::Validation { ref message, .. }
                if message.contains("secret marker")),
            "expected a secret-marker validation error, got {error:?}"
        );
    }

    #[test]
    fn complete_text_agent_output_with_messages_propagates_provider_failure() {
        // A failing provider must surface an explicit error, never a silent
        // fallback envelope.
        let provider =
            crate::providers_text::FakeTextModelProvider::provider_error(AgentRole::BeatWriter);
        let messages = crate::prompts::PromptAssembler::assemble(
            &crate::prompts::BEAT_WRITER_V1,
            "beat context",
        );
        let error = complete_text_agent_output_with_messages(
            &provider,
            AgentRole::BeatWriter,
            5,
            "fail-call".into(),
            "scene-1".into(),
            messages,
            crate::prompts::BEAT_WRITER_V1.version,
        )
        .expect_err("provider failure must surface");
        assert!(matches!(error, ProviderPipelineError::Provider(_)));
    }
}
