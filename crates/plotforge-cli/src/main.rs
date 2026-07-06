mod cli_output;

use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::{fs, io, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use cli_output::{
    OutputLanguage, cli_term, export_profile_target_label, none_label, print_studio_json,
    print_workshop_validation, render_check_summary, resolve_output_language, unsupported_label,
};
use dialoguer::Input;
use plotforge_export::{export_desktop_runtime_draft, export_static_web, export_static_web_zip};
use plotforge_runtime::{RuntimeSession, summarize_delta};
use plotforge_schema::{
    DESKTOP_RUNTIME_DRAFT_FILE, McpServerEntry, McpTransportConfig, McpTransportKind,
    ProjectCreationRequest, ProjectTemplateId, SteamSubmissionKitRequest, WorkshopPublishDraft,
    supported_export_profiles,
};
use plotforge_storage::{
    create_project_from_request, load_project, read_latest_runtime_snapshot, read_runtime_snapshot,
    validate_project, validate_runtime_snapshot_id, write_runtime_snapshot, write_trace,
};
use plotforge_workshop::{
    LocalOnlySteamworksUploadPort, SteamworksUploadConfig, block_workshop_library_item,
    delete_workshop_library_item, import_workshop_library_package, list_workshop_library,
    load_workshop_library_item, remix_workshop_library_item, report_workshop_library_item,
    upload_workshop_publish_draft, validate_workshop_package, write_steam_submission_kit,
    write_workshop_publish_draft,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

#[derive(Debug, Parser)]
#[command(name = "plotforge")]
#[command(about = "PlotForge CLI-first MVP")]
struct Cli {
    #[arg(long, global = true, value_enum)]
    language: Option<OutputLanguage>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    New(NewCommand),
    Check(ProjectPath),
    Play(PlayArgs),
    Trace(TraceCommand),
    Export(ExportCommand),
    Workshop(WorkshopCommand),
    Studio(StudioInvokeArgs),
    /// Manage MCP (Model Context Protocol) servers and tools. Thin
    /// orchestration only — delegates to `plotforge_studio` (registry IO +
    /// redaction live there), never reimplements business logic.
    Mcp(McpCommand),
}

#[derive(Debug, Args)]
struct ProjectPath {
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Debug, Args)]
struct NewCommand {
    #[command(subcommand)]
    command: NewSubcommand,
}

#[derive(Debug, Subcommand)]
enum NewSubcommand {
    Project(NewProjectArgs),
}

#[derive(Clone, Debug, ValueEnum)]
enum ProjectTemplateArg {
    HistoricalCrisis,
}

impl From<ProjectTemplateArg> for ProjectTemplateId {
    fn from(value: ProjectTemplateArg) -> Self {
        match value {
            ProjectTemplateArg::HistoricalCrisis => ProjectTemplateId::HistoricalCrisis,
        }
    }
}

#[derive(Debug, Args)]
struct NewProjectArgs {
    #[arg(long, default_value = "plotforge-project")]
    path: PathBuf,
    #[arg(long, value_enum, default_value_t = ProjectTemplateArg::HistoricalCrisis)]
    template: ProjectTemplateArg,
    #[arg(long)]
    concept: String,
    #[arg(long)]
    visual_style: String,
    #[arg(long)]
    voice_enabled: bool,
    #[arg(long)]
    initial_scene: String,
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Args)]
struct PlayArgs {
    #[arg(default_value = ".")]
    path: PathBuf,
    #[arg(long)]
    once: bool,
    #[arg(long, default_value = "continue")]
    input: String,
    #[arg(long)]
    save_id: Option<String>,
    #[arg(long)]
    restore_id: Option<String>,
    #[arg(long)]
    restore_latest: bool,
}

#[derive(Debug, Args)]
struct TraceCommand {
    #[command(subcommand)]
    command: TraceSubcommand,
}

#[derive(Debug, Subcommand)]
enum TraceSubcommand {
    Inspect(TraceInspectArgs),
}

#[derive(Debug, Args)]
struct TraceInspectArgs {
    path: PathBuf,
}

#[derive(Debug, Args)]
struct ExportCommand {
    #[command(subcommand)]
    command: ExportSubcommand,
}

#[derive(Debug, Subcommand)]
enum ExportSubcommand {
    Profiles,
    Static(ExportStaticArgs),
    Desktop(ExportDesktopArgs),
}

