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

use crate::{
    ReproducibilityMetadata, RuntimeSnapshot, RuntimeTrace, Scene, TurnUsageSummary, UsageInfo,
};

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
/// credentials, or secret markers. `usage` carries redaction-safe provider
/// token counts when available and stays absent for local or legacy runs.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PiAgentRunResult {
    pub descriptor: PiAgentDescriptor,
    pub reproducibility: ReproducibilityMetadata,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageInfo>,
    pub trace_id: Option<String>,
    pub evidence_summary: String,
}

/// A request to run the pi-Agent and commit its proposal as a runtime state
/// change. Unlike `PiAgentRunRequest` (redaction-safe, evidence-only), this
/// request carries the project path and the player's verbatim input because
/// the Studio layer must load the project, run rule evaluation, and commit
/// the agent's `ScenePlan` payload into runtime state. The request stays
/// local-only: it is never serialized into traces, export packages, or
/// contract bundles beyond this schema definition.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PiAgentApplyRequest {
    pub agent_id: String,
    pub run_seed: u64,
    pub project_path: String,
    pub player_input: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_id: Option<String>,
}

/// Redaction-safe moderation result exposed to Studio consumers. Categories
/// are provider-normalized labels only; raw provider responses never enter
/// this contract.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModerationOutcomeSummary {
    pub flagged: bool,
    pub categories: Vec<String>,
}

