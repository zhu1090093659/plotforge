use std::{
    error::Error,
    fs,
    path::Component,
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
        PathBuf::from("story/story_craft.toml"),
        PathBuf::from("story/emotional_arc.json"),
        PathBuf::from("story/plot_threads.toml"),
        PathBuf::from("story/story_bible.md"),
        PathBuf::from("story/style_guide.md"),
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
    use std::fs;

    use plotforge_storage::create_demo_project;
    use tempfile::tempdir;

    use super::{
        check_project, export_static_project, list_source_files, open_project, play_once_project,
        read_source_file, write_source_file,
    };

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

    #[test]
    fn list_source_files_marks_safe_text_surfaces_editable() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

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
                .any(|file| file.path == "scenes/court-crisis-001.scene.json" && !file.editable)
        );
    }

    #[test]
    fn read_source_file_returns_content_and_metadata() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

        let file = read_source_file(&project_path, "world/world.md").expect("read source");

        assert_eq!(file.path, "world/world.md");
        assert!(file.editable);
        assert!(file.content.contains("World Bible"));
    }

    #[test]
    fn read_source_file_rejects_supported_but_unlisted_files() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");
        fs::write(project_path.join("provider_config.json"), "{}\n").expect("provider config");

        let error = read_source_file(&project_path, "provider_config.json")
            .expect_err("unlisted source should fail");

        assert_eq!(error.code, "read_source_file");
        assert!(error.message.contains("not a project source file"));
    }

    #[test]
    fn write_source_file_updates_editable_markdown_only() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

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
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

        let traversal = write_source_file(&project_path, "../outside.md", "bad")
            .expect_err("traversal should fail");
        let readonly =
            write_source_file(&project_path, "game.toml", "bad").expect_err("readonly should fail");

        assert_eq!(traversal.code, "write_source_file");
        assert_eq!(readonly.code, "write_source_file");
        assert!(readonly.message.contains("not an editable source file"));
    }
}