#[derive(Debug, Args)]
struct ExportStaticArgs {
    #[arg(default_value = ".")]
    path: PathBuf,
    #[arg(long, default_value = "exports/static")]
    out: PathBuf,
    #[arg(long)]
    zip: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ExportDesktopArgs {
    #[arg(default_value = ".")]
    path: PathBuf,
    #[arg(long, default_value = "exports/desktop-runtime")]
    out: PathBuf,
}

#[derive(Debug, Args)]
struct WorkshopCommand {
    #[command(subcommand)]
    command: WorkshopSubcommand,
}

#[derive(Debug, Subcommand)]
enum WorkshopSubcommand {
    Validate(WorkshopPackageArgs),
    Import(WorkshopImportArgs),
    List(WorkshopLibraryArgs),
    Load(WorkshopLibraryItemArgs),
    Remix(WorkshopRemixArgs),
    Block(WorkshopReasonArgs),
    Report(WorkshopReasonArgs),
    Delete(WorkshopLibraryItemArgs),
    PublishDraft(WorkshopPublishDraftArgs),
    UploadDraft(WorkshopUploadDraftArgs),
    SubmissionKit(WorkshopSubmissionKitArgs),
}

#[derive(Debug, Args)]
struct WorkshopPackageArgs {
    package_dir: PathBuf,
}

#[derive(Debug, Args)]
struct WorkshopLibraryArgs {
    library_root: PathBuf,
}

#[derive(Debug, Args)]
struct WorkshopImportArgs {
    library_root: PathBuf,
    package_dir: PathBuf,
}

#[derive(Debug, Args)]
struct WorkshopLibraryItemArgs {
    library_root: PathBuf,
    local_id: String,
}

#[derive(Debug, Args)]
struct WorkshopRemixArgs {
    library_root: PathBuf,
    source_local_id: String,
    #[arg(long)]
    new_id: String,
    #[arg(long)]
    title: String,
}

#[derive(Debug, Args)]
struct WorkshopReasonArgs {
    library_root: PathBuf,
    local_id: String,
    #[arg(long)]
    reason: String,
}

#[derive(Debug, Args)]
struct WorkshopPublishDraftArgs {
    package_dir: PathBuf,
    #[arg(long, default_value = "exports/workshop-publish-draft")]
    out: PathBuf,
}

#[derive(Debug, Args)]
struct WorkshopUploadDraftArgs {
    package_dir: PathBuf,
    draft_json: PathBuf,
    #[arg(long)]
    enable: bool,
    #[arg(long)]
    app_access_confirmed: bool,
    #[arg(long)]
    credential_label: Option<String>,
}

#[derive(Debug, Args)]
struct WorkshopSubmissionKitArgs {
    package_dir: PathBuf,
    #[arg(long, default_value = "exports/steam-submission-kit")]
    out: PathBuf,
    /// Skip the interactive wizard; all fields must be supplied via flags.
    #[arg(long)]
    batch: bool,
    #[arg(long)]
    product_name: Option<String>,
    #[arg(long)]
    desktop_build_path: Option<String>,
    #[arg(long)]
    store_short_description: Option<String>,
    #[arg(long = "screenshot")]
    screenshot_paths: Vec<String>,
    #[arg(long = "capsule-asset")]
    capsule_asset_paths: Vec<String>,
    #[arg(long = "content-warning")]
    content_warnings: Vec<String>,
    #[arg(long = "safety-guardrail")]
    safety_guardrails: Vec<String>,
    #[arg(long)]
    user_reporting_path: Option<String>,
    #[arg(long)]
    moderation_policy: Option<String>,
    #[arg(long = "build-note")]
    build_notes: Vec<String>,
}

#[derive(Debug, Args)]
struct StudioInvokeArgs {
    command: String,
}

// --- MCP subcommand (Phase 5, P5.3) ---
// Thin CLI orchestration over the 7 MCP Studio commands. The CLI only
// collects parameters; validation + redaction stay in `plotforge_studio`
// (AGENTS.md:65). Interactive wizards use `dialoguer` and guard stdin TTY;
// `--batch` enables full flag-driven input for scripts/tests.

#[derive(Debug, Args)]
struct McpCommand {
    #[command(subcommand)]
    command: McpSub,
}

#[derive(Debug, Subcommand)]
enum McpSub {
    /// List registered MCP servers.
    List,
    /// Add or update a MCP server entry (interactive wizard by default; `--batch` for scripts).
    Add(McpAddArgs),
    /// Remove a MCP server entry by id.
    Remove(McpRemoveArgs),
    /// Probe a MCP server (spawn/connect → initialize → tools/list).
    Test(McpTestArgs),
    /// List tools exposed by a MCP server.
    Tools(McpToolsArgs),
    /// Invoke a MCP tool (batch only; takes a JSON arguments string).
    Invoke(McpInvokeArgs),
}

#[derive(Debug, Args)]
struct McpAddArgs {
    /// Skip the interactive wizard; all fields must be supplied via flags.
    #[arg(long)]
    batch: bool,
    #[arg(long)]
    id: Option<String>,
    /// Transport kind: stdio, sse, or http.
    #[arg(long)]
    kind: Option<String>,
    #[arg(long)]
    label: Option<String>,
    /// stdio transport: the command to spawn (e.g. `mcp-server-fs`).
    #[arg(long)]
    command: Option<String>,
    /// stdio transport: args for the spawned command (repeatable).
    #[arg(long = "arg")]
    args: Vec<String>,
    /// SSE/HTTP transport: the endpoint URL.
    #[arg(long)]
    endpoint_url: Option<String>,
    /// Environment variable name holding the credential (never the value).
    #[arg(long)]
    credential_env_var: Option<String>,
    #[arg(long, default_value = "true")]
    enabled: bool,
}

#[derive(Debug, Args)]
struct McpRemoveArgs {
    id: String,
}

#[derive(Debug, Args)]
struct McpTestArgs {
    id: String,
}

#[derive(Debug, Args)]
struct McpToolsArgs {
    id: String,
}

#[derive(Debug, Args)]
struct McpInvokeArgs {
    id: String,
    tool: String,
    /// JSON arguments matching the tool's input_schema.
    #[arg(long)]
    arguments: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let language = resolve_output_language(cli.language);
    match cli.command {
        Command::New(command) => handle_new(command, language),
        Command::Check(args) => handle_check(args, language),
        Command::Play(args) => handle_play(args, language),
        Command::Trace(command) => handle_trace(command, language),
        Command::Export(command) => handle_export(command, language),
        Command::Workshop(command) => handle_workshop(command, language),
        Command::Studio(args) => handle_studio(args),
        Command::Mcp(command) => handle_mcp(command, language),
    }
}

fn handle_studio(args: StudioInvokeArgs) -> Result<()> {
    let payload: Value =
        serde_json::from_reader(io::stdin()).context("parse studio command json")?;

    match args.command.as_str() {
        "create_project" => print_studio_json(studio_result(plotforge_studio::create_project(
            studio_arg::<PathBuf>(&payload, "path")?,
            studio_arg(&payload, "request")?,
            studio_arg(&payload, "force")?,
        ))?),
        "open_project" => print_studio_json(studio_result(plotforge_studio::open_project(
            studio_arg::<PathBuf>(&payload, "path")?,
        ))?),
        "check_project" => print_studio_json(studio_result(plotforge_studio::check_project(
            studio_arg::<PathBuf>(&payload, "path")?,
        ))?),
        "list_export_profiles" => print_studio_json(plotforge_studio::list_export_profiles()),
        "pi_agent_run" => print_studio_json(studio_result(plotforge_studio::pi_agent_run(
            studio_arg(&payload, "request")?,
        ))?),
        "pi_agent_capabilities" => {
            print_studio_json(studio_result(plotforge_studio::pi_agent_capabilities())?)
        }
        "read_world_edit_document" => print_studio_json(studio_result(
            plotforge_studio::read_world_edit_document(studio_arg::<PathBuf>(&payload, "path")?),
        )?),
        "update_world_edit_document" => print_studio_json(studio_result(
            plotforge_studio::update_world_edit_document(
                studio_arg::<PathBuf>(&payload, "path")?,
                studio_arg(&payload, "document")?,
            ),
        )?),
        "read_story_craft_edit_document" => print_studio_json(studio_result(
            plotforge_studio::read_story_craft_edit_document(studio_arg::<PathBuf>(
                &payload, "path",
            )?),
        )?),
        "update_story_craft_edit_document" => print_studio_json(studio_result(
            plotforge_studio::update_story_craft_edit_document(
                studio_arg::<PathBuf>(&payload, "path")?,
                studio_arg(&payload, "document")?,
            ),
        )?),
        "read_character_edit_document" => print_studio_json(studio_result(
            plotforge_studio::read_character_edit_document(studio_arg::<PathBuf>(
                &payload, "path",
            )?),
        )?),
        "update_character_edit_document" => print_studio_json(studio_result(
            plotforge_studio::update_character_edit_document(
                studio_arg::<PathBuf>(&payload, "path")?,
                studio_arg(&payload, "document")?,
            ),
        )?),
        "create_character" => {
            print_studio_json(studio_result(plotforge_studio::create_character(
                studio_arg::<PathBuf>(&payload, "path")?,
                studio_arg(&payload, "character")?,
            ))?)
        }
        "read_state_variables_edit_document" => print_studio_json(studio_result(
            plotforge_studio::read_state_variables_edit_document(studio_arg::<PathBuf>(
                &payload, "path",
            )?),
        )?),
        "update_state_variables_edit_document" => print_studio_json(studio_result(
            plotforge_studio::update_state_variables_edit_document(
                studio_arg::<PathBuf>(&payload, "path")?,
                studio_arg(&payload, "document")?,
            ),
        )?),
        "create_resource" => print_studio_json(studio_result(plotforge_studio::create_resource(
            studio_arg::<PathBuf>(&payload, "path")?,
            studio_arg(&payload, "resource")?,
        ))?),
        "read_rules_edit_document" => print_studio_json(studio_result(
            plotforge_studio::read_rules_edit_document(studio_arg::<PathBuf>(&payload, "path")?),
        )?),
        "update_rules_edit_document" => print_studio_json(studio_result(
            plotforge_studio::update_rules_edit_document(
                studio_arg::<PathBuf>(&payload, "path")?,
                studio_arg(&payload, "document")?,
            ),
        )?),
        "create_rule" => print_studio_json(studio_result(plotforge_studio::create_rule(
            studio_arg::<PathBuf>(&payload, "path")?,
            studio_arg(&payload, "rule")?,
        ))?),
        "generate_world_expansion" => {
            let expansion_goal: String = studio_arg(&payload, "expansion_goal")?;
            print_studio_json(studio_result(plotforge_studio::generate_world_expansion(
                studio_arg::<PathBuf>(&payload, "path")?,
                &expansion_goal,
            ))?)
        }
        "generate_story_craft" => {
            let concept: String = studio_arg(&payload, "concept")?;
            print_studio_json(studio_result(plotforge_studio::generate_story_craft(
                studio_arg::<PathBuf>(&payload, "path")?,
                &concept,
            ))?)
        }
        "generate_character" => {
            let concept: String = studio_arg(&payload, "concept")?;
            let role_hint: String = studio_arg(&payload, "role_hint")?;
            print_studio_json(studio_result(plotforge_studio::generate_character(
                studio_arg::<PathBuf>(&payload, "path")?,
                &concept,
                &role_hint,
            ))?)
        }
        "read_ai_safety_policy" => print_studio_json(studio_result(
            plotforge_studio::read_ai_safety_policy(studio_arg::<PathBuf>(&payload, "path")?),
        )?),
        "update_ai_safety_policy" => {
            print_studio_json(studio_result(plotforge_studio::update_ai_safety_policy(
                studio_arg::<PathBuf>(&payload, "path")?,
                studio_arg(&payload, "policy")?,
            ))?)
        }
        "read_visual_bible" => print_studio_json(studio_result(
            plotforge_studio::read_visual_bible(studio_arg::<PathBuf>(&payload, "path")?),
        )?),
        "update_visual_bible" => {
            print_studio_json(studio_result(plotforge_studio::update_visual_bible(
                studio_arg::<PathBuf>(&payload, "path")?,
                studio_arg(&payload, "visual_bible")?,
            ))?)
        }
        "read_audio_bible" => print_studio_json(studio_result(
            plotforge_studio::read_audio_bible(studio_arg::<PathBuf>(&payload, "path")?),
        )?),
        "update_audio_bible" => {
            print_studio_json(studio_result(plotforge_studio::update_audio_bible(
                studio_arg::<PathBuf>(&payload, "path")?,
                studio_arg(&payload, "audio_bible")?,
            ))?)
        }
        "play_once_project" => {
            let player_input: String = studio_arg(&payload, "player_input")?;
            print_studio_json(studio_result(plotforge_studio::play_once_project(
                studio_arg::<PathBuf>(&payload, "path")?,
                &player_input,
            ))?)
        }
        "play_once_project_with_save" => {
            let player_input: String = studio_arg(&payload, "player_input")?;
            let save_id: String = studio_arg(&payload, "save_id")?;
            print_studio_json(studio_result(
                plotforge_studio::play_once_project_with_save(
                    studio_arg::<PathBuf>(&payload, "path")?,
                    &player_input,
                    Some(&save_id),
                ),
            )?)
        }
        "play_once_project_from_snapshot" => {
            let player_input: String = studio_arg(&payload, "player_input")?;
            let snapshot_id: String = studio_arg(&payload, "snapshot_id")?;
            let save_id: Option<String> = studio_arg(&payload, "save_id")?;
            print_studio_json(studio_result(
                plotforge_studio::play_once_project_from_snapshot(
                    studio_arg::<PathBuf>(&payload, "path")?,
                    &player_input,
                    &snapshot_id,
                    save_id.as_deref(),
                ),
            )?)
        }
        "play_once_project_from_latest_snapshot" => {
            let player_input: String = studio_arg(&payload, "player_input")?;
            let save_id: Option<String> = studio_arg(&payload, "save_id")?;
            print_studio_json(studio_result(
                plotforge_studio::play_once_project_from_latest_snapshot(
                    studio_arg::<PathBuf>(&payload, "path")?,
                    &player_input,
                    save_id.as_deref(),
                ),
            )?)
        }
        "export_static_project" => {
            print_studio_json(studio_result(plotforge_studio::export_static_project(
                studio_arg::<PathBuf>(&payload, "path")?,
                studio_arg::<PathBuf>(&payload, "output_dir")?,
            ))?)
        }
        "export_static_project_zip" => {
            print_studio_json(studio_result(plotforge_studio::export_static_project_zip(
                studio_arg::<PathBuf>(&payload, "path")?,
                studio_arg::<PathBuf>(&payload, "output_dir")?,
                studio_arg::<PathBuf>(&payload, "archive_path")?,
            ))?)
        }
        "list_asset_records" => print_studio_json(studio_result(
            plotforge_studio::list_asset_records(studio_arg::<PathBuf>(&payload, "path")?),
        )?),
        "list_source_files" => print_studio_json(studio_result(
            plotforge_studio::list_source_files(studio_arg::<PathBuf>(&payload, "path")?),
        )?),
        "read_source_file" => {
            let relative_path: String = studio_arg(&payload, "relative_path")?;
            print_studio_json(studio_result(plotforge_studio::read_source_file(
                studio_arg::<PathBuf>(&payload, "path")?,
                &relative_path,
            ))?)
        }
        "write_source_file" => {
            let relative_path: String = studio_arg(&payload, "relative_path")?;
            let content: String = studio_arg(&payload, "content")?;
            print_studio_json(studio_result(plotforge_studio::write_source_file(
                studio_arg::<PathBuf>(&payload, "path")?,
                &relative_path,
                &content,
            ))?)
        }
        "pi_agent_apply_run" => print_studio_json(studio_result(
            plotforge_studio::pi_agent_apply_run(studio_arg(&payload, "request")?),
        )?),
        "list_providers" => print_studio_json(studio_result(plotforge_studio::list_providers())?),
        "upsert_provider" => print_studio_json(studio_result(plotforge_studio::upsert_provider(
            studio_arg(&payload, "entry")?,
        ))?),
        "delete_provider" => print_studio_json(studio_result(plotforge_studio::delete_provider(
            studio_arg::<String>(&payload, "id")?,
        ))?),
        "test_provider_connection" => print_studio_json(studio_result(
            plotforge_studio::test_provider_connection(studio_arg::<String>(&payload, "id")?),
        )?),
        "list_user_prompt_templates" => print_studio_json(studio_result(
            plotforge_studio::list_user_prompt_templates(),
        )?),
        "list_project_prompt_templates" => print_studio_json(studio_result(
            plotforge_studio::list_project_prompt_templates(studio_arg::<PathBuf>(
                &payload, "path",
            )?),
        )?),
        "upsert_user_prompt_template" => print_studio_json(studio_result(
            plotforge_studio::upsert_user_prompt_template(studio_arg(&payload, "template")?),
        )?),
        "upsert_project_prompt_template" => {
            let template = studio_arg(&payload, "template")?;
            print_studio_json(studio_result(
                plotforge_studio::upsert_project_prompt_template(
                    studio_arg::<PathBuf>(&payload, "path")?,
                    template,
                ),
            )?)
        }
        "delete_user_prompt_template" => print_studio_json(studio_result(
            plotforge_studio::delete_user_prompt_template(studio_arg::<String>(&payload, "id")?),
        )?),
        "delete_project_prompt_template" => {
            let id: String = studio_arg(&payload, "id")?;
            print_studio_json(studio_result(
                plotforge_studio::delete_project_prompt_template(
                    studio_arg::<PathBuf>(&payload, "path")?,
                    id,
                ),
            )?)
        }
        "list_skills" => print_studio_json(studio_result(plotforge_studio::list_skills())?),
        "refresh_skill_index" => {
            print_studio_json(studio_result(plotforge_studio::refresh_skill_index())?)
        }
        "import_skill" => print_studio_json(studio_result(plotforge_studio::import_skill(
            studio_arg::<String>(&payload, "skill_id")?,
        ))?),
        "read_skill_body" => print_studio_json(studio_result(plotforge_studio::read_skill_body(
            studio_arg::<String>(&payload, "skill_id")?,
        ))?),
        "enable_skill_for_project" => {
            let skill_id: String = studio_arg(&payload, "skill_id")?;
            let enabled: bool = studio_arg(&payload, "enabled")?;
            print_studio_json(studio_result(plotforge_studio::enable_skill_for_project(
                studio_arg::<PathBuf>(&payload, "path")?,
                skill_id,
                enabled,
            ))?)
        }
        "list_mcp_servers" => {
            print_studio_json(studio_result(plotforge_studio::list_mcp_servers())?)
        }
        "upsert_mcp_server" => print_studio_json(studio_result(
            plotforge_studio::upsert_mcp_server(studio_arg(&payload, "entry")?),
        )?),
        "delete_mcp_server" => print_studio_json(studio_result(
            plotforge_studio::delete_mcp_server(studio_arg::<String>(&payload, "id")?),
        )?),
        "test_mcp_server" => print_studio_json(studio_result(plotforge_studio::test_mcp_server(
            studio_arg::<String>(&payload, "id")?,
        ))?),
        "list_mcp_tools" => print_studio_json(studio_result(plotforge_studio::list_mcp_tools(
            studio_arg::<String>(&payload, "server_id")?,
        ))?),
        "invoke_mcp_tool" => print_studio_json(studio_result(plotforge_studio::invoke_mcp_tool(
            studio_arg(&payload, "request")?,
        ))?),
        "enable_mcp_server_for_project" => {
            let server_id: String = studio_arg(&payload, "server_id")?;
            let enabled: bool = studio_arg(&payload, "enabled")?;
            print_studio_json(studio_result(
                plotforge_studio::enable_mcp_server_for_project(
                    studio_arg::<PathBuf>(&payload, "path")?,
                    server_id,
                    enabled,
                ),
            )?)
        }
        other => anyhow::bail!("unknown studio command `{other}`"),
    }
}

fn studio_arg<T: DeserializeOwned>(payload: &Value, name: &str) -> Result<T> {
    let value = payload
        .get(name)
        .with_context(|| format!("studio command arg `{name}` is required"))?;
    serde_json::from_value(value.clone())
        .with_context(|| format!("parse studio command arg `{name}`"))
}

fn studio_result<T>(result: plotforge_studio::StudioCommandResult<T>) -> Result<T> {
    result.map_err(|source| anyhow::anyhow!("{}: {}", source.code, source.message))
}

/// CLI orchestration for MCP server + tool management. Thin wrapper that
/// delegates to `plotforge_studio` (registry IO + redaction live there); the
/// CLI only collects parameters and prints results (AGENTS.md:65). Interactive
/// wizards guard stdin TTY and bail to `--batch` when non-TTY.
fn handle_mcp(command: McpCommand, language: OutputLanguage) -> Result<()> {
    match command.command {
        McpSub::List => {
            let servers = studio_result(plotforge_studio::list_mcp_servers())?;
            match language {
                OutputLanguage::En => println!("mcp servers: {}", servers.len()),
                OutputLanguage::Zh => println!("MCP 服务器：{}", servers.len()),
            }
            for entry in &servers {
                println!(
                    "  {} ({}, {})",
                    entry.id,
                    entry.label,
                    transport_kind_label(entry.kind)
                );
            }
        }
        McpSub::Add(args) => {
            let entry = if args.batch {
                build_mcp_entry_from_flags(args)?
            } else {
                if !io::stdin().is_terminal() {
                    anyhow::bail!(
                        "mcp add interactive wizard requires a TTY; pass --batch with full arguments"
                    );
                }
                build_mcp_entry_wizard(language)?
            };
            let saved = studio_result(plotforge_studio::upsert_mcp_server(entry.clone()))
                .with_context(|| format!("upsert MCP server {}", entry.id))?;
            match language {
                OutputLanguage::En => println!("added mcp server {} ({})", saved.id, saved.label),
                OutputLanguage::Zh => println!("已添加 MCP 服务器 {}（{}）", saved.id, saved.label),
            }
        }
        McpSub::Remove(args) => {
            let removed = studio_result(plotforge_studio::delete_mcp_server(args.id.clone()))
                .with_context(|| format!("delete MCP server {}", args.id))?;
            match language {
                OutputLanguage::En => {
                    println!("removed mcp server {} ({})", removed.id, removed.label)
                }
                OutputLanguage::Zh => {
                    println!("已移除 MCP 服务器 {}（{}）", removed.id, removed.label)
                }
            }
        }
        McpSub::Test(args) => {
            let result = studio_result(plotforge_studio::test_mcp_server(args.id.clone()))
                .with_context(|| format!("test MCP server {}", args.id))?;
            match language {
                OutputLanguage::En => println!(
                    "mcp test {}: ok={}, tools={}, {}",
                    args.id, result.ok, result.tools_count, result.message
                ),
                OutputLanguage::Zh => println!(
                    "MCP 测试 {}：ok={}，工具={}，{}",
                    args.id, result.ok, result.tools_count, result.message
                ),
            }
        }
        McpSub::Tools(args) => {
            let tools = studio_result(plotforge_studio::list_mcp_tools(args.id.clone()))
                .with_context(|| format!("list MCP tools for {}", args.id))?;
            match language {
                OutputLanguage::En => println!("mcp tools ({}): {}", args.id, tools.len()),
                OutputLanguage::Zh => println!("MCP 工具（{}）：{}", args.id, tools.len()),
            }
            for tool in &tools {
                println!("  {} — {}", tool.name, tool.description);
            }
        }
        McpSub::Invoke(args) => {
            let arguments: Value = serde_json::from_str(&args.arguments)
                .with_context(|| format!("parse --arguments JSON for tool {}", args.tool))?;
            let request = plotforge_schema::McpToolCallRequest {
                server_id: args.id.clone(),
                tool_name: args.tool.clone(),
                arguments,
            };
            let result = studio_result(plotforge_studio::invoke_mcp_tool(request))
                .with_context(|| format!("invoke MCP tool {} on {}", args.tool, args.id))?;
            // Print the redaction-safe result as JSON (contract-typed).
            println!("{}", serde_json::to_string(&result)?);
        }
    }
    Ok(())
}

/// Build a `McpServerEntry` from `--batch` flags. All required fields must be
/// supplied; missing fields surface explicit errors (no silent defaults).
fn build_mcp_entry_from_flags(args: McpAddArgs) -> Result<McpServerEntry> {
    let id = args
        .id
        .ok_or_else(|| anyhow::anyhow!("--batch requires --id"))?;
    let kind = args
        .kind
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--batch requires --kind (stdio|sse|http)"))?;
    let kind = match kind {
        "stdio" => McpTransportKind::Stdio,
        "sse" => McpTransportKind::Sse,
        "http" => McpTransportKind::Http,
        other => anyhow::bail!("--kind must be stdio|sse|http, got `{other}`"),
    };
    let label = args
        .label
        .ok_or_else(|| anyhow::anyhow!("--batch requires --label"))?;
    let transport_config = match kind {
        McpTransportKind::Stdio => {
            let command = args
                .command
                .ok_or_else(|| anyhow::anyhow!("--kind stdio requires --command"))?;
            McpTransportConfig::Stdio {
                command,
                args: args.args,
                env: BTreeMap::new(),
            }
        }
        McpTransportKind::Sse => {
            let endpoint_url = args
                .endpoint_url
                .ok_or_else(|| anyhow::anyhow!("--kind sse requires --endpoint-url"))?;
            McpTransportConfig::Sse { endpoint_url }
        }
        McpTransportKind::Http => {
            let endpoint_url = args
                .endpoint_url
                .ok_or_else(|| anyhow::anyhow!("--kind http requires --endpoint-url"))?;
            McpTransportConfig::Http { endpoint_url }
        }
    };
    Ok(McpServerEntry {
        id,
        kind,
        label,
        transport_config,
        credential_env_var: args.credential_env_var.unwrap_or_default(),
        enabled: args.enabled,
    })
}

/// Interactive wizard for `mcp add`. Steps through id, kind, label, transport
/// fields, and credential_env_var name (never the value) via `dialoguer`.
fn build_mcp_entry_wizard(language: OutputLanguage) -> Result<McpServerEntry> {
    use dialoguer::Input;
    let id: String = Input::new()
        .with_prompt(prompt_label("MCP server id", "MCP 服务器 id", language))
        .interact_text()?;
    let kind_str: String = Input::new()
        .with_prompt(prompt_label(
            "transport kind (stdio|sse|http)",
            "传输类型 (stdio|sse|http)",
            language,
        ))
        .default("stdio".into())
        .interact_text()?;
    let kind = match kind_str.trim() {
        "stdio" => McpTransportKind::Stdio,
        "sse" => McpTransportKind::Sse,
        "http" => McpTransportKind::Http,
        other => anyhow::bail!("kind must be stdio|sse|http, got `{other}`"),
    };
    let label: String = Input::new()
        .with_prompt(prompt_label("label", "标签", language))
        .interact_text()?;
    let transport_config = match kind {
        McpTransportKind::Stdio => {
            let command: String = Input::new()
                .with_prompt(prompt_label("command to spawn", "要启动的命令", language))
                .interact_text()?;
            McpTransportConfig::Stdio {
                command,
                args: Vec::new(),
                env: BTreeMap::new(),
            }
        }
        McpTransportKind::Sse => {
            let endpoint_url: String = Input::new()
                .with_prompt(prompt_label("SSE endpoint URL", "SSE 端点 URL", language))
                .interact_text()?;
            McpTransportConfig::Sse { endpoint_url }
        }
        McpTransportKind::Http => {
            let endpoint_url: String = Input::new()
                .with_prompt(prompt_label("HTTP endpoint URL", "HTTP 端点 URL", language))
                .interact_text()?;
            McpTransportConfig::Http { endpoint_url }
        }
    };
    let credential_env_var: String = Input::new()
        .with_prompt(prompt_label(
            "credential env var name (e.g. MCP_TOKEN; leave empty for none)",
            "凭据环境变量名（例如 MCP_TOKEN；留空表示无）",
            language,
        ))
        .allow_empty(true)
        .interact_text()?;
    Ok(McpServerEntry {
        id,
        kind,
        label,
        transport_config,
        credential_env_var,
        enabled: true,
    })
}

fn transport_kind_label(kind: plotforge_schema::McpTransportKind) -> &'static str {
    match kind {
        plotforge_schema::McpTransportKind::Stdio => "stdio",
        plotforge_schema::McpTransportKind::Sse => "sse",
        plotforge_schema::McpTransportKind::Http => "http",
    }
}

