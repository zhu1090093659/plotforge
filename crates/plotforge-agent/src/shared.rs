//! Shared infrastructure for the agent crate.
//!
//! This module holds cross-cutting helpers and constants that are reused by
//! the text provider, image provider, TTS provider, pipeline, and scene
//! planner submodules. It contains no domain logic, so it sits at the bottom
//! of the module dependency graph.

use plotforge_media::{AssetRecordInput, AssetRegistry, MediaError};
use plotforge_schema::AgentProposalPayload;
use reqwest::header::HeaderValue;
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

/// Parses an HTTP `Retry-After` header value into milliseconds. Supports both
/// the delta-seconds form (`"120"`) and the HTTP-date form
/// (`"Wed, 21 Oct 2026 07:28:00 GMT"`). Returns `None` when the header is
/// absent or unparseable — the caller then falls back to its own backoff.
/// The parsed value is redaction-safe (a duration, not user content).
pub(crate) fn parse_retry_after(header: Option<&HeaderValue>) -> Option<u64> {
    let value = header?;
    let value = value.to_str().ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Delta-seconds form: a non-negative integer.
    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Some(seconds.saturating_mul(1000));
    }
    let date = httpdate_to_system_time(trimmed)?;
    let now = std::time::SystemTime::now();
    match date.duration_since(now) {
        Ok(duration) => Some(duration.as_millis().try_into().ok()?),
        // A past date means "retry now"; surface a zero delay so the retry loop
        // honours the header but does not stall.
        Err(_) => Some(0),
    }
}

/// Parses the IMF-fixdate form providers use for HTTP `Retry-After` dates
/// (`Wed, 21 Oct 2026 07:28:00 GMT`) into a `SystemTime` without introducing
/// a date/time dependency. Unsupported or malformed forms return `None`.
pub(crate) fn httpdate_to_system_time(value: &str) -> Option<std::time::SystemTime> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() != 6 {
        return None;
    }
    // parts: [weekday, day, month, year, time, "GMT"]
    let day: u32 = parts[1].parse().ok()?;
    let month = month_index(parts[2])?;
    let year: i32 = parts[3].parse().ok()?;
    let time_parts: Vec<&str> = parts[4].split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour: u32 = time_parts[0].parse().ok()?;
    let minute: u32 = time_parts[1].parse().ok()?;
    let second: u32 = time_parts[2].parse().ok()?;
    if parts[5] != "GMT" {
        return None;
    }
    let epoch_seconds = days_from_civil(year, month, day)? as i64 * 86_400
        + (hour as i64 * 3600)
        + (minute as i64 * 60)
        + second as i64;
    let duration = std::time::Duration::from_secs(epoch_seconds.max(0) as u64);
    Some(std::time::SystemTime::UNIX_EPOCH + duration)
}

pub(crate) fn month_index(name: &str) -> Option<u32> {
    match name {
        "Jan" => Some(1),
        "Feb" => Some(2),
        "Mar" => Some(3),
        "Apr" => Some(4),
        "May" => Some(5),
        "Jun" => Some(6),
        "Jul" => Some(7),
        "Aug" => Some(8),
        "Sep" => Some(9),
        "Oct" => Some(10),
        "Nov" => Some(11),
        "Dec" => Some(12),
        _ => None,
    }
}

/// Howard Hinnant's days-from-civil algorithm. Returns `None` for an invalid
/// month (1-12). Produces the count of days since 1970-01-01 for the given
/// (year, month, day), supporting the HTTP-date epoch conversion above.
pub(crate) fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m = month as i32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe as i64 * 365 + yoe as i64 / 4 - yoe as i64 / 100 + doy as i64;
    Some(era as i64 * 146_097 + doe - 719_468)
}

pub fn payload_kind(output: &AgentProposalPayload) -> &'static str {
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

pub fn stable_sha256_hash(value: &str) -> String {
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
