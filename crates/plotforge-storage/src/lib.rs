use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use plotforge_schema::{
    AiSafetyPolicy, AiUsageContentKind, AssetKind, AssetRecord, AssetReference, AssetReferenceKind,
    AssetSourceKind, AudioBible, AudioVoiceCard, Beat, BeatNext, Character, CharacterEditDocument,
    CharacterGenerationReport, CharacterGenerationRequest, Choice, Condition, Effect, GameProject,
    GenerationEvidence, GenerationStatus, MAX_REFERENCE_STRUCTURE_NOTE_CHARS,
    MAX_REFERENCE_SUMMARY_CHARS, MediaAssetReference, PlotThread, ProjectCreationReport,
    ProjectCreationRequest, ProjectData, ProjectTemplateId, ReferenceAnalysis, ReferenceRights,
    ReferenceSource, ReferenceSourceType, ReferenceStructureNote, ResourceDefinition, Rule,
    RulesEditDocument, RuntimeSnapshot, Scene, StateVariablesEditDocument, StoryCraftEditDocument,
    StoryCraftGenerationReport, StoryCraftGenerationRequest, StoryState, VisualBible,
    VisualStyleCard, WorldEditDocument, WorldGenerationReport, WorldGenerationRequest, WorldState,
    contains_secret_marker_text,
};
use plotforge_storycraft::dynasty_embers_story_craft;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

mod sqlite_cache;

pub use sqlite_cache::{
    SQLITE_CACHE_SCHEMA_VERSION, SqliteCacheSummary, read_sqlite_cache_summary,
    rebuild_sqlite_cache, sqlite_cache_path,
};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("project path already exists and is not empty: {0}")]
    ProjectExists(PathBuf),
    #[error("missing required project file: {0}")]
    MissingFile(PathBuf),
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("toml serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("toml parse error at {path}: {source}")]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("json error at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("sqlite error at {path}: {source}")]
    Sqlite {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    #[error("media cache error at {path}: {source}")]
    Media {
        path: PathBuf,
        #[source]
        source: plotforge_media::MediaError,
    },
    #[error("unsupported sqlite cache schema version: expected {expected}, got {actual}")]
    UnsupportedSqliteCacheSchemaVersion { expected: u32, actual: u32 },
    #[error("invalid sqlite cache value: {0}")]
    InvalidSqliteCacheValue(String),
    #[error("project path cannot be indexed as UTF-8: {0}")]
    InvalidProjectPath(PathBuf),
    #[error("invalid runtime snapshot id: {0}")]
    InvalidRuntimeSnapshotId(String),
    #[error("invalid project creation request field {field}: {reason}")]
    InvalidProjectCreationRequest { field: String, reason: String },
    #[error("invalid generation request field {field}: {reason}")]
    InvalidGenerationRequest { field: String, reason: String },
    #[error("invalid generation report for {surface}: {reason}")]
    InvalidGenerationReport { surface: String, reason: String },
    #[error("invalid AI safety policy field {field}: {reason}")]
    InvalidAiSafetyPolicy { field: String, reason: String },
    #[error("invalid structured edit for {surface} field {field}: {reason}")]
    InvalidStructuredEdit {
        surface: String,
        field: String,
        reason: String,
    },
    #[error("reference compliance error at {path}: {reason}")]
    ReferenceCompliance { path: PathBuf, reason: String },
}

pub const MAX_REFERENCE_RAW_TEXT_BYTES: u64 = 4096;
const AI_SAFETY_POLICY_PATH: &str = "safety/ai_safety_policy.toml";
const VISUAL_BIBLE_PATH: &str = "media/visual_bible.toml";
const AUDIO_BIBLE_PATH: &str = "media/audio_bible.toml";