/// The result of a `pi_agent_apply_run`: the redaction-safe pi-Agent evidence
/// envelope plus the runtime-visible outcome (committed scene + trace + optional
/// snapshot). This embeds the same shape `AgentChatRail` renders for a playtest
/// turn (`scene` + `trace`), so the rail can drive `pi_agent_apply_run` without
/// a separate `PlayOnceReport` (which lives in the Studio layer, not in
/// schema). `trace_path` and `snapshot_path` are absolute paths written by the
/// Studio layer; they never enter export packages.
/// `usage` mirrors `run.usage` for direct consumers of the apply result.
///
/// `image_generation_failed` carries a redaction-safe error message when the
/// optional scene background image generation step failed but the turn still
/// succeeded (failure isolation: an image provider outage never fails the
/// turn). It is `None` when no image provider was configured, when image
/// generation was skipped (e.g. a cached asset was reused), or when it
/// succeeded. The message is redaction-safe (no credentials, no raw provider
/// response bodies) and trace-visible so the user can diagnose the outage
/// rather than seeing a silent missing image. Per AGENTS.md: no silent
/// fallback — the field makes the image failure explicit even though the turn
/// succeeds.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PiAgentApplyResult {
    pub run: PiAgentRunResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageInfo>,
    /// Request-scoped totals reported by moderation, text/MCP, and image
    /// providers during this apply turn. Absent for legacy payloads and fully
    /// local turns with no provider usage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_usage: Option<TurnUsageSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moderation_outcome: Option<ModerationOutcomeSummary>,
    pub scene_key: String,
    pub scene: Scene,
    pub trace: RuntimeTrace,
    pub trace_path: String,
    /// The same `delta_summary` list `PlayOnceReport` carries, so the rail
    /// and `TraceDebugView` can render the "State deltas" count chip and
    /// summary without re-implementing `summarize_delta` in TypeScript
    /// (forbidden by AGENTS.md). Populated by the Studio layer from
    /// `summarize_delta(&trace.world_state_delta)`.
    #[serde(default)]
    pub delta_summary: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<RuntimeSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_path: Option<String>,
    /// Redaction-safe image-generation failure message. `None` when image
    /// generation was not attempted, was skipped (cache hit), or succeeded.
    /// `Some(message)` when image generation failed but the turn still
    /// succeeded — the failure is explicit and trace-visible, never silent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_generation_failed: Option<String>,
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
            usage: None,
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
            usage: None,
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

    fn sample_descriptor() -> PiAgentDescriptor {
        PiAgentDescriptor {
            agent_id: "pi-agent-local".into(),
            is_local_pi: true,
            capabilities: vec![PiAgentCapability {
                id: "pi-agent.text-generation".into(),
                label: "Text generation".into(),
                status: "wired".into(),
                source: "local-mock-text-provider".into(),
                evidence: "FakeTextModelProvider envelope".into(),
            }],
        }
    }

    #[test]
    fn pi_agent_run_result_backward_compatible_without_usage() {
        let legacy = PiAgentRunResult {
            descriptor: sample_descriptor(),
            reproducibility: ReproducibilityMetadata::local_mock(7),
            usage: None,
            trace_id: None,
            evidence_summary: "redacted".into(),
        };
        let value = serde_json::to_value(&legacy).expect("serialize legacy run result");
        assert!(value.get("usage").is_none());

        let decoded: PiAgentRunResult =
            serde_json::from_value(value).expect("deserialize legacy run result");

        assert_eq!(decoded.usage, None);
    }

    #[test]
    fn pi_agent_apply_request_roundtrips_json() {
        let request = PiAgentApplyRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 11,
            project_path: "/tmp/project".into(),
            player_input: "Take the witness stand.".into(),
            save_id: Some("snap-001".into()),
            restore_id: None,
        };
        let encoded = serde_json::to_string_pretty(&request).expect("serialize apply request");
        let decoded: PiAgentApplyRequest =
            serde_json::from_str(&encoded).expect("deserialize apply request");
        assert_eq!(decoded, request);
    }

    #[test]
    fn pi_agent_apply_request_rejects_secret_fields() {
        let request = PiAgentApplyRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 11,
            project_path: "/tmp/project".into(),
            player_input: "benign".into(),
            save_id: None,
            restore_id: None,
        };
        let mut value = serde_json::to_value(&request).expect("apply request value");
        value["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<PiAgentApplyRequest>(value)
            .expect_err("api_key field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    fn sample_apply_result() -> PiAgentApplyResult {
        let run = PiAgentRunResult {
            descriptor: sample_descriptor(),
            reproducibility: ReproducibilityMetadata::local_mock(11),
            usage: None,
            trace_id: Some("pi-agent-trace-011".into()),
            evidence_summary: "pi-Agent committed a scene plan proposal.".into(),
        };
        let scene = Scene {
            key: "provider-scene-011".into(),
            title: "Witness Stand".into(),
            location: "Courthouse".into(),
            dramatic_purpose: "Pressure the witness".into(),
            hook: "A crack in the testimony".into(),
            background_asset: String::new(),
            audio_refs: Vec::new(),
            character_ids: Vec::new(),
            plot_thread_updates: Default::default(),
            beats: Vec::new(),
            entry_beat_id: None,
        };
        let trace = RuntimeTrace {
            id: "trace-011".into(),
            timestamp_ms: 11,
            reproducibility: ReproducibilityMetadata::local_mock(11),
            player_input: Some("redacted".into()),
            selected_choice: None,
            action_intent: None,
            rule_result: None,
            planner_result: None,
            diagnostics: Vec::new(),
            world_state_before: crate::WorldState {
                resources: Default::default(),
                flags: Default::default(),
                triggered_events: Vec::new(),
            },
            world_state_delta: crate::WorldDelta {
                resource_changes: Default::default(),
                resource_sets: Default::default(),
                flags: Default::default(),
                triggered_events: Vec::new(),
            },
            world_state_after: crate::WorldState {
                resources: Default::default(),
                flags: Default::default(),
                triggered_events: Vec::new(),
            },
            story_state_before: crate::StoryState {
                current_scene_key: "scene-001".into(),
                current_beat_id: None,
                completed_scene_keys: Vec::new(),
                turn: 10,
            },
            story_state_after: crate::StoryState {
                current_scene_key: "provider-scene-011".into(),
                current_beat_id: None,
                completed_scene_keys: vec!["scene-001".into()],
                turn: 11,
            },
            narrative_review: None,
            media_references: Vec::new(),
            errors: Vec::new(),
            fallback_used: false,
        };
        PiAgentApplyResult {
            run: run.clone(),
            usage: None,
            turn_usage: None,
            moderation_outcome: None,
            scene_key: scene.key.clone(),
            scene: scene.clone(),
            trace: trace.clone(),
            trace_path: "/tmp/project/traces/trace-011.json".into(),
            delta_summary: Vec::new(),
            snapshot: None,
            snapshot_path: None,
            image_generation_failed: None,
        }
    }

    #[test]
    fn pi_agent_apply_result_roundtrips_with_runtime_shapes() {
        let result = sample_apply_result();

        let encoded = serde_json::to_string_pretty(&result).expect("serialize apply result");
        let decoded: PiAgentApplyResult =
            serde_json::from_str(&encoded).expect("deserialize apply result");
        assert_eq!(decoded, result);
        assert_eq!(decoded.scene.title, "Witness Stand");
        assert_eq!(decoded.trace.id, "trace-011");

        let value: serde_json::Value = serde_json::from_str(&encoded).expect("apply result value");
        assert!(value.get("raw_provider_response").is_none());
        assert!(value.get("api_key").is_none());
    }

    #[test]
    fn pi_agent_apply_result_backward_compatible_without_usage() {
        let legacy = sample_apply_result();
        let value = serde_json::to_value(&legacy).expect("serialize legacy apply result");
        assert!(value.get("usage").is_none());
        assert!(value["run"].get("usage").is_none());

        let decoded: PiAgentApplyResult =
            serde_json::from_value(value).expect("deserialize legacy apply result");

        assert_eq!(decoded.usage, None);
        assert_eq!(decoded.run.usage, None);
    }

    #[test]
    fn pi_agent_apply_result_roundtrips_turn_usage_and_defaults_when_absent() {
        let mut result = sample_apply_result();
        result.turn_usage = Some(TurnUsageSummary {
            total_input_tokens: 31,
            total_output_tokens: 9,
            total_spent_cost_units: 17,
        });

        let encoded = serde_json::to_value(&result).expect("serialize turn usage");
        let decoded: PiAgentApplyResult =
            serde_json::from_value(encoded.clone()).expect("deserialize turn usage");
        assert_eq!(decoded, result);

        let mut legacy = encoded;
        legacy
            .as_object_mut()
            .expect("apply result object")
            .remove("turn_usage");
        let decoded_legacy: PiAgentApplyResult =
            serde_json::from_value(legacy).expect("deserialize legacy apply result");
        assert_eq!(decoded_legacy.turn_usage, None);
    }

    #[test]
    fn pi_agent_apply_result_rejects_raw_provider_fields() {
        let result = PiAgentApplyResult {
            run: PiAgentRunResult {
                descriptor: sample_descriptor(),
                reproducibility: ReproducibilityMetadata::local_mock(11),
                usage: None,
                trace_id: None,
                evidence_summary: "redacted".into(),
            },
            usage: None,
            turn_usage: None,
            moderation_outcome: None,
            scene_key: "provider-scene-011".into(),
            scene: Scene {
                key: "provider-scene-011".into(),
                title: "Witness Stand".into(),
                location: "Courthouse".into(),
                dramatic_purpose: "Pressure".into(),
                hook: "A crack".into(),
                background_asset: String::new(),
                audio_refs: Vec::new(),
                character_ids: Vec::new(),
                plot_thread_updates: Default::default(),
                beats: Vec::new(),
                entry_beat_id: None,
            },
            trace: RuntimeTrace {
                id: "trace-011".into(),
                timestamp_ms: 11,
                reproducibility: ReproducibilityMetadata::local_mock(11),
                player_input: None,
                selected_choice: None,
                action_intent: None,
                rule_result: None,
                planner_result: None,
                diagnostics: Vec::new(),
                world_state_before: crate::WorldState {
                    resources: Default::default(),
                    flags: Default::default(),
                    triggered_events: Vec::new(),
                },
                world_state_delta: crate::WorldDelta {
                    resource_changes: Default::default(),
                    resource_sets: Default::default(),
                    flags: Default::default(),
                    triggered_events: Vec::new(),
                },
                world_state_after: crate::WorldState {
                    resources: Default::default(),
                    flags: Default::default(),
                    triggered_events: Vec::new(),
                },
                story_state_before: crate::StoryState {
                    current_scene_key: "scene-001".into(),
                    current_beat_id: None,
                    completed_scene_keys: Vec::new(),
                    turn: 10,
                },
                story_state_after: crate::StoryState {
                    current_scene_key: "provider-scene-011".into(),
                    current_beat_id: None,
                    completed_scene_keys: Vec::new(),
                    turn: 11,
                },
                narrative_review: None,
                media_references: Vec::new(),
                errors: Vec::new(),
                fallback_used: false,
            },
            trace_path: "/tmp/traces/trace-011.json".into(),
            delta_summary: Vec::new(),
            snapshot: None,
            snapshot_path: None,
            image_generation_failed: None,
        };
        let mut value = serde_json::to_value(&result).expect("apply result value");
        value["raw_provider_response"] =
            serde_json::json!("raw provider body with sk-test-secret-marker");
        let error = serde_json::from_value::<PiAgentApplyResult>(value)
            .expect_err("raw provider response should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn pi_agent_apply_result_carries_image_generation_failure() {
        // The image_generation_failed field carries a redaction-safe message
        // when image gen failed but the turn succeeded. It must round-trip
        // and skip-serialize when None.
        let result = PiAgentApplyResult {
            run: PiAgentRunResult {
                descriptor: sample_descriptor(),
                reproducibility: ReproducibilityMetadata::local_mock(11),
                usage: None,
                trace_id: None,
                evidence_summary: "redacted".into(),
            },
            usage: None,
            turn_usage: None,
            moderation_outcome: None,
            scene_key: "provider-scene-011".into(),
            scene: Scene {
                key: "provider-scene-011".into(),
                title: "Witness Stand".into(),
                location: "Courthouse".into(),
                dramatic_purpose: "Pressure".into(),
                hook: "A crack".into(),
                background_asset: String::new(),
                audio_refs: Vec::new(),
                character_ids: Vec::new(),
                plot_thread_updates: Default::default(),
                beats: Vec::new(),
                entry_beat_id: None,
            },
            trace: RuntimeTrace {
                id: "trace-011".into(),
                timestamp_ms: 11,
                reproducibility: ReproducibilityMetadata::local_mock(11),
                player_input: None,
                selected_choice: None,
                action_intent: None,
                rule_result: None,
                planner_result: None,
                diagnostics: Vec::new(),
                world_state_before: crate::WorldState {
                    resources: Default::default(),
                    flags: Default::default(),
                    triggered_events: Vec::new(),
                },
                world_state_delta: crate::WorldDelta {
                    resource_changes: Default::default(),
                    resource_sets: Default::default(),
                    flags: Default::default(),
                    triggered_events: Vec::new(),
                },
                world_state_after: crate::WorldState {
                    resources: Default::default(),
                    flags: Default::default(),
                    triggered_events: Vec::new(),
                },
                story_state_before: crate::StoryState {
                    current_scene_key: "scene-001".into(),
                    current_beat_id: None,
                    completed_scene_keys: Vec::new(),
                    turn: 10,
                },
                story_state_after: crate::StoryState {
                    current_scene_key: "provider-scene-011".into(),
                    current_beat_id: None,
                    completed_scene_keys: Vec::new(),
                    turn: 11,
                },
                narrative_review: None,
                media_references: Vec::new(),
                errors: Vec::new(),
                fallback_used: false,
            },
            trace_path: "/tmp/traces/trace-011.json".into(),
            delta_summary: Vec::new(),
            snapshot: None,
            snapshot_path: None,
            image_generation_failed: Some("image provider timed out: connection refused".into()),
        };
        let encoded = serde_json::to_string_pretty(&result).expect("serialize");
        assert!(
            encoded.contains("image_generation_failed"),
            "field must serialize when Some: {encoded}"
        );
        let decoded: PiAgentApplyResult = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(decoded, result);
        assert_eq!(
            decoded.image_generation_failed.as_deref(),
            Some("image provider timed out: connection refused")
        );
    }

    #[test]
    fn pi_agent_apply_result_omits_image_generation_failed_when_none() {
        // When None, the field is skip-serialized so existing consumers that
        // never set it see no change in the wire shape.
        let mut result = PiAgentApplyResult {
            run: PiAgentRunResult {
                descriptor: sample_descriptor(),
                reproducibility: ReproducibilityMetadata::local_mock(11),
                usage: None,
                trace_id: None,
                evidence_summary: "redacted".into(),
            },
            usage: None,
            turn_usage: None,
            moderation_outcome: None,
            scene_key: "scene-1".into(),
            scene: Scene {
                key: "scene-1".into(),
                title: "T".into(),
                location: "L".into(),
                dramatic_purpose: "D".into(),
                hook: "H".into(),
                background_asset: String::new(),
                audio_refs: Vec::new(),
                character_ids: Vec::new(),
                plot_thread_updates: Default::default(),
                beats: Vec::new(),
                entry_beat_id: None,
            },
            trace: RuntimeTrace {
                id: "t".into(),
                timestamp_ms: 0,
                reproducibility: ReproducibilityMetadata::local_mock(0),
                player_input: None,
                selected_choice: None,
                action_intent: None,
                rule_result: None,
                planner_result: None,
                diagnostics: Vec::new(),
                world_state_before: crate::WorldState {
                    resources: Default::default(),
                    flags: Default::default(),
                    triggered_events: Vec::new(),
                },
                world_state_delta: crate::WorldDelta {
                    resource_changes: Default::default(),
                    resource_sets: Default::default(),
                    flags: Default::default(),
                    triggered_events: Vec::new(),
                },
                world_state_after: crate::WorldState {
                    resources: Default::default(),
                    flags: Default::default(),
                    triggered_events: Vec::new(),
                },
                story_state_before: crate::StoryState {
                    current_scene_key: "scene-1".into(),
                    current_beat_id: None,
                    completed_scene_keys: Vec::new(),
                    turn: 0,
                },
                story_state_after: crate::StoryState {
                    current_scene_key: "scene-1".into(),
                    current_beat_id: None,
                    completed_scene_keys: Vec::new(),
                    turn: 1,
                },
                narrative_review: None,
                media_references: Vec::new(),
                errors: Vec::new(),
                fallback_used: false,
            },
            trace_path: "/tmp/t.json".into(),
            delta_summary: Vec::new(),
            snapshot: None,
            snapshot_path: None,
            image_generation_failed: None,
        };
        let encoded = serde_json::to_string(&result).expect("serialize");
        assert!(
            !encoded.contains("image_generation_failed"),
            "field must be omitted when None: {encoded}"
        );
        // Backward compat: a payload without the field deserializes to None.
        let legacy: PiAgentApplyResult =
            serde_json::from_str(&encoded).expect("deserialize legacy");
        assert_eq!(legacy.image_generation_failed, None);
        result.image_generation_failed = None;
        assert_eq!(legacy, result);
    }

    #[test]
    fn moderation_outcome_summary_roundtrips_and_rejects_unknown_fields() {
        let outcome = ModerationOutcomeSummary {
            flagged: true,
            categories: vec!["violence".into(), "harassment".into()],
        };
        let encoded = serde_json::to_string(&outcome).expect("serialize moderation outcome");
        let decoded: ModerationOutcomeSummary =
            serde_json::from_str(&encoded).expect("deserialize moderation outcome");
        assert_eq!(decoded, outcome);

        let mut value = serde_json::to_value(outcome).expect("moderation outcome value");
        value["raw_response"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<ModerationOutcomeSummary>(value)
            .expect_err("unknown moderation outcome field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn pi_agent_apply_result_roundtrips_moderation_outcome() {
        let mut result = sample_apply_result();
        result.moderation_outcome = Some(ModerationOutcomeSummary {
            flagged: false,
            categories: Vec::new(),
        });

        let encoded = serde_json::to_string(&result).expect("serialize apply result");
        let decoded: PiAgentApplyResult =
            serde_json::from_str(&encoded).expect("deserialize apply result");
        assert_eq!(decoded, result);
    }

    #[test]
    fn pi_agent_apply_result_backward_compatible_without_moderation_outcome() {
        let result = sample_apply_result();
        let mut value = serde_json::to_value(result).expect("serialize legacy apply result");
        value
            .as_object_mut()
            .expect("apply result object")
            .remove("moderation_outcome");

        let decoded: PiAgentApplyResult =
            serde_json::from_value(value).expect("legacy apply result deserialize");
        assert!(decoded.moderation_outcome.is_none());
    }
}
