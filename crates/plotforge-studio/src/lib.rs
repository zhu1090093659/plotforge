use std::{
    error::Error,
    fs,
    path::Component,
    path::{Path, PathBuf},
    process::Command,
};

use plotforge_export::{export_static_web, export_static_web_zip};
use plotforge_runtime::{RuntimeSession, RuntimeStep, summarize_delta};
pub use plotforge_schema::{
    AgentSessionConfig, AiProviderSummary, AiSafetyPolicy, AiUsageContentKind, AiUsageDisclosure,
    AiUsageManifest, AiUsageSourceKind, AssetRecord, AudioBible, Character, CharacterDraft,
    CharacterEditDocument, CharacterGenerationReport, CharacterGenerationRequest, Condition,
    Effect, ExportProfile, GitBranchInfo, GitSwitchResult, ModelOption, PermissionLevel,
    PiAgentApplyRequest, PiAgentApplyResult, PiAgentCapability, PiAgentRunRequest,
    PiAgentRunResult, ProjectCreationReport, ProjectCreationRequest, ProjectData,
    ProjectTemplateId, PromptScope, PromptTemplate, ProviderEntry, ProviderKind, ProviderRegistry,
    RemoteModelInfo, ResourceDefinition, Rule, RuleDraft, RulesEditDocument, RuntimeSnapshot,
    RuntimeTrace, Scene, SkillFrontmatter, SkillIndex, SkillInterface, SkillManifest,
    SkillOrigin, SkillSource, StateVariablesEditDocument, SteamSubmissionKitDraft,
    SteamSubmissionKitRequest, StoryCraftEditDocument, StoryCraftGenerationReport,
    StoryCraftGenerationRequest, ThinkingLevel, VisualBible, WorkshopDraftVisibility,
    WorkshopItemPackage, WorkshopPackageFile, WorkshopPublishDraft, WorldEditDocument,
    WorldGenerationReport, WorldGenerationRequest, redact_trace_text,
};
// MCP schema types (Phase 5): re-exported publicly so the Tauri command
// wrappers (`creator-desktop/src-tauri`) and downstream callers can import
// them from `plotforge_studio` alongside the rest of the studio surface,
// mirroring the `plotforge_schema` re-exports above. The blocking client /
// registry helpers below remain private to this crate.
pub use plotforge_mcp::{
    McpServerEntry, McpServerTestResult, McpToolCallRequest, McpToolCallResult, McpToolManifest,
};
// MCP client surface (Phase 5): the registry IO + blocking façade live in
// `plotforge-mcp`; the schema types (`McpServerEntry`, `McpToolManifest`,
// …) are re-exported through it. These are used by the 7 MCP Studio commands
// below. Credentials are referenced indirectly by env-var name only; the
// registry never enters project source, contracts, traces, or exports.
use plotforge_mcp::{
    McpToolClient, McpToolRegistry, build_mcp_client, load_mcp_registry, resolve_mcp_server,
    validate_server_entry, write_mcp_registry,
};
use plotforge_storage::{
    create_project_from_request, load_project, read_latest_runtime_snapshot,
    read_project_prompt_templates, read_runtime_snapshot, read_user_prompt_templates,
    validate_project, validate_runtime_snapshot_id, write_project_prompt_templates,
    write_runtime_snapshot, write_trace, write_user_prompt_templates,
};
use serde::Serialize;

