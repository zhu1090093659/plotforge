use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use plotforge_export::{export_desktop_runtime_draft, export_static_web, export_static_web_zip};
use plotforge_runtime::{RuntimeSession, summarize_delta};
use plotforge_schema::{
    DESKTOP_RUNTIME_DRAFT_FILE, ExportProfileTarget, ProjectCreationRequest, ProjectTemplateId,
    supported_export_profiles,
};
use plotforge_storage::{
    create_demo_project, create_project_from_request, load_project, read_latest_runtime_snapshot,
    read_runtime_snapshot, validate_project, validate_runtime_snapshot_id, write_runtime_snapshot,
    write_trace,
};

#[derive(Debug, Parser)]
#[command(name = "plotforge")]
#[command(about = "PlotForge CLI-first MVP")]
struct Cli {
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
    Demo(NewDemoArgs),
    Project(NewProjectArgs),
}

#[derive(Debug, Args)]
struct NewDemoArgs {
    #[arg(long, default_value = "dynasty-embers")]
    path: PathBuf,
    #[arg(long)]
    force: bool,
}

#[derive(Clone, Debug, ValueEnum)]
enum ProjectTemplateArg {
    HistoricalCrisis,
    DynastyEmbers,
}

impl From<ProjectTemplateArg> for ProjectTemplateId {
    fn from(value: ProjectTemplateArg) -> Self {
        match value {
            ProjectTemplateArg::HistoricalCrisis => ProjectTemplateId::HistoricalCrisis,
            ProjectTemplateArg::DynastyEmbers => ProjectTemplateId::DynastyEmbers,
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
    #[arg(long, default_value = "朕决定加征辽饷")]
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::New(command) => handle_new(command),
        Command::Check(args) => handle_check(args),
        Command::Play(args) => handle_play(args),
        Command::Trace(command) => handle_trace(command),
        Command::Export(command) => handle_export(command),
    }
}

fn handle_new(command: NewCommand) -> Result<()> {
    match command.command {
        NewSubcommand::Demo(args) => {
            let project = create_demo_project(&args.path, args.force)
                .with_context(|| format!("create demo project at {}", args.path.display()))?;
            println!("created {} at {}", project.game.title, args.path.display());
        }
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
            println!(
                "created project {} at {} ({} files)",
                report.project.game.title,
                args.path.display(),
                report.files_created.len()
            );
        }
    }
    Ok(())
}

fn handle_check(args: ProjectPath) -> Result<()> {
    let project = validate_project(&args.path)
        .with_context(|| format!("validate project at {}", args.path.display()))?;
    println!(
        "ok: {} ({} scenes, {} rules, {} characters)",
        project.game.title,
        project.scenes.len(),
        project.rules.len(),
        project.characters.len()
    );
    Ok(())
}

fn handle_play(args: PlayArgs) -> Result<()> {
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

    println!("scene: {} - {}", step.scene.key, step.scene.title);
    println!(
        "choice: {}",
        step.trace.selected_choice.as_deref().unwrap_or("none")
    );
    println!("delta:");
    for line in summarize_delta(&step.trace.world_state_delta) {
        println!("  {line}");
    }
    println!("trace: {}", trace_path.display());
    if let Some(snapshot_path) = snapshot_path {
        println!("snapshot: {}", snapshot_path.display());
    }
    Ok(())
}

