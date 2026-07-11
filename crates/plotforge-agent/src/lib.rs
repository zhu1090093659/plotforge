//! `plotforge-agent` crate root.
//!
//! The crate is split into focused submodules (text/image/TTS providers, the
//! scene planner, the text-output pipeline, validation, and shared helpers).
//! This file only declares the modules and re-exports the public API so the
//! crate surface stays identical to the pre-split monolith: every symbol that
//! used to live here is still reachable as `plotforge_agent::…` with the same
//! name and signature. The `pi_agent` facade is additive and lives in its own
//! module.

mod mcp_loop;
mod moderation_loop;
mod pi_agent;
mod pipelines;
mod prompts;
mod providers_http;
mod providers_image;
mod providers_moderation;
mod providers_text;
mod providers_tts;
mod registry;
mod scene_planner;
mod shared;
mod skills;
mod throttle;
mod usage_reporter;
mod validation;

// Re-export the public API so downstream consumers (and the crate's own
// `crate::` paths used by the pi-Agent facade) keep resolving at the crate
// root with the same names.
pub use mcp_loop::{
    McpLoopError, complete_with_mcp_tools, complete_with_mcp_tools_with_usage_reporter,
    parse_mcp_tool_calls,
};
pub use pi_agent::{PiAgent, PiAgentError, pi_agent_capabilities, pi_agent_capabilities_with_mcp};
pub use pipelines::{
    generate_character, generate_character_with_provider, generate_story_craft,
    generate_story_craft_with_provider, generate_world_expansion,
    generate_world_expansion_with_provider,
};
pub use providers_http::{
    AnthropicMessagesClient, OpenAiCompatibleClient, OpenAiResponsesClient, shared_blocking_client,
};
pub use providers_image::{
    FakeImageProvider, ImageGenerationRequest, ImageGenerationResponse, ImageProvider,
    ImageProviderError, ImageProviderErrorKind, OpenAiImageClient, SceneImagePipeline,
    SceneImagePipelineError, SceneImageRequest, SceneImageResult,
};
pub use providers_moderation::{
    FakeModerationProvider, ModerationProvider, ModerationProviderError,
    ModerationProviderErrorKind, ModerationRequest, ModerationResponse, OpenAiModerationClient,
};
pub use providers_text::{
    ConfiguredTextModelProvider, EnvCredentialResolver, FakeTextModelProvider,
    ProviderCredentialError, ProviderCredentialResolver, TextModelClient, TextModelClientRequest,
    TextModelProvider, TextModelProviderError, TextModelProviderErrorKind, TextModelRequest,
    TextModelResponse, TextProviderConfig, TextProviderConfigError,
};
// Public re-exports of the prompt template system. `ChatMessage` and
// `MessageRole` are also embedded in the additive `TextModelRequest.messages`
// field, so downstream consumers build messages through these types.
pub use prompts::{
    AssembledContext, BEAT_WRITER_V1, ChatMessage, ContextBudget, MessageRole, PLOT_DOCTOR_V1,
    ProjectContext, PromptAssembler, PromptTemplate, SCENE_PLANNER_V1, assemble_context,
    estimate_tokens, prompt_version_for_role,
};
pub use providers_tts::{
    FakeTtsProvider, OpenAiTtsClient, TtsPipeline, TtsPipelineError, TtsPipelineResult,
    TtsProvider, TtsProviderError, TtsProviderErrorKind, TtsProviderOutput, TtsRequest, TtsTarget,
};
pub use registry::{
    LOCAL_PI_MODEL_ID, ModelDiscoveryError, OptionalEnvCredentialResolver, ProviderBuildError,
    ProviderRegistryError, build_image_provider, build_moderation_provider, build_provider_client,
    build_text_provider, build_tts_provider, fetch_provider_models, fetch_provider_models_to,
    load_provider_registry, load_provider_registry_from, local_pi_provider, moderation_config_hash,
    provider_registry_path, resolve_image_provider, resolve_moderation_provider,
    resolve_provider_for_model, resolve_tts_provider, user_config_dir, write_provider_registry,
    write_provider_registry_to,
};
pub use scene_planner::{
    ImageProviderAgentPipeline, MockAgentPipeline, ProviderAgentPipeline, ScenePlan,
    ScenePlanRequest, ScenePlanner, ScenePlannerError,
};
pub use skills::{
    SkillError, discover_skill_roots, import_external_skill, import_external_skill_to,
    load_skill_body, parse_skill_frontmatter, parse_skill_interface, plot_forge_user_dir,
    read_cached_skill_index, read_skill_index_from, scan_all_skills, scan_skill, skill_index_path,
    write_skill_index, write_skill_index_to,
};
pub use usage_reporter::{ProviderUsageIdentity, UsageReporter};
pub use validation::{
    AgentProposalValidationError, scene_from_proposals, validate_agent_output_proposal,
    validate_beat_drafts_proposal, validate_character_proposal, validate_review_proposal,
    validate_scene_plan_proposal, validate_scene_proposal_parts,
    validate_story_craft_plan_proposal, validate_world_expansion_proposal,
};

// Crate-internal re-exports so the pi-Agent facade can keep referencing the
// shared text-output pipeline, the payload-kind helper, and the redaction
// helper through stable `crate::` paths. These do not widen the public API.
#[allow(unused_imports)] // RetryPolicy + retry variant are test-facing only
pub(crate) use pipelines::{
    RetryPolicy, complete_text_agent_output, complete_text_agent_output_with_messages,
    complete_text_agent_output_with_retry,
};
pub(crate) use plotforge_schema::contains_secret_marker_text;

/// Public re-export of the payload-kind helper so the Studio layer can name
/// an envelope's payload kind in redaction-safe error messages.
pub use shared::payload_kind;
/// Public re-export of the stable sha256 helper so the Studio layer can
/// derive redaction-safe prompt hashes without re-implementing the hash.
pub use shared::stable_sha256_hash;

// Private binding so the inline test module's `use super::{fake_success_response}`
// keeps resolving after the helper moved to the text provider module. Tests are
// the only consumer of this private path; it must not become public.
#[cfg(test)]
use providers_text::fake_success_response;

#[cfg(test)]
mod tests;
