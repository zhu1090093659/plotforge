use std::{
    error::Error,
    fs,
    path::Component,
    path::{Path, PathBuf},
};

use plotforge_export::{export_static_web, export_static_web_zip};
use plotforge_runtime::{RuntimeSession, summarize_delta};
pub use plotforge_schema::{
    AiSafetyPolicy, AssetRecord, AudioBible, Character, CharacterEditDocument,
    CharacterGenerationReport, CharacterGenerationRequest, Condition, Effect, ExportProfile,
    ProjectCreationReport, ProjectCreationRequest, ProjectData, ProjectTemplateId,
    ResourceDefinition, Rule, RulesEditDocument, RuntimeSnapshot, RuntimeTrace, Scene,
    StateVariablesEditDocument, StoryCraftEditDocument, StoryCraftGenerationReport,
    StoryCraftGenerationRequest, VisualBible, WorldEditDocument, WorldGenerationReport,
    WorldGenerationRequest,
};
use plotforge_storage::{
    create_project_from_request, load_project, read_latest_runtime_snapshot, read_runtime_snapshot,
    validate_project, validate_runtime_snapshot_id, write_runtime_snapshot, write_trace,
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
    use std::fs;

    use plotforge_storage::create_demo_project;
    use tempfile::tempdir;

    use super::{
        AiSafetyPolicy, Character, Effect, ResourceDefinition, Rule, check_project,
        create_character, create_project, create_resource, create_rule, export_static_project,
        export_static_project_zip, generate_character, generate_story_craft,
        generate_world_expansion, list_asset_records, list_export_profiles, list_source_files,
        open_project, play_once_project, play_once_project_from_latest_snapshot,
        play_once_project_from_snapshot, play_once_project_with_save, read_ai_safety_policy,
        read_character_edit_document, read_rules_edit_document, read_source_file,
        read_state_variables_edit_document, read_story_craft_edit_document,
        read_world_edit_document, update_ai_safety_policy, update_story_craft_edit_document,
        update_world_edit_document, write_source_file,
    };

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
    fn structured_edit_commands_delegate_to_storage_and_reopen() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

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
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

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
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

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
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

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
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

        let project = open_project(&project_path).expect("open project");

        assert_eq!(project.game.title, "Dynasty Embers");
        assert_eq!(project.game.entry_scene, "court-crisis-001");
        assert_eq!(project.scenes.len(), 1);
    }

    #[test]
    fn list_asset_records_returns_rebuilt_media_registry_records() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

        let records = list_asset_records(&project_path).expect("asset records");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, plotforge_schema::AssetKind::Image);
        assert_eq!(records[0].references[0].reference_id, "court-crisis-001");
        assert_eq!(records[0].references[0].slot, "background_asset");
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

        let report = play_once_project(&project_path, "朕决定加征辽饷").expect("play once");

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
    fn play_once_project_saves_and_restores_runtime_snapshot() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

        let first = play_once_project_with_save(&project_path, "朕决定加征辽饷", Some("save-001"))
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
            "先拨内帑稳住边军军饷",
            "save-001",
            Some("save-002"),
        )
        .expect("restored play");

        assert_eq!(second.trace.selected_choice.as_deref(), Some("pay-army"));
        assert_eq!(second.trace.story_state_before.turn, 1);
        assert_eq!(second.trace.story_state_after.turn, 2);
        assert_eq!(second.snapshot.as_ref().expect("snapshot").id, "save-002");
        assert!(
            project_path
                .join("saves/save-002.runtime_snapshot.json")
                .is_file()
        );

        let latest = play_once_project_from_latest_snapshot(
            &project_path,
            "朕决定加征辽饷",
            Some("save-003"),
        )
        .expect("latest restored play");
        assert_eq!(latest.trace.story_state_before.turn, 2);
        assert_eq!(latest.snapshot.as_ref().expect("snapshot").id, "save-003");
    }

    #[test]
    fn play_once_project_rejects_corrupted_runtime_snapshot_explicitly() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");
        fs::write(
            project_path.join("saves/corrupt.runtime_snapshot.json"),
            "{not-json\n",
        )
        .expect("corrupt snapshot");

        let error =
            play_once_project_from_snapshot(&project_path, "朕决定加征辽饷", "corrupt", None)
                .expect_err("corrupt snapshot");

        assert_eq!(error.code, "play_once_restore_snapshot");
        assert!(error.message.contains("json error"));
        assert!(!project_path.join("traces/latest.json").exists());
    }

    #[test]
    fn play_once_project_rejects_invalid_save_id_before_trace_write() {
        let temp = tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");
        create_demo_project(&project_path, true).expect("demo");

        let error = play_once_project_with_save(&project_path, "朕决定加征辽饷", Some("../escape"))
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
        let project_path = temp.path().join("dynasty-embers");
        let export_path = temp.path().join("export");
        create_demo_project(&project_path, true).expect("demo");

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
        let project_path = temp.path().join("dynasty-embers");
        let export_path = temp.path().join("export");
        let archive_path = temp.path().join("dynasty-embers-static.zip");
        create_demo_project(&project_path, true).expect("demo");

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
        assert_eq!(play.scene.key, "court-crisis-001");
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

    fn sample_creation_request() -> super::ProjectCreationRequest {
        super::ProjectCreationRequest {
            template: super::ProjectTemplateId::HistoricalCrisis,
            concept: "A regency court must survive a winter coup.".into(),
            visual_style: "ink wash court drama".into(),
            voice_enabled: true,
            initial_scene_request: "Open on an empty granary ledger.".into(),
        }
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
}