fn handle_trace(command: TraceCommand) -> Result<()> {
    match command.command {
        TraceSubcommand::Inspect(args) => {
            let text = fs::read_to_string(&args.path)
                .with_context(|| format!("read trace {}", args.path.display()))?;
            let trace: plotforge_schema::RuntimeTrace =
                serde_json::from_str(&text).context("parse trace json")?;
            println!("trace: {}", trace.id);
            println!("run seed: {}", trace.reproducibility.run_seed);
            println!("prompt version: {}", trace.reproducibility.prompt_version);
            println!("model version: {}", trace.reproducibility.model_version);
            println!(
                "provider config hash: {}",
                trace.reproducibility.provider_config_hash
            );
            println!(
                "trace evidence id: {}",
                trace.reproducibility.trace_id.as_deref().unwrap_or("none")
            );
            println!(
                "snapshot evidence id: {}",
                trace
                    .reproducibility
                    .snapshot_id
                    .as_deref()
                    .unwrap_or("none")
            );
            println!(
                "player input: {}",
                trace.player_input.as_deref().unwrap_or("none")
            );
            println!(
                "selected: {}",
                trace.selected_choice.as_deref().unwrap_or("none")
            );
            if let Some(intent) = trace.action_intent.as_ref() {
                println!(
                    "intent: {}",
                    intent.action_type.as_deref().unwrap_or("unsupported")
                );
                println!("intent status: {:?}", intent.status);
                println!(
                    "intent choice: {}",
                    intent.choice_id.as_deref().unwrap_or("none")
                );
                println!(
                    "intent action: {}",
                    intent.action_type.as_deref().unwrap_or("unsupported")
                );
                println!("intent matched terms: {}", intent.matched_terms.join(", "));
                println!(
                    "intent reason: {}",
                    intent.reason.as_deref().unwrap_or("none")
                );
            }
            if let Some(rule_result) = trace.rule_result.as_ref() {
                println!(
                    "rule: {} (delta empty: {}, committed: {})",
                    rule_result.action_type, rule_result.delta_empty, rule_result.state_committed
                );
                println!("rule action: {}", rule_result.action_type);
                println!("rule delta empty: {}", rule_result.delta_empty);
                println!("rule committed: {}", rule_result.state_committed);
                if let Some(error) = rule_result.error.as_ref() {
                    println!("rule error: {} - {}", error.code, error.message);
                }
            }
            if let Some(planner_result) = trace.planner_result.as_ref() {
                println!(
                    "planner: {} (fallback: {})",
                    planner_result.scene_key.as_deref().unwrap_or("none"),
                    planner_result.fallback_used
                );
                println!(
                    "planner requested: {}",
                    planner_result.requested_action_type
                );
                println!(
                    "planner scene: {}",
                    planner_result.scene_key.as_deref().unwrap_or("none")
                );
                println!("planner fallback: {}", planner_result.fallback_used);
                if let Some(error) = planner_result.error.as_ref() {
                    println!("planner error: {} - {}", error.code, error.message);
                }
            }
            println!("fallback: {}", trace.fallback_used);
            println!(
                "story before: scene={} beat={} turn={}",
                trace.story_state_before.current_scene_key,
                trace
                    .story_state_before
                    .current_beat_id
                    .as_deref()
                    .unwrap_or("none"),
                trace.story_state_before.turn
            );
            println!(
                "story after: scene={} beat={} turn={}",
                trace.story_state_after.current_scene_key,
                trace
                    .story_state_after
                    .current_beat_id
                    .as_deref()
                    .unwrap_or("none"),
                trace.story_state_after.turn
            );
            println!("world delta:");
            for line in summarize_delta(&trace.world_state_delta) {
                println!("  {line}");
            }
            println!("diagnostics: {}", trace.diagnostics.len());
            for diagnostic in &trace.diagnostics {
                println!(
                    "diagnostic: {:?} {:?} - {}",
                    diagnostic.stage, diagnostic.status, diagnostic.message
                );
            }
            println!("media references: {}", trace.media_references.len());
            for media in &trace.media_references {
                println!(
                    "media: {:?} {} {} -> {}",
                    media.reference.reference_kind,
                    media.reference.reference_id,
                    media.reference.slot,
                    media.project_path
                );
            }
            for error in &trace.errors {
                println!("error: {} - {}", error.code, error.message);
            }
            if let Some(review) = trace.narrative_review.as_ref() {
                println!("review scene: {}", review.scene_key);
                println!("review score: {}", review.score);
                println!("review issues: {}", review.issues.len());
                for issue in &review.issues {
                    println!(
                        "review issue: {:?} {:?} - {}",
                        issue.kind, issue.severity, issue.message
                    );
                }
            }
        }
    }
    Ok(())
}

fn handle_export(command: ExportCommand) -> Result<()> {
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
                println!(
                    "exported static zip to {} ({} files)",
                    zip_report.archive_path.display(),
                    zip_report.archived_files.len()
                );
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
            println!(
                "exported static player to {} ({} files)",
                report.output_dir.display(),
                report.files_written.len()
            );
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
            println!(
                "exported desktop runtime draft to {} ({} files)",
                report.output_dir.display(),
                report.files_written.len()
            );
            println!(
                "desktop runtime draft: {}",
                report.output_dir.join(DESKTOP_RUNTIME_DRAFT_FILE).display()
            );
            println!(
                "desktop build notes: {}",
                report.output_dir.join("desktop-build-notes.md").display()
            );
        }
    }
    Ok(())
}

fn export_profile_target_label(target: &ExportProfileTarget) -> &'static str {
    match target {
        ExportProfileTarget::StaticWeb => "static_web",
        ExportProfileTarget::DynamicWeb => "dynamic_web",
        ExportProfileTarget::DesktopBundle => "desktop_bundle",
        ExportProfileTarget::SteamWorkshop => "steam_workshop",
        ExportProfileTarget::SteamSubmissionKit => "steam_submission_kit",
    }
}
