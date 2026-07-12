//! P5.4 (#188) — Export-package leak guard for MCP configuration.
//!
//! Defence-in-depth integration tests asserting that MCP server
//! configuration never leaks into a static or desktop export bundle:
//! - the project-scoped `.plotforge/agent-config.json` (which carries
//!   `enabled_mcp_servers`) is excluded by the existing `.plotforge/`
//!   denylist, and no file content in the export references `mcp`,
//!   `mcp.json`, or `enabled_mcp_servers`;
//! - the user-global `~/.plotforge/mcp.json` registry never touches
//!   project paths, so it cannot enter an export bundle by construction
//!   — but we assert it explicitly when a project has MCP enabled.
//!
//! These tests assert only "MCP config absent from export bundle"; they do
//! not re-assert the broader allowlist/denylist behavior covered by the
//! unit tests in `crates/plotforge-export/src/lib.rs` and the sibling
//! `static_export.rs` integration tests.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use plotforge_export::{export_desktop_runtime_draft, export_static_web, export_static_web_zip};
use plotforge_schema::{
    AI_USAGE_MANIFEST_FILE, AgentSessionConfig, ProjectCreationRequest, ProjectTemplateId,
};

mod common;

fn expected_asset_free_export_files() -> BTreeSet<PathBuf> {
    common::expected_export_files()
        .into_iter()
        .filter(|path| path != Path::new("assets/generated/placeholder.png"))
        .collect()
}

fn expected_asset_free_desktop_files() -> BTreeSet<PathBuf> {
    common::expected_desktop_export_files()
        .into_iter()
        .filter(|path| path != Path::new("assets/generated/placeholder.png"))
        .collect()
}

/// A redaction-safe `AgentSessionConfig` fixture with MCP enabled, mirroring
/// the per-project `.plotforge/agent-config.json` shape persisted by the
/// Studio `set_agent_session_config` command.
fn agent_config_with_mcp_enabled() -> AgentSessionConfig {
    AgentSessionConfig {
        model_id: "local-pi".into(),
        permission_level: Default::default(),
        thinking_level: Default::default(),
        enabled_skills: Vec::new(),
        enabled_mcp_servers: vec!["local-fs".into()],
    }
}

/// A redaction-safe snapshot of user-global `~/.plotforge/mcp.json` content.
/// This never touches project paths in production; it is embedded here only
/// so the leak guard can prove the export never copies it even when a project
/// has MCP enabled. It carries no secrets — only a `credential_env_var` name.
const USER_GLOBAL_MCP_REGISTRY_SNIPPET: &str = r#"{
  "version": "2026-07-06",
  "servers": [
    {
      "id": "local-fs",
      "kind": "stdio",
      "label": "Local filesystem MCP server",
      "transport_config": { "command": "mcp-server-local-fs", "args": [] },
      "credential_env_var": "MCP_LOCAL_FS_TOKEN",
      "enabled": true
    }
  ]
}"#;

fn create_starter_project(project_path: &Path) {
    plotforge_storage::create_project_from_request(
        project_path,
        ProjectCreationRequest {
            template: ProjectTemplateId::HistoricalCrisis,
            concept: "A local starter project for export leak-guard tests.".into(),
            visual_style: "clear readable test style".into(),
            voice_enabled: false,
            initial_scene_request: "A creator opens a fresh PlotForge project.".into(),
        },
        false,
    )
    .expect("create starter project");
}

/// Write a per-project `.plotforge/agent-config.json` with MCP enabled and a
/// stray user-global `mcp.json` snapshot somewhere a leaky export could pick
/// it up. Both files are redaction-safe (no secret values, only env-var
/// names). The leak guard proves neither reaches the export bundle.
fn seed_mcp_config_in_project(project_path: &Path) {
    let plotforge_dir = project_path.join(".plotforge");
    fs::create_dir_all(&plotforge_dir).expect("plotforge dir");
    let agent_config = agent_config_with_mcp_enabled();
    let agent_config_json = serde_json::to_string_pretty(&agent_config).expect("agent config");
    fs::write(plotforge_dir.join("agent-config.json"), agent_config_json)
        .expect("write agent-config.json");
    // A stray user-global-style mcp.json placed inside the project tree. In
    // production this lives at ~/.plotforge/mcp.json and never touches the
    // project; here we drop a copy in the project root to prove the export
    // allowlist never admits it regardless of where a leaky writer might
    // place it.
    fs::write(
        project_path.join("mcp.json"),
        USER_GLOBAL_MCP_REGISTRY_SNIPPET,
    )
    .expect("write stray mcp.json");
}

/// Recursively collect every file under `root` as a path relative to `root`.
fn collect_relative_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk(root, root, &mut files);
    files.sort();
    files
}

fn walk(root: &Path, current: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(current).expect("read dir") {
        let entry = entry.expect("dir entry");
        let full = entry.path();
        if full.is_dir() {
            walk(root, &full, out);
        } else {
            let relative = full.strip_prefix(root).expect("strip prefix").to_path_buf();
            out.push(relative);
        }
    }
}