#[derive(Debug, Serialize, Deserialize)]
struct ResourceFile {
    resources: Vec<ResourceDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoryCraftFile {
    story_craft: plotforge_schema::StoryCraftState,
}

#[derive(Debug, Serialize, Deserialize)]
struct RulesFile {
    rules: Vec<Rule>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlotThreadsFile {
    plot_threads: Vec<PlotThread>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ForbiddenFactsFile {
    forbidden_facts: Vec<String>,
}

pub fn create_demo_project(
    path: impl AsRef<Path>,
    force: bool,
) -> Result<ProjectData, StorageError> {
    let path = path.as_ref();
    ensure_project_path_available(path, force)?;
    let project = dynasty_embers_project();
    fs::create_dir_all(path).map_io(path)?;
    create_project_dirs(path)?;
    write_project(path, &project, None)?;
    load_project(path)
}

pub fn create_project_from_request(
    path: impl AsRef<Path>,
    request: ProjectCreationRequest,
    force: bool,
) -> Result<ProjectCreationReport, StorageError> {
    let path = path.as_ref();
    validate_project_creation_request(&request)?;
    ensure_project_path_available(path, force)?;

    let project = project_from_creation_request(path, &request);
    fs::create_dir_all(path).map_io(path)?;
    create_project_dirs(path)?;
    let files_created = write_project(path, &project, Some(&request))?
        .into_iter()
        .map(|path| path.display().to_string())
        .collect();
    let project = load_project(path)?;

    Ok(ProjectCreationReport {
        project_path: path.display().to_string(),
        template: request.template,
        concept: request.concept,
        visual_style: request.visual_style,
        voice_enabled: request.voice_enabled,
        initial_scene_request: request.initial_scene_request,
        files_created,
        project,
    })
}

pub fn load_project(path: impl AsRef<Path>) -> Result<ProjectData, StorageError> {
    let path = path.as_ref();
    let game = read_toml::<GameProject>(&path.join("game.toml"))?;
    let resources = read_toml::<ResourceFile>(&path.join("world/resources.toml"))?.resources;
    let world_state = read_json::<WorldState>(&path.join("world/initial_state.json"))?;
    let story_craft =
        read_toml::<StoryCraftFile>(&path.join("story/story_craft.toml"))?.story_craft;
    let story_state = read_json::<StoryState>(&path.join("saves/initial_story_state.json"))?;
    let characters = read_collection(path.join("characters"), ".character.toml")?;
    let rules = read_toml::<RulesFile>(&path.join("rules/rules.toml"))?.rules;
    let scenes = read_collection(path.join("scenes"), ".scene.json")?;
    validate_scene_audio_references(&scenes)?;
    let visual_bible = read_visual_bible(path)?;
    let audio_bible = read_audio_bible(path)?;
    let ai_safety_policy = read_ai_safety_policy(path)?;
    let mut project = ProjectData {
        game,
        resources,
        world_state,
        story_state,
        story_craft,
        characters,
        rules,
        scenes,
        visual_bible,
        audio_bible,
        asset_records: Vec::new(),
        ai_safety_policy,
    };
    project.asset_records = rebuild_asset_records(path, &project)?;

    Ok(project)
}

pub fn validate_project(path: impl AsRef<Path>) -> Result<ProjectData, StorageError> {
    let path = path.as_ref();
    validate_reference_library(path)?;
    let project = load_project(path)?;
    let entry_scene = project.game.entry_scene.as_str();
    if project.scene(entry_scene).is_none() {
        return Err(StorageError::MissingFile(PathBuf::from(format!(
            "scenes/{entry_scene}.scene.json"
        ))));
    }
    Ok(project)
}

pub fn read_world_edit_document(
    project_path: impl AsRef<Path>,
) -> Result<WorldEditDocument, StorageError> {
    let project_path = project_path.as_ref();
    Ok(WorldEditDocument {
        world_bible_markdown: read_text(&project_path.join("world/world.md"))?,
        canon_markdown: read_text(&project_path.join("world/canon.md"))?,
        forbidden_facts: read_json::<ForbiddenFactsFile>(
            &project_path.join("world/forbidden_facts.json"),
        )?
        .forbidden_facts,
    })
}

pub fn update_world_edit_document(
    project_path: impl AsRef<Path>,
    document: WorldEditDocument,
) -> Result<WorldEditDocument, StorageError> {
    let project_path = project_path.as_ref();
    validate_world_edit_document(&document)?;
    write_text(
        &project_path.join("world/world.md"),
        &document.world_bible_markdown,
    )?;
    write_text(
        &project_path.join("world/canon.md"),
        &document.canon_markdown,
    )?;
    write_json(
        &project_path.join("world/forbidden_facts.json"),
        &ForbiddenFactsFile {
            forbidden_facts: document.forbidden_facts.clone(),
        },
    )?;
    Ok(document)
}

pub fn read_story_craft_edit_document(
    project_path: impl AsRef<Path>,
) -> Result<StoryCraftEditDocument, StorageError> {
    let project_path = project_path.as_ref();
    Ok(StoryCraftEditDocument {
        story_bible_markdown: read_text(&project_path.join("story/story_bible.md"))?,
        style_guide_markdown: read_text(&project_path.join("story/style_guide.md"))?,
        story_craft: read_toml::<StoryCraftFile>(&project_path.join("story/story_craft.toml"))?
            .story_craft,
    })
}

pub fn update_story_craft_edit_document(
    project_path: impl AsRef<Path>,
    document: StoryCraftEditDocument,
) -> Result<StoryCraftEditDocument, StorageError> {
    let project_path = project_path.as_ref();
    let characters = read_collection(project_path.join("characters"), ".character.toml")?;
    validate_story_craft_edit_document(&document, &characters)?;
    write_text(
        &project_path.join("story/story_bible.md"),
        &document.story_bible_markdown,
    )?;
    write_text(
        &project_path.join("story/style_guide.md"),
        &document.style_guide_markdown,
    )?;
    write_toml(
        &project_path.join("story/story_craft.toml"),
        &StoryCraftFile {
            story_craft: document.story_craft.clone(),
        },
    )?;
    write_json(
        &project_path.join("story/emotional_arc.json"),
        &document.story_craft.emotional_arc,
    )?;
    write_toml(
        &project_path.join("story/plot_threads.toml"),
        &PlotThreadsFile {
            plot_threads: document.story_craft.plot_threads.clone(),
        },
    )?;
    Ok(document)
}

pub fn read_character_edit_document(
    project_path: impl AsRef<Path>,
) -> Result<CharacterEditDocument, StorageError> {
    Ok(CharacterEditDocument {
        characters: read_collection(project_path.as_ref().join("characters"), ".character.toml")?,
    })
}

pub fn update_character_edit_document(
    project_path: impl AsRef<Path>,
    document: CharacterEditDocument,
) -> Result<CharacterEditDocument, StorageError> {
    let project_path = project_path.as_ref();
    validate_character_edit_document(&document)?;
    clear_collection_files(&project_path.join("characters"), ".character.toml")?;
    for character in &document.characters {
        write_toml(
            &project_path.join(format!("characters/{}.character.toml", character.id)),
            character,
        )?;
    }
    Ok(document)
}

pub fn create_character(
    project_path: impl AsRef<Path>,
    character: Character,
) -> Result<CharacterEditDocument, StorageError> {
    let project_path = project_path.as_ref();
    let mut document = read_character_edit_document(project_path)?;
    if document
        .characters
        .iter()
        .any(|existing| existing.id == character.id)
    {
        return Err(structured_edit_error(
            "characters",
            "characters.id",
            format!("duplicate character id: {}", character.id),
        ));
    }
    document.characters.push(character);
    update_character_edit_document(project_path, document)
}

pub fn read_state_variables_edit_document(
    project_path: impl AsRef<Path>,
) -> Result<StateVariablesEditDocument, StorageError> {
    let project_path = project_path.as_ref();
    Ok(StateVariablesEditDocument {
        resources: read_toml::<ResourceFile>(&project_path.join("world/resources.toml"))?.resources,
        initial_world_state: read_json(&project_path.join("world/initial_state.json"))?,
        initial_story_state: read_json(&project_path.join("saves/initial_story_state.json"))?,
    })
}

pub fn update_state_variables_edit_document(
    project_path: impl AsRef<Path>,
    document: StateVariablesEditDocument,
) -> Result<StateVariablesEditDocument, StorageError> {
    let project_path = project_path.as_ref();
    validate_state_variables_edit_document(&document)?;
    write_toml(
        &project_path.join("world/resources.toml"),
        &ResourceFile {
            resources: document.resources.clone(),
        },
    )?;
    write_json(
        &project_path.join("world/initial_state.json"),
        &document.initial_world_state,
    )?;
    write_json(
        &project_path.join("saves/initial_story_state.json"),
        &document.initial_story_state,
    )?;
    Ok(document)
}

pub fn create_resource(
    project_path: impl AsRef<Path>,
    resource: ResourceDefinition,
) -> Result<StateVariablesEditDocument, StorageError> {
    let project_path = project_path.as_ref();
    let mut document = read_state_variables_edit_document(project_path)?;
    if document
        .resources
        .iter()
        .any(|existing| existing.key == resource.key)
    {
        return Err(structured_edit_error(
            "state_variables",
            "resources.key",
            format!("duplicate resource key: {}", resource.key),
        ));
    }
    document
        .initial_world_state
        .resources
        .insert(resource.key.clone(), resource.initial);
    document.resources.push(resource);
    update_state_variables_edit_document(project_path, document)
}

pub fn read_rules_edit_document(
    project_path: impl AsRef<Path>,
) -> Result<RulesEditDocument, StorageError> {
    Ok(RulesEditDocument {
        rules: read_toml::<RulesFile>(&project_path.as_ref().join("rules/rules.toml"))?.rules,
    })
}

pub fn update_rules_edit_document(
    project_path: impl AsRef<Path>,
    document: RulesEditDocument,
) -> Result<RulesEditDocument, StorageError> {
    let project_path = project_path.as_ref();
    let resources =
        read_toml::<ResourceFile>(&project_path.join("world/resources.toml"))?.resources;
    validate_rules_edit_document(&document, &resources)?;
    write_toml(
        &project_path.join("rules/rules.toml"),
        &RulesFile {
            rules: document.rules.clone(),
        },
    )?;
    Ok(document)
}

pub fn create_rule(
    project_path: impl AsRef<Path>,
    rule: Rule,
) -> Result<RulesEditDocument, StorageError> {
    let project_path = project_path.as_ref();
    let mut document = read_rules_edit_document(project_path)?;
    if document.rules.iter().any(|existing| existing.id == rule.id) {
        return Err(structured_edit_error(
            "rules",
            "rules.id",
            format!("duplicate rule id: {}", rule.id),
        ));
    }
    document.rules.push(rule);
    update_rules_edit_document(project_path, document)
}

pub fn build_world_generation_request(
    project_path: impl AsRef<Path>,
    expansion_goal: impl Into<String>,
) -> Result<WorldGenerationRequest, StorageError> {
    let expansion_goal = expansion_goal.into();
    validate_generation_text_field("expansion_goal", &expansion_goal)?;
    Ok(WorldGenerationRequest {
        expansion_goal,
        document: read_world_edit_document(project_path)?,
    })
}

pub fn apply_world_generation_report(
    project_path: impl AsRef<Path>,
    report: WorldGenerationReport,
) -> Result<WorldGenerationReport, StorageError> {
    ensure_generation_report_applicable("world", &report.evidence)?;
    update_world_edit_document(project_path, report.document.clone())?;
    Ok(report)
}

pub fn build_story_craft_generation_request(
    project_path: impl AsRef<Path>,
    concept: impl Into<String>,
) -> Result<StoryCraftGenerationRequest, StorageError> {
    let project_path = project_path.as_ref();
    let concept = concept.into();
    validate_generation_text_field("concept", &concept)?;
    let world = read_world_edit_document(project_path)?;
    Ok(StoryCraftGenerationRequest {
        concept,
        world_bible_markdown: world.world_bible_markdown,
        canon_markdown: world.canon_markdown,
        forbidden_facts: world.forbidden_facts,
        document: read_story_craft_edit_document(project_path)?,
        characters: read_character_edit_document(project_path)?.characters,
    })
}

pub fn apply_story_craft_generation_report(
    project_path: impl AsRef<Path>,
    report: StoryCraftGenerationReport,
) -> Result<StoryCraftGenerationReport, StorageError> {
    ensure_generation_report_applicable("story_craft", &report.evidence)?;
    update_story_craft_edit_document(project_path, report.document.clone())?;
    Ok(report)
}

pub fn build_character_generation_request(
    project_path: impl AsRef<Path>,
    concept: impl Into<String>,
    role_hint: impl Into<String>,
) -> Result<CharacterGenerationRequest, StorageError> {
    let project_path = project_path.as_ref();
    let concept = concept.into();
    let role_hint = role_hint.into();
    validate_generation_text_field("concept", &concept)?;
    validate_generation_text_field("role_hint", &role_hint)?;
    Ok(CharacterGenerationRequest {
        concept,
        role_hint,
        world_bible_markdown: read_text(&project_path.join("world/world.md"))?,
        story_bible_markdown: read_text(&project_path.join("story/story_bible.md"))?,
        existing_characters: read_character_edit_document(project_path)?.characters,
    })
}

pub fn apply_character_generation_report(
    project_path: impl AsRef<Path>,
    report: CharacterGenerationReport,
) -> Result<CharacterGenerationReport, StorageError> {
    ensure_generation_report_applicable("characters", &report.evidence)?;
    create_character(project_path, report.character.clone())?;
    Ok(report)
}

pub fn read_ai_safety_policy(
    project_path: impl AsRef<Path>,
) -> Result<AiSafetyPolicy, StorageError> {
    let path = project_path.as_ref().join(AI_SAFETY_POLICY_PATH);
    if !path.exists() {
        return Ok(default_ai_safety_policy());
    }
    let policy = read_toml::<AiSafetyPolicy>(&path)?;
    validate_ai_safety_policy(&policy)?;
    Ok(normalize_ai_safety_policy(policy))
}

pub fn update_ai_safety_policy(
    project_path: impl AsRef<Path>,
    policy: AiSafetyPolicy,
) -> Result<AiSafetyPolicy, StorageError> {
    validate_ai_safety_policy(&policy)?;
    let policy = normalize_ai_safety_policy(policy);
    write_toml(&project_path.as_ref().join(AI_SAFETY_POLICY_PATH), &policy)?;
    Ok(policy)
}

pub fn read_visual_bible(project_path: impl AsRef<Path>) -> Result<VisualBible, StorageError> {
    let path = project_path.as_ref().join(VISUAL_BIBLE_PATH);
    if !path.exists() {
        return Ok(VisualBible::default());
    }
    read_toml::<VisualBible>(&path)
}

pub fn update_visual_bible(
    project_path: impl AsRef<Path>,
    visual_bible: VisualBible,
) -> Result<VisualBible, StorageError> {
    write_toml(
        &project_path.as_ref().join(VISUAL_BIBLE_PATH),
        &visual_bible,
    )?;
    Ok(visual_bible)
}

pub fn read_audio_bible(project_path: impl AsRef<Path>) -> Result<AudioBible, StorageError> {
    let path = project_path.as_ref().join(AUDIO_BIBLE_PATH);
    if !path.exists() {
        return Ok(AudioBible::default());
    }
    read_toml::<AudioBible>(&path)
}

pub fn update_audio_bible(
    project_path: impl AsRef<Path>,
    audio_bible: AudioBible,
) -> Result<AudioBible, StorageError> {
    write_toml(&project_path.as_ref().join(AUDIO_BIBLE_PATH), &audio_bible)?;
    Ok(audio_bible)
}

pub fn list_asset_records(
    project_path: impl AsRef<Path>,
) -> Result<Vec<AssetRecord>, StorageError> {
    let project_path = project_path.as_ref();
    let project = load_project(project_path)?;
    Ok(project.asset_records)
}

pub fn attach_scene_audio_reference(
    project_path: impl AsRef<Path>,
    scene_key: &str,
    reference: MediaAssetReference,
) -> Result<Scene, StorageError> {
    let project_path = project_path.as_ref();
    validate_identifier("scene_audio", "scene_key", scene_key)?;
    validate_audio_reference("scene_audio", &reference)?;
    let mut scene = read_scene_file(project_path, scene_key)?;
    upsert_media_reference(&mut scene.audio_refs, reference);
    write_scene_file(project_path, &scene)?;
    Ok(scene)
}

pub fn attach_beat_audio_reference(
    project_path: impl AsRef<Path>,
    scene_key: &str,
    beat_id: &str,
    reference: MediaAssetReference,
) -> Result<Scene, StorageError> {
    let project_path = project_path.as_ref();
    validate_identifier("beat_audio", "scene_key", scene_key)?;
    validate_identifier("beat_audio", "beat_id", beat_id)?;
    validate_audio_reference("beat_audio", &reference)?;
    let mut scene = read_scene_file(project_path, scene_key)?;
    let beat = scene
        .beats
        .iter_mut()
        .find(|beat| beat.id == beat_id)
        .ok_or_else(|| {
            StorageError::MissingFile(PathBuf::from(format!(
                "scenes/{scene_key}.scene.json#{beat_id}"
            )))
        })?;
    upsert_media_reference(&mut beat.audio_refs, reference);
    write_scene_file(project_path, &scene)?;
    Ok(scene)
}

fn ensure_project_path_available(path: &Path, force: bool) -> Result<(), StorageError> {
    if path.exists() && !force && path.read_dir().map_io(path)?.next().is_some() {
        Err(StorageError::ProjectExists(path.to_path_buf()))
    } else {
        Ok(())
    }
}

fn validate_project_creation_request(request: &ProjectCreationRequest) -> Result<(), StorageError> {
    validate_creation_text_field("concept", &request.concept)?;
    validate_creation_text_field("visual_style", &request.visual_style)?;
    validate_creation_text_field("initial_scene_request", &request.initial_scene_request)?;
    Ok(())
}

fn validate_creation_text_field(field: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::InvalidProjectCreationRequest {
            field: field.into(),
            reason: "must not be empty".into(),
        });
    }
    if contains_secret_marker_text(value) {
        return Err(StorageError::InvalidProjectCreationRequest {
            field: field.into(),
            reason: "must not contain secret markers".into(),
        });
    }
    Ok(())
}

fn validate_generation_text_field(field: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::InvalidGenerationRequest {
            field: field.into(),
            reason: "must not be empty".into(),
        });
    }
    if contains_secret_marker_text(value) {
        return Err(StorageError::InvalidGenerationRequest {
            field: field.into(),
            reason: "must not contain secret markers".into(),
        });
    }
    Ok(())
}

