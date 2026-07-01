//! Shared infrastructure for the agent crate.
//!
//! This module holds cross-cutting helpers and constants that are reused by
//! the text provider, image provider, TTS provider, pipeline, and scene
//! planner submodules. It contains no domain logic and depends only on the
//! schema and hashing crates, so it sits at the bottom of the module
//! dependency graph.

use plotforge_media::{AssetRecordInput, AssetRegistry, MediaError};
use plotforge_schema::AgentProposalPayload;
use sha2::{Digest, Sha256};
use std::path::Path;

pub(crate) const IMAGE_JOB_TIMEOUT_MS: u64 = 60_000;
pub(crate) const IMAGE_JOB_MAX_ATTEMPTS: u32 = 2;
pub(crate) const IMAGE_JOB_ESTIMATED_COST_UNITS: u64 = 1;
pub(crate) const TTS_JOB_TIMEOUT_MS: u64 = 30_000;
pub(crate) const TTS_JOB_MAX_ATTEMPTS: u32 = 2;
pub(crate) const TTS_JOB_ESTIMATED_COST_UNITS: u64 = 1;
pub(crate) const SCENE_BACKGROUND_SLOT: &str = "background_asset";
pub(crate) const TEXT_PROMPT_VERSION: &str = "plotforge-agent-text-prompt-v1";
pub(crate) const FAKE_TEXT_MODEL_VERSION: &str = "fake-text-model-v1";
pub(crate) const FAKE_TEXT_PROVIDER_CONFIG_HASH: &str = "sha256:fake-text-provider-config-v1";

pub(crate) fn payload_kind(output: &AgentProposalPayload) -> &'static str {
    match output {
        AgentProposalPayload::WorldExpansion(_) => "world_expansion",
        AgentProposalPayload::StoryCraftPlan(_) => "story_craft_plan",
        AgentProposalPayload::CharacterProfile(_) => "character_profile",
        AgentProposalPayload::ScenePlan(_) => "scene_plan",
        AgentProposalPayload::BeatDrafts(_) => "beat_drafts",
        AgentProposalPayload::Review(_) => "review",
    }
}

pub(crate) fn stable_prompt_hash(prompt: &str) -> String {
    stable_sha256_hash(prompt)
}

pub(crate) fn stable_sha256_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn choice_input_terms(action_type: &str) -> Vec<String> {
    let terms: &[&str] = match action_type {
        "continue" => &["continue", "hear", "minister", "听", "继续", "陈情"],
        "raise_tax" => &["raise", "tax", "levy", "加征", "港税"],
        "inspect_corruption" => &["inspect", "corruption", "严查", "贪墨", "查"],
        "pay_army" => &["pay", "army", "军饷", "拨", "内帑", "边军"],
        _ => &[],
    };
    terms.iter().map(|term| (*term).to_string()).collect()
}

pub(crate) fn insert_media_bytes(
    registry: &mut AssetRegistry,
    project_root: Option<&Path>,
    input: AssetRecordInput,
    bytes: &[u8],
) -> Result<String, MediaError> {
    match project_root {
        Some(project_root) => registry.insert_project_bytes(project_root, input, bytes),
        None => registry.insert_bytes(input, bytes),
    }
}