fn handle_new(command: NewCommand, language: OutputLanguage) -> Result<()> {
    match command.command {
        NewSubcommand::Project(args) => {
            let request = ProjectCreationRequest {
                template: args.template.into(),
                concept: args.concept,
                visual_style: args.visual_style,
                voice_enabled: args.voice_enabled,
                initial_scene_request: args.initial_scene,
            };
            let report = create_project_from_request(&args.path, request, args.force)
                .with_context(|| format!("create project at {}", args.path.display()))?;
            match language {
                OutputLanguage::En => println!(
                    "created project {} at {} ({} files)",
                    report.project.game.title,
                    args.path.display(),
                    report.files_created.len()
                ),
                OutputLanguage::Zh => println!(
                    "已创建项目 {} 于 {}（{} 个文件）",
                    report.project.game.title,
                    args.path.display(),
                    report.files_created.len()
                ),
            }
        }
    }
    Ok(())
}

fn handle_check(args: ProjectPath, language: OutputLanguage) -> Result<()> {
    let project = validate_project(&args.path)
        .with_context(|| format!("validate project at {}", args.path.display()))?;
    render_check_summary(language, &project);
    Ok(())
}

fn handle_play(args: PlayArgs, language: OutputLanguage) -> Result<()> {
    if !args.once {
        anyhow::bail!("interactive play is not implemented in the MVP; pass --once");
    }
    if let Some(save_id) = args.save_id.as_deref() {
        validate_runtime_snapshot_id(save_id).context("validate runtime snapshot id")?;
    }

    let project = load_project(&args.path)
        .with_context(|| format!("load project at {}", args.path.display()))?;
    let mut session = if args.restore_latest {
        let snapshot = read_latest_runtime_snapshot(&args.path).with_context(|| {
            format!("read latest runtime snapshot under {}", args.path.display())
        })?;
        RuntimeSession::from_snapshot(project, snapshot)
            .context("restore latest runtime snapshot")?
    } else if let Some(snapshot_id) = args.restore_id.as_ref() {
        let snapshot = read_runtime_snapshot(&args.path, snapshot_id).with_context(|| {
            format!(
                "read runtime snapshot `{snapshot_id}` under {}",
                args.path.display()
            )
        })?;
        RuntimeSession::from_snapshot(project, snapshot)
            .with_context(|| format!("restore runtime snapshot `{snapshot_id}`"))?
    } else {
        RuntimeSession::new(project)
    };
    let step = session
        .play_once(&args.input)
        .context("run one deterministic play step")?;
    let trace_path = write_trace(&args.path, &step.trace)
        .with_context(|| format!("write trace under {}", args.path.display()))?;
    let snapshot_path = if let Some(save_id) = args.save_id.as_ref() {
        let snapshot = session.snapshot(save_id, step.trace.timestamp_ms);
        Some(
            write_runtime_snapshot(&args.path, &snapshot).with_context(|| {
                format!(
                    "write runtime snapshot `{save_id}` under {}",
                    args.path.display()
                )
            })?,
        )
    } else {
        None
    };

    println!(
        "{}: {} - {}",
        cli_term(language, "scene"),
        step.scene.key,
        step.scene.title
    );
    println!(
        "{}: {}",
        cli_term(language, "choice"),
        none_label(language, step.trace.selected_choice.as_deref())
    );
    println!("{}:", cli_term(language, "delta"));
    for line in summarize_delta(&step.trace.world_state_delta) {
        println!("  {line}");
    }
    println!("{}: {}", cli_term(language, "trace"), trace_path.display());
    if let Some(snapshot_path) = snapshot_path {
        println!(
            "{}: {}",
            cli_term(language, "snapshot"),
            snapshot_path.display()
        );
    }
    Ok(())
}