fn ensure_generation_report_applicable(
    surface: &str,
    evidence: &GenerationEvidence,
) -> Result<(), StorageError> {
    match evidence.status {
        GenerationStatus::Succeeded if evidence.fallback_used => Err(invalid_generation_report(
            surface,
            "succeeded report marked fallback_used",
        )),
        GenerationStatus::Succeeded => Ok(()),
        GenerationStatus::Fallback if !evidence.fallback_used => Err(invalid_generation_report(
            surface,
            "fallback report must mark fallback_used",
        )),
        GenerationStatus::Fallback => Ok(()),
        GenerationStatus::Failed => Err(invalid_generation_report(
            surface,
            "failed generation reports are not applicable",
        )),
    }?;

    if let Some(error) = &evidence.error {
        validate_generation_report_text(surface, "evidence.error.code", &error.code)?;
        validate_generation_report_text(surface, "evidence.error.message", &error.message)?;
    }
    Ok(())
}

fn validate_generation_report_text(
    surface: &str,
    field: &str,
    value: &str,
) -> Result<(), StorageError> {
    if contains_secret_marker_text(value) {
        Err(invalid_generation_report(
            surface,
            format!("{field} must not contain secret markers"),
        ))
    } else {
        Ok(())
    }
}

fn invalid_generation_report(
    surface: impl Into<String>,
    reason: impl Into<String>,
) -> StorageError {
    StorageError::InvalidGenerationReport {
        surface: surface.into(),
        reason: reason.into(),
    }
}

fn validate_ai_safety_policy(policy: &AiSafetyPolicy) -> Result<(), StorageError> {
    for (field, value) in [
        ("user_reporting_path", policy.user_reporting_path.as_str()),
        ("moderation_policy", policy.moderation_policy.as_str()),
    ] {
        validate_ai_safety_policy_text(field, value)?;
    }
    for guardrail in &policy.safety_guardrails {
        validate_ai_safety_policy_text("safety_guardrails", guardrail)?;
    }
    for evidence_id in &policy.evidence_ids {
        validate_ai_safety_policy_text("evidence_ids", evidence_id)?;
    }
    for notice in &policy.notices {
        validate_ai_safety_policy_text("notices", notice)?;
    }
    if let Some(source_path) = &policy.policy_source_path {
        validate_ai_safety_policy_text("policy_source_path", source_path)?;
    }
    if let Some(policy_hash) = &policy.policy_hash {
        validate_ai_safety_policy_text("policy_hash", policy_hash)?;
    }
    Ok(())
}

fn validate_ai_safety_policy_text(field: &str, value: &str) -> Result<(), StorageError> {
    if contains_secret_marker_text(value) {
        Err(StorageError::InvalidAiSafetyPolicy {
            field: field.into(),
            reason: "must not contain secret markers".into(),
        })
    } else {
        Ok(())
    }
}

fn normalize_ai_safety_policy(mut policy: AiSafetyPolicy) -> AiSafetyPolicy {
    policy.policy_source_path = Some(AI_SAFETY_POLICY_PATH.into());
    policy
}

fn default_ai_safety_policy() -> AiSafetyPolicy {
    AiSafetyPolicy {
        live_generated_content_enabled: false,
        content_kinds: vec![
            AiUsageContentKind::Text,
            AiUsageContentKind::Image,
            AiUsageContentKind::Voice,
        ],
        safety_guardrails: vec![
            "Provider credentials stay outside project source, traces, and exports.".into(),
            "Raw provider responses are not stored in project files.".into(),
            "Generated content requires creator review before distribution.".into(),
        ],
        user_reporting_path: "local-creator-review".into(),
        moderation_policy:
            "Live provider-backed generation is disabled by default; generated output must be reviewed before export."
                .into(),
        human_review_required: true,
        moderation_queue_enabled: false,
        policy_source_path: Some(AI_SAFETY_POLICY_PATH.into()),
        evidence_ids: Vec::new(),
        policy_hash: None,
        notices: vec![
            "This local policy is descriptive evidence only and is not platform approval.".into(),
        ],
    }
}

fn default_visual_bible() -> VisualBible {
    VisualBible {
        style_cards: vec![
            VisualStyleCard {
                id: "winter-court-ink".into(),
                title: "Winter court ink wash".into(),
                summary: "Cold parchment, controlled brush texture, and restrained imperial color."
                    .into(),
                prompt: "Restrained historical court drama, ink wash texture, clear rank signals."
                    .into(),
                palette: vec!["soot".into(), "aged jade".into(), "muted vermilion".into()],
                tags: vec!["court".into(), "historical".into(), "grounded".into()],
                reference_asset_ids: Vec::new(),
            },
            VisualStyleCard {
                id: "official-portrait".into(),
                title: "Official portrait restraint".into(),
                summary: "Half-length figures with clear office markers and no fantasy armor."
                    .into(),
                prompt: "Grounded official portrait, reserved posture, simple palace background."
                    .into(),
                palette: vec!["ink".into(), "paper".into(), "dark red".into()],
                tags: vec!["portrait".into(), "character".into()],
                reference_asset_ids: Vec::new(),
            },
        ],
    }
}

fn default_audio_bible() -> AudioBible {
    AudioBible {
        voice_cards: vec![
            AudioVoiceCard {
                id: "court-censor".into(),
                title: "Court Censor".into(),
                summary: "Precise, public-minded, and clipped under pressure.".into(),
                voice: "formal senior court official".into(),
                delivery: "measured accusation, low volume, hard consonants".into(),
                tags: vec!["voice".into(), "court".into(), "discipline".into()],
                sample_text: Some("The law remembers what favor tries to hide.".into()),
                reference_asset_ids: Vec::new(),
            },
            AudioVoiceCard {
                id: "war-minister".into(),
                title: "Minister of War".into(),
                summary: "Terse logistics language with visible urgency.".into(),
                voice: "military administrator".into(),
                delivery: "short phrases, controlled urgency".into(),
                tags: vec!["voice".into(), "military".into()],
                sample_text: Some("A payroll delay becomes a frontier order.".into()),
                reference_asset_ids: Vec::new(),
            },
        ],
    }
}

fn rebuild_asset_records(
    project_path: &Path,
    project: &ProjectData,
) -> Result<Vec<AssetRecord>, StorageError> {
    let mut registry = plotforge_media::AssetRegistry::new();
    registry
        .register_project_assets(project_path, project)
        .map_err(|source| StorageError::Media {
            path: project_path.to_path_buf(),
            source,
        })?;
    Ok(registry.records().cloned().collect())
}

