//! `plotforge-agent` crate root.
//!
//! The crate is split into focused submodules (text/image/TTS providers, the
//! scene planner, the text-output pipeline, validation, and shared helpers).
//! This file only declares the modules and re-exports the public API so the
//! crate surface stays identical to the pre-split monolith: every symbol that
//! used to live here is still reachable as `plotforge_agent::…` with the same
//! name and signature. The `pi_agent` facade is additive and lives in its own
//! module.

mod pi_agent;
mod pipelines;
mod providers_image;
mod providers_text;
mod providers_tts;
mod scene_planner;
mod shared;
mod validation;

// Re-export the public API so downstream consumers (and the crate's own
// `crate::` paths used by the pi-Agent facade) keep resolving at the crate
// root with the same names.
pub use pi_agent::{PiAgent, PiAgentError, pi_agent_capabilities};
pub use pipelines::{
    generate_character, generate_character_with_provider, generate_story_craft,
    generate_story_craft_with_provider, generate_world_expansion,
    generate_world_expansion_with_provider,
};
pub use providers_image::{
    FakeImageProvider, ImageGenerationRequest, ImageGenerationResponse, ImageProvider,
    ImageProviderError, ImageProviderErrorKind, SceneImagePipeline, SceneImagePipelineError,
    SceneImageRequest, SceneImageResult,
};
pub use providers_text::{
    ConfiguredTextModelProvider, EnvCredentialResolver, FakeTextModelProvider,
    ProviderCredentialError, ProviderCredentialResolver, TextModelClient, TextModelClientRequest,
    TextModelProvider, TextModelProviderError, TextModelProviderErrorKind, TextModelRequest,
    TextModelResponse, TextProviderConfig, TextProviderConfigError,
};
pub use providers_tts::{
    FakeTtsProvider, TtsPipeline, TtsPipelineError, TtsPipelineResult, TtsProvider,
    TtsProviderError, TtsProviderErrorKind, TtsProviderOutput, TtsRequest, TtsTarget,
};
pub use scene_planner::{
    ImageProviderAgentPipeline, MockAgentPipeline, ProviderAgentPipeline, ScenePlan,
    ScenePlanRequest, ScenePlanner, ScenePlannerError,
};
pub use validation::{
    AgentProposalValidationError, scene_from_proposals, validate_agent_output_proposal,
    validate_beat_drafts_proposal, validate_character_proposal, validate_review_proposal,
    validate_scene_plan_proposal, validate_scene_proposal_parts,
    validate_story_craft_plan_proposal, validate_world_expansion_proposal,
};

// Crate-internal re-exports so the pi-Agent facade can keep referencing the
// shared text-output pipeline, the payload-kind helper, and the redaction
// helper through stable `crate::` paths. These do not widen the public API.
pub(crate) use pipelines::complete_text_agent_output;
pub(crate) use plotforge_schema::contains_secret_marker_text;
pub(crate) use shared::payload_kind;

// Private binding so the inline test module's `use super::{fake_success_response}`
// keeps resolving after the helper moved to the text provider module. Tests are
// the only consumer of this private path; it must not become public.
#[cfg(test)]
use providers_text::fake_success_response;

#[cfg(test)]
mod tests;