fn handle_trace(command: TraceCommand, language: OutputLanguage) -> Result<()> {
    match command.command {
        TraceSubcommand::Inspect(args) => {
            let text = fs::read_to_string(&args.path)
                .with_context(|| format!("read trace {}", args.path.display()))?;
            let trace: plotforge_schema::RuntimeTrace =
                serde_json::from_str(&text).context("parse trace json")?;
            println!("{}: {}", cli_term(language, "trace"), trace.id);
            println!(
                "{}: {}",
                cli_term(language, "run seed"),
                trace.reproducibility.run_seed
            );
            println!(
                "{}: {}",
                cli_term(language, "prompt version"),
                trace.reproducibility.prompt_version
            );
            println!(
                "{}: {}",
                cli_term(language, "model version"),
                trace.reproducibility.model_version
            );
            println!(
                "{}: {}",
                cli_term(language, "provider config hash"),
                trace.reproducibility.provider_config_hash
            );
            println!(
                "{}: {}",
                cli_term(language, "trace evidence id"),
                none_label(language, trace.reproducibility.trace_id.as_deref())
            );
            println!(
                "{}: {}",
                cli_term(language, "snapshot evidence id"),
                none_label(language, trace.reproducibility.snapshot_id.as_deref())
            );
            println!(
                "{}: {}",
                cli_term(language, "player input"),
                none_label(language, trace.player_input.as_deref())
            );
            println!(
                "{}: {}",
                cli_term(language, "selected"),
                none_label(language, trace.selected_choice.as_deref())
            );
            if let Some(intent) = trace.action_intent.as_ref() {
                println!(
                    "{}: {}",
                    cli_term(language, "intent"),
                    unsupported_label(language, intent.action_type.as_deref())
                );
                println!("intent status: {:?}", intent.status);
                println!(
                    "{}: {}",
                    cli_term(language, "intent choice"),
                    none_label(language, intent.choice_id.as_deref())
                );
                println!(
                    "{}: {}",
                    cli_term(language, "intent action"),
                    unsupported_label(language, intent.action_type.as_deref())
                );
                println!(
                    "{}: {}",
                    cli_term(language, "intent matched terms"),
                    intent.matched_terms.join(", ")
                );
                println!(
                    "{}: {}",
                    cli_term(language, "intent reason"),
                    none_label(language, intent.reason.as_deref())
                );
            }
            if let Some(rule_result) = trace.rule_result.as_ref() {
                println!(
                    "{}: {} ({}: {}, {}: {})",
                    cli_term(language, "rule"),
                    rule_result.action_type,
                    cli_term(language, "delta empty"),
                    rule_result.delta_empty,
                    cli_term(language, "committed"),
                    rule_result.state_committed
                );
                println!(
                    "{}: {}",
                    cli_term(language, "rule action"),
                    rule_result.action_type
                );
                println!(
                    "{}: {}",
                    cli_term(language, "rule delta empty"),
                    rule_result.delta_empty
                );
                println!(
                    "{}: {}",
                    cli_term(language, "rule committed"),
                    rule_result.state_committed
                );
                if let Some(error) = rule_result.error.as_ref() {
                    println!(
                        "{}: {} - {}",
                        cli_term(language, "rule error"),
                        error.code,
                        error.message
                    );
                }
            }
            if let Some(planner_result) = trace.planner_result.as_ref() {
                println!(
                    "{}: {} ({}: {})",
                    cli_term(language, "planner"),
                    none_label(language, planner_result.scene_key.as_deref()),
                    cli_term(language, "fallback"),
                    planner_result.fallback_used
                );
                println!(
                    "{}: {}",
                    cli_term(language, "planner requested"),
                    planner_result.requested_action_type
                );
                println!(
                    "{}: {}",
                    cli_term(language, "planner scene"),
                    none_label(language, planner_result.scene_key.as_deref())
                );
                println!(
                    "{}: {}",
                    cli_term(language, "planner fallback"),
                    planner_result.fallback_used
                );
                if let Some(error) = planner_result.error.as_ref() {
                    println!(
                        "{}: {} - {}",
                        cli_term(language, "planner error"),
                        error.code,
                        error.message
                    );
                }
            }
            println!(
                "{}: {}",
                cli_term(language, "fallback"),
                trace.fallback_used
            );
            println!(
                "{}: {}={} {}={} {}={}",
                cli_term(language, "story before"),
                cli_term(language, "scene"),
                trace.story_state_before.current_scene_key,
                cli_term(language, "beat"),
                none_label(
                    language,
                    trace.story_state_before.current_beat_id.as_deref()
                ),
                cli_term(language, "turn"),
                trace.story_state_before.turn
            );
            println!(
                "{}: {}={} {}={} {}={}",
                cli_term(language, "story after"),
                cli_term(language, "scene"),
                trace.story_state_after.current_scene_key,
                cli_term(language, "beat"),
                none_label(language, trace.story_state_after.current_beat_id.as_deref()),
                cli_term(language, "turn"),
                trace.story_state_after.turn
            );
            println!("{}:", cli_term(language, "world delta"));
            for line in summarize_delta(&trace.world_state_delta) {
                println!("  {line}");
            }
            println!(
                "{}: {}",
                cli_term(language, "diagnostics"),
                trace.diagnostics.len()
            );
            for diagnostic in &trace.diagnostics {
                println!(
                    "{}: {:?} {:?} - {}",
                    cli_term(language, "diagnostic"),
                    diagnostic.stage,
                    diagnostic.status,
                    diagnostic.message
                );
            }
            println!(
                "{}: {}",
                cli_term(language, "media references"),
                trace.media_references.len()
            );
            for media in &trace.media_references {
                println!(
                    "{}: {:?} {} {} -> {}",
                    cli_term(language, "media"),
                    media.reference.reference_kind,
                    media.reference.reference_id,
                    media.reference.slot,
                    media.project_path
                );
            }
            for error in &trace.errors {
                println!(
                    "{}: {} - {}",
                    cli_term(language, "error"),
                    error.code,
                    error.message
                );
            }
            if let Some(review) = trace.narrative_review.as_ref() {
                println!(
                    "{}: {}",
                    cli_term(language, "review scene"),
                    review.scene_key
                );
                println!("{}: {}", cli_term(language, "review score"), review.score);
                println!(
                    "{}: {}",
                    cli_term(language, "review issues"),
                    review.issues.len()
                );
                for issue in &review.issues {
                    println!(
                        "{}: {:?} {:?} - {}",
                        cli_term(language, "review issue"),
                        issue.kind,
                        issue.severity,
                        issue.message
                    );
                }
            }
        }
    }
    Ok(())
}