fn rebuild_asset_records_from_project_files(project: &ProjectData) -> Vec<AssetRecord> {
    let mut registry = plotforge_media::AssetRegistry::new();
    for scene in &project.scenes {
        registry
            .insert_bytes(
                plotforge_media::AssetRecordInput {
                    kind: AssetKind::Image,
                    source: AssetSourceKind::Generated,
                    project_path: scene.background_asset.clone(),
                    export_path: Some(scene.background_asset.clone()),
                    provider_metadata: None,
                    references: vec![AssetReference {
                        reference_kind: AssetReferenceKind::Scene,
                        reference_id: scene.key.clone(),
                        slot: "background_asset".into(),
                    }],
                },
                PLACEHOLDER_PNG,
            )
            .expect("built-in demo asset path and bytes are valid");
    }
    registry.records().cloned().collect()
}

fn validate_world_edit_document(document: &WorldEditDocument) -> Result<(), StorageError> {
    validate_structured_text(
        "world",
        "world_bible_markdown",
        &document.world_bible_markdown,
    )?;
    validate_structured_text("world", "canon_markdown", &document.canon_markdown)?;
    validate_unique_text_list("world", "forbidden_facts", &document.forbidden_facts)
}

fn validate_story_craft_edit_document(
    document: &StoryCraftEditDocument,
    characters: &[Character],
) -> Result<(), StorageError> {
    validate_structured_text(
        "story_craft",
        "story_bible_markdown",
        &document.story_bible_markdown,
    )?;
    validate_structured_text(
        "story_craft",
        "style_guide_markdown",
        &document.style_guide_markdown,
    )?;
    validate_story_craft_state("story_craft", &document.story_craft, characters)
}

fn validate_character_edit_document(document: &CharacterEditDocument) -> Result<(), StorageError> {
    let mut ids = BTreeSet::new();
    for character in &document.characters {
        validate_identifier("characters", "characters.id", &character.id)?;
        if !ids.insert(character.id.as_str()) {
            return Err(structured_edit_error(
                "characters",
                "characters.id",
                format!("duplicate character id: {}", character.id),
            ));
        }
        validate_structured_text("characters", "characters.name", &character.name)?;
        validate_structured_text("characters", "characters.role", &character.role)?;
        validate_unique_text_list("characters", "characters.traits", &character.traits)?;
        validate_structured_text(
            "characters",
            "characters.visual_card",
            &character.visual_card,
        )?;
        validate_structured_text("characters", "characters.voice_card", &character.voice_card)?;
        if let Some(portrait_request) = &character.portrait_request {
            validate_structured_text(
                "characters",
                "characters.portrait_request.prompt_summary",
                &portrait_request.prompt_summary,
            )?;
            validate_structured_text(
                "characters",
                "characters.portrait_request.style",
                &portrait_request.style,
            )?;
            validate_structured_text(
                "characters",
                "characters.portrait_request.target_asset_slot",
                &portrait_request.target_asset_slot,
            )?;
            validate_structured_text(
                "characters",
                "characters.portrait_request.prompt_hash",
                &portrait_request.prompt_hash,
            )?;
            validate_structured_text(
                "characters",
                "characters.portrait_request.provider_config_hash",
                &portrait_request.provider_config_hash,
            )?;
            validate_unique_text_list(
                "characters",
                "characters.portrait_request.reference_asset_ids",
                &portrait_request.reference_asset_ids,
            )?;
        }
    }
    Ok(())
}

fn validate_audio_reference(
    surface: &str,
    reference: &MediaAssetReference,
) -> Result<(), StorageError> {
    if !matches!(reference.kind, AssetKind::Audio | AssetKind::Voice) {
        return Err(structured_edit_error(
            surface,
            "audio_refs.kind",
            "must be audio or voice",
        ));
    }
    if reference.source == AssetSourceKind::External {
        return Err(structured_edit_error(
            surface,
            "audio_refs.source",
            "external audio references are not allowed in local project source",
        ));
    }
    validate_optional_structured_text(
        surface,
        "audio_refs.asset_id",
        reference.asset_id.as_deref(),
    )?;
    validate_asset_reference_path(surface, "audio_refs.project_path", &reference.project_path)?;
    validate_asset_reference_path(surface, "audio_refs.export_path", &reference.export_path)?;
    validate_structured_text(surface, "audio_refs.slot", &reference.slot)
}

fn validate_scene_audio_references(scenes: &[Scene]) -> Result<(), StorageError> {
    for scene in scenes {
        for reference in &scene.audio_refs {
            validate_audio_reference("scene_audio", reference)?;
        }
        for beat in &scene.beats {
            for reference in &beat.audio_refs {
                validate_audio_reference("beat_audio", reference)?;
            }
        }
    }
    Ok(())
}

fn validate_asset_reference_path(
    surface: &str,
    field: &str,
    value: &str,
) -> Result<(), StorageError> {
    validate_structured_text(surface, field, value)?;
    let path = Path::new(value);
    if path.is_absolute() || !path.starts_with("assets") {
        return Err(structured_edit_error(
            surface,
            field,
            "must be a relative assets path",
        ));
    }
    for component in path.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(structured_edit_error(
                surface,
                field,
                "must not contain parent or special components",
            ));
        }
    }
    Ok(())
}

fn validate_state_variables_edit_document(
    document: &StateVariablesEditDocument,
) -> Result<(), StorageError> {
    let mut resource_keys = BTreeSet::new();
    for resource in &document.resources {
        validate_identifier("state_variables", "resources.key", &resource.key)?;
        if !resource_keys.insert(resource.key.as_str()) {
            return Err(structured_edit_error(
                "state_variables",
                "resources.key",
                format!("duplicate resource key: {}", resource.key),
            ));
        }
        validate_structured_text("state_variables", "resources.label", &resource.label)?;
        if resource.min > resource.max {
            return Err(structured_edit_error(
                "state_variables",
                "resources.min",
                format!("min {} is greater than max {}", resource.min, resource.max),
            ));
        }
        if resource.initial < resource.min || resource.initial > resource.max {
            return Err(structured_edit_error(
                "state_variables",
                "resources.initial",
                format!(
                    "initial {} must be between min {} and max {}",
                    resource.initial, resource.min, resource.max
                ),
            ));
        }
    }

    let world_keys = document
        .initial_world_state
        .resources
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if resource_keys != world_keys {
        return Err(structured_edit_error(
            "state_variables",
            "initial_world_state.resources",
            "initial world state resources must exactly match resource definitions",
        ));
    }
    for (key, value) in &document.initial_world_state.resources {
        let resource = document
            .resources
            .iter()
            .find(|resource| resource.key == *key)
            .expect("resource key equality checked above");
        if *value < resource.min || *value > resource.max {
            return Err(structured_edit_error(
                "state_variables",
                "initial_world_state.resources",
                format!(
                    "{key} value {value} must be between min {} and max {}",
                    resource.min, resource.max
                ),
            ));
        }
    }
    for key in document.initial_world_state.flags.keys() {
        validate_identifier("state_variables", "initial_world_state.flags", key)?;
    }
    for event in &document.initial_world_state.triggered_events {
        validate_identifier(
            "state_variables",
            "initial_world_state.triggered_events",
            event,
        )?;
    }
    validate_identifier(
        "state_variables",
        "initial_story_state.current_scene_key",
        &document.initial_story_state.current_scene_key,
    )?;
    for scene_key in &document.initial_story_state.completed_scene_keys {
        validate_identifier(
            "state_variables",
            "initial_story_state.completed_scene_keys",
            scene_key,
        )?;
    }
    Ok(())
}

fn validate_rules_edit_document(
    document: &RulesEditDocument,
    resources: &[ResourceDefinition],
) -> Result<(), StorageError> {
    let resource_keys = resources
        .iter()
        .map(|resource| resource.key.as_str())
        .collect::<BTreeSet<_>>();
    let mut rule_ids = BTreeSet::new();
    for rule in &document.rules {
        validate_identifier("rules", "rules.id", &rule.id)?;
        if !rule_ids.insert(rule.id.as_str()) {
            return Err(structured_edit_error(
                "rules",
                "rules.id",
                format!("duplicate rule id: {}", rule.id),
            ));
        }
        validate_identifier("rules", "rules.action_type", &rule.action_type)?;
        if rule.effects.is_empty() {
            return Err(structured_edit_error(
                "rules",
                "rules.effects",
                format!("rule {} must have at least one effect", rule.id),
            ));
        }
        for condition in &rule.conditions {
            validate_condition_references(condition, &resource_keys)?;
        }
        for effect in &rule.effects {
            validate_effect_references(effect, &resource_keys)?;
        }
    }
    Ok(())
}