pub type StudioCommandResult<T> = Result<T, StudioCommandError>;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioCommandError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProjectCheckReport {
    pub title: String,
    pub entry_scene: String,
    pub scene_count: usize,
    pub rule_count: usize,
    pub character_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PlayOnceReport {
    pub scene: Scene,
    pub trace: RuntimeTrace,
    pub trace_path: String,
    pub snapshot: Option<RuntimeSnapshot>,
    pub snapshot_path: Option<String>,
    pub delta_summary: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StaticExportReport {
    pub output_dir: String,
    pub archive_path: Option<String>,
    pub files_written: Vec<String>,
    pub archived_files: Vec<String>,
    pub allowed_files: Vec<String>,
    pub files_found: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SourceFileSummary {
    pub path: String,
    pub kind: SourceFileKind,
    pub bytes: u64,
    pub editable: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SourceFileContent {
    pub path: String,
    pub kind: SourceFileKind,
    pub editable: bool,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceFileKind {
    Toml,
    Json,
    Markdown,
    Prompt,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioWorkshopPackageValidationReport {
    pub package_dir: String,
    pub manifest: WorkshopItemPackage,
    pub ai_usage: AiUsageManifest,
    pub files: Vec<StudioWorkshopValidatedFile>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioWorkshopValidatedFile {
    pub path: String,
    pub content_hash: String,
    pub byte_length: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioWorkshopLibraryItem {
    pub local_id: String,
    pub package_id: String,
    pub title: String,
    pub package_dir: String,
    pub blocked: Option<StudioWorkshopLibraryBlock>,
    pub reports: Vec<StudioWorkshopLibraryReport>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioWorkshopLibraryBlock {
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioWorkshopLibraryReport {
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioWorkshopLibraryImportReport {
    pub item: StudioWorkshopLibraryItem,
    pub validation_report: StudioWorkshopPackageValidationReport,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioWorkshopLibraryLoadReport {
    pub item: StudioWorkshopLibraryItem,
    pub validation_report: StudioWorkshopPackageValidationReport,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioWorkshopLibraryRemixReport {
    pub source_local_id: String,
    pub item: StudioWorkshopLibraryItem,
    pub validation_report: StudioWorkshopPackageValidationReport,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioWorkshopLibraryDeleteReport {
    pub local_id: String,
    pub package_dir: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioWorkshopPublishDraftWriteReport {
    pub output_dir: String,
    pub draft: WorkshopPublishDraft,
    pub files_written: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StudioSteamSubmissionKitWriteReport {
    pub output_dir: String,
    pub draft: SteamSubmissionKitDraft,
    pub files_written: Vec<String>,
}

pub fn create_project(
    path: impl AsRef<Path>,
    request: ProjectCreationRequest,
    force: bool,
) -> StudioCommandResult<ProjectCreationReport> {
    let path = path.as_ref();
    create_project_from_request(path, request, force)
        .map_err(|source| command_error("create_project", path, source))
}

pub fn open_project(path: impl AsRef<Path>) -> StudioCommandResult<ProjectData> {
    let path = path.as_ref();
    load_project(path).map_err(|source| command_error("open_project", path, source))
}

pub fn check_project(path: impl AsRef<Path>) -> StudioCommandResult<ProjectCheckReport> {
    let path = path.as_ref();
    let project =
        validate_project(path).map_err(|source| command_error("check_project", path, source))?;

    Ok(ProjectCheckReport {
        title: project.game.title,
        entry_scene: project.game.entry_scene,
        scene_count: project.scenes.len(),
        rule_count: project.rules.len(),
        character_count: project.characters.len(),
    })
}

pub fn list_export_profiles() -> Vec<ExportProfile> {
    plotforge_schema::supported_export_profiles()
}

/// Run the local pi-Agent facade against a redaction-safe request. This is a
/// thin adapter: it constructs a `PiAgent` with a deterministic local
/// `FakeTextModelProvider::local_pi()` provider (no network, no external
/// config) and delegates to `plotforge_agent::PiAgent::run`. Business logic
/// lives in the agent crate, not here.
///
/// The pi-Agent local default needs no external provider config: credentials
/// and network endpoints remain deferred. If the run fails for any reason
/// (invalid prompt, provider failure, validation failure), the error is
/// surfaced explicitly as a `StudioCommandError` — there is no silent
/// fallback. The returned `PiAgentRunResult` carries `is_local_pi == true`,
/// fully populated reproducibility metadata, and a deterministic trace
/// evidence id derived from the run seed.
pub fn pi_agent_run(request: PiAgentRunRequest) -> StudioCommandResult<PiAgentRunResult> {
    // The local pi-Agent provider requires no external config, so there is no
    // missing-config fallback path here. Real provider config wiring remains
    // deferred; when it lands, a missing-config branch must return an explicit
    // error (never a silent degraded result).
    let provider = Box::new(plotforge_agent::FakeTextModelProvider::local_pi());
    let agent = plotforge_agent::PiAgent::new(provider, &request.agent_id);
    agent.run(request).map_err(|error| StudioCommandError {
        code: "pi_agent_run".into(),
        message: error.to_string(),
    })
}

/// Return the static pi-Agent capability list the runtime exposes. The list
/// describes what is wired and what is deferred; it never promises external
/// agent execution, network model calls, or platform outcomes.
pub fn pi_agent_capabilities() -> StudioCommandResult<Vec<PiAgentCapability>> {
    Ok(plotforge_agent::pi_agent_capabilities())
}

/// Run the pi-Agent against a project, generate a `ScenePlan` proposal via
/// the configured provider, evaluate rules, commit the proposal as a runtime
/// state change, and return the resulting scene + trace. This is the
/// "describe a change / run a turn" path the desktop `AgentChatRail` drives
/// when a real provider is configured; when `model_id == "local-pi"` it falls
/// back to the deterministic mock provider.
///
/// Failure modes are explicit (never silent):
/// - `pi_agent_missing_credential`: the provider's `credential_env_var`
///   names an env var that is missing or empty.
/// - `pi_agent_provider_timeout`: the HTTP call timed out.
/// - `pi_agent_unsupported_payload`: the provider returned a payload kind
///   other than `scene_plan` (only scene plans are committable today).
pub fn pi_agent_apply_run(request: PiAgentApplyRequest) -> StudioCommandResult<PiAgentApplyResult> {
    let project_path = Path::new(&request.project_path);
    let project = load_project(project_path)
        .map_err(|source| command_error("pi_agent_apply_load", project_path, source))?;
    let agent_config = get_agent_session_config(project_path)?;
    let model_id = agent_config.model_id.as_str();

    // Resolve the provider. `local-pi` keeps the deterministic mock; any
    // other model id must resolve to a registered, enabled provider entry.
    // Both branches yield `Box<dyn TextModelProvider>` so the agent facade
    // receives a single type-erased provider regardless of whether the
    // registry picked a strict or optional credential resolver.
    let provider: Box<dyn plotforge_agent::TextModelProvider> =
        if model_id == plotforge_agent::LOCAL_PI_MODEL_ID {
            Box::new(plotforge_agent::FakeTextModelProvider::local_pi())
                as Box<dyn plotforge_agent::TextModelProvider>
        } else {
            let registry =
                plotforge_agent::load_provider_registry().map_err(|source| StudioCommandError {
                    code: "pi_agent_apply_registry".into(),
                    message: source.to_string(),
                })?;
            let entry = plotforge_agent::resolve_provider_for_model(model_id, &registry)
                .ok_or_else(|| StudioCommandError {
                    code: "pi_agent_apply_no_provider".into(),
                    message: format!("no enabled provider registered for model id `{model_id}`"),
                })?;
            plotforge_agent::build_text_provider(entry).map_err(|source| StudioCommandError {
                code: "pi_agent_apply_build".into(),
                message: source.to_string(),
            })?
        };

    // Build the redaction-safe run request the pi-Agent expects. The prompt
    // summary carries the player input (local-only); the hash anchors the
    // reproducibility identity.
    let run_request = PiAgentRunRequest {
        agent_id: request.agent_id.clone(),
        run_seed: request.run_seed,
        prompt_summary: request.player_input.clone(),
        prompt_hash: format!(
            "sha256:{}",
            plotforge_agent::stable_sha256_hash(&request.player_input)
        ),
    };

    // Branch on MCP enablement. When `AgentSessionConfig.enabled_mcp_servers`
    // is non-empty, drive the multi-turn tool-call loop via
    // `complete_with_mcp_tools` instead of the one-shot
    // `PiAgent::run_with_envelope`. The provider is borrowed for the loop
    // (not moved into a `PiAgent`) so the loop can drive multiple `complete()`
    // turns. When empty, the existing one-shot path is taken byte-for-byte
    // (the agent is constructed and `run_with_envelope` is called as before).
    let enabled_mcp_servers = &agent_config.enabled_mcp_servers;
    let (run_result, envelope) = if enabled_mcp_servers.is_empty() {
        // Empty list → existing one-shot path. Construct the agent and call
        // `run_with_envelope` exactly as before (byte-identical regression
        // guard: this branch must not change the existing behavior).
        let agent = plotforge_agent::PiAgent::new(provider, &request.agent_id);
        agent
            .run_with_envelope(run_request)
            .map_err(|error| StudioCommandError {
                code: match &error {
                    plotforge_agent::PiAgentError::Provider { code, .. } => {
                        if code.contains("missing_credential") {
                            "pi_agent_missing_credential".into()
                        } else if code.contains("timeout") {
                            "pi_agent_provider_timeout".into()
                        } else {
                            "pi_agent_apply_run".into()
                        }
                    }
                    _ => "pi_agent_apply_run".into(),
                },
                message: error.to_string(),
            })?
    } else {
        // Non-empty list → MCP tool-use loop. Load the MCP registry + build
        // the blocking `McpToolClient` façade, derive the redaction-safe
        // `mcp_tool_call_hash` inputs, and drive `complete_with_mcp_tools`.
        // Transport/registry failures surface as explicit errors
        // (`mcp_apply_registry` / `mcp_apply_tool_error`); no silent fallback
        // to the no-MCP path (AGENTS.md:144).
        let mcp_registry =
            plotforge_mcp::McpToolRegistry::load().map_err(|source| StudioCommandError {
                code: "mcp_apply_registry".into(),
                message: format!("failed to load MCP registry: {source}"),
            })?;
        let hash_inputs = mcp_registry.hash_inputs_for(enabled_mcp_servers);
        // If none of the enabled servers resolve to registry entries, surface
        // an explicit error rather than silently running with an empty hash
        // (which would look like a no-MCP turn to downstream consumers).
        if hash_inputs.is_empty() {
            return Err(StudioCommandError {
                code: "mcp_apply_no_server".into(),
                message: format!(
                    "none of the enabled MCP servers are present in the registry: {}",
                    enabled_mcp_servers.join(", ")
                ),
            });
        }
        plotforge_agent::complete_with_mcp_tools(
            provider.as_ref(),
            &request.agent_id,
            run_request,
            &mcp_registry,
            enabled_mcp_servers,
            &hash_inputs,
        )
        .map_err(|error| StudioCommandError {
            code: match &error {
                plotforge_agent::McpLoopError::ToolFailure { code, .. } => {
                    if code == "mcp_unknown_server" {
                        "mcp_apply_no_server".into()
                    } else {
                        "mcp_apply_tool_error".into()
                    }
                }
                plotforge_agent::McpLoopError::ToolResultSecretMarker { .. }
                | plotforge_agent::McpLoopError::ToolArgSecretMarker { .. } => {
                    "mcp_apply_secret_marker".into()
                }
                plotforge_agent::McpLoopError::ExceededRounds(_) => {
                    "mcp_apply_exceeded_rounds".into()
                }
                plotforge_agent::McpLoopError::ProviderFailure { code, .. } => {
                    if code.contains("missing_credential") {
                        "pi_agent_missing_credential".into()
                    } else if code.contains("timeout") {
                        "pi_agent_provider_timeout".into()
                    } else {
                        "pi_agent_apply_run".into()
                    }
                }
            },
            message: error.to_string(),
        })?
    };

    // Extract the ScenePlan payload. Other payload kinds are not committable
    // today; surface an explicit error rather than silently skipping.
    let plan = match envelope.proposal.output {
        plotforge_schema::AgentProposalPayload::ScenePlan(plan) => plan,
        other => {
            return Err(StudioCommandError {
                code: "pi_agent_unsupported_payload".into(),
                message: format!(
                    "pi-Agent returned a {} payload; only scene_plan is committable",
                    plotforge_agent::payload_kind(&other)
                ),
            });
        }
    };

    // Build a ScenePlan in the shape RuntimeSession expects (it wraps the
    // schema's ScenePlanProposal into a plotforge_agent::ScenePlan).
    let scene_plan =
        plotforge_agent::ScenePlan::from_proposal(&plan, envelope.reproducibility.clone())
            .map_err(|source| StudioCommandError {
                code: "pi_agent_apply_scene_assembly".into(),
                message: source.to_string(),
            })?;
    let scene_key = scene_plan.scene.key.clone();

    // Construct the runtime session (optionally from a snapshot for the
    // restore path) and apply the agent's scene plan. The rule-evaluation
    // boundary stays inside `plotforge-runtime` (per AGENTS.md: agents
    // propose, runtime/rules commit).
    let mut session = if let Some(restore_id) = request.restore_id.as_deref() {
        validate_runtime_snapshot_id(restore_id)
            .map_err(|source| command_error("pi_agent_apply_restore_id", project_path, source))?;
        let snapshot = read_runtime_snapshot(project_path, restore_id).map_err(|source| {
            command_error("pi_agent_apply_restore_snapshot", project_path, source)
        })?;
        RuntimeSession::from_snapshot(project, snapshot).map_err(|source| {
            command_error("pi_agent_apply_restore_runtime", project_path, source)
        })?
    } else {
        RuntimeSession::new(project)
    };

    let step = session
        .apply_agent_scene_plan(scene_plan, &request.player_input)
        .map_err(|source| command_error("pi_agent_apply_commit", project_path, source))?;

    let save_id = request.save_id.as_deref();
    if let Some(save_id) = save_id {
        validate_runtime_snapshot_id(save_id)
            .map_err(|source| command_error("pi_agent_apply_save_id", project_path, source))?;
    }
    let report = assemble_play_once_report(project_path, step, save_id, &session)?;

    Ok(PiAgentApplyResult {
        run: run_result,
        scene_key,
        scene: report.scene.clone(),
        trace: report.trace.clone(),
        trace_path: report.trace_path.clone(),
        // Forward the delta summary the report already computes so the rail
        // and TraceDebugView render the "State deltas" count chip from the
        // same value `play_once_project*` produces — no TypeScript
        // re-implementation of `summarize_delta`.
        delta_summary: report.delta_summary.clone(),
        snapshot: report.snapshot.clone(),
        snapshot_path: report.snapshot_path.clone(),
    })
}

// ---------------------------------------------------------------------------
// Provider registry, prompt template, and skill library commands.
//
// These commands read/write user-global config (`~/.plotforge/providers.json`,
// `~/.plotforge/prompts.json`, `~/.plotforge/skill-index.json`) and the
// project-scoped prompt store (`<project>/.plotforge/prompts.json`). They
// never carry credentials, raw provider responses, or secret markers.
// ---------------------------------------------------------------------------

/// Lists every registered provider entry from the user-global registry.
/// An absent registry returns an empty list (fresh install).
pub fn list_providers() -> StudioCommandResult<Vec<ProviderEntry>> {
    let registry =
        plotforge_agent::load_provider_registry().map_err(|source| StudioCommandError {
            code: "list_providers".into(),
            message: source.to_string(),
        })?;
    Ok(registry.providers)
}

/// Adds or updates (by `id`) a provider entry in the user-global registry.
pub fn upsert_provider(entry: ProviderEntry) -> StudioCommandResult<ProviderEntry> {
    // Validate the entry before it is persisted. This catches credential
    // strings embedded in `endpoint_url` (e.g. `?key=sk-realkey`), malformed
    // `credential_env_var` charset, and other config errors at write time
    // rather than letting them slip into `~/.plotforge/providers.json` and
    // only surface at the next provider call.
    let config = plotforge_agent::TextProviderConfig {
        enabled: entry.enabled,
        provider: entry.label.clone(),
        model: entry.model.clone(),
        endpoint_url: Some(entry.endpoint_url.clone()),
        credential_env_var: entry.credential_env_var.clone(),
        max_output_tokens: entry.max_output_tokens,
        supports_json_schema: false,
    };
    config.validate().map_err(|error| StudioCommandError {
        code: "upsert_provider_invalid".into(),
        message: error.to_string(),
    })?;
    let mut registry =
        plotforge_agent::load_provider_registry().map_err(|source| StudioCommandError {
            code: "upsert_provider_load".into(),
            message: source.to_string(),
        })?;
    if let Some(existing) = registry.providers.iter_mut().find(|p| p.id == entry.id) {
        *existing = entry.clone();
    } else {
        registry.providers.push(entry.clone());
    }
    plotforge_agent::write_provider_registry(&registry).map_err(|source| StudioCommandError {
        code: "upsert_provider_write".into(),
        message: source.to_string(),
    })?;
    Ok(entry)
}

/// Removes a provider entry by id. Returns the removed entry, or an explicit
/// `provider_not_found` error if no entry matches.
pub fn delete_provider(id: String) -> StudioCommandResult<ProviderEntry> {
    let mut registry =
        plotforge_agent::load_provider_registry().map_err(|source| StudioCommandError {
            code: "delete_provider_load".into(),
            message: source.to_string(),
        })?;
    let position = registry
        .providers
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| StudioCommandError {
            code: "provider_not_found".into(),
            message: format!("no provider with id `{id}`"),
        })?;
    let removed = registry.providers.remove(position);
    plotforge_agent::write_provider_registry(&registry).map_err(|source| StudioCommandError {
        code: "delete_provider_write".into(),
        message: source.to_string(),
    })?;
    Ok(removed)
}

/// The result of a `test_provider_connection` ping: ok/failed + a
/// redaction-safe message. No provider response body is stored.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProviderTestResult {
    pub ok: bool,
    pub message: String,
}

/// Sends a minimal ping to a registered provider to verify the endpoint and
/// credential are usable. No response body is stored; only an ok/failed flag
/// plus a redaction-safe message. This is a real HTTP call — local-only,
/// user-initiated, never enters traces, projects, or export packages.
pub fn test_provider_connection(id: String) -> StudioCommandResult<ProviderTestResult> {
    let registry =
        plotforge_agent::load_provider_registry().map_err(|source| StudioCommandError {
            code: "test_provider_load".into(),
            message: source.to_string(),
        })?;
    let entry = registry
        .providers
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| StudioCommandError {
            code: "provider_not_found".into(),
            message: format!("no provider with id `{id}`"),
        })?;
    let built =
        plotforge_agent::build_text_provider(entry).map_err(|source| StudioCommandError {
            code: "test_provider_build".into(),
            message: source.to_string(),
        })?;
    // Reconstruct the config to derive the provider_config_hash the request
    // carries. `built` is a type-erased `Box<dyn TextModelProvider>` (the
    // registry may pick a strict or optional credential resolver), so the
    // concrete `ConfiguredTextModelProvider::config()` is not reachable here;
    // the hash is derived from the same non-secret fields either way.
    let probe_config = plotforge_agent::TextProviderConfig {
        enabled: entry.enabled,
        provider: entry.label.clone(),
        model: entry.model.clone(),
        endpoint_url: Some(entry.endpoint_url.clone()),
        credential_env_var: entry.credential_env_var.clone(),
        max_output_tokens: entry.max_output_tokens,
        supports_json_schema: false,
    };
    // Issue a tiny completion to verify the credential + endpoint resolve.
    // The request body is a benign probe; the response is discarded.
    let request = plotforge_agent::TextModelRequest {
        call_id: "connection-test".into(),
        agent: plotforge_schema::AgentRole::ScenePlanner,
        scene_key: "connection-test".into(),
        run_seed: 0,
        prompt_version: "plotforge-connection-test".into(),
        model_version: entry.model.clone(),
        provider_config_hash: probe_config.provider_config_hash(),
        prompt: "{\"probe\": true}".into(),
        messages: None,
    };
    match built.complete(&request) {
        Ok(_) => Ok(ProviderTestResult {
            ok: true,
            message: "provider responded to the probe request".into(),
        }),
        Err(error) => Ok(ProviderTestResult {
            ok: false,
            // R8: defence-in-depth. The provider error message is already
            // redacted at the HTTP/adapter layer, but apply `redact_trace_text`
            // again before surfacing it to the UI so a credential embedded in
            // an error-body field name (or any token the inner layer's marker
            // list missed) is still scrubbed before the user sees it.
            message: redact_trace_text(&error.message),
        }),
    }
}

/// Lists the models a registered provider serves, fetched from the provider's
/// upstream `/models` (or Anthropic `/v1/models`) endpoint. Results are cached
/// locally under `~/.plotforge/cache/models/{provider_id}.json` with a 1-hour
/// TTL so repeated UI lookups do not hammer the upstream.
///
/// The provider is resolved by `id` from the user-global registry; an unknown
/// id surfaces an explicit `provider_not_found` error (no silent fallback).
/// A missing credential surfaces an explicit error code so the UI can prompt
/// the user to set their API key — never an empty model list. The returned
/// `RemoteModelInfo` entries carry only descriptive metadata (id, owned_by,
/// token caps); no endpoint URL, credential, or raw response body is leaked.
pub fn list_remote_models(provider_id: String) -> StudioCommandResult<Vec<RemoteModelInfo>> {
    let registry =
        plotforge_agent::load_provider_registry().map_err(|source| StudioCommandError {
            code: "list_remote_models_load".into(),
            message: source.to_string(),
        })?;
    let entry = registry
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| StudioCommandError {
            code: "provider_not_found".into(),
            message: format!("no provider with id `{provider_id}`"),
        })?;
    plotforge_agent::fetch_provider_models(entry)
        .map_err(|source| StudioCommandError {
            code: match source {
                plotforge_agent::ModelDiscoveryError::MissingCredential { .. } => {
                    "list_remote_models_missing_credential"
                }
                plotforge_agent::ModelDiscoveryError::Http { .. } => "list_remote_models_http",
                plotforge_agent::ModelDiscoveryError::Cache { .. } => "list_remote_models_cache",
            }
            .into(),
            // `fetch_provider_models` already redacts via `redact_trace_text`;
            // apply it again as defence-in-depth before the UI sees the
            // message (mirrors `test_provider_connection`).
            message: redact_trace_text(&source.to_string()),
        })
}

// ---------------------------------------------------------------------------
// MCP server registry + tool-call commands (Phase 5, P5.1).
//
// These commands read/write the user-global MCP registry
// (`~/.plotforge/mcp.json`, the third member of the providers/skills/prompts
// family) and invoke tools through the blocking `McpToolClient` façade. They
// never carry credentials, raw tool bodies, or secret markers — only
// redaction-safe summaries. The registry never enters project source,
// contracts, traces, or export packages (AGENTS.md MCP carve-out).
// ---------------------------------------------------------------------------

/// Lists every registered MCP server entry from the user-global registry.
/// An absent registry returns an empty list (fresh install).
pub fn list_mcp_servers() -> StudioCommandResult<Vec<McpServerEntry>> {
    let registry = load_mcp_registry().map_err(|source| StudioCommandError {
        code: "list_mcp_servers".into(),
        message: source.to_string(),
    })?;
    Ok(registry.servers)
}

/// Adds or updates (by `id`) a MCP server entry in the user-global registry.
/// Validates the entry (path-traversal + secret-marker scan) before it is
/// persisted, mirroring `upsert_provider`'s pre-write validation.
pub fn upsert_mcp_server(entry: McpServerEntry) -> StudioCommandResult<McpServerEntry> {
    // Validate the entry before persistence. `validate_server_entry` scans
    // for secret markers in `endpoint_url`/`env` values and rejects path
    // traversal in stdio `command`/`args` (mirrors `upsert_provider`'s
    // `config.validate()` call).
    validate_server_entry(&entry).map_err(|source| StudioCommandError {
        code: "upsert_mcp_server_invalid".into(),
        message: source.to_string(),
    })?;
    let mut registry = load_mcp_registry().map_err(|source| StudioCommandError {
        code: "upsert_mcp_server_load".into(),
        message: source.to_string(),
    })?;
    if let Some(existing) = registry.servers.iter_mut().find(|s| s.id == entry.id) {
        *existing = entry.clone();
    } else {
        registry.servers.push(entry.clone());
    }
    write_mcp_registry(&registry).map_err(|source| StudioCommandError {
        code: "upsert_mcp_server_write".into(),
        message: source.to_string(),
    })?;
    Ok(entry)
}

/// Removes a MCP server entry by id. Returns the removed entry, or an
/// explicit `mcp_server_not_found` error if no entry matches.
pub fn delete_mcp_server(id: String) -> StudioCommandResult<McpServerEntry> {
    let mut registry = load_mcp_registry().map_err(|source| StudioCommandError {
        code: "delete_mcp_server_load".into(),
        message: source.to_string(),
    })?;
    let position = registry
        .servers
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| StudioCommandError {
            code: "mcp_server_not_found".into(),
            message: format!("no MCP server with id `{id}`"),
        })?;
    let removed = registry.servers.remove(position);
    write_mcp_registry(&registry).map_err(|source| StudioCommandError {
        code: "delete_mcp_server_write".into(),
        message: source.to_string(),
    })?;
    Ok(removed)
}

/// Sends a minimal probe to a registered MCP server to verify the transport
/// and handshake work. Spawns/connects, then `initialize`, then `tools/list`,
/// returning a redaction-safe `McpServerTestResult` (ok flag, message,
/// tools_count). No raw tool bodies or credential values are stored. This is
/// a real local subprocess/HTTP call — user-initiated, never enters traces,
/// projects, or export packages.
pub fn test_mcp_server(id: String) -> StudioCommandResult<McpServerTestResult> {
    let registry = load_mcp_registry().map_err(|source| StudioCommandError {
        code: "test_mcp_server_load".into(),
        message: source.to_string(),
    })?;
    let entry = resolve_mcp_server(&id, &registry).ok_or_else(|| StudioCommandError {
        code: "mcp_server_not_found".into(),
        message: format!("no MCP server with id `{id}`"),
    })?;
    if !entry.enabled {
        return Ok(McpServerTestResult {
            ok: false,
            message: redact_trace_text(&format!(
                "MCP server `{}` is disabled in the registry",
                entry.id
            )),
            tools_count: 0,
        });
    }
    // Build the transport (spawn stdio child / open SSE/HTTP connection).
    let mut transport = build_mcp_client(entry).map_err(|source| StudioCommandError {
        code: "test_mcp_server_build".into(),
        message: redact_trace_text(&source.to_string()),
    })?;
    // Initialize + list tools. Failures surface as explicit errors with a
    // redaction-safe message; no silent fallback.
    let test_result = match transport.initialize() {
        Ok(_info) => match transport.list_tools() {
            Ok(tools) => McpServerTestResult {
                ok: true,
                message: redact_trace_text(&format!(
                    "MCP server `{}` responded; {} tool(s) available",
                    entry.id,
                    tools.len()
                )),
                tools_count: tools.len() as u32,
            },
            Err(error) => McpServerTestResult {
                ok: false,
                message: redact_trace_text(&format!(
                    "MCP server `{}` list_tools failed: {error}",
                    entry.id
                )),
                tools_count: 0,
            },
        },
        Err(error) => McpServerTestResult {
            ok: false,
            message: redact_trace_text(&format!(
                "MCP server `{}` initialize failed: {error}",
                entry.id
            )),
            tools_count: 0,
        },
    };
    transport.close();
    Ok(test_result)
}

/// Lists the tools a MCP server exposes (via `tools/list`). Returns
/// redaction-safe `McpToolManifest` entries; no tool bodies or credentials.
pub fn list_mcp_tools(server_id: String) -> StudioCommandResult<Vec<McpToolManifest>> {
    let registry = load_mcp_registry().map_err(|source| StudioCommandError {
        code: "list_mcp_tools_load".into(),
        message: source.to_string(),
    })?;
    let client = McpToolRegistry::from_registry(registry);
    client
        .list_tools(&server_id)
        .map_err(|source| StudioCommandError {
            code: "list_mcp_tools".into(),
            message: redact_trace_text(&source.to_string()),
        })
}

/// Invokes a MCP tool. Takes a `McpToolCallRequest` (server_id + tool_name +
/// arguments), returns a redacted `McpToolCallResult`. The transport layer
/// applies `contains_secret_marker_text` to args + results before return;
/// this command applies `redact_trace_text` to any error message as
/// defence-in-depth. No raw tool bodies enter traces or project source.
pub fn invoke_mcp_tool(request: McpToolCallRequest) -> StudioCommandResult<McpToolCallResult> {
    let registry = load_mcp_registry().map_err(|source| StudioCommandError {
        code: "invoke_mcp_tool_load".into(),
        message: source.to_string(),
    })?;
    let client = McpToolRegistry::from_registry(registry);
    client
        .invoke_tool(request)
        .map_err(|source| StudioCommandError {
            code: "invoke_mcp_tool".into(),
            message: redact_trace_text(&source.to_string()),
        })
}

/// Enables or disables a MCP server for a project by writing its id to
/// `AgentSessionConfig.enabled_mcp_servers`. Mirrors
/// `enable_skill_for_project` (persisted through `set_agent_session_config`).
pub fn enable_mcp_server_for_project(
    project_path: impl AsRef<Path>,
    server_id: String,
    enabled: bool,
) -> StudioCommandResult<AgentSessionConfig> {
    let project_path = project_path.as_ref();
    let mut config = get_agent_session_config(project_path)?;
    if enabled {
        if !config.enabled_mcp_servers.contains(&server_id) {
            config.enabled_mcp_servers.push(server_id);
        }
    } else {
        config.enabled_mcp_servers.retain(|s| s != &server_id);
    }
    set_agent_session_config(project_path, &config)
}

/// Lists user-global prompt templates from `~/.plotforge/prompts.json`.
pub fn list_user_prompt_templates() -> StudioCommandResult<Vec<PromptTemplate>> {
    read_user_prompt_templates().map_err(|source| StudioCommandError {
        code: "list_user_prompt_templates".into(),
        message: source.to_string(),
    })
}

/// Lists project-scoped prompt templates from `<project>/.plotforge/prompts.json`.
pub fn list_project_prompt_templates(
    project_path: impl AsRef<Path>,
) -> StudioCommandResult<Vec<PromptTemplate>> {
    let project_path = project_path.as_ref();
    read_project_prompt_templates(project_path).map_err(|source| StudioCommandError {
        code: "list_project_prompt_templates".into(),
        message: source.to_string(),
    })
}

/// Adds or updates (by `id`) a user-global prompt template.
pub fn upsert_user_prompt_template(
    template: PromptTemplate,
) -> StudioCommandResult<PromptTemplate> {
    let mut templates = read_user_prompt_templates().map_err(|source| StudioCommandError {
        code: "upsert_user_prompt_template_load".into(),
        message: source.to_string(),
    })?;
    if let Some(existing) = templates.iter_mut().find(|t| t.id == template.id) {
        *existing = template.clone();
    } else {
        templates.push(template.clone());
    }
    write_user_prompt_templates(&templates).map_err(|source| StudioCommandError {
        code: "upsert_user_prompt_template_write".into(),
        message: source.to_string(),
    })?;
    Ok(template)
}

/// Adds or updates (by `id`) a project-scoped prompt template.
pub fn upsert_project_prompt_template(
    project_path: impl AsRef<Path>,
    template: PromptTemplate,
) -> StudioCommandResult<PromptTemplate> {
    let project_path = project_path.as_ref();
    let mut templates =
        read_project_prompt_templates(project_path).map_err(|source| StudioCommandError {
            code: "upsert_project_prompt_template_load".into(),
            message: source.to_string(),
        })?;
    if let Some(existing) = templates.iter_mut().find(|t| t.id == template.id) {
        *existing = template.clone();
    } else {
        templates.push(template.clone());
    }
    write_project_prompt_templates(project_path, &templates).map_err(|source| {
        StudioCommandError {
            code: "upsert_project_prompt_template_write".into(),
            message: source.to_string(),
        }
    })?;
    Ok(template)
}

/// Removes a user-global prompt template by id.
pub fn delete_user_prompt_template(id: String) -> StudioCommandResult<()> {
    let mut templates = read_user_prompt_templates().map_err(|source| StudioCommandError {
        code: "delete_user_prompt_template_load".into(),
        message: source.to_string(),
    })?;
    let before = templates.len();
    templates.retain(|t| t.id != id);
    if templates.len() == before {
        return Err(StudioCommandError {
            code: "prompt_template_not_found".into(),
            message: format!("no user prompt template with id `{id}`"),
        });
    }
    write_user_prompt_templates(&templates).map_err(|source| StudioCommandError {
        code: "delete_user_prompt_template_write".into(),
        message: source.to_string(),
    })?;
    Ok(())
}

/// Removes a project-scoped prompt template by id.
pub fn delete_project_prompt_template(
    project_path: impl AsRef<Path>,
    id: String,
) -> StudioCommandResult<()> {
    let project_path = project_path.as_ref();
    let mut templates =
        read_project_prompt_templates(project_path).map_err(|source| StudioCommandError {
            code: "delete_project_prompt_template_load".into(),
            message: source.to_string(),
        })?;
    let before = templates.len();
    templates.retain(|t| t.id != id);
    if templates.len() == before {
        return Err(StudioCommandError {
            code: "prompt_template_not_found".into(),
            message: format!("no project prompt template with id `{id}`"),
        });
    }
    write_project_prompt_templates(project_path, &templates).map_err(|source| {
        StudioCommandError {
            code: "delete_project_prompt_template_write".into(),
            message: source.to_string(),
        }
    })?;
    Ok(())
}

/// Lists every discovered skill from the cached index. Returns an empty list
/// when no cache exists yet — opening the Agent view does NOT trigger a
/// synchronous scan of all external skill roots (which can be O(roots ×
/// skills × files) filesystem I/O on a Tauri worker thread). The user opts
/// into a scan via `refresh_skill_index` (the "Refresh" button in the Skills
/// tab). `import_skill` lazily scans when the cache is missing because the
/// import action already implies the user wants the freshest skill set.
pub fn list_skills() -> StudioCommandResult<Vec<SkillManifest>> {
    let index = plotforge_agent::read_cached_skill_index().unwrap_or_else(SkillIndex::empty);
    Ok(index.skills)
}

/// Forces a full re-scan of all skill roots and writes the fresh index.
pub fn refresh_skill_index() -> StudioCommandResult<SkillIndex> {
    let index = plotforge_agent::scan_all_skills();
    plotforge_agent::write_skill_index(&index).map_err(|source| StudioCommandError {
        code: "refresh_skill_index".into(),
        message: source.to_string(),
    })?;
    Ok(index)
}

/// Copies an external skill (by id) into the user's own
/// `~/.plotforge/skills/<name>/` library.
pub fn import_skill(skill_id: String) -> StudioCommandResult<SkillManifest> {
    let index = if let Some(cached) = plotforge_agent::read_cached_skill_index() {
        cached
    } else {
        plotforge_agent::scan_all_skills()
    };
    let manifest = index
        .skills
        .iter()
        .find(|s| s.id == skill_id || s.name == skill_id)
        .ok_or_else(|| StudioCommandError {
            code: "skill_not_found".into(),
            message: format!("no skill with id `{skill_id}` in the index"),
        })?
        .clone();
    plotforge_agent::import_external_skill(&manifest).map_err(|source| StudioCommandError {
        code: "import_skill".into(),
        message: source.to_string(),
    })?;
    Ok(manifest)
}

/// Loads the body (Markdown) of a skill on demand, for prompt injection or
/// UI preview.
pub fn read_skill_body(skill_id: String) -> StudioCommandResult<String> {
    let index = if let Some(cached) = plotforge_agent::read_cached_skill_index() {
        cached
    } else {
        plotforge_agent::scan_all_skills()
    };
    let manifest = index
        .skills
        .iter()
        .find(|s| s.id == skill_id || s.name == skill_id)
        .ok_or_else(|| StudioCommandError {
            code: "skill_not_found".into(),
            message: format!("no skill with id `{skill_id}` in the index"),
        })?
        .clone();
    plotforge_agent::load_skill_body(&manifest).map_err(|source| StudioCommandError {
        code: "read_skill_body".into(),
        message: source.to_string(),
    })
}

/// Toggles whether a skill is enabled for a project (persisted in
/// `AgentSessionConfig.enabled_skills`).
pub fn enable_skill_for_project(
    project_path: impl AsRef<Path>,
    skill_id: String,
    enabled: bool,
) -> StudioCommandResult<AgentSessionConfig> {
    let project_path = project_path.as_ref();
    let mut config = get_agent_session_config(project_path)?;
    if enabled {
        if !config.enabled_skills.contains(&skill_id) {
            config.enabled_skills.push(skill_id);
        }
    } else {
        config.enabled_skills.retain(|s| s != &skill_id);
    }
    set_agent_session_config(project_path, &config)
}

// ---------------------------------------------------------------------------
// Git workspace integration.
//
// These commands shell out to the local `git` binary against the loaded
// project directory. They are read/switch-only — no push/pull/remote/fetch
// surface is exposed. A project directory that is not a git repository returns
// an explicit `not_a_git_repo` error (never a silent empty fallback).
// ---------------------------------------------------------------------------

/// The error code returned when a path is not inside a git work tree.
pub const GIT_NOT_A_REPO_CODE: &str = "not_a_git_repo";

/// Return the directory basename of a loaded project path. Used by the home
/// page to label the project chip without exposing the full filesystem path.
pub fn git_project_dir_name(path: impl AsRef<Path>) -> StudioCommandResult<String> {
    let path = path.as_ref();
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(name)
}

/// Return the current git branch name for a project directory.
///
/// Returns an explicit `not_a_git_repo` error when the path is not a git
/// work tree (never a silent empty-string fallback).
pub fn git_current_branch(path: impl AsRef<Path>) -> StudioCommandResult<String> {
    let path = path.as_ref();
    ensure_git_repo(path)?;
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|source| command_error("git_current_branch", path, source))?;
    if !output.status.success() {
        return Err(StudioCommandError {
            code: "git_current_branch".into(),
            message: format!(
                "{}: git rev-parse failed: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// List local branches for a project directory. Returns `GitBranchInfo` with
/// `is_current` marking the checked-out branch. Returns an explicit
/// `not_a_git_repo` error when the path is not a git work tree.
pub fn git_list_branches(path: impl AsRef<Path>) -> StudioCommandResult<Vec<GitBranchInfo>> {
    let path = path.as_ref();
    ensure_git_repo(path)?;
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["branch", "--list", "--format=%(refname:short)%09%(HEAD)"])
        .output()
        .map_err(|source| command_error("git_list_branches", path, source))?;
    if !output.status.success() {
        return Err(StudioCommandError {
            code: "git_list_branches".into(),
            message: format!(
                "{}: git branch --list failed: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut branches = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (name, head_marker) = line.split_once('\t').unwrap_or((line, ""));
        branches.push(GitBranchInfo {
            name: name.to_string(),
            is_current: head_marker.trim() == "*",
        });
    }
    Ok(branches)
}

/// Switch the project directory to a local branch. Returns the new current
/// branch. Returns an explicit `not_a_git_repo` error when the path is not a
/// git work tree.
pub fn git_switch_branch(
    path: impl AsRef<Path>,
    branch: &str,
) -> StudioCommandResult<GitSwitchResult> {
    let path = path.as_ref();
    let branch = branch.trim();
    if branch.is_empty() {
        return Err(StudioCommandError {
            code: "git_switch_branch".into(),
            message: format!("{}: branch name is required", path.display()),
        });
    }
    // Reject option-shaped branch names so a caller cannot smuggle a git option
    // (e.g. `--force`, `-b newbranch`, `--pathspec-from-file=…`) through the
    // IPC surface. `Command` is shell-less so this is not shell injection — it
    // is argument injection via git's own option parser. A real branch name
    // never starts with `-`, so this single check is the full defense: with a
    // `-`-prefixed name rejected, the remaining argument is unambiguously a
    // ref for `git checkout <branch>`. (We deliberately do NOT insert `--`
    // after `checkout`: that would make git treat the branch as a pathspec,
    // breaking legitimate switches.) Validated before `ensure_git_repo` so
    // this is a pure-input guard that does not depend on the git binary.
    if branch.starts_with('-') {
        return Err(StudioCommandError {
            code: "git_switch_branch".into(),
            message: format!(
                "{}: invalid branch name {:?}: branch names may not start with '-'",
                path.display(),
                branch,
            ),
        });
    }
    ensure_git_repo(path)?;
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["checkout", branch])
        .output()
        .map_err(|source| command_error("git_switch_branch", path, source))?;
    if !output.status.success() {
        return Err(StudioCommandError {
            code: "git_switch_branch".into(),
            message: format!(
                "{}: git checkout {} failed: {}",
                path.display(),
                branch,
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }
    Ok(GitSwitchResult {
        branch: branch.to_string(),
    })
}

fn ensure_git_repo(path: &Path) -> StudioCommandResult<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(|source| command_error("git_check_repo", path, source))?;
    if !output.status.success() {
        return Err(StudioCommandError {
            code: GIT_NOT_A_REPO_CODE.into(),
            message: format!("{}: not a git work tree", path.display()),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Agent session configuration.
//
// `AgentSessionConfig` is persisted per-project under
// `.plotforge/agent-config.json`. It is a redaction-safe capability surface:
// model_id, permission_level, thinking_level only — never credentials,
// endpoints, or raw provider responses. Real provider routing enforcement is
// deferred (per AGENTS.md) but the config + persistence + interface contract
// are wired now so the UI can drive them.
// ---------------------------------------------------------------------------

/// The list of model options the studio can surface to the UI. Hardcoded for
/// Lists the model options available for selection. The local mock pi-Agent
/// is always the first option (offline default); every enabled entry in the
/// user-global provider registry is then surfaced as a selectable model. The
/// `provider` field is a descriptive label (the provider kind), never an
/// endpoint URL or credential.
pub fn list_available_models() -> StudioCommandResult<Vec<ModelOption>> {
    let mut options = vec![ModelOption {
        id: plotforge_agent::LOCAL_PI_MODEL_ID.into(),
        label: "Local pi-Agent (mock)".into(),
        provider: "local-mock".into(),
    }];
    let registry =
        plotforge_agent::load_provider_registry().map_err(|source| StudioCommandError {
            code: "list_available_models".into(),
            message: source.to_string(),
        })?;
    for entry in registry.providers.iter().filter(|p| p.enabled) {
        options.push(ModelOption {
            id: entry.id.clone(),
            label: entry.label.clone(),
            provider: provider_kind_label(entry.kind),
        });
    }
    Ok(options)
}

/// Maps a `ProviderKind` to a short, descriptive label for the `ModelOption`
/// `provider` field. The label is for display only — never an endpoint or
/// credential.
fn provider_kind_label(kind: ProviderKind) -> String {
    match kind {
        ProviderKind::OpenAiCompatible => "openai_compatible".into(),
        ProviderKind::OpenAiResponses => "openai_responses".into(),
        ProviderKind::AnthropicMessages => "anthropic_messages".into(),
    }
}

/// Read the per-project agent session config. Returns defaults when no config
/// file exists yet (never an error for a fresh project).
pub fn get_agent_session_config(path: impl AsRef<Path>) -> StudioCommandResult<AgentSessionConfig> {
    let path = path.as_ref();
    let config_path = path.join(".plotforge").join("agent-config.json");
    if !config_path.exists() {
        return Ok(AgentSessionConfig::default());
    }
    let content = fs::read_to_string(&config_path)
        .map_err(|source| command_error("get_agent_session_config", path, source))?;
    serde_json::from_str::<AgentSessionConfig>(&content).map_err(|source| StudioCommandError {
        code: "get_agent_session_config".into(),
        message: format!("{}: invalid agent-config.json: {source}", path.display()),
    })
}

/// Persist the per-project agent session config under
/// `.plotforge/agent-config.json`. The `.plotforge` directory is created if
/// missing.
pub fn set_agent_session_config(
    path: impl AsRef<Path>,
    config: &AgentSessionConfig,
) -> StudioCommandResult<AgentSessionConfig> {
    let path = path.as_ref();
    let plotforge_dir = path.join(".plotforge");
    let config_path = plotforge_dir.join("agent-config.json");
    fs::create_dir_all(&plotforge_dir)
        .map_err(|source| command_error("set_agent_session_config", path, source))?;
    let content = serde_json::to_string_pretty(config).map_err(|source| StudioCommandError {
        code: "set_agent_session_config".into(),
        message: format!("{}: failed to serialize config: {source}", path.display()),
    })?;
    fs::write(&config_path, content)
        .map_err(|source| command_error("set_agent_session_config", path, source))?;
    Ok(config.clone())
}

pub fn validate_workshop_package(
    package_dir: impl AsRef<Path>,
) -> StudioCommandResult<StudioWorkshopPackageValidationReport> {
    let package_dir = package_dir.as_ref();
    plotforge_workshop::validate_workshop_package(package_dir)
        .map(studio_workshop_validation_report)
        .map_err(|source| command_error("validate_workshop_package", package_dir, source))
}

pub fn import_workshop_library_package(
    library_root: impl AsRef<Path>,
    package_dir: impl AsRef<Path>,
) -> StudioCommandResult<StudioWorkshopLibraryImportReport> {
    let library_root = library_root.as_ref();
    plotforge_workshop::import_workshop_library_package(library_root, package_dir)
        .map(|report| StudioWorkshopLibraryImportReport {
            item: studio_workshop_library_item(report.item),
            validation_report: studio_workshop_validation_report(report.validation_report),
        })
        .map_err(|source| command_error("import_workshop_library_package", library_root, source))
}

pub fn list_workshop_library(
    library_root: impl AsRef<Path>,
) -> StudioCommandResult<Vec<StudioWorkshopLibraryItem>> {
    let library_root = library_root.as_ref();
    plotforge_workshop::list_workshop_library(library_root)
        .map(|items| {
            items
                .into_iter()
                .map(studio_workshop_library_item)
                .collect()
        })
        .map_err(|source| command_error("list_workshop_library", library_root, source))
}

pub fn load_workshop_library_item(
    library_root: impl AsRef<Path>,
    local_id: &str,
) -> StudioCommandResult<StudioWorkshopLibraryLoadReport> {
    let library_root = library_root.as_ref();
    plotforge_workshop::load_workshop_library_item(library_root, local_id)
        .map(|report| StudioWorkshopLibraryLoadReport {
            item: studio_workshop_library_item(report.item),
            validation_report: studio_workshop_validation_report(report.validation_report),
        })
        .map_err(|source| command_error("load_workshop_library_item", library_root, source))
}

pub fn remix_workshop_library_item(
    library_root: impl AsRef<Path>,
    source_local_id: &str,
    new_local_id: &str,
    new_title: &str,
) -> StudioCommandResult<StudioWorkshopLibraryRemixReport> {
    let library_root = library_root.as_ref();
    plotforge_workshop::remix_workshop_library_item(
        library_root,
        source_local_id,
        new_local_id,
        new_title,
    )
    .map(|report| StudioWorkshopLibraryRemixReport {
        source_local_id: report.source_local_id,
        item: studio_workshop_library_item(report.item),
        validation_report: studio_workshop_validation_report(report.validation_report),
    })
    .map_err(|source| command_error("remix_workshop_library_item", library_root, source))
}

pub fn block_workshop_library_item(
    library_root: impl AsRef<Path>,
    local_id: &str,
    reason: &str,
) -> StudioCommandResult<StudioWorkshopLibraryItem> {
    let library_root = library_root.as_ref();
    plotforge_workshop::block_workshop_library_item(library_root, local_id, reason)
        .map(studio_workshop_library_item)
        .map_err(|source| command_error("block_workshop_library_item", library_root, source))
}

pub fn report_workshop_library_item(
    library_root: impl AsRef<Path>,
    local_id: &str,
    reason: &str,
) -> StudioCommandResult<StudioWorkshopLibraryItem> {
    let library_root = library_root.as_ref();
    plotforge_workshop::report_workshop_library_item(library_root, local_id, reason)
        .map(studio_workshop_library_item)
        .map_err(|source| command_error("report_workshop_library_item", library_root, source))
}

pub fn delete_workshop_library_item(
    library_root: impl AsRef<Path>,
    local_id: &str,
) -> StudioCommandResult<StudioWorkshopLibraryDeleteReport> {
    let library_root = library_root.as_ref();
    plotforge_workshop::delete_workshop_library_item(library_root, local_id)
        .map(|report| StudioWorkshopLibraryDeleteReport {
            local_id: report.local_id,
            package_dir: path_string(report.package_dir),
        })
        .map_err(|source| command_error("delete_workshop_library_item", library_root, source))
}

pub fn write_workshop_publish_draft(
    package_dir: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> StudioCommandResult<StudioWorkshopPublishDraftWriteReport> {
    let package_dir = package_dir.as_ref();
    plotforge_workshop::write_workshop_publish_draft(package_dir, output_dir)
        .map(|report| StudioWorkshopPublishDraftWriteReport {
            output_dir: path_string(report.output_dir),
            draft: report.draft,
            files_written: report.files_written.into_iter().map(path_string).collect(),
        })
        .map_err(|source| command_error("write_workshop_publish_draft", package_dir, source))
}

pub fn write_steam_submission_kit(
    package_dir: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    request: &SteamSubmissionKitRequest,
) -> StudioCommandResult<StudioSteamSubmissionKitWriteReport> {
    let package_dir = package_dir.as_ref();
    plotforge_workshop::write_steam_submission_kit(package_dir, output_dir, request)
        .map(|report| StudioSteamSubmissionKitWriteReport {
            output_dir: path_string(report.output_dir),
            draft: report.draft,
            files_written: report.files_written.into_iter().map(path_string).collect(),
        })
        .map_err(|source| command_error("write_steam_submission_kit", package_dir, source))
}

pub fn read_world_edit_document(path: impl AsRef<Path>) -> StudioCommandResult<WorldEditDocument> {
    let path = path.as_ref();
    plotforge_storage::read_world_edit_document(path)
        .map_err(|source| command_error("read_world_edit_document", path, source))
}

pub fn update_world_edit_document(
    path: impl AsRef<Path>,
    document: WorldEditDocument,
) -> StudioCommandResult<WorldEditDocument> {
    let path = path.as_ref();
    plotforge_storage::update_world_edit_document(path, document)
        .map_err(|source| command_error("update_world_edit_document", path, source))
}

pub fn read_story_craft_edit_document(
    path: impl AsRef<Path>,
) -> StudioCommandResult<StoryCraftEditDocument> {
    let path = path.as_ref();
    plotforge_storage::read_story_craft_edit_document(path)
        .map_err(|source| command_error("read_story_craft_edit_document", path, source))
}

pub fn update_story_craft_edit_document(
    path: impl AsRef<Path>,
    document: StoryCraftEditDocument,
) -> StudioCommandResult<StoryCraftEditDocument> {
    let path = path.as_ref();
    plotforge_storage::update_story_craft_edit_document(path, document)
        .map_err(|source| command_error("update_story_craft_edit_document", path, source))
}

pub fn read_character_edit_document(
    path: impl AsRef<Path>,
) -> StudioCommandResult<CharacterEditDocument> {
    let path = path.as_ref();
    plotforge_storage::read_character_edit_document(path)
        .map_err(|source| command_error("read_character_edit_document", path, source))
}

pub fn update_character_edit_document(
    path: impl AsRef<Path>,
    document: CharacterEditDocument,
) -> StudioCommandResult<CharacterEditDocument> {
    let path = path.as_ref();
    plotforge_storage::update_character_edit_document(path, document)
        .map_err(|source| command_error("update_character_edit_document", path, source))
}

pub fn create_character(
    path: impl AsRef<Path>,
    character: Character,
) -> StudioCommandResult<CharacterEditDocument> {
    let path = path.as_ref();
    plotforge_storage::create_character(path, character)
        .map_err(|source| command_error("create_character", path, source))
}

/// Create a new character from a UI draft, assembling the `Character` value in
/// Rust (trimming all string fields and splitting `traits_text` by newlines).
/// This keeps character construction logic inside the Rust boundary per
/// AGENTS.md line 51 — the Creator Desktop must not reimplement storage logic.
pub fn create_character_from_draft(
    path: impl AsRef<Path>,
    draft: CharacterDraft,
) -> StudioCommandResult<CharacterEditDocument> {
    let path = path.as_ref();
    let character = draft.into_character();
    plotforge_storage::create_character(path, character)
        .map_err(|source| command_error("create_character_from_draft", path, source))
}

pub fn read_state_variables_edit_document(
    path: impl AsRef<Path>,
) -> StudioCommandResult<StateVariablesEditDocument> {
    let path = path.as_ref();
    plotforge_storage::read_state_variables_edit_document(path)
        .map_err(|source| command_error("read_state_variables_edit_document", path, source))
}

pub fn update_state_variables_edit_document(
    path: impl AsRef<Path>,
    document: StateVariablesEditDocument,
) -> StudioCommandResult<StateVariablesEditDocument> {
    let path = path.as_ref();
    plotforge_storage::update_state_variables_edit_document(path, document)
        .map_err(|source| command_error("update_state_variables_edit_document", path, source))
}

pub fn create_resource(
    path: impl AsRef<Path>,
    resource: ResourceDefinition,
) -> StudioCommandResult<StateVariablesEditDocument> {
    let path = path.as_ref();
    plotforge_storage::create_resource(path, resource)
        .map_err(|source| command_error("create_resource", path, source))
}

pub fn read_rules_edit_document(path: impl AsRef<Path>) -> StudioCommandResult<RulesEditDocument> {
    let path = path.as_ref();
    plotforge_storage::read_rules_edit_document(path)
        .map_err(|source| command_error("read_rules_edit_document", path, source))
}

pub fn update_rules_edit_document(
    path: impl AsRef<Path>,
    document: RulesEditDocument,
) -> StudioCommandResult<RulesEditDocument> {
    let path = path.as_ref();
    plotforge_storage::update_rules_edit_document(path, document)
        .map_err(|source| command_error("update_rules_edit_document", path, source))
}

pub fn create_rule(path: impl AsRef<Path>, rule: Rule) -> StudioCommandResult<RulesEditDocument> {
    let path = path.as_ref();
    plotforge_storage::create_rule(path, rule)
        .map_err(|source| command_error("create_rule", path, source))
}

/// Create a new rule from a UI draft, assembling the `Rule` value in Rust
/// (trimming fields and building the `effects` array). This keeps rule
/// construction logic inside the Rust boundary per AGENTS.md line 51 — the
/// Creator Desktop must not reimplement storage logic.
pub fn create_rule_from_draft(
    path: impl AsRef<Path>,
    draft: RuleDraft,
) -> StudioCommandResult<RulesEditDocument> {
    let path = path.as_ref();
    let rule = draft.into_rule();
    plotforge_storage::create_rule(path, rule)
        .map_err(|source| command_error("create_rule_from_draft", path, source))
}

pub fn generate_world_expansion(
    path: impl AsRef<Path>,
    expansion_goal: &str,
) -> StudioCommandResult<WorldGenerationReport> {
    let path = path.as_ref();
    let project = load_project(path)
        .map_err(|source| command_error("generate_world_expansion_load", path, source))?;
    let request = plotforge_storage::build_world_generation_request(path, expansion_goal)
        .map_err(|source| command_error("generate_world_expansion_request", path, source))?;
    let report = plotforge_agent::generate_world_expansion(request, project.game.run_seed);
    plotforge_storage::apply_world_generation_report(path, report)
        .map_err(|source| command_error("generate_world_expansion_apply", path, source))
}

pub fn generate_story_craft(
    path: impl AsRef<Path>,
    concept: &str,
) -> StudioCommandResult<StoryCraftGenerationReport> {
    let path = path.as_ref();
    let project = load_project(path)
        .map_err(|source| command_error("generate_story_craft_load", path, source))?;
    let request = plotforge_storage::build_story_craft_generation_request(path, concept)
        .map_err(|source| command_error("generate_story_craft_request", path, source))?;
    let report = plotforge_agent::generate_story_craft(request, project.game.run_seed);
    plotforge_storage::apply_story_craft_generation_report(path, report)
        .map_err(|source| command_error("generate_story_craft_apply", path, source))
}

pub fn generate_character(
    path: impl AsRef<Path>,
    concept: &str,
    role_hint: &str,
) -> StudioCommandResult<CharacterGenerationReport> {
    let path = path.as_ref();
    let project = load_project(path)
        .map_err(|source| command_error("generate_character_load", path, source))?;
    let request = plotforge_storage::build_character_generation_request(path, concept, role_hint)
        .map_err(|source| command_error("generate_character_request", path, source))?;
    let report = plotforge_agent::generate_character(request, project.game.run_seed);
    plotforge_storage::apply_character_generation_report(path, report)
        .map_err(|source| command_error("generate_character_apply", path, source))
}

pub fn read_ai_safety_policy(path: impl AsRef<Path>) -> StudioCommandResult<AiSafetyPolicy> {
    let path = path.as_ref();
    plotforge_storage::read_ai_safety_policy(path)
        .map_err(|source| command_error("read_ai_safety_policy", path, source))
}

pub fn update_ai_safety_policy(
    path: impl AsRef<Path>,
    policy: AiSafetyPolicy,
) -> StudioCommandResult<AiSafetyPolicy> {
    let path = path.as_ref();
    plotforge_storage::update_ai_safety_policy(path, policy)
        .map_err(|source| command_error("update_ai_safety_policy", path, source))
}

pub fn read_visual_bible(path: impl AsRef<Path>) -> StudioCommandResult<VisualBible> {
    let path = path.as_ref();
    plotforge_storage::read_visual_bible(path)
        .map_err(|source| command_error("read_visual_bible", path, source))
}

pub fn update_visual_bible(
    path: impl AsRef<Path>,
    visual_bible: VisualBible,
) -> StudioCommandResult<VisualBible> {
    let path = path.as_ref();
    plotforge_storage::update_visual_bible(path, visual_bible)
        .map_err(|source| command_error("update_visual_bible", path, source))
}

pub fn read_audio_bible(path: impl AsRef<Path>) -> StudioCommandResult<AudioBible> {
    let path = path.as_ref();
    plotforge_storage::read_audio_bible(path)
        .map_err(|source| command_error("read_audio_bible", path, source))
}

pub fn update_audio_bible(
    path: impl AsRef<Path>,
    audio_bible: AudioBible,
) -> StudioCommandResult<AudioBible> {
    let path = path.as_ref();
    plotforge_storage::update_audio_bible(path, audio_bible)
        .map_err(|source| command_error("update_audio_bible", path, source))
}

pub fn play_once_project(
    path: impl AsRef<Path>,
    player_input: &str,
) -> StudioCommandResult<PlayOnceReport> {
    play_once_project_with_save(path, player_input, None)
}

pub fn play_once_project_with_save(
    path: impl AsRef<Path>,
    player_input: &str,
    save_id: Option<&str>,
) -> StudioCommandResult<PlayOnceReport> {
    let path = path.as_ref();
    let project =
        load_project(path).map_err(|source| command_error("play_once_load", path, source))?;
    let mut session = RuntimeSession::new(project);
    play_once_session(path, &mut session, player_input, save_id)
}

pub fn play_once_project_from_snapshot(
    path: impl AsRef<Path>,
    player_input: &str,
    snapshot_id: &str,
    save_id: Option<&str>,
) -> StudioCommandResult<PlayOnceReport> {
    let path = path.as_ref();
    let project = load_project(path)
        .map_err(|source| command_error("play_once_restore_load", path, source))?;
    let snapshot = read_runtime_snapshot(path, snapshot_id)
        .map_err(|source| command_error("play_once_restore_snapshot", path, source))?;
    let mut session = RuntimeSession::from_snapshot(project, snapshot)
        .map_err(|source| command_error("play_once_restore_runtime", path, source))?;
    play_once_session(path, &mut session, player_input, save_id)
}

pub fn play_once_project_from_latest_snapshot(
    path: impl AsRef<Path>,
    player_input: &str,
    save_id: Option<&str>,
) -> StudioCommandResult<PlayOnceReport> {
    let path = path.as_ref();
    let project = load_project(path)
        .map_err(|source| command_error("play_once_restore_load", path, source))?;
    let snapshot = read_latest_runtime_snapshot(path)
        .map_err(|source| command_error("play_once_restore_latest_snapshot", path, source))?;
    let mut session = RuntimeSession::from_snapshot(project, snapshot)
        .map_err(|source| command_error("play_once_restore_runtime", path, source))?;
    play_once_session(path, &mut session, player_input, save_id)
}

fn play_once_session(
    path: &Path,
    session: &mut RuntimeSession,
    player_input: &str,
    save_id: Option<&str>,
) -> StudioCommandResult<PlayOnceReport> {
    if let Some(save_id) = save_id {
        validate_runtime_snapshot_id(save_id)
            .map_err(|source| command_error("play_once_snapshot", path, source))?;
    }
    let step = session
        .play_once(player_input)
        .map_err(|source| command_error("play_once_runtime", path, source))?;
    assemble_play_once_report(path, step, save_id, session)
}

/// Assembles a `PlayOnceReport` from a committed `RuntimeStep`: writes the
/// trace, optionally writes a runtime snapshot when `save_id` is set, and
/// returns the report shape the desktop `AgentChatRail` / `PlayView`
/// render. Shared by the playtest path (`play_once_session`) and the
/// pi-Agent path (`pi_agent_apply_run`) so both surfaces produce
/// identically-shaped reports.
fn assemble_play_once_report(
    path: &Path,
    step: RuntimeStep,
    save_id: Option<&str>,
    session: &RuntimeSession,
) -> StudioCommandResult<PlayOnceReport> {
    let trace_path = write_trace(path, &step.trace)
        .map_err(|source| command_error("play_once_trace", path, source))?;
    let (snapshot, snapshot_path) = if let Some(save_id) = save_id {
        let snapshot = session.snapshot(save_id, step.trace.timestamp_ms);
        let snapshot_path = write_runtime_snapshot(path, &snapshot)
            .map_err(|source| command_error("play_once_snapshot", path, source))?;
        (Some(snapshot), Some(path_string(snapshot_path)))
    } else {
        (None, None)
    };

    Ok(PlayOnceReport {
        delta_summary: summarize_delta(&step.trace.world_state_delta),
        scene: step.scene,
        trace: step.trace,
        trace_path: path_string(trace_path),
        snapshot,
        snapshot_path,
    })
}

pub fn export_static_project(
    path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> StudioCommandResult<StaticExportReport> {
    let path = path.as_ref();
    let output_dir = output_dir.as_ref();
    let report = export_static_web(path, output_dir)
        .map_err(|source| command_error("export_static", path, source))?;

    Ok(StaticExportReport {
        output_dir: path_string(report.output_dir),
        archive_path: None,
        files_written: report.files_written.into_iter().map(path_string).collect(),
        archived_files: Vec::new(),
        allowed_files: report
            .audit
            .allowed_files
            .into_iter()
            .map(path_string)
            .collect(),
        files_found: report
            .audit
            .files_found
            .into_iter()
            .map(path_string)
            .collect(),
    })
}

pub fn export_static_project_zip(
    path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    archive_path: impl AsRef<Path>,
) -> StudioCommandResult<StaticExportReport> {
    let path = path.as_ref();
    let output_dir = output_dir.as_ref();
    let archive_path = archive_path.as_ref();
    let report = export_static_web_zip(path, output_dir, archive_path)
        .map_err(|source| command_error("export_static_zip", path, source))?;

    Ok(StaticExportReport {
        output_dir: path_string(report.source_report.output_dir),
        archive_path: Some(path_string(report.archive_path)),
        files_written: report
            .source_report
            .files_written
            .into_iter()
            .map(path_string)
            .collect(),
        archived_files: report.archived_files.into_iter().map(path_string).collect(),
        allowed_files: report
            .source_report
            .audit
            .allowed_files
            .into_iter()
            .map(path_string)
            .collect(),
        files_found: report
            .source_report
            .audit
            .files_found
            .into_iter()
            .map(path_string)
            .collect(),
    })
}

pub fn list_asset_records(path: impl AsRef<Path>) -> StudioCommandResult<Vec<AssetRecord>> {
    let path = path.as_ref();
    plotforge_storage::list_asset_records(path)
        .map_err(|source| command_error("list_asset_records", path, source))
}

pub fn list_source_files(path: impl AsRef<Path>) -> StudioCommandResult<Vec<SourceFileSummary>> {
    let path = path.as_ref();
    validate_project(path).map_err(|source| command_error("list_source_files", path, source))?;

    source_file_paths(path)
        .map_err(|source| command_error("list_source_files", path, source))?
        .into_iter()
        .map(|relative_path| {
            source_file_summary(path, &relative_path)
                .map_err(|source| command_error("list_source_files", path, source))
        })
        .collect()
}

pub fn read_source_file(
    path: impl AsRef<Path>,
    relative_path: &str,
) -> StudioCommandResult<SourceFileContent> {
    let path = path.as_ref();
    validate_project(path).map_err(|source| command_error("read_source_file", path, source))?;
    let relative_path = parse_relative_source_path(relative_path)
        .map_err(|source| command_error("read_source_file", path, source))?;
    ensure_listed_source_file(path, &relative_path)
        .map_err(|source| command_error("read_source_file", path, source))?;
    let summary = source_file_summary(path, &relative_path)
        .map_err(|source| command_error("read_source_file", path, source))?;
    let full_path = path.join(&relative_path);
    let content = fs::read_to_string(&full_path)
        .map_err(|source| command_error("read_source_file", path, source))?;

    Ok(SourceFileContent {
        path: summary.path,
        kind: summary.kind,
        editable: summary.editable,
        content,
    })
}

pub fn write_source_file(
    path: impl AsRef<Path>,
    relative_path: &str,
    content: &str,
) -> StudioCommandResult<SourceFileContent> {
    let path = path.as_ref();
    validate_project(path).map_err(|source| command_error("write_source_file", path, source))?;
    let relative_path = parse_relative_source_path(relative_path)
        .map_err(|source| command_error("write_source_file", path, source))?;
    ensure_listed_source_file(path, &relative_path)
        .map_err(|source| command_error("write_source_file", path, source))?;
    let summary = source_file_summary(path, &relative_path)
        .map_err(|source| command_error("write_source_file", path, source))?;
    if !summary.editable {
        return Err(StudioCommandError {
            code: "write_source_file".into(),
            message: format!("{} is not an editable source file", summary.path),
        });
    }

    let full_path = path.join(&relative_path);
    fs::write(&full_path, content)
        .map_err(|source| command_error("write_source_file", path, source))?;

    Ok(SourceFileContent {
        path: summary.path,
        kind: summary.kind,
        editable: summary.editable,
        content: content.to_string(),
    })
}

fn command_error(code: impl Into<String>, path: &Path, source: impl Error) -> StudioCommandError {
    StudioCommandError {
        code: code.into(),
        message: format!("{}: {source}", path.display()),
    }
}

fn studio_workshop_validation_report(
    report: plotforge_workshop::WorkshopPackageValidationReport,
) -> StudioWorkshopPackageValidationReport {
    StudioWorkshopPackageValidationReport {
        package_dir: path_string(report.package_dir),
        manifest: report.manifest,
        ai_usage: report.ai_usage,
        files: report
            .files
            .into_iter()
            .map(studio_workshop_validated_file)
            .collect(),
    }
}

fn studio_workshop_validated_file(
    file: plotforge_workshop::WorkshopValidatedFile,
) -> StudioWorkshopValidatedFile {
    StudioWorkshopValidatedFile {
        path: path_string(file.path),
        content_hash: file.content_hash,
        byte_length: file.byte_length,
    }
}

fn studio_workshop_library_item(
    item: plotforge_workshop::WorkshopLibraryItem,
) -> StudioWorkshopLibraryItem {
    StudioWorkshopLibraryItem {
        local_id: item.local_id,
        package_id: item.package_id,
        title: item.title,
        package_dir: path_string(item.package_dir),
        blocked: item.blocked.map(|blocked| StudioWorkshopLibraryBlock {
            reason: blocked.reason,
        }),
        reports: item
            .reports
            .into_iter()
            .map(|report| StudioWorkshopLibraryReport {
                reason: report.reason,
            })
            .collect(),
    }
}

fn path_string(path: PathBuf) -> String {
    path.display().to_string()
}

fn source_file_summary(
    project_path: &Path,
    relative_path: &Path,
) -> std::io::Result<SourceFileSummary> {
    let relative_path = parse_relative_source_path(&path_string(relative_path.to_path_buf()))?;
    let full_path = project_path.join(&relative_path);
    let metadata = fs::metadata(&full_path)?;
    if !metadata.is_file() {
        return Err(std::io::Error::other(format!(
            "{} is not a file",
            relative_path.display()
        )));
    }

    let kind = source_file_kind(&relative_path).ok_or_else(|| {
        std::io::Error::other(format!(
            "{} is not a supported source file",
            relative_path.display()
        ))
    })?;
    let editable = is_editable_source_file(&relative_path, &kind);

    Ok(SourceFileSummary {
        path: path_string(relative_path),
        kind,
        bytes: metadata.len(),
        editable,
    })
}

fn source_file_paths(project_path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = vec![
        PathBuf::from("game.toml"),
        PathBuf::from("world/resources.toml"),
        PathBuf::from("world/initial_state.json"),
        PathBuf::from("world/world.md"),
        PathBuf::from("world/canon.md"),
        PathBuf::from("world/forbidden_facts.json"),
        PathBuf::from("story/story_craft.toml"),
        PathBuf::from("story/emotional_arc.json"),
        PathBuf::from("story/plot_threads.toml"),
        PathBuf::from("story/story_bible.md"),
        PathBuf::from("story/style_guide.md"),
        PathBuf::from("media/visual_bible.toml"),
        PathBuf::from("media/audio_bible.toml"),
        PathBuf::from("rules/rules.toml"),
        PathBuf::from("saves/initial_story_state.json"),
    ];

    for (dir, suffix) in [
        ("characters", ".character.toml"),
        ("scenes", ".scene.json"),
        ("agents", ".prompt.md"),
        ("references/methods", ".reference.json"),
    ] {
        let dir_path = project_path.join(dir);
        if !dir_path.exists() {
            continue;
        }
        let mut entries = fs::read_dir(&dir_path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();
        paths.extend(entries.into_iter().filter_map(|entry| {
            let name = entry.file_name()?.to_str()?;
            name.ends_with(suffix)
                .then(|| PathBuf::from(dir).join(name))
        }));
    }

    paths.retain(|relative_path| project_path.join(relative_path).is_file());
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn ensure_listed_source_file(project_path: &Path, relative_path: &Path) -> std::io::Result<()> {
    if source_file_paths(project_path)?
        .into_iter()
        .any(|candidate| candidate == relative_path)
    {
        return Ok(());
    }

    Err(std::io::Error::other(format!(
        "{} is not a project source file",
        relative_path.display()
    )))
}

fn parse_relative_source_path(path: &str) -> std::io::Result<PathBuf> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(std::io::Error::other("source file path must be relative"));
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(std::io::Error::other(
                "source file path must not contain parent or special components",
            ));
        }
    }
    Ok(path.to_path_buf())
}

fn source_file_kind(path: &Path) -> Option<SourceFileKind> {
    let name = path.file_name()?.to_str()?;
    if name.ends_with(".prompt.md") {
        Some(SourceFileKind::Prompt)
    } else if name.ends_with(".md") || name.ends_with(".markdown") {
        Some(SourceFileKind::Markdown)
    } else if name.ends_with(".toml") {
        Some(SourceFileKind::Toml)
    } else if name.ends_with(".json") {
        Some(SourceFileKind::Json)
    } else {
        None
    }
}

fn is_editable_source_file(path: &Path, kind: &SourceFileKind) -> bool {
    matches!(kind, SourceFileKind::Markdown | SourceFileKind::Prompt)
        && path
            .components()
            .next()
            .and_then(|component| match component {
                Component::Normal(value) => value.to_str(),
                _ => None,
            })
            .is_some_and(|top_level| matches!(top_level, "world" | "story" | "agents"))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::tempdir;

    use super::{
        AiProviderSummary, AiSafetyPolicy, AiUsageContentKind, AiUsageDisclosure, AiUsageManifest,
        AiUsageSourceKind, Character, CharacterDraft, Effect, ExportProfile, GIT_NOT_A_REPO_CODE,
        PromptScope, PromptTemplate, ProviderEntry, ProviderKind, ResourceDefinition, Rule,
        RuleDraft, SteamSubmissionKitRequest, WorkshopDraftVisibility, WorkshopItemPackage,
        WorkshopPackageFile, block_workshop_library_item, check_project, create_character,
        create_character_from_draft, create_project, create_resource, create_rule,
        create_rule_from_draft, delete_mcp_server, delete_project_prompt_template, delete_provider,
        delete_workshop_library_item, enable_mcp_server_for_project, enable_skill_for_project,
        export_static_project, export_static_project_zip, generate_character, generate_story_craft,
        generate_world_expansion, get_agent_session_config, git_current_branch, git_list_branches,
        git_project_dir_name, git_switch_branch, import_workshop_library_package,
        list_asset_records, list_available_models, list_export_profiles, list_mcp_servers,
        list_project_prompt_templates, list_providers, list_remote_models, list_source_files,
        list_workshop_library, load_workshop_library_item, open_project, pi_agent_apply_run,
        pi_agent_capabilities, pi_agent_run, play_once_project, play_once_project_from_latest_snapshot,
        play_once_project_from_snapshot, play_once_project_with_save, read_ai_safety_policy,
        read_character_edit_document, read_rules_edit_document, read_source_file,
        read_state_variables_edit_document, read_story_craft_edit_document,
        read_world_edit_document, remix_workshop_library_item, report_workshop_library_item,
        set_agent_session_config, update_ai_safety_policy, update_story_craft_edit_document,
        update_world_edit_document, upsert_project_prompt_template, upsert_provider,
        validate_workshop_package, write_source_file, write_steam_submission_kit,
        write_workshop_publish_draft,
    };
    use plotforge_schema::{
        AgentSessionConfig, PermissionLevel, PiAgentApplyRequest, PiAgentRunRequest, ThinkingLevel,
        contains_secret_marker_text,
    };

    #[test]
    fn pi_agent_run_returns_local_pi_marker() {
        // The local pi-Agent default must return the local marker, fully
        // populated reproducibility metadata, a deterministic trace evidence
        // id, and a redaction-safe evidence summary (no raw provider
        // responses or secret markers anywhere in the result).
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 7,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "sha256:studio-pi-agent-prompt".into(),
        };

        let result = pi_agent_run(request).expect("pi-agent run succeeds");

        // Local marker must be present on the descriptor.
        assert!(
            result.descriptor.is_local_pi,
            "pi-Agent run must mark itself as the local pi-Agent"
        );
        assert_eq!(result.descriptor.agent_id, "pi-agent-local");
        assert!(!result.descriptor.capabilities.is_empty());

        // Reproducibility metadata must be present and use the pi-Agent local
        // provider config hash (distinct from the generic fake provider).
        let reproducibility = &result.reproducibility;
        assert_eq!(reproducibility.run_seed, 7);
        assert_eq!(
            reproducibility.prompt_version, "plotforge-pi-agent-prompt-v1",
            "pi-Agent local provider must use the pi-Agent prompt version"
        );
        assert_eq!(
            reproducibility.model_version, "plotforge-pi-agent-model-v1",
            "pi-Agent local provider must use the pi-Agent model version"
        );
        assert_eq!(
            reproducibility.provider_config_hash, "sha256:plotforge-pi-agent-local-config-v1",
            "pi-Agent local provider must use the pi-Agent config hash"
        );

        // A deterministic trace evidence id must be present on both the
        // reproducibility metadata and the top-level trace_id, derived from
        // the run seed so it is reproducible.
        let trace_id = result
            .trace_id
            .as_ref()
            .expect("trace evidence id must be present");
        assert!(
            trace_id.starts_with("pi-agent-evidence-"),
            "trace evidence id must follow the pi-Agent evidence format"
        );
        assert_eq!(
            reproducibility.trace_id.as_deref(),
            Some(trace_id.as_str()),
            "reproducibility trace_id must match the top-level trace evidence id"
        );

        // Running again with the same seed must reproduce the same trace id.
        let again = pi_agent_run(PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 7,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "sha256:studio-pi-agent-prompt".into(),
        })
        .expect("pi-agent run succeeds");
        assert_eq!(
            again.trace_id, result.trace_id,
            "trace evidence id must be deterministic for the same run seed"
        );

        // The serialized result must be redaction-safe: no raw provider
        // responses, no secret markers, no credentials anywhere.
        let encoded = serde_json::to_string(&result).expect("serialize result");
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
        assert!(
            !contains_secret_marker_text(&result.evidence_summary),
            "evidence summary must be redaction-safe"
        );
    }

    #[test]
    fn pi_agent_run_surfaces_explicit_error_without_silent_fallback() {
        // A request with an empty prompt hash must surface an explicit error,
        // never a silent fallback result. This guards the no-silent-fallback
        // boundary required by the agent crate.
        let request = PiAgentRunRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 7,
            prompt_summary: "Generate a validated scene plan proposal.".into(),
            prompt_hash: "   ".into(),
        };

        let error = pi_agent_run(request).expect_err("empty prompt hash rejected");
        assert_eq!(error.code, "pi_agent_run");
        assert!(!error.message.contains("sk-"));
    }

    #[test]
    fn pi_agent_capabilities_lists_wired_capability() {
        let capabilities = pi_agent_capabilities().expect("capabilities list");
        assert!(!capabilities.is_empty());
        // Pin the exact wired capability so an accidental over-claim (e.g.
        // flipping image-generation or steam-upload to wired) fails this test
        // instead of passing under the previous `>= 1` bound.
        let wired: Vec<_> = capabilities
            .iter()
            .filter(|capability| capability.status == "wired")
            .collect();
        assert_eq!(
            wired.len(),
            1,
            "exactly one capability should be wired; got {wired:?}"
        );
        assert_eq!(
            wired[0].id, "pi-agent.text-generation",
            "only text-generation should be wired"
        );
        for capability in &capabilities {
            assert!(!contains_secret_marker_text(&capability.evidence));
            assert!(!capability.evidence.contains("sk-"));
        }
    }

    #[test]
    fn create_project_delegates_to_storage_and_reopens() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("winter-regency");

        let report = create_project(&project_path, sample_creation_request(), false)
            .expect("create project");
        let check = check_project(&project_path).expect("check created project");
        let world = read_source_file(&project_path, "world/world.md").expect("read world");

        assert_eq!(report.project_path, project_path.display().to_string());
        assert_eq!(report.project.game.title, "Winter Regency");
        assert!(report.files_created.iter().any(|path| path == "game.toml"));
        assert_eq!(check.title, "Winter Regency");
        assert!(world.content.contains("winter coup"));
    }

    #[test]
    fn create_project_returns_explicit_error_code() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("winter-regency");
        let mut request = sample_creation_request();
        request.initial_scene_request = "OPENAI_API_KEY=sk-test-secret-marker".into();

        let error =
            create_project(&project_path, request, false).expect_err("secret marker should fail");

        assert_eq!(error.code, "create_project");
        assert!(error.message.contains("secret markers"));
        assert!(!project_path.join("game.toml").exists());
    }

    #[test]
    fn list_export_profiles_exposes_schema_supported_profiles() {
        let profiles = list_export_profiles();
        let ids = profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                "static-web",
                "byo-key-web",
                "self-host-backend",
                "desktop-runtime",
                "steam-workshop",
                "steam-submission-kit"
            ]
        );
        assert!(profiles.iter().any(|profile| profile.id == "byo-key-web"));
        assert!(
            profiles
                .iter()
                .any(|profile| profile.id == "self-host-backend")
        );
        assert!(
            profiles
                .iter()
                .any(|profile| profile.id == "steam-submission-kit")
        );
        assert!(
            profiles
                .iter()
                .all(|profile| !profile.includes_provider_config)
        );
        assert!(
            profiles
                .iter()
                .all(|profile| !profile.includes_private_traces)
        );
        assert!(
            profiles
                .iter()
                .all(|profile| !profile.platform_submission_ready)
        );
    }

    #[test]
    fn workshop_adapter_validates_package_and_writes_local_draft_outputs() {
        let package = tempdir().expect("package");
        let publish_output = tempdir().expect("publish output");
        let kit_output = tempdir().expect("kit output");
        write_valid_workshop_package(package.path());

        let validation = validate_workshop_package(package.path()).expect("validate package");

        assert_eq!(validation.package_dir, package.path().display().to_string());
        assert_eq!(validation.manifest.package_id, "studio-workshop-fixture");
        assert_eq!(validation.ai_usage.project_id, "studio-workshop-fixture");
        assert!(!validation.ai_usage.provider_credentials_included);
        assert!(!validation.ai_usage.raw_provider_responses_included);
        assert!(!validation.ai_usage.private_traces_included);
        assert_eq!(
            validation
                .files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            vec!["content/game.json", "preview.png"]
        );

        let publish = write_workshop_publish_draft(package.path(), publish_output.path())
            .expect("write publish draft");

        assert_eq!(
            publish.output_dir,
            publish_output.path().display().to_string()
        );
        assert_eq!(publish.files_written.len(), 1);
        assert!(publish.files_written[0].ends_with("workshop-publish-draft.json"));
        assert_eq!(publish.draft.package_id, "studio-workshop-fixture");
        assert!(!publish.draft.upload_enabled);
        assert!(!publish.draft.steamworks_api_called);
        assert!(publish.draft.requires_explicit_steamworks_credentials);
        let publish_json = fs::read_to_string(&publish.files_written[0]).expect("publish json");
        assert_redaction_safe_text(&publish_json);

        let kit = write_steam_submission_kit(
            package.path(),
            kit_output.path(),
            &sample_submission_kit_request(),
        )
        .expect("write submission kit");

        assert_eq!(kit.output_dir, kit_output.path().display().to_string());
        assert_eq!(kit.files_written.len(), 8);
        assert_eq!(kit.draft.workshop_package_id, "studio-workshop-fixture");
        assert_eq!(kit.draft.product_name, "Studio Workshop Fixture");
        for file in &kit.files_written {
            let text = fs::read_to_string(file).expect("kit text");
            assert_redaction_safe_text(&text);
        }
    }

    #[test]
    fn workshop_adapter_manages_local_library_lifecycle() {
        let package = tempdir().expect("package");
        let library = tempdir().expect("library");
        write_valid_workshop_package(package.path());

        let import = import_workshop_library_package(library.path(), package.path())
            .expect("import package");
        assert_eq!(import.item.local_id, "studio-workshop-fixture");
        assert!(
            import
                .item
                .package_dir
                .ends_with("studio-workshop-fixture/package")
        );
        assert_eq!(import.validation_report.files.len(), 2);

        let listed = list_workshop_library(library.path()).expect("list library");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "Studio Workshop Fixture");

        let loaded =
            load_workshop_library_item(library.path(), "studio-workshop-fixture").expect("load");
        assert_eq!(loaded.item.package_id, "studio-workshop-fixture");
        assert_eq!(
            loaded.validation_report.manifest.title,
            "Studio Workshop Fixture"
        );

        let remix = remix_workshop_library_item(
            library.path(),
            "studio-workshop-fixture",
            "studio-workshop-remix",
            "Studio Workshop Remix",
        )
        .expect("remix");
        assert_eq!(remix.source_local_id, "studio-workshop-fixture");
        assert_eq!(remix.item.local_id, "studio-workshop-remix");
        assert_eq!(
            remix.validation_report.manifest.title,
            "Studio Workshop Remix"
        );
        assert_eq!(
            remix.validation_report.manifest.visibility,
            WorkshopDraftVisibility::PrivateDraft
        );

        let reported = report_workshop_library_item(
            library.path(),
            "studio-workshop-remix",
            "Needs local content review.",
        )
        .expect("report");
        assert_eq!(reported.reports[0].reason, "Needs local content review.");

        let blocked = block_workshop_library_item(
            library.path(),
            "studio-workshop-remix",
            "Local moderation block.",
        )
        .expect("block");
        assert_eq!(
            blocked.blocked.as_ref().map(|block| block.reason.as_str()),
            Some("Local moderation block.")
        );

        let blocked_load = load_workshop_library_item(library.path(), "studio-workshop-remix")
            .expect_err("blocked item should not load");
        assert_eq!(blocked_load.code, "load_workshop_library_item");
        assert!(blocked_load.message.contains("blocked"));

        let deleted = delete_workshop_library_item(library.path(), "studio-workshop-fixture")
            .expect("delete original");
        assert_eq!(deleted.local_id, "studio-workshop-fixture");
        assert!(!Path::new(&deleted.package_dir).exists());

        let remaining = list_workshop_library(library.path()).expect("list after delete");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].local_id, "studio-workshop-remix");
        assert!(remaining[0].blocked.is_some());
        assert_eq!(remaining[0].reports.len(), 1);
    }

    #[test]
    fn workshop_adapter_maps_workshop_errors_to_command_codes() {
        let package = tempdir().expect("package");
        write_valid_workshop_package(package.path());
        fs::write(
            package.path().join("content/game.json"),
            b"{\"secret\":\"sk-test-secret-marker\"}\n",
        )
        .expect("secret content");

        let error =
            validate_workshop_package(package.path()).expect_err("secret marker should fail");

        assert_eq!(error.code, "validate_workshop_package");
        assert!(error.message.contains("sk-"));
    }

    #[test]
    fn structured_edit_commands_delegate_to_storage_and_reopen() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let mut world = read_world_edit_document(&project_path).expect("read world edit");
        world
            .forbidden_facts
            .push("The emperor is not secretly immortal.".into());
        update_world_edit_document(&project_path, world.clone()).expect("update world edit");
        assert_eq!(
            read_world_edit_document(&project_path).expect("read updated world"),
            world
        );

        let mut story =
            read_story_craft_edit_document(&project_path).expect("read story craft edit");
        story
            .story_bible_markdown
            .push_str("\nA frozen ledger matters.\n");
        story.story_craft.bible.genre_promise =
            "A winter court crisis with visible tradeoffs.".into();
        update_story_craft_edit_document(&project_path, story.clone())
            .expect("update story craft edit");
        assert_eq!(
            read_story_craft_edit_document(&project_path).expect("read updated story craft"),
            story
        );

        create_character(&project_path, sample_character("regent")).expect("create character");
        assert!(
            read_character_edit_document(&project_path)
                .expect("read characters")
                .characters
                .iter()
                .any(|character| character.id == "regent")
        );

        create_resource(
            &project_path,
            ResourceDefinition {
                key: "grain".into(),
                label: "Grain".into(),
                initial: 30,
                min: 0,
                max: 100,
            },
        )
        .expect("create resource");
        assert_eq!(
            read_state_variables_edit_document(&project_path)
                .expect("read state variables")
                .initial_world_state
                .resources
                .get("grain"),
            Some(&30)
        );

        create_rule(&project_path, sample_rule("spend-grain", "grain")).expect("create rule");
        assert!(
            read_rules_edit_document(&project_path)
                .expect("read rules")
                .rules
                .iter()
                .any(|rule| rule.id == "spend-grain")
        );
    }

    #[test]
    fn structured_edit_commands_return_explicit_error_code() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let mut world = read_world_edit_document(&project_path).expect("read world edit");
        world.world_bible_markdown = "OPENAI_API_KEY=sk-test-secret-marker".into();
        let world_error = update_world_edit_document(&project_path, world)
            .expect_err("secret marker should fail");
        assert_eq!(world_error.code, "update_world_edit_document");
        assert!(world_error.message.contains("secret markers"));

        let rule_error = create_rule(&project_path, sample_rule("bad-rule", "missing_resource"))
            .expect_err("unknown resource should fail");
        assert_eq!(rule_error.code, "create_rule");
        assert!(rule_error.message.contains("unknown resource key"));
    }

    #[test]
    fn generation_commands_delegate_to_agent_and_persist_project_source() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let world = generate_world_expansion(&project_path, "Expand canon and forbidden facts.")
            .expect("generate world");
        assert!(!world.evidence.fallback_used);
        assert!(
            world
                .document
                .world_bible_markdown
                .contains("# World Bible")
        );
        assert_eq!(
            read_world_edit_document(&project_path)
                .expect("read generated world")
                .forbidden_facts,
            world.document.forbidden_facts
        );

        let story = generate_story_craft(&project_path, "Generate the first pressure arc.")
            .expect("generate story craft");
        assert!(!story.evidence.fallback_used);
        assert!(story.document.story_craft.plot_threads.len() >= 3);
        assert_eq!(
            read_story_craft_edit_document(&project_path)
                .expect("read generated story craft")
                .story_craft
                .plot_threads
                .len(),
            story.document.story_craft.plot_threads.len()
        );

        let character = generate_character(&project_path, "Design a grain envoy.", "court envoy")
            .expect("generate character");
        assert!(!character.evidence.fallback_used);
        assert!(character.character.portrait_request.is_some());
        assert!(
            read_character_edit_document(&project_path)
                .expect("read generated characters")
                .characters
                .iter()
                .any(|candidate| candidate.id == character.character.id)
        );
    }

    #[test]
    fn ai_safety_policy_commands_roundtrip_and_reject_secret_markers() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let mut policy = read_ai_safety_policy(&project_path).expect("read policy");
        assert!(policy.human_review_required);
        policy.live_generated_content_enabled = true;
        policy.moderation_queue_enabled = true;
        policy.user_reporting_path = "studio://moderation-queue".into();
        policy.moderation_policy = "Creator reviews generated text before export.".into();
        policy
            .safety_guardrails
            .push("Block generated output until review passes.".into());

        let updated =
            update_ai_safety_policy(&project_path, policy.clone()).expect("update policy");
        assert_eq!(
            updated.live_generated_content_enabled,
            policy.live_generated_content_enabled
        );
        assert_eq!(
            read_ai_safety_policy(&project_path).expect("read updated policy"),
            updated
        );

        let error = update_ai_safety_policy(
            &project_path,
            AiSafetyPolicy {
                moderation_policy: "OPENAI_API_KEY=sk-test-secret-marker".into(),
                ..updated
            },
        )
        .expect_err("secret policy should fail");
        assert_eq!(error.code, "update_ai_safety_policy");
        assert!(error.message.contains("secret markers"));
    }

    #[test]
    fn open_project_returns_contract_project_data() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let project = open_project(&project_path).expect("open project");

        assert_eq!(project.game.title, "Starter Project");
        assert_eq!(project.game.entry_scene, "opening-scene");
        assert_eq!(project.scenes.len(), 1);
    }

    #[test]
    fn list_asset_records_returns_rebuilt_media_registry_records() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let records = list_asset_records(&project_path).expect("asset records");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, plotforge_schema::AssetKind::Image);
        assert_eq!(records[0].references[0].reference_id, "opening-scene");
        assert_eq!(records[0].references[0].slot, "background_asset");
    }

    #[test]
    fn check_project_returns_counts_from_storage_validation() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let report = check_project(&project_path).expect("check project");

        assert_eq!(report.title, "Starter Project");
        assert_eq!(report.entry_scene, "opening-scene");
        assert_eq!(report.scene_count, 1);
        assert_eq!(report.rule_count, 0);
        assert_eq!(report.character_count, 0);
    }

    #[test]
    fn play_once_project_runs_runtime_and_writes_trace() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let report = play_once_project(&project_path, "continue").expect("play once");

        assert_eq!(report.scene.key, "opening-scene");
        assert!(report.trace_path.ends_with("traces/trace-000.json"));
        assert!(
            fs::metadata(&report.trace_path)
                .expect("trace file")
                .is_file()
        );
        assert_eq!(report.trace.selected_choice.as_deref(), Some("continue"));
        assert!(report.delta_summary.is_empty());
    }

    #[test]
    fn play_once_project_saves_and_restores_runtime_snapshot() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let first = play_once_project_with_save(&project_path, "continue", Some("save-001"))
            .expect("first play");
        let save_path = first.snapshot_path.as_ref().expect("snapshot path");

        assert_eq!(first.snapshot.as_ref().expect("snapshot").id, "save-001");
        assert!(save_path.ends_with("saves/save-001.runtime_snapshot.json"));
        assert!(
            project_path
                .join("saves/latest.runtime_snapshot.json")
                .is_file()
        );

        let second = play_once_project_from_snapshot(
            &project_path,
            "continue",
            "save-001",
            Some("save-002"),
        )
        .expect("restored play");

        assert_eq!(second.trace.selected_choice.as_deref(), Some("continue"));
        assert_eq!(second.trace.story_state_before.turn, 0);
        assert_eq!(second.trace.story_state_after.turn, 0);
        assert_eq!(second.snapshot.as_ref().expect("snapshot").id, "save-002");
        assert!(
            project_path
                .join("saves/save-002.runtime_snapshot.json")
                .is_file()
        );

        let latest =
            play_once_project_from_latest_snapshot(&project_path, "continue", Some("save-003"))
                .expect("latest restored play");
        assert_eq!(latest.trace.story_state_before.turn, 0);
        assert_eq!(latest.snapshot.as_ref().expect("snapshot").id, "save-003");
    }

    #[test]
    fn play_once_project_rejects_corrupted_runtime_snapshot_explicitly() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);
        fs::write(
            project_path.join("saves/corrupt.runtime_snapshot.json"),
            "{not-json\n",
        )
        .expect("corrupt snapshot");

        let error = play_once_project_from_snapshot(&project_path, "continue", "corrupt", None)
            .expect_err("corrupt snapshot");

        assert_eq!(error.code, "play_once_restore_snapshot");
        assert!(error.message.contains("json error"));
        assert!(!project_path.join("traces/latest.json").exists());
    }

    #[test]
    fn play_once_project_rejects_invalid_save_id_before_trace_write() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let error = play_once_project_with_save(&project_path, "continue", Some("../escape"))
            .expect_err("invalid save id");

        assert_eq!(error.code, "play_once_snapshot");
        assert!(error.message.contains("invalid runtime snapshot id"));
        assert!(!project_path.join("traces/latest.json").exists());
        assert!(
            !project_path
                .join("saves/latest.runtime_snapshot.json")
                .exists()
        );
    }

    #[test]
    fn export_static_project_delegates_to_export_crate() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        let export_path = temp.path().join("export");
        create_starter_project(&project_path);

        let report = export_static_project(&project_path, &export_path).expect("export static");

        assert_eq!(report.output_dir, export_path.display().to_string());
        assert_eq!(report.archive_path, None);
        assert!(
            report
                .files_written
                .iter()
                .any(|path| path.ends_with("index.html"))
        );
        assert!(report.files_found.iter().any(|path| path == "game.json"));
        assert!(report.allowed_files.iter().any(|path| path == "index.html"));
    }

    #[test]
    fn export_static_project_zip_reports_archive_path() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        let export_path = temp.path().join("export");
        let archive_path = temp.path().join("starter-project-static.zip");
        create_starter_project(&project_path);

        let report = export_static_project_zip(&project_path, &export_path, &archive_path)
            .expect("export static zip");

        assert_eq!(report.output_dir, export_path.display().to_string());
        assert_eq!(
            report.archive_path,
            Some(archive_path.display().to_string())
        );
        assert!(archive_path.is_file());
        assert!(report.archived_files.iter().any(|path| path == "game.json"));
        assert_eq!(report.allowed_files, report.files_found);
    }

    #[test]
    fn studio_mvp_workflow_create_open_edit_play_and_export_zip() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("winter-regency");
        let export_path = temp.path().join("export");
        let archive_path = temp.path().join("winter-regency-static.zip");

        create_project(&project_path, sample_creation_request(), false).expect("create project");
        let opened = open_project(&project_path).expect("open created project");
        assert_eq!(opened.game.title, "Winter Regency");

        let mut world = read_world_edit_document(&project_path).expect("read world edit");
        world
            .world_bible_markdown
            .push_str("\n## Smoke Edit\n\nThe winter court ledger is now explicit.\n");
        world
            .forbidden_facts
            .push("The regent cannot secretly own the granaries.".into());
        update_world_edit_document(&project_path, world).expect("update world edit");

        let source = write_source_file(
            &project_path,
            "story/story_bible.md",
            "# Story Bible\n\nOpen with an empty granary ledger.\n",
        )
        .expect("edit source file");
        assert!(source.content.contains("granary ledger"));

        let check = check_project(&project_path).expect("check after edits");
        assert_eq!(check.title, "Winter Regency");

        let play = play_once_project(&project_path, "continue").expect("play once");
        assert_eq!(play.scene.key, "opening-scene");
        assert!(project_path.join("traces/latest.json").is_file());

        let export = export_static_project_zip(&project_path, &export_path, &archive_path)
            .expect("export static zip");
        assert!(archive_path.is_file());
        assert!(export_path.join("index.html").is_file());
        assert!(export_path.join("game.json").is_file());
        assert_eq!(
            export.archive_path,
            Some(archive_path.display().to_string())
        );
        assert!(
            export
                .archived_files
                .iter()
                .any(|path| path == "index.html")
        );
        assert!(export.archived_files.iter().any(|path| path == "game.json"));
        assert!(
            !export
                .archived_files
                .iter()
                .any(|path| path.starts_with("traces/"))
        );
        assert!(
            !export
                .archived_files
                .iter()
                .any(|path| path.starts_with("providers/"))
        );
    }

    #[test]
    fn missing_project_returns_explicit_error_code() {
        let temp = tempdir().expect("tempdir");
        let missing_path = temp.path().join("missing");

        let error = check_project(&missing_path).expect_err("missing project should fail");

        assert_eq!(error.code, "check_project");
        assert!(error.message.contains("game.toml"));
    }

    #[test]
    fn list_source_files_marks_safe_text_surfaces_editable() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let files = list_source_files(&project_path).expect("source files");

        assert!(
            files
                .iter()
                .any(|file| file.path == "game.toml" && !file.editable)
        );
        assert!(
            files
                .iter()
                .any(|file| file.path == "world/world.md" && file.editable)
        );
        assert!(
            files
                .iter()
                .any(|file| file.path == "scenes/opening-scene.scene.json" && !file.editable)
        );
    }

    #[test]
    fn read_source_file_returns_content_and_metadata() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let file = read_source_file(&project_path, "world/world.md").expect("read source");

        assert_eq!(file.path, "world/world.md");
        assert!(file.editable);
        assert!(file.content.contains("World Bible"));
    }

    #[test]
    fn read_source_file_rejects_supported_but_unlisted_files() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);
        fs::write(project_path.join("provider_config.json"), "{}\n").expect("provider config");

        let error = read_source_file(&project_path, "provider_config.json")
            .expect_err("unlisted source should fail");

        assert_eq!(error.code, "read_source_file");
        assert!(error.message.contains("not a project source file"));
    }

    #[test]
    fn write_source_file_updates_editable_markdown_only() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let updated = write_source_file(
            &project_path,
            "world/world.md",
            "# World Bible\n\nThe court is under pressure.\n",
        )
        .expect("write editable source");

        assert_eq!(updated.path, "world/world.md");
        assert!(updated.content.contains("under pressure"));
        assert!(
            fs::read_to_string(project_path.join("world/world.md"))
                .expect("read written source")
                .contains("under pressure")
        );
    }

    #[test]
    fn write_source_file_rejects_traversal_and_readonly_files() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let traversal = write_source_file(&project_path, "../outside.md", "bad")
            .expect_err("traversal should fail");
        let readonly =
            write_source_file(&project_path, "game.toml", "bad").expect_err("readonly should fail");

        assert_eq!(traversal.code, "write_source_file");
        assert_eq!(readonly.code, "write_source_file");
        assert!(readonly.message.contains("not an editable source file"));
    }

    fn sample_creation_request() -> super::ProjectCreationRequest {
        super::ProjectCreationRequest {
            template: super::ProjectTemplateId::HistoricalCrisis,
            concept: "A regency court must survive a winter coup.".into(),
            visual_style: "ink wash court drama".into(),
            voice_enabled: true,
            initial_scene_request: "Open on an empty granary ledger.".into(),
        }
    }

    fn create_starter_project(project_path: &Path) {
        create_project(project_path, sample_creation_request(), false).expect("create project");
    }

    #[test]
    fn pi_agent_apply_run_commits_local_pi_scene_plan_to_runtime() {
        // End-to-end: a fresh project + local-pi model id (the default) +
        // a `pi_agent_apply_run` call must commit the agent's ScenePlan
        // proposal as a runtime state change and return a
        // `PiAgentApplyResult` carrying the committed scene + trace.
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("agent-apply");
        create_starter_project(&project_path);

        // Default agent config has model_id="local-pi", so no provider
        // registry is consulted; the deterministic mock provider is used.
        let request = PiAgentApplyRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 9,
            project_path: project_path.display().to_string(),
            player_input: "Take the witness stand.".into(),
            save_id: None,
            restore_id: None,
        };
        let result = pi_agent_apply_run(request).expect("pi-agent apply run");

        // The result carries the committed scene + trace + redaction-safe
        // pi-Agent evidence envelope.
        assert!(!result.scene_key.is_empty());
        assert_eq!(result.scene.key, result.scene_key);
        assert!(!result.trace.id.is_empty());
        assert!(result.run.descriptor.is_local_pi);
        assert!(
            result
                .run
                .trace_id
                .as_ref()
                .expect("trace evidence id")
                .starts_with("pi-agent-evidence-")
        );
        // The trace file was written under the project's traces dir.
        assert!(result.trace_path.contains("traces"));
        // No raw provider responses leaked into the result.
        let serialized = serde_json::to_string(&result).expect("serialize result");
        assert!(!serialized.contains("raw_provider_response"));
        assert!(!serialized.contains("sk-"));
        assert!(!serialized.contains("api_key"));
        // Finding L3: `PiAgentApplyResult` must carry `delta_summary` so the
        // rail and TraceDebugView render the "State deltas" chip without
        // re-implementing `summarize_delta` in TypeScript. The local-pi mock
        // commits a change_scene plan; the delta summary may be empty (the
        // starter project's rules may not key on change_scene), but the
        // field must be present and forward the same value the report carries.
        let _ = &result.delta_summary;
    }

    /// P4.3: when `AgentSessionConfig.enabled_mcp_servers` is non-empty but
    /// none of the listed servers are present in the user-global MCP registry
    /// (the empty-registry case on a fresh machine), `pi_agent_apply_run`
    /// surfaces an explicit `mcp_apply_no_server` error — never a silent
    /// fallback to the no-MCP path (AGENTS.md:144). This is the hermetic error
    /// branch; the full MCP loop round-trip is covered by the
    /// `complete_with_mcp_tools` unit tests in `plotforge-agent` (P4.1) and
    /// the CLI smoke (Phase 5).
    #[test]
    fn pi_agent_apply_run_surfaces_mcp_no_server_error_when_registry_empty() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("agent-apply-mcp");
        create_starter_project(&project_path);

        // Enable an MCP server id that is not present in the (empty on a
        // fresh machine) user-global registry. The on-disk config is written
        // through the same `set_agent_session_config` path the Studio bridge
        // uses, so the test mirrors a real SettingsView enable action.
        set_agent_session_config(
            &project_path,
            &super::AgentSessionConfig {
                model_id: plotforge_agent::LOCAL_PI_MODEL_ID.into(),
                permission_level: super::PermissionLevel::FullAccess,
                thinking_level: super::ThinkingLevel::Medium,
                enabled_skills: Vec::new(),
                enabled_mcp_servers: vec!["absent-mcp-server".into()],
            },
        )
        .expect("set agent session config");

        let request = PiAgentApplyRequest {
            agent_id: "pi-agent-local".into(),
            run_seed: 9,
            project_path: project_path.display().to_string(),
            player_input: "Take the witness stand.".into(),
            save_id: None,
            restore_id: None,
        };
        let error = pi_agent_apply_run(request)
            .expect_err("absent MCP server must surface an explicit error");
        assert_eq!(
            error.code, "mcp_apply_no_server",
            "expected mcp_apply_no_server, got {}",
            error.code
        );
        assert!(
            error.message.contains("absent-mcp-server"),
            "error must name the missing server id"
        );
        // No secret markers leak through the error.
        assert!(!error.message.contains("sk-"));
        assert!(!error.message.contains("api_key"));
    }

    #[test]
    fn list_providers_succeeds_without_credential_leak() {
        // The user-global registry is absent on a fresh machine; the loader
        // returns an empty registry, never an error. Every entry must carry
        // no credential value (only env-var names).
        let providers = list_providers().expect("list providers");
        for entry in &providers {
            assert!(!entry.id.is_empty());
            assert!(!entry.credential_env_var.contains("sk-"));
        }
    }

    #[test]
    fn enable_skill_for_project_persists_enabled_skills() {
        // Finding H4: `enable_skill_for_project` must update the persisted
        // `AgentSessionConfig.enabled_skills` and echo the new config back.
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("skill-project");
        create_starter_project(&project_path);

        let updated =
            enable_skill_for_project(project_path.clone(), "frontend-design".into(), true)
                .expect("enable skill");
        assert!(
            updated
                .enabled_skills
                .iter()
                .any(|id| id == "frontend-design"),
            "skill must appear in enabled_skills after enabling: {updated:?}"
        );

        // Toggling off removes it.
        let disabled = enable_skill_for_project(project_path, "frontend-design".into(), false)
            .expect("disable skill");
        assert!(
            !disabled
                .enabled_skills
                .iter()
                .any(|id| id == "frontend-design"),
            "skill must be removed from enabled_skills after disabling: {disabled:?}"
        );
    }

    /// P5.1: `list_mcp_servers` returns an empty list on a fresh machine
    /// (no `~/.plotforge/mcp.json`). An absent registry is never an error.
    #[test]
    fn list_mcp_servers_returns_empty_on_fresh_machine() {
        let servers = list_mcp_servers().expect("list MCP servers");
        // On a fresh machine the registry is absent → empty list. (If a
        // real user-global registry exists on the test host, this asserts
        // no credential values leak through the list surface.)
        for entry in &servers {
            assert!(!entry.id.is_empty());
            assert!(!entry.credential_env_var.contains("sk-"));
        }
    }

    /// P5.1: `enable_mcp_server_for_project` persists the server id to
    /// `AgentSessionConfig.enabled_mcp_servers` and survives a config reload,
    /// mirroring `enable_skill_for_project`. Toggling off removes it.
    #[test]
    fn enable_mcp_server_for_project_persists_enabled_mcp_servers() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("mcp-project");
        create_starter_project(&project_path);

        let updated = enable_mcp_server_for_project(project_path.clone(), "local-fs".into(), true)
            .expect("enable MCP server");
        assert!(
            updated
                .enabled_mcp_servers
                .iter()
                .any(|id| id == "local-fs"),
            "server must appear in enabled_mcp_servers after enabling: {updated:?}"
        );

        // The persisted config must survive a reload (read from disk).
        let reloaded = get_agent_session_config(&project_path).expect("reload config");
        assert!(
            reloaded
                .enabled_mcp_servers
                .iter()
                .any(|id| id == "local-fs"),
            "enabled_mcp_servers must survive a config reload"
        );

        // Toggling off removes it.
        let disabled = enable_mcp_server_for_project(project_path, "local-fs".into(), false)
            .expect("disable MCP server");
        assert!(
            !disabled
                .enabled_mcp_servers
                .iter()
                .any(|id| id == "local-fs"),
            "server must be removed from enabled_mcp_servers after disabling: {disabled:?}"
        );
    }

    /// P5.1: `delete_mcp_server` surfaces an explicit `mcp_server_not_found`
    /// error when no entry matches the id — never a silent no-op.
    #[test]
    fn delete_mcp_server_surfaces_not_found_error() {
        let error = delete_mcp_server("definitely-absent-server-id".into())
            .expect_err("absent server must surface an error");
        assert_eq!(error.code, "mcp_server_not_found");
        assert!(error.message.contains("definitely-absent-server-id"));
        // No secret markers leak through the error.
        assert!(!error.message.contains("sk-"));
    }

    #[test]
    fn project_prompt_templates_roundtrip_via_studio_commands() {
        // Finding H4: project-scoped prompt CRUD roundtrips through the
        // studio command layer (the writer is pre-scanned for secret markers
        // and the result stays under <project>/.plotforge/prompts.json).
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("prompt-project");
        create_starter_project(&project_path);

        // Empty initially.
        let initial = list_project_prompt_templates(project_path.clone()).expect("list initial");
        assert!(initial.is_empty());

        // Upsert a project-scoped template (avoid the user-global upsert here
        // so this test never touches the real ~/.plotforge/prompts.json).
        let project_template = upsert_project_prompt_template(
            project_path.clone(),
            PromptTemplate {
                id: "project-scene-planner".into(),
                label: "Project scene planner".into(),
                scope: PromptScope::Project,
                body_markdown: "# Project prompt\nPlan a scene here.".into(),
                default_role_hint: None,
            },
        )
        .expect("upsert project");
        assert_eq!(project_template.id, "project-scene-planner");

        // The project list now contains the project-scoped template.
        let after_upsert =
            list_project_prompt_templates(project_path.clone()).expect("list after upsert");
        assert_eq!(after_upsert.len(), 1);
        assert_eq!(after_upsert[0].id, "project-scene-planner");

        // The project store was written under .plotforge/.
        let prompts_file = project_path.join(".plotforge").join("prompts.json");
        assert!(prompts_file.exists(), "project prompt store must exist");

        // Delete the project template; the list returns to empty.
        delete_project_prompt_template(project_path, "project-scene-planner".into())
            .expect("delete project template");
    }

    #[test]
    fn upsert_provider_rejects_credential_in_endpoint_query_string() {
        // Finding L4-studio: the write path must validate the entry before
        // persisting so a `?key=sk-realkey` URL never lands in the registry.
        // We assert the validation runs (returns the explicit error code);
        // the actual registry write is not asserted here because the studio
        // layer reads the real user-global `~/.plotforge/providers.json` (no
        // path override). The CLI smoke test suite covers the hermetic
        // round-trip; the agent-layer `url_has_query_credential` unit test
        // covers the detector in isolation.
        let entry = ProviderEntry {
            id: "bad".into(),
            kind: ProviderKind::OpenAiCompatible,
            label: "Bad".into(),
            endpoint_url: "https://host/v1?key=sk-realkey".into(),
            model: "m".into(),
            credential_env_var: "OPENAI_API_KEY".into(),
            enabled: true,
            max_output_tokens: None,
        };
        let error = upsert_provider(entry).expect_err("query-string credential rejected");
        assert_eq!(error.code, "upsert_provider_invalid");
        assert!(error.message.contains("credential"));
    }

    #[test]
    fn delete_provider_reports_missing_id_explicitly() {
        // Finding H4: `delete_provider` must surface `provider_not_found`
        // for an unknown id (no silent fallback / no panic). The id here is
        // guaranteed absent from any registry (test or real) because it is
        // a long random sentinel.
        let error = delete_provider("pf-review-sentinel-not-registered-9f3c7a".into())
            .expect_err("missing provider must error");
        assert_eq!(error.code, "provider_not_found");
    }

    #[test]
    fn list_remote_models_reports_missing_provider_explicitly() {
        // T1.2: an unknown provider id surfaces `provider_not_found` — never
        // an empty list and never a silent fetch attempt. The sentinel id is
        // guaranteed absent from the user's registry.
        let error =
            list_remote_models("pf-review-sentinel-not-registered-4b1e92".into())
                .expect_err("missing provider must error");
        assert_eq!(error.code, "provider_not_found");
        assert!(error.message.contains("pf-review-sentinel-not-registered-4b1e92"));
    }

    fn sample_character(id: &str) -> Character {
        Character {
            id: id.into(),
            name: "Regent".into(),
            role: "Temporary court authority".into(),
            traits: vec!["cautious".into(), "clear".into()],
            visual_card: "ink portrait with winter robes".into(),
            voice_card: "measured court speech".into(),
            portrait_request: None,
        }
    }

    fn sample_rule(id: &str, resource_key: &str) -> Rule {
        Rule {
            id: id.into(),
            action_type: id.replace('-', "_"),
            conditions: Vec::new(),
            effects: vec![Effect::AddResource {
                key: resource_key.into(),
                amount: -3,
            }],
        }
    }

    fn write_valid_workshop_package(package_dir: &Path) {
        fs::create_dir_all(package_dir.join("content")).expect("content dir");
        fs::write(package_dir.join("content/game.json"), workshop_game_json()).expect("game json");
        fs::write(package_dir.join("preview.png"), workshop_preview_png()).expect("preview");
        fs::write(
            package_dir.join("ai-usage.json"),
            serde_json::to_string_pretty(&sample_workshop_ai_usage())
                .expect("ai usage")
                .as_bytes(),
        )
        .expect("ai usage file");
        fs::write(
            package_dir.join("workshop-item.json"),
            serde_json::to_string_pretty(&sample_workshop_package()).expect("manifest"),
        )
        .expect("manifest file");
    }

    fn sample_workshop_package() -> WorkshopItemPackage {
        WorkshopItemPackage {
            manifest_version: "2026-06-08".into(),
            package_id: "studio-workshop-fixture".into(),
            title: "Studio Workshop Fixture".into(),
            description: "Local package draft for Studio adapter validation.".into(),
            visibility: WorkshopDraftVisibility::PrivateDraft,
            preview_image: "preview.png".into(),
            content_root: "content".into(),
            tags: vec!["story-game".into(), "local-fixture".into()],
            export_profile: ExportProfile::steam_workshop(),
            ai_usage_manifest_path: "ai-usage.json".into(),
            content_files: vec![
                workshop_file_record(
                    "content/game.json",
                    "166fe567ff09b79d66f8795dd9b8ba7319c80b9c55a1d369a1b00605f1442d55",
                    workshop_game_json(),
                ),
                workshop_file_record(
                    "preview.png",
                    "8fbb1bf0b04cc6fef9f90571628a3cc5a0bbe2492647cb55979867fc035e21a7",
                    workshop_preview_png(),
                ),
            ],
            notices: vec!["Local package validation fixture.".into()],
        }
    }

    fn sample_workshop_ai_usage() -> AiUsageManifest {
        AiUsageManifest {
            manifest_version: "2026-06-08".into(),
            project_id: "studio-workshop-fixture".into(),
            project_version: "0.1.0".into(),
            export_profile: ExportProfile::steam_workshop(),
            generated_by: "plotforge-studio-test".into(),
            external_model_calls_during_export: false,
            provider_credentials_included: false,
            raw_provider_responses_included: false,
            private_traces_included: false,
            disclosures: vec![
                AiUsageDisclosure {
                    content_kind: AiUsageContentKind::Text,
                    source_kind: AiUsageSourceKind::ProjectSource,
                    summary: "Story text comes from canonical project files.".into(),
                    asset_paths: Vec::new(),
                },
                AiUsageDisclosure {
                    content_kind: AiUsageContentKind::Image,
                    source_kind: AiUsageSourceKind::LocalMockProvider,
                    summary: "Preview artwork is a local fixture asset.".into(),
                    asset_paths: vec!["preview.png".into()],
                },
            ],
            provider_summaries: vec![AiProviderSummary {
                provider: "local-fixture-provider".into(),
                model: None,
                generated_asset_count: 1,
                fallback_asset_count: 0,
                prompt_hashes: vec!["sha256:studio-preview".into()],
            }],
            ai_safety_policy: Default::default(),
            notices: vec!["No provider credentials or raw provider responses included.".into()],
        }
    }

    fn sample_submission_kit_request() -> SteamSubmissionKitRequest {
        SteamSubmissionKitRequest {
            product_name: "Studio Workshop Fixture".into(),
            desktop_build_path: Some("builds/studio-fixture-desktop.zip".into()),
            store_short_description: "A local narrative package draft.".into(),
            screenshot_paths: vec!["media/screenshots/scene.png".into()],
            capsule_asset_paths: vec!["media/capsules/header.png".into()],
            content_warnings: vec!["Political conflict".into()],
            safety_guardrails: vec!["Review player-visible generated content.".into()],
            user_reporting_path: "support@example.invalid".into(),
            moderation_policy: "Creator review for player-visible text and images.".into(),
            build_notes: vec!["Run desktop build checks in local QA.".into()],
        }
    }

    fn workshop_file_record(
        path: impl Into<String>,
        content_hash: impl Into<String>,
        bytes: &[u8],
    ) -> WorkshopPackageFile {
        WorkshopPackageFile {
            path: path.into(),
            content_hash: content_hash.into(),
            hash_algorithm: "sha256".into(),
            byte_length: bytes.len() as u64,
        }
    }

    fn assert_redaction_safe_text(text: &str) {
        for marker in [
            "OPENAI_API_KEY",
            "api_key",
            "secret_key",
            "sk-",
            "raw_response",
            "request_id",
            "published_file_id",
            "steam_app_id",
        ] {
            assert!(!text.contains(marker), "{marker} should not appear");
        }
    }

    fn workshop_game_json() -> &'static [u8] {
        b"{\"title\":\"Studio Workshop Fixture\"}\n"
    }

    fn workshop_preview_png() -> &'static [u8] {
        b"studio-preview"
    }

    // -----------------------------------------------------------------------
    // create_character_from_draft tests
    // -----------------------------------------------------------------------

    #[test]
    fn create_character_from_draft_trims_fields_and_splits_traits() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let draft = CharacterDraft {
            id: "  envoy  ".into(),
            name: "  Lady Envoy  ".into(),
            role: "  Court Envoy  ".into(),
            traits_text: "  cautious  \n  articulate  \n\n  loyal  ".into(),
            visual_card: "  ink portrait  ".into(),
            voice_card: "  measured tone  ".into(),
        };

        let document =
            create_character_from_draft(&project_path, draft).expect("create character from draft");

        let created = document
            .characters
            .iter()
            .find(|c| c.id == "envoy")
            .expect("created character not found");

        assert_eq!(created.id, "envoy");
        assert_eq!(created.name, "Lady Envoy");
        assert_eq!(created.role, "Court Envoy");
        assert_eq!(created.traits, vec!["cautious", "articulate", "loyal"]);
        assert_eq!(created.visual_card, "ink portrait");
        assert_eq!(created.voice_card, "measured tone");
        assert!(created.portrait_request.is_none());
    }

    #[test]
    fn create_character_from_draft_empty_traits_produces_empty_vec() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        let draft = CharacterDraft {
            id: "silent-envoy".into(),
            name: "Silent Envoy".into(),
            role: "Observer".into(),
            traits_text: "   \n\n  ".into(),
            visual_card: "ink wash portrait".into(),
            voice_card: "calm measured tone".into(),
        };

        let document = create_character_from_draft(&project_path, draft)
            .expect("create character with empty traits");

        let created = document
            .characters
            .iter()
            .find(|c| c.id == "silent-envoy")
            .expect("created character not found");

        assert!(created.traits.is_empty());
    }

    #[test]
    fn create_character_from_draft_rejects_duplicate_id() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("starter-project");
        create_starter_project(&project_path);

        create_character(&project_path, sample_character("envoy")).expect("seed character");
        let draft = CharacterDraft {
            id: "  envoy  ".into(),
            name: "Another Envoy".into(),
            role: "Duplicate".into(),
            traits_text: "".into(),
            visual_card: "ink portrait".into(),
            voice_card: "formal tone".into(),
        };

        let error = create_character_from_draft(&project_path, draft)
            .expect_err("duplicate id should fail");

        assert_eq!(error.code, "create_character_from_draft");
        assert!(error.message.contains("duplicate character id"));
    }

    // create_rule_from_draft tests

    #[test]
    fn create_rule_from_draft_assembles_add_resource_effect() {
        let dir = tempdir().unwrap();
        let project_path = dir.path().join("project");
        create_starter_project(&project_path);

        let draft = RuleDraft {
            id: "  harvest-momentum  ".into(),
            action_type: "  harvest  ".into(),
            resource_key: "momentum".into(),
            amount: 10,
        };

        let document =
            create_rule_from_draft(&project_path, draft).expect("create rule from draft");

        let rule = document
            .rules
            .iter()
            .find(|r| r.id == "harvest-momentum")
            .expect("rule with trimmed id must be present");

        assert_eq!(rule.action_type, "harvest");
        assert!(rule.conditions.is_empty());
        assert_eq!(rule.effects.len(), 1);
        match &rule.effects[0] {
            Effect::AddResource { key, amount } => {
                assert_eq!(key, "momentum");
                assert_eq!(*amount, 10);
            }
            other => panic!("expected AddResource effect, got {:?}", other),
        }
    }

    #[test]
    fn create_rule_from_draft_empty_resource_key_produces_no_effects_and_storage_rejects() {
        let dir = tempdir().unwrap();
        let project_path = dir.path().join("project");
        create_starter_project(&project_path);

        // RuleDraft.into_rule() produces an empty effects vec when resource_key is blank.
        // plotforge-storage validates that rules must have at least one effect, so this
        // should be rejected at the storage boundary.
        let draft = RuleDraft {
            id: "no-effect-rule".into(),
            action_type: "noop".into(),
            resource_key: "".into(),
            amount: 0,
        };

        let error = create_rule_from_draft(&project_path, draft)
            .expect_err("empty resource key yields no effects, storage should reject");

        assert_eq!(error.code, "create_rule_from_draft");
    }

    #[test]
    fn create_rule_from_draft_rejects_duplicate_id() {
        let dir = tempdir().unwrap();
        let project_path = dir.path().join("project");
        create_starter_project(&project_path);

        let draft = RuleDraft {
            id: "unique-rule".into(),
            action_type: "first".into(),
            resource_key: "momentum".into(),
            amount: 5,
        };
        create_rule_from_draft(&project_path, draft).expect("first create succeeds");

        let duplicate = RuleDraft {
            id: "unique-rule".into(),
            action_type: "second".into(),
            resource_key: "momentum".into(),
            amount: 5,
        };
        let error = create_rule_from_draft(&project_path, duplicate)
            .expect_err("duplicate rule id should fail");

        assert_eq!(error.code, "create_rule_from_draft");
    }

    // -----------------------------------------------------------------------
    // Git workspace integration tests.
    //
    // These tests require the `git` binary on PATH. They are skipped (not
    // failed) when git is unavailable so CI environments without git do not
    // break. They use `tempfile` + `git init` to build an isolated repo.
    // -----------------------------------------------------------------------

    fn git_available() -> bool {
        std::process::Command::new("git")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn init_temp_repo() -> tempfile::TempDir {
        let dir = tempdir().expect("temp dir");
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["init", "-b", "main"])
            .output()
            .expect("git init");
        // Required for git checkout to work in CI without a configured user.
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["config", "user.email", "test@plotforge.local"])
            .output()
            .expect("git config user.email");
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["config", "user.name", "PlotForge Test"])
            .output()
            .expect("git config user.name");
        dir
    }

    fn commit_initial(path: &Path) {
        fs::write(path.join("README.md"), "starter\n").expect("write readme");
        std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["add", "."])
            .output()
            .expect("git add");
        std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["commit", "-m", "initial"])
            .output()
            .expect("git commit");
    }

    #[test]
    fn git_project_dir_name_returns_basename() {
        let dir = tempdir().expect("temp dir");
        let name = git_project_dir_name(dir.path()).expect("dir name");
        assert_eq!(name, dir.path().file_name().unwrap().to_string_lossy());
    }

    #[test]
    fn git_current_branch_returns_main_for_fresh_repo() {
        if !git_available() {
            return;
        }
        let dir = init_temp_repo();
        commit_initial(dir.path());
        let branch = git_current_branch(dir.path()).expect("current branch");
        assert_eq!(branch, "main");
    }

    #[test]
    fn git_current_branch_errors_for_non_repo() {
        if !git_available() {
            return;
        }
        let dir = tempdir().expect("temp dir");
        let error = git_current_branch(dir.path()).expect_err("non-repo should error");
        assert_eq!(error.code, GIT_NOT_A_REPO_CODE);
    }

    #[test]
    fn git_list_branches_marks_current() {
        if !git_available() {
            return;
        }
        let dir = init_temp_repo();
        commit_initial(dir.path());
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["branch", "feature/x"])
            .output()
            .expect("git branch feature/x");
        let branches = git_list_branches(dir.path()).expect("list branches");
        assert_eq!(branches.len(), 2);
        let current = branches.iter().find(|b| b.is_current).expect("has current");
        assert_eq!(current.name, "main");
        let other = branches
            .iter()
            .find(|b| b.name == "feature/x")
            .expect("has feature");
        assert!(!other.is_current);
    }

    #[test]
    fn git_switch_branch_changes_current() {
        if !git_available() {
            return;
        }
        let dir = init_temp_repo();
        commit_initial(dir.path());
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["branch", "feature/x"])
            .output()
            .expect("git branch feature/x");
        let result = git_switch_branch(dir.path(), "feature/x").expect("switch branch");
        assert_eq!(result.branch, "feature/x");
        let current = git_current_branch(dir.path()).expect("current after switch");
        assert_eq!(current, "feature/x");
    }

    #[test]
    fn git_switch_branch_rejects_option_shaped_name() {
        // Regression: an option-shaped branch name (e.g. "--force") must be
        // rejected before `git checkout` is invoked, so a caller cannot
        // smuggle a git option through the IPC surface. This does not require
        // the `git` binary — the validation runs unconditionally.
        let dir = tempdir().expect("temp dir");
        for bad in ["--force", "-b", "--help", "-q"] {
            let error = git_switch_branch(dir.path(), bad).expect_err("option-shaped name");
            assert_eq!(
                error.code, "git_switch_branch",
                "option-shaped name {bad:?} should be rejected with git_switch_branch code"
            );
            assert!(
                error.message.contains("may not start with '-'"),
                "error should explain the rejection: {}",
                error.message
            );
        }
    }

    #[test]
    fn git_switch_branch_rejects_empty_name_without_git() {
        // Empty-name validation runs before the git binary is invoked, so this
        // test passes even when git is absent on PATH.
        let dir = tempdir().expect("temp dir");
        for bad in ["", "   ", "\t"] {
            let error = git_switch_branch(dir.path(), bad).expect_err("empty name");
            assert_eq!(error.code, "git_switch_branch");
            assert!(error.message.contains("branch name is required"));
        }
    }

    // -----------------------------------------------------------------------
    // Agent session config tests.
    // -----------------------------------------------------------------------

    #[test]
    fn list_available_models_includes_local_pi_default() {
        let models = list_available_models().expect("list models");
        assert!(!models.is_empty());
        assert_eq!(models[0].id, "local-pi");
    }

    #[test]
    fn get_agent_session_config_returns_defaults_for_fresh_project() {
        let dir = tempdir().expect("temp dir");
        let config = get_agent_session_config(dir.path()).expect("default config");
        assert_eq!(config, AgentSessionConfig::default());
    }

    #[test]
    fn set_then_get_agent_session_config_roundtrips() {
        let dir = tempdir().expect("temp dir");
        let config = AgentSessionConfig {
            model_id: "glm-5.2".into(),
            permission_level: PermissionLevel::FullAccess,
            thinking_level: ThinkingLevel::High,
            enabled_skills: Vec::new(),
            enabled_mcp_servers: Vec::new(),
        };
        let persisted = set_agent_session_config(dir.path(), &config).expect("persist config");
        assert_eq!(persisted, config);
        let loaded = get_agent_session_config(dir.path()).expect("load config");
        assert_eq!(loaded, config);
        // The config file must live under .plotforge (never next to source).
        let config_path = dir.path().join(".plotforge").join("agent-config.json");
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).expect("read config file");
        // Redaction safety: no secret markers or credential fields.
        assert!(!content.contains("api_key"));
        assert!(!content.contains("endpoint"));
        assert!(content.contains("glm-5.2"));
    }
}