fn handle_export(command: ExportCommand, language: OutputLanguage) -> Result<()> {
    match command.command {
        ExportSubcommand::Profiles => {
            for profile in supported_export_profiles() {
                println!(
                    "{} target={} requires_network_at_runtime={} includes_provider_config={} includes_private_traces={} platform_submission_ready={}",
                    profile.id,
                    export_profile_target_label(&profile.target),
                    profile.requires_network_at_runtime,
                    profile.includes_provider_config,
                    profile.includes_private_traces,
                    profile.platform_submission_ready
                );
            }
        }
        ExportSubcommand::Static(args) => {
            let report = if let Some(zip_path) = args.zip.as_ref() {
                let zip_report = export_static_web_zip(&args.path, &args.out, zip_path)
                    .with_context(|| {
                        format!(
                            "export static web zip from {} to {} and {}",
                            args.path.display(),
                            args.out.display(),
                            zip_path.display()
                        )
                    })?;
                match language {
                    OutputLanguage::En => println!(
                        "exported static zip to {} ({} files)",
                        zip_report.archive_path.display(),
                        zip_report.archived_files.len()
                    ),
                    OutputLanguage::Zh => println!(
                        "已导出静态 zip 到 {}（{} 个文件）",
                        zip_report.archive_path.display(),
                        zip_report.archived_files.len()
                    ),
                }
                zip_report.source_report
            } else {
                export_static_web(&args.path, &args.out).with_context(|| {
                    format!(
                        "export static web from {} to {}",
                        args.path.display(),
                        args.out.display()
                    )
                })?
            };
            match language {
                OutputLanguage::En => println!(
                    "exported static player to {} ({} files)",
                    report.output_dir.display(),
                    report.files_written.len()
                ),
                OutputLanguage::Zh => println!(
                    "已导出静态播放器到 {}（{} 个文件）",
                    report.output_dir.display(),
                    report.files_written.len()
                ),
            }
        }
        ExportSubcommand::Desktop(args) => {
            let report =
                export_desktop_runtime_draft(&args.path, &args.out).with_context(|| {
                    format!(
                        "export desktop runtime draft from {} to {}",
                        args.path.display(),
                        args.out.display()
                    )
                })?;
            match language {
                OutputLanguage::En => println!(
                    "exported desktop runtime draft to {} ({} files)",
                    report.output_dir.display(),
                    report.files_written.len()
                ),
                OutputLanguage::Zh => println!(
                    "已导出桌面运行时草稿到 {}（{} 个文件）",
                    report.output_dir.display(),
                    report.files_written.len()
                ),
            }
            println!(
                "{}: {}",
                cli_term(language, "desktop runtime draft"),
                report.output_dir.join(DESKTOP_RUNTIME_DRAFT_FILE).display()
            );
            println!(
                "{}: {}",
                cli_term(language, "desktop build notes"),
                report.output_dir.join("desktop-build-notes.md").display()
            );
        }
    }
    Ok(())
}