fn validate_story_craft_state(
    surface: &str,
    story_craft: &plotforge_schema::StoryCraftState,
    characters: &[Character],
) -> Result<(), StorageError> {
    validate_structured_text(
        surface,
        "bible.genre_promise",
        &story_craft.bible.genre_promise,
    )?;
    validate_structured_text(
        surface,
        "bible.central_question",
        &story_craft.bible.central_question,
    )?;
    validate_optional_structured_text(
        surface,
        "bible.target_audience",
        story_craft.bible.target_audience.as_deref(),
    )?;
    validate_optional_structured_text(
        surface,
        "bible.prose_style_guide",
        story_craft.bible.prose_style_guide.as_deref(),
    )?;
    validate_unique_text_list(
        surface,
        "bible.target_emotions",
        &story_craft.bible.target_emotions,
    )?;
    validate_unique_text_list(
        surface,
        "bible.core_foreshadowing",
        &story_craft.bible.core_foreshadowing,
    )?;
    validate_unique_text_list(
        surface,
        "bible.emotional_contract",
        &story_craft.bible.emotional_contract,
    )?;
    validate_unique_text_list(
        surface,
        "bible.banned_cliches",
        &story_craft.bible.banned_cliches,
    )?;
    validate_structured_text(
        surface,
        "bible.hook_strategy.primary_hook",
        &story_craft.bible.hook_strategy.primary_hook,
    )?;
    validate_unique_text_list(
        surface,
        "bible.hook_strategy.recurring_hook_patterns",
        &story_craft.bible.hook_strategy.recurring_hook_patterns,
    )?;
    if let Some(reversal) = &story_craft.bible.reversal_strategy {
        validate_structured_text(
            surface,
            "bible.reversal_strategy.principle",
            &reversal.principle,
        )?;
    }

    let mut reference_ids = BTreeSet::new();
    for reference in &story_craft.bible.reference_modules {
        validate_identifier(surface, "bible.reference_modules.id", &reference.id)?;
        if !reference_ids.insert(reference.id.as_str()) {
            return Err(structured_edit_error(
                surface,
                "bible.reference_modules.id",
                format!("duplicate reference module id: {}", reference.id),
            ));
        }
        validate_structured_text(surface, "bible.reference_modules.title", &reference.title)?;
        validate_structured_text(
            surface,
            "bible.reference_modules.summary",
            &reference.summary,
        )?;
    }

    let mut promise_ids = BTreeSet::new();
    for promise in &story_craft.active_promises {
        validate_identifier(surface, "active_promises.id", &promise.id)?;
        if !promise_ids.insert(promise.id.as_str()) {
            return Err(structured_edit_error(
                surface,
                "active_promises.id",
                format!("duplicate active promise id: {}", promise.id),
            ));
        }
        validate_structured_text(surface, "active_promises.text", &promise.text)?;
        validate_identifier(
            surface,
            "active_promises.introduced_at",
            &promise.introduced_at,
        )?;
        validate_optional_structured_text(
            surface,
            "active_promises.payoff_hint",
            promise.payoff_hint.as_deref(),
        )?;
    }

    let character_ids = characters
        .iter()
        .map(|character| character.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut thread_ids = BTreeSet::new();
    for thread in &story_craft.plot_threads {
        validate_identifier(surface, "plot_threads.id", &thread.id)?;
        if !thread_ids.insert(thread.id.as_str()) {
            return Err(structured_edit_error(
                surface,
                "plot_threads.id",
                format!("duplicate plot thread id: {}", thread.id),
            ));
        }
        validate_structured_text(surface, "plot_threads.title", &thread.title)?;
        validate_structured_text(surface, "plot_threads.promise", &thread.promise)?;
        validate_identifier(surface, "plot_threads.introduced_at", &thread.introduced_at)?;
        validate_structured_text(surface, "plot_threads.last_update", &thread.last_update)?;
        validate_optional_structured_text(
            surface,
            "plot_threads.expected_payoff",
            thread.expected_payoff.as_deref(),
        )?;
        for character_id in &thread.related_characters {
            validate_identifier(surface, "plot_threads.related_characters", character_id)?;
            if !character_ids.contains(character_id.as_str()) {
                return Err(structured_edit_error(
                    surface,
                    "plot_threads.related_characters",
                    format!("unknown character id: {character_id}"),
                ));
            }
        }
        for flag in &thread.related_world_flags {
            validate_identifier(surface, "plot_threads.related_world_flags", flag)?;
        }
    }

    let mut arc_ids = BTreeSet::new();
    for arc in &story_craft.character_arcs {
        validate_identifier(surface, "character_arcs.id", &arc.id)?;
        if !arc_ids.insert(arc.id.as_str()) {
            return Err(structured_edit_error(
                surface,
                "character_arcs.id",
                format!("duplicate character arc id: {}", arc.id),
            ));
        }
        validate_identifier(surface, "character_arcs.character_id", &arc.character_id)?;
        if !character_ids.contains(arc.character_id.as_str()) {
            return Err(structured_edit_error(
                surface,
                "character_arcs.character_id",
                format!("unknown character id: {}", arc.character_id),
            ));
        }
        validate_structured_text(surface, "character_arcs.desire", &arc.desire)?;
        validate_structured_text(surface, "character_arcs.pressure", &arc.pressure)?;
        validate_structured_text(surface, "character_arcs.current_state", &arc.current_state)?;
        validate_structured_text(surface, "character_arcs.target_state", &arc.target_state)?;
    }

    let mut note_ids = BTreeSet::new();
    for note in &story_craft.review_notes {
        validate_identifier(surface, "review_notes.id", &note.id)?;
        if !note_ids.insert(note.id.as_str()) {
            return Err(structured_edit_error(
                surface,
                "review_notes.id",
                format!("duplicate review note id: {}", note.id),
            ));
        }
        if let Some(scene_key) = &note.scene_key {
            validate_identifier(surface, "review_notes.scene_key", scene_key)?;
        }
        validate_structured_text(surface, "review_notes.message", &note.message)?;
    }

    for arc in &story_craft.emotional_arc {
        validate_identifier(surface, "emotional_arc.scene_key", &arc.scene_key)?;
        validate_structured_text(surface, "emotional_arc.target_emotion", &arc.target_emotion)?;
    }

    Ok(())
}

fn validate_condition_references(
    condition: &Condition,
    resource_keys: &BTreeSet<&str>,
) -> Result<(), StorageError> {
    match condition {
        Condition::ResourceAtLeast { key, .. } | Condition::ResourceAtMost { key, .. } => {
            validate_known_resource_key("rules", "rules.conditions.key", key, resource_keys)
        }
        Condition::FlagEquals { key, .. } => {
            validate_identifier("rules", "rules.conditions.key", key)
        }
    }
}

fn validate_effect_references(
    effect: &Effect,
    resource_keys: &BTreeSet<&str>,
) -> Result<(), StorageError> {
    match effect {
        Effect::AddResource { key, .. } | Effect::SetResource { key, .. } => {
            validate_known_resource_key("rules", "rules.effects.key", key, resource_keys)
        }
        Effect::SetFlag { key, .. } => validate_identifier("rules", "rules.effects.key", key),
        Effect::TriggerEvent { event } => {
            validate_identifier("rules", "rules.effects.event", event)
        }
    }
}

fn validate_known_resource_key(
    surface: &str,
    field: &str,
    key: &str,
    resource_keys: &BTreeSet<&str>,
) -> Result<(), StorageError> {
    validate_identifier(surface, field, key)?;
    if resource_keys.contains(key) {
        Ok(())
    } else {
        Err(structured_edit_error(
            surface,
            field,
            format!("unknown resource key: {key}"),
        ))
    }
}

fn validate_unique_text_list(
    surface: &str,
    field: &str,
    values: &[String],
) -> Result<(), StorageError> {
    let mut seen = BTreeSet::new();
    for value in values {
        validate_structured_text(surface, field, value)?;
        if !seen.insert(value.trim()) {
            return Err(structured_edit_error(
                surface,
                field,
                format!("duplicate value: {}", value.trim()),
            ));
        }
    }
    Ok(())
}

fn validate_optional_structured_text(
    surface: &str,
    field: &str,
    value: Option<&str>,
) -> Result<(), StorageError> {
    match value {
        Some(value) => validate_structured_text(surface, field, value),
        None => Ok(()),
    }
}

fn validate_structured_text(surface: &str, field: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(structured_edit_error(surface, field, "must not be empty"));
    }
    if contains_secret_marker_text(value) {
        return Err(structured_edit_error(
            surface,
            field,
            "must not contain secret markers",
        ));
    }
    Ok(())
}

fn validate_identifier(surface: &str, field: &str, value: &str) -> Result<(), StorageError> {
    validate_structured_text(surface, field, value)?;
    let valid = value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(structured_edit_error(
            surface,
            field,
            "may only contain ASCII letters, numbers, hyphen, and underscore",
        ))
    }
}

fn structured_edit_error(
    surface: impl Into<String>,
    field: impl Into<String>,
    reason: impl Into<String>,
) -> StorageError {
    StorageError::InvalidStructuredEdit {
        surface: surface.into(),
        field: field.into(),
        reason: reason.into(),
    }
}

fn project_from_creation_request(path: &Path, request: &ProjectCreationRequest) -> ProjectData {
    let mut project = match request.template {
        ProjectTemplateId::HistoricalCrisis | ProjectTemplateId::DynastyEmbers => {
            dynasty_embers_project()
        }
    };
    let project_id = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(slugify_project_id)
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| "plotforge-project".into());
    let title = title_from_project_id(&project_id);

    project.game.id = project_id;
    project.game.title = title;
    project.game.description = request.concept.trim().to_string();
    project.story_craft.bible.genre_promise = request.concept.trim().to_string();
    project.story_craft.bible.prose_style_guide = Some(request.visual_style.trim().to_string());
    if let Some(scene) = project.scenes.first_mut() {
        scene.dramatic_purpose = request.initial_scene_request.trim().to_string();
        scene.hook = request.initial_scene_request.trim().to_string();
    }
    project
}

fn slugify_project_id(input: &str) -> String {
    let mut output = String::new();
    let mut previous_dash = false;
    for character in input.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash && !output.is_empty() {
            output.push('-');
            previous_dash = true;
        }
    }
    output.trim_end_matches('-').to_string()
}

fn title_from_project_id(project_id: &str) -> String {
    project_id
        .split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn world_bible_markdown(request: Option<&ProjectCreationRequest>) -> String {
    match request {
        Some(request) => format!(
            "# World Bible\n\n## Concept\n\n{}\n\n## Template\n\n{}\n",
            request.concept.trim(),
            template_label(&request.template)
        ),
        None => "# World Bible\n\nThe dynasty is still standing, but every resource is under pressure.\n".into(),
    }
}

fn story_bible_markdown(request: Option<&ProjectCreationRequest>) -> String {
    match request {
        Some(request) => format!(
            "# Story Bible\n\n## Concept\n\n{}\n\n## Initial Scene Request\n\n{}\n",
            request.concept.trim(),
            request.initial_scene_request.trim()
        ),
        None => {
            "# Story Bible\n\nThe throne must trade stability, silver, and legitimacy to survive.\n"
                .into()
        }
    }
}

fn style_guide_markdown(request: Option<&ProjectCreationRequest>) -> String {
    match request {
        Some(request) => format!(
            "# Style Guide\n\n## Visual Style\n\n{}\n\n## Voice\n\n{}\n",
            request.visual_style.trim(),
            if request.voice_enabled {
                "Voice generation requested for this project."
            } else {
                "Voice generation disabled for this project."
            }
        ),
        None => "# Style Guide\n\nTense, concrete, political, and consequence-driven. Avoid empty grandeur.\n".into(),
    }
}

fn template_label(template: &ProjectTemplateId) -> &'static str {
    match template {
        ProjectTemplateId::HistoricalCrisis => "Historical Crisis",
        ProjectTemplateId::DynastyEmbers => "Dynasty Embers",
    }
}