/// Assert no file under the export bundle lives under `.plotforge/`, and no
/// file content references `mcp`, `mcp.json`, or `enabled_mcp_servers`. This
/// is the P5.4 leak-guard invariant; it does not re-assert the full allowlist.
fn assert_no_mcp_leak(output_dir: &Path) {
    let files = collect_relative_files(output_dir);
    for relative in &files {
        let components: Vec<String> = relative
            .components()
            .filter_map(|component| component.as_os_str().to_str().map(String::from))
            .collect();
        assert!(
            !components.iter().any(|component| component == ".plotforge"),
            "export bundle leaked a .plotforge/ path: {}",
            relative.display()
        );
        assert!(
            relative != Path::new("mcp.json"),
            "export bundle leaked a stray mcp.json at its root: {}",
            relative.display()
        );
        let full = output_dir.join(relative);
        let bytes = fs::read(&full).expect("read exported file");
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("enabled_mcp_servers"),
            "export bundle file {} leaked enabled_mcp_servers reference",
            relative.display()
        );
        assert!(
            !text.contains("mcp.json"),
            "export bundle file {} leaked a mcp.json reference",
            relative.display()
        );
        assert!(
            !text.contains("local-fs"),
            "export bundle file {} leaked an MCP server id reference",
            relative.display()
        );
        assert!(
            !text.contains("MCP_LOCAL_FS_TOKEN"),
            "export bundle file {} leaked an MCP credential env-var name",
            relative.display()
        );
    }
}

#[test]
fn static_export_omits_mcp_config_when_project_has_mcp_enabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_starter_project(&project_path);
    seed_mcp_config_in_project(&project_path);

    let report = export_static_web(&project_path, &output_dir).expect("export");

    // The audit allowlist never admits `.plotforge/` or a stray `mcp.json`.
    assert_eq!(
        report
            .audit
            .files_found
            .into_iter()
            .collect::<BTreeSet<_>>(),
        expected_asset_free_export_files()
    );
    assert!(!output_dir.join(".plotforge").exists());
    assert!(!output_dir.join("mcp.json").exists());
    assert_no_mcp_leak(&output_dir);
}

#[test]
fn static_export_zip_omits_mcp_config_when_project_has_mcp_enabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    let archive_path = temp.path().join("starter-project-static.zip");
    create_starter_project(&project_path);
    seed_mcp_config_in_project(&project_path);

    let report =
        export_static_web_zip(&project_path, &output_dir, &archive_path).expect("export zip");

    // The zip archive mirrors the audited source bundle exactly.
    let archived = report.archived_files.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(archived, expected_asset_free_export_files());
    assert!(
        !archived.iter().any(|path| {
            path.components()
                .any(|component| component.as_os_str() == ".plotforge")
        }),
        "export zip leaked a .plotforge/ path"
    );
    assert!(
        !archived.iter().any(|path| path == Path::new("mcp.json")),
        "export zip leaked a stray mcp.json"
    );
    // Walk the extracted-on-disk export dir too, so the content scan runs
    // against the same materialised bundle the static export produced.
    assert!(!output_dir.join(".plotforge").exists());
    assert!(!output_dir.join("mcp.json").exists());
    assert_no_mcp_leak(&output_dir);
}

#[test]
fn desktop_runtime_draft_export_omits_mcp_config_when_project_has_mcp_enabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("desktop-export");
    create_starter_project(&project_path);
    seed_mcp_config_in_project(&project_path);

    let report = export_desktop_runtime_draft(&project_path, &output_dir).expect("desktop export");

    let expected_files = expected_asset_free_desktop_files();
    assert_eq!(
        report
            .audit
            .files_found
            .into_iter()
            .collect::<BTreeSet<_>>(),
        expected_files,
    );
    assert!(!output_dir.join(".plotforge").exists());
    assert!(!output_dir.join("mcp.json").exists());
    assert_no_mcp_leak(&output_dir);
    // The desktop runtime draft JSON itself must not reference MCP config.
    let draft_text =
        fs::read_to_string(output_dir.join("desktop-runtime-draft.json")).expect("read draft");
    assert!(
        !draft_text.contains("enabled_mcp_servers"),
        "desktop runtime draft leaked enabled_mcp_servers"
    );
    assert!(
        !draft_text.contains("mcp.json"),
        "desktop runtime draft leaked a mcp.json reference"
    );
}

#[test]
fn static_export_ai_usage_manifest_has_no_mcp_references_when_mcp_enabled() {
    // The AI usage manifest is the export's external-facing disclosure surface.
    // Even though the project has MCP enabled, the manifest must not echo MCP
    // server ids, the mcp.json filename, or the enabled_mcp_servers field.
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_starter_project(&project_path);
    seed_mcp_config_in_project(&project_path);

    export_static_web(&project_path, &output_dir).expect("export");

    let usage_text =
        fs::read_to_string(output_dir.join(AI_USAGE_MANIFEST_FILE)).expect("read ai usage");
    assert!(
        !usage_text.contains("enabled_mcp_servers"),
        "AI usage manifest leaked enabled_mcp_servers"
    );
    assert!(
        !usage_text.contains("mcp.json"),
        "AI usage manifest leaked a mcp.json reference"
    );
    assert!(
        !usage_text.contains("local-fs"),
        "AI usage manifest leaked an MCP server id"
    );
}