fn handle_workshop(command: WorkshopCommand, language: OutputLanguage) -> Result<()> {
    match command.command {
        WorkshopSubcommand::Validate(args) => {
            let report = validate_workshop_package(&args.package_dir).with_context(|| {
                format!(
                    "validate workshop package at {}",
                    args.package_dir.display()
                )
            })?;
            print_workshop_validation(language, "workshop package ok", &report);
        }
        WorkshopSubcommand::Import(args) => {
            let report = import_workshop_library_package(&args.library_root, &args.package_dir)
                .with_context(|| {
                    format!(
                        "import workshop package {} into {}",
                        args.package_dir.display(),
                        args.library_root.display()
                    )
                })?;
            println!(
                "imported workshop item {} ({})",
                report.item.local_id, report.item.title
            );
            print_workshop_validation(
                language,
                "validated imported package",
                &report.validation_report,
            );
        }
        WorkshopSubcommand::List(args) => {
            let items = list_workshop_library(&args.library_root).with_context(|| {
                format!("list workshop library {}", args.library_root.display())
            })?;
            match language {
                OutputLanguage::En => println!("workshop library items: {}", items.len()),
                OutputLanguage::Zh => println!("Workshop 库条目：{}", items.len()),
            }
            for item in items {
                println!(
                    "{} title={} blocked={} reports={}",
                    item.local_id,
                    item.title,
                    item.blocked.is_some(),
                    item.reports.len()
                );
            }
        }
        WorkshopSubcommand::Load(args) => {
            let report = load_workshop_library_item(&args.library_root, &args.local_id)
                .with_context(|| {
                    format!(
                        "load workshop library item {} from {}",
                        args.local_id,
                        args.library_root.display()
                    )
                })?;
            println!(
                "loaded workshop item {} ({})",
                report.item.local_id, report.item.title
            );
            print_workshop_validation(
                language,
                "validated loaded package",
                &report.validation_report,
            );
        }
        WorkshopSubcommand::Remix(args) => {
            let report = remix_workshop_library_item(
                &args.library_root,
                &args.source_local_id,
                &args.new_id,
                &args.title,
            )
            .with_context(|| {
                format!(
                    "remix workshop item {} into {} under {}",
                    args.source_local_id,
                    args.new_id,
                    args.library_root.display()
                )
            })?;
            println!(
                "remixed workshop item {} -> {} ({})",
                report.source_local_id, report.item.local_id, report.item.title
            );
            print_workshop_validation(
                language,
                "validated remixed package",
                &report.validation_report,
            );
        }
        WorkshopSubcommand::Block(args) => {
            let item =
                block_workshop_library_item(&args.library_root, &args.local_id, &args.reason)
                    .with_context(|| {
                        format!(
                            "block workshop item {} under {}",
                            args.local_id,
                            args.library_root.display()
                        )
                    })?;
            let reason = item
                .blocked
                .as_ref()
                .map(|blocked| blocked.reason.as_str())
                .unwrap_or("none");
            println!("blocked workshop item {} reason={reason}", item.local_id);
        }
        WorkshopSubcommand::Report(args) => {
            let item =
                report_workshop_library_item(&args.library_root, &args.local_id, &args.reason)
                    .with_context(|| {
                        format!(
                            "report workshop item {} under {}",
                            args.local_id,
                            args.library_root.display()
                        )
                    })?;
            println!(
                "reported workshop item {} reports={}",
                item.local_id,
                item.reports.len()
            );
        }
        WorkshopSubcommand::Delete(args) => {
            let report = delete_workshop_library_item(&args.library_root, &args.local_id)
                .with_context(|| {
                    format!(
                        "delete workshop item {} under {}",
                        args.local_id,
                        args.library_root.display()
                    )
                })?;
            println!(
                "deleted workshop item {} package_dir={}",
                report.local_id,
                report.package_dir.display()
            );
        }
        WorkshopSubcommand::PublishDraft(args) => {
            let report =
                write_workshop_publish_draft(&args.package_dir, &args.out).with_context(|| {
                    format!(
                        "write workshop publish draft from {} to {}",
                        args.package_dir.display(),
                        args.out.display()
                    )
                })?;
            println!(
                "wrote workshop publish draft for {} to {} ({} files)",
                report.draft.package_id,
                report.output_dir.display(),
                report.files_written.len()
            );
            println!(
                "upload_enabled={} steamworks_api_called={}",
                report.draft.upload_enabled, report.draft.steamworks_api_called
            );
        }
        WorkshopSubcommand::UploadDraft(args) => {
            let text = fs::read_to_string(&args.draft_json)
                .with_context(|| format!("read publish draft {}", args.draft_json.display()))?;
            let draft: WorkshopPublishDraft =
                serde_json::from_str(&text).context("parse workshop publish draft json")?;
            let config = SteamworksUploadConfig {
                enabled: args.enable,
                credential_label: args.credential_label,
                app_access_confirmed: args.app_access_confirmed,
            };
            let port = LocalOnlySteamworksUploadPort;
            let report = upload_workshop_publish_draft(&port, &config, &args.package_dir, &draft)
                .with_context(|| {
                format!(
                    "run gated local Workshop upload adapter for {}",
                    args.package_dir.display()
                )
            })?;
            println!(
                "workshop upload adapter {} attempted={} steamworks_api_called={} package={}",
                report.adapter_name,
                report.upload_attempted,
                report.steamworks_api_called,
                report.package_id
            );
            println!("{}", report.message);
        }
        WorkshopSubcommand::SubmissionKit(args) => {
            let package_dir = args.package_dir.clone();
            let out = args.out.clone();
            let request = build_submission_kit_request(&args, language)?;
            let report =
                write_steam_submission_kit(&package_dir, &out, &request).with_context(|| {
                    format!(
                        "write Steam Submission Kit drafts from {} to {}",
                        package_dir.display(),
                        out.display()
                    )
                })?;
            println!(
                "wrote Steam Submission Kit drafts for {} to {} ({} files)",
                report.draft.workshop_package_id,
                report.output_dir.display(),
                report.files_written.len()
            );
        }
    }
    Ok(())
}