pub fn write_reference_analysis(
    project_path: impl AsRef<Path>,
    analysis: &ReferenceAnalysis,
) -> Result<PathBuf, StorageError> {
    let project_path = project_path.as_ref();
    let relative_path = PathBuf::from(format!(
        "references/user_imports/{}.reference.json",
        analysis.id
    ));
    validate_reference_analysis(&relative_path, analysis)?;
    let path = project_path.join(relative_path);
    write_json(&path, analysis)?;
    Ok(path)
}

pub fn validate_reference_library(project_path: impl AsRef<Path>) -> Result<(), StorageError> {
    let project_path = project_path.as_ref();
    for path in reference_analysis_paths(project_path)? {
        let analysis = read_json::<ReferenceAnalysis>(&path)?;
        let relative_path = path
            .strip_prefix(project_path)
            .unwrap_or(path.as_path())
            .to_path_buf();
        validate_reference_analysis(&relative_path, &analysis)?;
    }
    validate_no_large_raw_reference_text(project_path)
}

pub fn write_trace(
    project_path: impl AsRef<Path>,
    trace: &plotforge_schema::RuntimeTrace,
) -> Result<PathBuf, StorageError> {
    let traces_dir = project_path.as_ref().join("traces");
    fs::create_dir_all(&traces_dir).map_io(&traces_dir)?;
    let trace_path = traces_dir.join(format!("{}.json", trace.id));
    write_json(&trace_path, trace)?;
    let latest_path = traces_dir.join("latest.json");
    write_json(&latest_path, trace)?;
    Ok(trace_path)
}

pub fn write_runtime_snapshot(
    project_path: impl AsRef<Path>,
    snapshot: &RuntimeSnapshot,
) -> Result<PathBuf, StorageError> {
    validate_runtime_snapshot_id(&snapshot.id)?;
    let saves_dir = project_path.as_ref().join("saves");
    fs::create_dir_all(&saves_dir).map_io(&saves_dir)?;
    let snapshot_path = saves_dir.join(runtime_snapshot_file_name(&snapshot.id));
    write_json(&snapshot_path, snapshot)?;
    let latest_path = saves_dir.join("latest.runtime_snapshot.json");
    write_json(&latest_path, snapshot)?;
    Ok(snapshot_path)
}

pub fn read_runtime_snapshot(
    project_path: impl AsRef<Path>,
    snapshot_id: &str,
) -> Result<RuntimeSnapshot, StorageError> {
    validate_runtime_snapshot_id(snapshot_id)?;
    read_json(
        &project_path
            .as_ref()
            .join("saves")
            .join(runtime_snapshot_file_name(snapshot_id)),
    )
}

pub fn read_latest_runtime_snapshot(
    project_path: impl AsRef<Path>,
) -> Result<RuntimeSnapshot, StorageError> {
    read_json(
        &project_path
            .as_ref()
            .join("saves/latest.runtime_snapshot.json"),
    )
}

pub fn dynasty_embers_project() -> ProjectData {
    let game = GameProject {
        id: "dynasty-embers".into(),
        title: "Dynasty Embers".into(),
        version: "0.1.0".into(),
        description: "Historical crisis simulation about a collapsing dynasty.".into(),
        entry_scene: "court-crisis-001".into(),
        run_seed: 7,
    };
    let resources = vec![
        resource("treasury", "Treasury", 40, 0, 100),
        resource("public_order", "Public order", 55, 0, 100),
        resource("army_morale", "Army morale", 45, 0, 100),
        resource("court_stability", "Court stability", 50, 0, 100),
        resource("local_control", "Local control", 48, 0, 100),
        resource("enemy_pressure", "Enemy pressure", 70, 0, 100),
    ];
    let world_state = WorldState {
        resources: resources
            .iter()
            .map(|definition| (definition.key.clone(), definition.initial))
            .collect(),
        flags: BTreeMap::new(),
        triggered_events: Vec::new(),
    };
    let story_state = StoryState {
        current_scene_key: game.entry_scene.clone(),
        current_beat_id: Some("court-crisis-001-beat-001".into()),
        completed_scene_keys: Vec::new(),
        turn: 0,
    };

    let mut project = ProjectData {
        game,
        resources,
        world_state,
        story_state,
        story_craft: dynasty_embers_story_craft(),
        characters: dynasty_characters(),
        rules: dynasty_rules(),
        scenes: vec![initial_scene()],
        visual_bible: default_visual_bible(),
        audio_bible: default_audio_bible(),
        asset_records: Vec::new(),
        ai_safety_policy: default_ai_safety_policy(),
    };
    project.asset_records = rebuild_asset_records_from_project_files(&project);
    project
}

fn create_project_dirs(path: &Path) -> Result<(), StorageError> {
    for dir in [
        "world",
        "story",
        "safety",
        "characters",
        "locations",
        "rules",
        "events",
        "media",
        "agents",
        "references/methods",
        "references/user_imports",
        "references/analyses",
        "reviews/narrative_reviews",
        "assets/images",
        "assets/voices",
        "assets/generated",
        "saves",
        "traces",
        "exports",
        "scenes",
    ] {
        fs::create_dir_all(path.join(dir)).map_io(path.join(dir))?;
    }
    Ok(())
}

fn write_project(
    path: &Path,
    project: &ProjectData,
    request: Option<&ProjectCreationRequest>,
) -> Result<Vec<PathBuf>, StorageError> {
    let mut files = Vec::new();

    write_toml_tracked(path, "game.toml", &project.game, &mut files)?;
    write_toml_tracked(
        path,
        "world/resources.toml",
        &ResourceFile {
            resources: project.resources.clone(),
        },
        &mut files,
    )?;
    write_json_tracked(
        path,
        "world/initial_state.json",
        &project.world_state,
        &mut files,
    )?;
    write_text_tracked(
        path,
        "world/world.md",
        &world_bible_markdown(request),
        &mut files,
    )?;
    write_text_tracked(
        path,
        "world/canon.md",
        "# Canon Rules\n\n- The player is the final authority.\n- Every order has visible state consequences.\n- Court factions respond to short-term tradeoffs.\n",
        &mut files,
    )?;
    write_json_tracked(
        path,
        "world/forbidden_facts.json",
        &ForbiddenFactsFile {
            forbidden_facts: Vec::new(),
        },
        &mut files,
    )?;
    write_toml_tracked(
        path,
        AI_SAFETY_POLICY_PATH,
        &normalize_ai_safety_policy(project.ai_safety_policy.clone()),
        &mut files,
    )?;
    write_toml_tracked(path, VISUAL_BIBLE_PATH, &project.visual_bible, &mut files)?;
    write_toml_tracked(path, AUDIO_BIBLE_PATH, &project.audio_bible, &mut files)?;
    write_toml_tracked(
        path,
        "story/story_craft.toml",
        &StoryCraftFile {
            story_craft: project.story_craft.clone(),
        },
        &mut files,
    )?;
    write_json_tracked(
        path,
        "story/emotional_arc.json",
        &project.story_craft.emotional_arc,
        &mut files,
    )?;
    write_toml_tracked(
        path,
        "story/plot_threads.toml",
        &PlotThreadsFile {
            plot_threads: project.story_craft.plot_threads.clone(),
        },
        &mut files,
    )?;
    write_text_tracked(
        path,
        "story/story_bible.md",
        &story_bible_markdown(request),
        &mut files,
    )?;
    write_text_tracked(
        path,
        "story/style_guide.md",
        &style_guide_markdown(request),
        &mut files,
    )?;
    for character in &project.characters {
        write_toml_tracked(
            path,
            &format!("characters/{}.character.toml", character.id),
            character,
            &mut files,
        )?;
    }
    write_toml_tracked(
        path,
        "rules/rules.toml",
        &RulesFile {
            rules: project.rules.clone(),
        },
        &mut files,
    )?;
    write_json_tracked(
        path,
        "saves/initial_story_state.json",
        &project.story_state,
        &mut files,
    )?;
    for scene in &project.scenes {
        write_json_tracked(
            path,
            &format!("scenes/{}.scene.json", scene.key),
            scene,
            &mut files,
        )?;
    }
    write_text_tracked(
        path,
        "agents/scene_planner.prompt.md",
        "Plan one scene from validated state.\n",
        &mut files,
    )?;
    write_text_tracked(
        path,
        "agents/beat_writer.prompt.md",
        "Write beats from a scene plan.\n",
        &mut files,
    )?;
    write_text_tracked(
        path,
        "agents/plot_doctor.prompt.md",
        "Review hook, progress, choice quality, character fit, and threads.\n",
        &mut files,
    )?;
    write_text_tracked(
        path,
        "references/README.md",
        "# Reference Library\n\nStore metadata, short summaries, and structure notes only. Do not store raw copyrighted bodies here.\n",
        &mut files,
    )?;
    write_json_tracked(
        path,
        "references/methods/political-crisis-patterns.reference.json",
        &demo_reference_analysis(),
        &mut files,
    )?;
    write_text_tracked(path, "AGENTS.md", project_agents_md(), &mut files)?;
    write_placeholder_png_tracked(path, "assets/generated/placeholder.png", &mut files)?;
    write_placeholder_png_tracked(path, "assets/generated/court-crisis-001.png", &mut files)?;
    files.sort();
    Ok(files)
}

