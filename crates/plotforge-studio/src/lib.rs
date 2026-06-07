use std::{
    error::Error,
    path::{Path, PathBuf},
};

use plotforge_export::export_static_web;
use plotforge_runtime::{RuntimeSession, summarize_delta};
pub use plotforge_schema::{ProjectData, RuntimeTrace, Scene};
use plotforge_storage::{load_project, validate_project, write_trace};
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
    pub delta_summary: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct StaticExportReport {
    pub output_dir: String,
    pub files_written: Vec<String>,
    pub allowed_files: Vec<String>,
    pub files_found: Vec<String>,
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

pub fn play_once_project(
    path: impl AsRef<Path>,
    player_input: &str,
) -> StudioCommandResult<PlayOnceReport> {
    let path = path.as_ref();
    let project =
        load_project(path).map_err(|source| command_error("play_once_load", path, source))?;
    let mut session = RuntimeSession::new(project);
    let step = session
        .play_once(player_input)
        .map_err(|source| command_error("play_once_runtime", path, source))?;
    let trace_path = write_trace(path, &step.trace)
        .map_err(|source| command_error("play_once_trace", path, source))?;

    Ok(PlayOnceReport {
        delta_summary: summarize_delta(&step.trace.world_state_delta),
        scene: step.scene,
        trace: step.trace,
        trace_path: path_string(trace_path),
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
        files_written: report.files_written.into_iter().map(path_string).collect(),
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

fn command_error(code: impl Into<String>, path: &Path, source: impl Error) -> StudioCommandError {
    StudioCommandError {
        code: code.into(),
        message: format!("{}: {source}", path.display()),
    }
}

fn path_string(path: PathBuf) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use plotforge_storage::create_demo_project;
    use tempfile::tempdir;

    use super::{check_project, export_static_project, open_project, play_once_project};

    #[test]
    fn open_project_returns_contract_project_data() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

        let project = open_project(&project_path).expect("open project");

        assert_eq!(project.game.title, "Dynasty Embers");
        assert_eq!(project.game.entry_scene, "court-crisis-001");
        assert_eq!(project.scenes.len(), 1);
    }

    #[test]
    fn check_project_returns_counts_from_storage_validation() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

        let report = check_project(&project_path).expect("check project");

        assert_eq!(report.title, "Dynasty Embers");
        assert_eq!(report.entry_scene, "court-crisis-001");
        assert_eq!(report.scene_count, 1);
        assert_eq!(report.rule_count, 3);
        assert_eq!(report.character_count, 6);
    }

    #[test]
    fn play_once_project_runs_runtime_and_writes_trace() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

        let report = play_once_project(&project_path, "朕决定加征辽饷，同时严查贪墨官员。")
            .expect("play once");

        assert_eq!(report.scene.key, "court-crisis-001");
        assert!(report.trace_path.ends_with("traces/trace-001.json"));
        assert!(
            fs::metadata(&report.trace_path)
                .expect("trace file")
                .is_file()
        );
        assert_eq!(report.trace.selected_choice.as_deref(), Some("raise-tax"));
        assert!(
            report
                .delta_summary
                .iter()
                .any(|line| line == "treasury: +12")
        );
    }

    #[test]
    fn export_static_project_delegates_to_export_crate() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        let export_path = temp.path().join("export");
        create_demo_project(&project_path, true).expect("demo");

        let report = export_static_project(&project_path, &export_path).expect("export static");

        assert_eq!(report.output_dir, export_path.display().to_string());
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
    fn missing_project_returns_explicit_error_code() {
        let temp = tempdir().expect("tempdir");
        let missing_path = temp.path().join("missing");

        let error = check_project(&missing_path).expect_err("missing project should fail");

        assert_eq!(error.code, "check_project");
        assert!(error.message.contains("game.toml"));
    }
}