/// Build the Steam Submission Kit request either from full `--batch` flags or
/// by stepping through an interactive dialoguer wizard. The wizard only
/// collects parameters; all validation and draft generation stay in
/// `plotforge-workshop`.
fn build_submission_kit_request(
    args: &WorkshopSubmissionKitArgs,
    language: OutputLanguage,
) -> Result<SteamSubmissionKitRequest> {
    if args.batch {
        build_submission_kit_batch(args)
    } else {
        build_submission_kit_interactive(args, language)
    }
}

/// `--batch` path: every required field must already be supplied via flags.
fn build_submission_kit_batch(
    args: &WorkshopSubmissionKitArgs,
) -> Result<SteamSubmissionKitRequest> {
    let product_name = args
        .product_name
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--batch requires --product-name"))?;
    let store_short_description = args
        .store_short_description
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--batch requires --store-short-description"))?;
    let user_reporting_path = args
        .user_reporting_path
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--batch requires --user-reporting-path"))?;
    let moderation_policy = args
        .moderation_policy
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--batch requires --moderation-policy"))?;
    Ok(SteamSubmissionKitRequest {
        product_name,
        desktop_build_path: args.desktop_build_path.clone(),
        store_short_description,
        screenshot_paths: args.screenshot_paths.clone(),
        capsule_asset_paths: args.capsule_asset_paths.clone(),
        content_warnings: args.content_warnings.clone(),
        safety_guardrails: args.safety_guardrails.clone(),
        user_reporting_path,
        moderation_policy,
        build_notes: args.build_notes.clone(),
    })
}

