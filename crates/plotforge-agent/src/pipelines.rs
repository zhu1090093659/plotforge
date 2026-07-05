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
    if contains_secret_marker_text(&prompt) {
        return Err(ProviderPipelineError::Validation {
            agent,
            message: "text generation prompt contained a secret marker".into(),
        });
    }

    let reproducibility = provider.reproducibility_metadata(run_seed);
    let model_request = crate::providers_text::TextModelRequest {
        call_id,
        agent: agent.clone(),
        scene_key,
        run_seed: reproducibility.run_seed,
        prompt_version: reproducibility.prompt_version.clone(),
        model_version: reproducibility.model_version.clone(),
        provider_config_hash: reproducibility.provider_config_hash.clone(),
        prompt,
    };
    let response = provider
        .complete(&model_request)
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