fn write_toml_tracked<T: Serialize>(
    project_path: &Path,
    relative_path: &str,
    value: &T,
    files: &mut Vec<PathBuf>,
) -> Result<(), StorageError> {
    write_toml(&project_path.join(relative_path), value)?;
    files.push(PathBuf::from(relative_path));
    Ok(())
}

fn write_json_tracked<T: Serialize>(
    project_path: &Path,
    relative_path: &str,
    value: &T,
    files: &mut Vec<PathBuf>,
) -> Result<(), StorageError> {
    write_json(&project_path.join(relative_path), value)?;
    files.push(PathBuf::from(relative_path));
    Ok(())
}

fn write_text_tracked(
    project_path: &Path,
    relative_path: &str,
    text: &str,
    files: &mut Vec<PathBuf>,
) -> Result<(), StorageError> {
    write_text(&project_path.join(relative_path), text)?;
    files.push(PathBuf::from(relative_path));
    Ok(())
}

fn write_placeholder_png_tracked(
    project_path: &Path,
    relative_path: &str,
    files: &mut Vec<PathBuf>,
) -> Result<(), StorageError> {
    write_placeholder_png(&project_path.join(relative_path))?;
    files.push(PathBuf::from(relative_path));
    Ok(())
}

fn write_toml<T: Serialize>(path: &Path, value: &T) -> Result<(), StorageError> {
    let encoded = toml::to_string_pretty(value)?;
    write_text(path, &encoded)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), StorageError> {
    let encoded = serde_json::to_string_pretty(value).map_err(|source| StorageError::Json {
        path: path.to_path_buf(),
        source,
    })?;
    write_text(path, &(encoded + "\n"))
}

fn read_scene_file(project_path: &Path, scene_key: &str) -> Result<Scene, StorageError> {
    read_json(&project_path.join(format!("scenes/{scene_key}.scene.json")))
}

fn write_scene_file(project_path: &Path, scene: &Scene) -> Result<(), StorageError> {
    validate_identifier("scene", "scene.key", &scene.key)?;
    write_json(
        &project_path.join(format!("scenes/{}.scene.json", scene.key)),
        scene,
    )
}

fn upsert_media_reference(
    references: &mut Vec<MediaAssetReference>,
    reference: MediaAssetReference,
) {
    if let Some(existing) = references
        .iter_mut()
        .find(|existing| existing.slot == reference.slot)
    {
        *existing = reference;
        return;
    }
    references.push(reference);
}

fn write_text(path: &Path, text: &str) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_io(parent)?;
    }
    fs::write(path, text).map_io(path)
}

fn read_text(path: &Path) -> Result<String, StorageError> {
    fs::read_to_string(path).map_io(path)
}

pub fn write_placeholder_png(path: &Path) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_io(parent)?;
    }
    fs::write(path, PLACEHOLDER_PNG).map_io(path)
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T, StorageError> {
    let text = fs::read_to_string(path).map_io(path)?;
    toml::from_str(&text).map_err(|source| StorageError::TomlParse {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, StorageError> {
    let text = fs::read_to_string(path).map_io(path)?;
    serde_json::from_str(&text).map_err(|source| StorageError::Json {
        path: path.to_path_buf(),
        source,
    })
}

fn read_collection<T: DeserializeOwned>(
    dir: PathBuf,
    suffix: &str,
) -> Result<Vec<T>, StorageError> {
    let mut paths = fs::read_dir(&dir)
        .map_io(&dir)?
        .map(|entry| entry.map(|entry| entry.path()).map_io(&dir))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();

    paths
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(suffix))
        })
        .map(|path| {
            if suffix.ends_with(".json") {
                read_json(&path)
            } else {
                read_toml(&path)
            }
        })
        .collect()
}

fn clear_collection_files(dir: &Path, suffix: &str) -> Result<(), StorageError> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_io(dir)? {
        let path = entry.map_io(dir)?.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(suffix))
        {
            fs::remove_file(&path).map_io(&path)?;
        }
    }
    Ok(())
}

fn reference_analysis_paths(project_path: &Path) -> Result<Vec<PathBuf>, StorageError> {
    let mut paths = Vec::new();
    for relative_dir in [
        "references/methods",
        "references/user_imports",
        "references/analyses",
    ] {
        let dir = project_path.join(relative_dir);
        if !dir.exists() {
            continue;
        }
        let mut entries = fs::read_dir(&dir)
            .map_io(&dir)?
            .map(|entry| entry.map(|entry| entry.path()).map_io(&dir))
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();
        paths.extend(entries.into_iter().filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".reference.json"))
        }));
    }

    Ok(paths)
}

fn validate_reference_analysis(
    relative_path: &Path,
    analysis: &ReferenceAnalysis,
) -> Result<(), StorageError> {
    validate_reference_id(relative_path, &analysis.id)?;
    validate_non_empty(relative_path, "title", &analysis.title)?;
    validate_non_empty(relative_path, "source.citation", &analysis.source.citation)?;
    validate_non_empty(relative_path, "summary", &analysis.summary)?;
    validate_char_limit(
        relative_path,
        "summary",
        &analysis.summary,
        MAX_REFERENCE_SUMMARY_CHARS,
    )?;
    validate_reference_rights(relative_path, &analysis.source)?;

    for note in &analysis.structure_notes {
        validate_non_empty(relative_path, "structure_notes.label", &note.label)?;
        validate_non_empty(relative_path, "structure_notes.summary", &note.summary)?;
        validate_char_limit(
            relative_path,
            "structure_notes.summary",
            &note.summary,
            MAX_REFERENCE_STRUCTURE_NOTE_CHARS,
        )?;
    }

    Ok(())
}

fn validate_reference_id(path: &Path, id: &str) -> Result<(), StorageError> {
    validate_non_empty(path, "id", id)?;
    let valid = id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(reference_compliance_error(
            path,
            "id may only contain ASCII letters, numbers, hyphen, and underscore",
        ))
    }
}

fn validate_reference_rights(path: &Path, source: &ReferenceSource) -> Result<(), StorageError> {
    let rights_require_authorization =
        matches!(
            source.rights,
            ReferenceRights::UserOwned | ReferenceRights::UserAuthorized
        ) || matches!(source.source_type, ReferenceSourceType::UserImport);
    if rights_require_authorization && !source.user_authorized {
        Err(reference_compliance_error(
            path,
            "user imports and user-owned references require explicit authorization metadata",
        ))
    } else {
        Ok(())
    }
}

fn validate_no_large_raw_reference_text(project_path: &Path) -> Result<(), StorageError> {
    let user_imports = project_path.join("references/user_imports");
    if !user_imports.exists() {
        return Ok(());
    }

    let mut stack = vec![user_imports];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_io(&dir)? {
            let path = entry.map_io(&dir)?.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !is_raw_reference_text_path(&path) {
                continue;
            }
            let bytes = fs::metadata(&path).map_io(&path)?.len();
            if bytes > MAX_REFERENCE_RAW_TEXT_BYTES {
                let relative_path = path
                    .strip_prefix(project_path)
                    .unwrap_or(path.as_path())
                    .to_path_buf();
                return Err(reference_compliance_error(
                    &relative_path,
                    format!(
                        "large raw reference text is not allowed ({bytes} bytes > {MAX_REFERENCE_RAW_TEXT_BYTES} bytes); store metadata, short summary, and structure notes instead"
                    ),
                ));
            }
        }
    }

    Ok(())
}

fn is_raw_reference_text_path(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".reference.json"))
    {
        return false;
    }

    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "txt" | "md" | "markdown"))
}

fn validate_non_empty(path: &Path, field: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        Err(reference_compliance_error(
            path,
            format!("{field} must not be empty"),
        ))
    } else {
        Ok(())
    }
}

fn validate_char_limit(
    path: &Path,
    field: &str,
    value: &str,
    max_chars: usize,
) -> Result<(), StorageError> {
    let chars = value.chars().count();
    if chars > max_chars {
        Err(reference_compliance_error(
            path,
            format!("{field} is too long ({chars} chars > {max_chars} chars)"),
        ))
    } else {
        Ok(())
    }
}