/// Interactive wizard: prompts for each field, pre-filling any flag-supplied
/// values. Refuses to run without a TTY so pipes/CI get an explicit error
/// directing them to `--batch` instead of hanging on a blocked stdin.
fn build_submission_kit_interactive(
    args: &WorkshopSubmissionKitArgs,
    language: OutputLanguage,
) -> Result<SteamSubmissionKitRequest> {
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "submission-kit interactive wizard requires a TTY; pass --batch with full arguments"
        );
    }

    let product_name = prompt_required_string(
        args.product_name.as_deref(),
        "Product name",
        "产品名称",
        language,
    )?;
    let store_short_description = prompt_required_string(
        args.store_short_description.as_deref(),
        "Store short description",
        "商店简短描述",
        language,
    )?;
    let desktop_build_path = prompt_optional_string(
        args.desktop_build_path.as_deref(),
        "Desktop build path (optional)",
        "桌面构建路径（可选）",
        language,
    )?;
    let user_reporting_path = prompt_required_string(
        args.user_reporting_path.as_deref(),
        "User reporting path",
        "用户举报路径",
        language,
    )?;
    let moderation_policy = prompt_required_string(
        args.moderation_policy.as_deref(),
        "Moderation policy",
        "审核策略",
        language,
    )?;
    let screenshot_paths = prompt_string_list(
        &args.screenshot_paths,
        "Screenshot path",
        "截图路径",
        language,
    )?;
    let capsule_asset_paths = prompt_string_list(
        &args.capsule_asset_paths,
        "Capsule asset path",
        "胶囊素材路径",
        language,
    )?;
    let content_warnings = prompt_string_list(
        &args.content_warnings,
        "Content warning",
        "内容警告",
        language,
    )?;
    let safety_guardrails = prompt_string_list(
        &args.safety_guardrails,
        "Safety guardrail",
        "安全护栏",
        language,
    )?;
    let build_notes = prompt_string_list(&args.build_notes, "Build note", "构建说明", language)?;

    Ok(SteamSubmissionKitRequest {
        product_name,
        desktop_build_path,
        store_short_description,
        screenshot_paths,
        capsule_asset_paths,
        content_warnings,
        safety_guardrails,
        user_reporting_path,
        moderation_policy,
        build_notes,
    })
}

fn prompt_label(en: &str, zh: &str, language: OutputLanguage) -> String {
    match language {
        OutputLanguage::En => en.to_string(),
        OutputLanguage::Zh => zh.to_string(),
    }
}

fn prompt_required_string(
    default: Option<&str>,
    en: &str,
    zh: &str,
    language: OutputLanguage,
) -> Result<String> {
    let prompt = prompt_label(en, zh, language);
    let input = Input::<String>::new().with_prompt(prompt);
    let input = if let Some(default) = default {
        input.default(default.to_string())
    } else {
        input
    };
    let value: String = input
        .validate_with(|v: &String| -> Result<(), &str> {
            if v.trim().is_empty() {
                Err("value is required")
            } else {
                Ok(())
            }
        })
        .interact_text()?;
    Ok(value)
}

fn prompt_optional_string(
    default: Option<&str>,
    en: &str,
    zh: &str,
    language: OutputLanguage,
) -> Result<Option<String>> {
    let prompt = prompt_label(en, zh, language);
    let input = Input::<String>::new().with_prompt(prompt).allow_empty(true);
    let input = if let Some(default) = default {
        input.default(default.to_string())
    } else {
        input
    };
    let value: String = input.interact_text()?;
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed))
    }
}

fn prompt_string_list(
    existing: &[String],
    en: &str,
    zh: &str,
    language: OutputLanguage,
) -> Result<Vec<String>> {
    let label = prompt_label(en, zh, language);
    let mut values: Vec<String> = existing.to_vec();
    loop {
        let prompt = format!("{} #{} (empty to finish)", label, values.len() + 1);
        let value: String = Input::<String>::new()
            .with_prompt(prompt)
            .allow_empty(true)
            .interact_text()?;
        if value.trim().is_empty() {
            break;
        }
        values.push(value.trim().to_string());
    }
    Ok(values)
}
