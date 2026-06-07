use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use plotforge_export::export_static_web;
use plotforge_runtime::{RuntimeSession, summarize_delta};
use plotforge_storage::{create_demo_project, load_project, validate_project, write_trace};

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
}

#[derive(Debug, Args)]
struct NewDemoArgs {
    #[arg(long, default_value = "dynasty-embers")]
    path: PathBuf,
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Args)]
struct PlayArgs {
    #[arg(default_value = ".")]
    path: PathBuf,
    #[arg(long)]
    once: bool,
    #[arg(long, default_value = "朕决定加征辽饷，同时严查贪墨官员。")]
    input: String,
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
    Static(ExportStaticArgs),
}

#[derive(Debug, Args)]
struct ExportStaticArgs {
    #[arg(default_value = ".")]
    path: PathBuf,
    #[arg(long, default_value = "exports/static")]
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

    let project = load_project(&args.path)
        .with_context(|| format!("load project at {}", args.path.display()))?;
    let mut session = RuntimeSession::new(project);
    let step = session
        .play_once(&args.input)
        .context("run one deterministic play step")?;
    let trace_path = write_trace(&args.path, &step.trace)
        .with_context(|| format!("write trace under {}", args.path.display()))?;

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
            println!(
                "selected: {}",
                trace.selected_choice.as_deref().unwrap_or("none")
            );
            if let Some(intent) = trace.action_intent.as_ref() {
                println!(
                    "intent: {}",
                    intent.action_type.as_deref().unwrap_or("unsupported")
                );
            }
            if let Some(rule_result) = trace.rule_result.as_ref() {
                println!(
                    "rule: {} (delta empty: {}, committed: {})",
                    rule_result.action_type, rule_result.delta_empty, rule_result.state_committed
                );
            }
            if let Some(planner_result) = trace.planner_result.as_ref() {
                println!(
                    "planner: {} (fallback: {})",
                    planner_result.scene_key.as_deref().unwrap_or("none"),
                    planner_result.fallback_used
                );
            }
            println!("fallback: {}", trace.fallback_used);
            println!("world delta:");
            for line in summarize_delta(&trace.world_state_delta) {
                println!("  {line}");
            }
            println!("diagnostics: {}", trace.diagnostics.len());
            for error in &trace.errors {
                println!("error: {} - {}", error.code, error.message);
            }
            if let Some(review) = trace.narrative_review.as_ref() {
                println!("review score: {}", review.score);
                println!("review issues: {}", review.issues.len());
            }
        }
    }
    Ok(())
}

fn handle_export(command: ExportCommand) -> Result<()> {
    match command.command {
        ExportSubcommand::Static(args) => {
            let report = export_static_web(&args.path, &args.out).with_context(|| {
                format!(
                    "export static web from {} to {}",
                    args.path.display(),
                    args.out.display()
                )
            })?;
            println!(
                "exported static player to {} ({} files)",
                report.output_dir.display(),
                report.files_written.len()
            );
        }
    }
    Ok(())
}