fn reference_compliance_error(path: &Path, reason: impl Into<String>) -> StorageError {
    StorageError::ReferenceCompliance {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

fn runtime_snapshot_file_name(snapshot_id: &str) -> String {
    format!("{snapshot_id}.runtime_snapshot.json")
}

pub fn validate_runtime_snapshot_id(snapshot_id: &str) -> Result<(), StorageError> {
    let valid = !snapshot_id.is_empty()
        && snapshot_id != "latest"
        && snapshot_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(StorageError::InvalidRuntimeSnapshotId(snapshot_id.into()))
    }
}

fn resource(key: &str, label: &str, initial: i32, min: i32, max: i32) -> ResourceDefinition {
    ResourceDefinition {
        key: key.into(),
        label: label.into(),
        initial,
        min,
        max,
    }
}

fn dynasty_characters() -> Vec<Character> {
    vec![
        character(
            "grand-secretary",
            "Grand Secretary",
            "Court administrator",
            &["cautious", "faction-aware"],
        ),
        character(
            "war-minister",
            "Minister of War",
            "Military logistics",
            &["urgent", "pragmatic"],
        ),
        character(
            "eunuch-director",
            "Eunuch Director",
            "Palace intelligence channel",
            &["watchful", "ambiguous"],
        ),
        character(
            "border-general",
            "Border General",
            "Frontier commander",
            &["loyal-if-paid", "blunt"],
        ),
        character(
            "censor",
            "Court Censor",
            "Moral and legal critic",
            &["severe", "public-minded"],
        ),
        character(
            "provincial-governor",
            "Provincial Governor",
            "Local implementation",
            &["strained", "risk-averse"],
        ),
    ]
}

fn character(id: &str, name: &str, role: &str, traits: &[&str]) -> Character {
    Character {
        id: id.into(),
        name: name.into(),
        role: role.into(),
        traits: traits
            .iter()
            .map(|trait_name| (*trait_name).into())
            .collect(),
        visual_card: format!("{name}, restrained historical portrait"),
        voice_card: format!("{name}, concise court speech"),
        portrait_request: None,
    }
}

fn dynasty_rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "raise-tax-emergency".into(),
            action_type: "raise_tax".into(),
            conditions: vec![Condition::ResourceAtMost {
                key: "treasury".into(),
                value: 80,
            }],
            effects: vec![
                Effect::AddResource {
                    key: "treasury".into(),
                    amount: 12,
                },
                Effect::AddResource {
                    key: "public_order".into(),
                    amount: -8,
                },
                Effect::AddResource {
                    key: "court_stability".into(),
                    amount: -5,
                },
                Effect::AddResource {
                    key: "army_morale".into(),
                    amount: 4,
                },
                Effect::TriggerEvent {
                    event: "local_tax_resistance".into(),
                },
            ],
        },
        Rule {
            id: "inspect-corruption-ledger".into(),
            action_type: "inspect_corruption".into(),
            conditions: Vec::new(),
            effects: vec![
                Effect::AddResource {
                    key: "treasury".into(),
                    amount: 4,
                },
                Effect::AddResource {
                    key: "court_stability".into(),
                    amount: -8,
                },
                Effect::SetFlag {
                    key: "corruption_investigation".into(),
                    value: true,
                },
                Effect::TriggerEvent {
                    event: "officials_submit_memorials".into(),
                },
            ],
        },
        Rule {
            id: "pay-border-army".into(),
            action_type: "pay_army".into(),
            conditions: vec![Condition::ResourceAtLeast {
                key: "treasury".into(),
                value: 10,
            }],
            effects: vec![
                Effect::AddResource {
                    key: "treasury".into(),
                    amount: -10,
                },
                Effect::AddResource {
                    key: "army_morale".into(),
                    amount: 12,
                },
                Effect::AddResource {
                    key: "enemy_pressure".into(),
                    amount: -3,
                },
                Effect::TriggerEvent {
                    event: "border_army_paid".into(),
                },
            ],
        },
    ]
}

fn demo_reference_analysis() -> ReferenceAnalysis {
    ReferenceAnalysis {
        id: "political-crisis-patterns".into(),
        title: "Political Crisis Pattern Notes".into(),
        source: ReferenceSource {
            source_type: ReferenceSourceType::MethodTemplate,
            rights: ReferenceRights::GenericMethod,
            citation: "PlotForge built-in generic method template".into(),
            user_authorized: false,
        },
        summary: "Escalate political pressure through visible tradeoffs, not copied source prose."
            .into(),
        structure_notes: vec![
            ReferenceStructureNote {
                label: "resource squeeze".into(),
                summary: "Start with a concrete shortage the ruler cannot ignore.".into(),
            },
            ReferenceStructureNote {
                label: "legitimacy collision".into(),
                summary: "Make each practical fix damage trust, order, or faction alignment."
                    .into(),
            },
        ],
        tags: vec!["method".into(), "political".into(), "pacing".into()],
    }
}

fn initial_scene() -> Scene {
    let first_beat_id = "court-crisis-001-beat-001".to_string();
    let second_beat_id = "court-crisis-001-beat-002".to_string();
    Scene {
        key: "court-crisis-001".into(),
        title: "The Red Deficit Ledger".into(),
        location: "Qianqing Palace".into(),
        dramatic_purpose: "Force the player to choose between revenue, order, and military loyalty.".into(),
        hook: "The border payroll ledger arrives with a fresh red deficit mark beside the army columns.".into(),
        background_asset: "assets/generated/court-crisis-001.png".into(),
        audio_refs: Vec::new(),
        character_ids: vec![
            "grand-secretary".into(),
            "war-minister".into(),
            "eunuch-director".into(),
        ],
        plot_thread_updates: BTreeMap::from([(
            "border-payroll".into(),
            "The unpaid army becomes the first visible crisis.".into(),
        )]),
        entry_beat_id: Some(first_beat_id.clone()),
        beats: vec![Beat {
            id: first_beat_id,
            text: "The court kneels around a ledger that says the border army is two months from mutiny.".into(),
            speaker: Some("grand-secretary".into()),
            line_delivery: Some("measured court alarm".into()),
            audio_refs: Vec::new(),
            choices: vec![
                Choice {
                    id: "continue-council".into(),
                    label: "听一位大臣继续陈情".into(),
                    action_type: "continue".into(),
                    input_terms: vec![
                        "continue".into(),
                        "minister".into(),
                        "hear".into(),
                        "听".into(),
                        "继续".into(),
                        "陈情".into(),
                    ],
                    dramatic_purpose: "Stay in the council scene to gather more pressure before issuing an order.".into(),
                    change_scene: false,
                },
                Choice {
                    id: "raise-tax".into(),
                    label: "加征辽饷，立刻补军饷".into(),
                    action_type: "raise_tax".into(),
                    input_terms: vec![
                        "raise".into(),
                        "tax".into(),
                        "levy".into(),
                        "加征".into(),
                        "辽饷".into(),
                    ],
                    dramatic_purpose: "Trade public order for immediate treasury relief.".into(),
                    change_scene: true,
                },
                Choice {
                    id: "inspect-corruption".into(),
                    label: "严查军饷贪墨".into(),
                    action_type: "inspect_corruption".into(),
                    input_terms: vec![
                        "inspect".into(),
                        "corruption".into(),
                        "严查".into(),
                        "贪墨".into(),
                        "查".into(),
                    ],
                    dramatic_purpose: "Seek stolen funds while angering court factions.".into(),
                    change_scene: true,
                },
                Choice {
                    id: "pay-army".into(),
                    label: "先拨内帑稳住边军".into(),
                    action_type: "pay_army".into(),
                    input_terms: vec![
                        "pay".into(),
                        "army".into(),
                        "军饷".into(),
                        "拨".into(),
                        "内帑".into(),
                        "边军".into(),
                    ],
                    dramatic_purpose: "Spend scarce treasury to buy military time.".into(),
                    change_scene: true,
                },
            ],
            next: BeatNext::Beat(second_beat_id.clone()),
        }, Beat {
            id: second_beat_id,
            text: "The war minister steps forward: delay will keep the court calm today, but the frontier will remember it tomorrow.".into(),
            speaker: Some("war-minister".into()),
            line_delivery: Some("terse warning".into()),
            audio_refs: Vec::new(),
            choices: vec![
                Choice {
                    id: "raise-tax".into(),
                    label: "加征辽饷，立刻补军饷".into(),
                    action_type: "raise_tax".into(),
                    input_terms: vec![
                        "raise".into(),
                        "tax".into(),
                        "levy".into(),
                        "加征".into(),
                        "辽饷".into(),
                    ],
                    dramatic_purpose: "Trade public order for immediate treasury relief.".into(),
                    change_scene: true,
                },
                Choice {
                    id: "inspect-corruption".into(),
                    label: "严查军饷贪墨".into(),
                    action_type: "inspect_corruption".into(),
                    input_terms: vec![
                        "inspect".into(),
                        "corruption".into(),
                        "严查".into(),
                        "贪墨".into(),
                        "查".into(),
                    ],
                    dramatic_purpose: "Seek stolen funds while angering court factions.".into(),
                    change_scene: true,
                },
                Choice {
                    id: "pay-army".into(),
                    label: "先拨内帑稳住边军".into(),
                    action_type: "pay_army".into(),
                    input_terms: vec![
                        "pay".into(),
                        "army".into(),
                        "军饷".into(),
                        "拨".into(),
                        "内帑".into(),
                        "边军".into(),
                    ],
                    dramatic_purpose: "Spend scarce treasury to buy military time.".into(),
                    change_scene: true,
                },
            ],
            next: BeatNext::Scene,
        }],
    }
}

fn project_agents_md() -> &'static str {
    "# AGENTS.md\n\n## Project goal\n\nBuild a playable PlotForge story project from local files.\n\n## Commands\n\n- `plotforge check .`\n- `plotforge play . --once`\n- `plotforge export static . --out exports/static`\n\n## Rules\n\n- Files are source of truth.\n- Do not put API keys in project files or exports.\n- AI output proposes content; engine rules commit state.\n- Reference imports store metadata, rights, short summaries, and structure notes only; do not store large raw copyrighted bodies.\n"
}

pub(crate) trait IoContext<T> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, StorageError>;
}

impl<T> IoContext<T> for Result<T, std::io::Error> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, StorageError> {
        self.map_err(|source| StorageError::Io {
            path: path.as_ref().to_path_buf(),
            source,
        })
    }
}

const PLACEHOLDER_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 100, 96, 96, 248, 15, 0, 1,
    5, 1, 2, 161, 13, 197, 111, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[cfg(test)]
mod tests {
    use super::{create_demo_project, load_project, validate_project};

    #[test]
    fn creates_and_loads_demo_project() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");

        create_demo_project(&project_path, false).expect("create demo");
        let loaded = load_project(&project_path).expect("load project");
        validate_project(&project_path).expect("validate project");

        assert_eq!(loaded.game.id, "dynasty-embers");
        assert!(project_path.join("story/story_craft.toml").exists());
        assert!(project_path.join("AGENTS.md").exists());
        assert!(
            loaded
                .rules
                .iter()
                .any(|rule| rule.action_type == "raise_tax")
        );
    }
}
