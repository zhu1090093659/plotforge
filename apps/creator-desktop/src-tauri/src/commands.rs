use plotforge_studio::{
    AiSafetyPolicy, AssetRecord, AudioBible, Character, CharacterEditDocument,
    CharacterGenerationReport, ExportProfile, PiAgentCapability, PiAgentRunRequest,
    PiAgentRunResult, PlayOnceReport, ProjectCheckReport, ProjectCreationReport,
    ProjectCreationRequest, ProjectData, ResourceDefinition, Rule, RulesEditDocument,
    SourceFileContent, SourceFileSummary, StateVariablesEditDocument, StaticExportReport,
    StoryCraftEditDocument, StoryCraftGenerationReport, StudioCommandError, VisualBible,
    WorldEditDocument, WorldGenerationReport,
};

#[tauri::command(rename_all = "snake_case")]
pub fn create_project(
    path: String,
    request: ProjectCreationRequest,
    force: bool,
) -> Result<ProjectCreationReport, StudioCommandError> {
    plotforge_studio::create_project(path, request, force)
}

#[tauri::command(rename_all = "snake_case")]
pub fn open_project(path: String) -> Result<ProjectData, StudioCommandError> {
    plotforge_studio::open_project(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn check_project(path: String) -> Result<ProjectCheckReport, StudioCommandError> {
    plotforge_studio::check_project(path)
}

#[tauri::command]
pub fn list_export_profiles() -> Vec<ExportProfile> {
    plotforge_studio::list_export_profiles()
}

#[tauri::command(rename_all = "snake_case")]
pub fn read_world_edit_document(path: String) -> Result<WorldEditDocument, StudioCommandError> {
    plotforge_studio::read_world_edit_document(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_world_edit_document(
    path: String,
    document: WorldEditDocument,
) -> Result<WorldEditDocument, StudioCommandError> {
    plotforge_studio::update_world_edit_document(path, document)
}

#[tauri::command(rename_all = "snake_case")]
pub fn read_story_craft_edit_document(
    path: String,
) -> Result<StoryCraftEditDocument, StudioCommandError> {
    plotforge_studio::read_story_craft_edit_document(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_story_craft_edit_document(
    path: String,
    document: StoryCraftEditDocument,
) -> Result<StoryCraftEditDocument, StudioCommandError> {
    plotforge_studio::update_story_craft_edit_document(path, document)
}

#[tauri::command(rename_all = "snake_case")]
pub fn read_character_edit_document(
    path: String,
) -> Result<CharacterEditDocument, StudioCommandError> {
    plotforge_studio::read_character_edit_document(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_character_edit_document(
    path: String,
    document: CharacterEditDocument,
) -> Result<CharacterEditDocument, StudioCommandError> {
    plotforge_studio::update_character_edit_document(path, document)
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_character(
    path: String,
    character: Character,
) -> Result<CharacterEditDocument, StudioCommandError> {
    plotforge_studio::create_character(path, character)
}

#[tauri::command(rename_all = "snake_case")]
pub fn read_state_variables_edit_document(
    path: String,
) -> Result<StateVariablesEditDocument, StudioCommandError> {
    plotforge_studio::read_state_variables_edit_document(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_state_variables_edit_document(
    path: String,
    document: StateVariablesEditDocument,
) -> Result<StateVariablesEditDocument, StudioCommandError> {
    plotforge_studio::update_state_variables_edit_document(path, document)
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_resource(
    path: String,
    resource: ResourceDefinition,
) -> Result<StateVariablesEditDocument, StudioCommandError> {
    plotforge_studio::create_resource(path, resource)
}

#[tauri::command(rename_all = "snake_case")]
pub fn read_rules_edit_document(path: String) -> Result<RulesEditDocument, StudioCommandError> {
    plotforge_studio::read_rules_edit_document(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_rules_edit_document(
    path: String,
    document: RulesEditDocument,
) -> Result<RulesEditDocument, StudioCommandError> {
    plotforge_studio::update_rules_edit_document(path, document)
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_rule(path: String, rule: Rule) -> Result<RulesEditDocument, StudioCommandError> {
    plotforge_studio::create_rule(path, rule)
}

#[tauri::command(rename_all = "snake_case")]
pub fn generate_world_expansion(
    path: String,
    expansion_goal: String,
) -> Result<WorldGenerationReport, StudioCommandError> {
    plotforge_studio::generate_world_expansion(path, &expansion_goal)
}

#[tauri::command(rename_all = "snake_case")]
pub fn generate_story_craft(
    path: String,
    concept: String,
) -> Result<StoryCraftGenerationReport, StudioCommandError> {
    plotforge_studio::generate_story_craft(path, &concept)
}

#[tauri::command(rename_all = "snake_case")]
pub fn generate_character(
    path: String,
    concept: String,
    role_hint: String,
) -> Result<CharacterGenerationReport, StudioCommandError> {
    plotforge_studio::generate_character(path, &concept, &role_hint)
}

#[tauri::command(rename_all = "snake_case")]
pub fn read_ai_safety_policy(path: String) -> Result<AiSafetyPolicy, StudioCommandError> {
    plotforge_studio::read_ai_safety_policy(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_ai_safety_policy(
    path: String,
    policy: AiSafetyPolicy,
) -> Result<AiSafetyPolicy, StudioCommandError> {
    plotforge_studio::update_ai_safety_policy(path, policy)
}

#[tauri::command(rename_all = "snake_case")]
pub fn read_visual_bible(path: String) -> Result<VisualBible, StudioCommandError> {
    plotforge_studio::read_visual_bible(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_visual_bible(
    path: String,
    visual_bible: VisualBible,
) -> Result<VisualBible, StudioCommandError> {
    plotforge_studio::update_visual_bible(path, visual_bible)
}

#[tauri::command(rename_all = "snake_case")]
pub fn read_audio_bible(path: String) -> Result<AudioBible, StudioCommandError> {
    plotforge_studio::read_audio_bible(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_audio_bible(
    path: String,
    audio_bible: AudioBible,
) -> Result<AudioBible, StudioCommandError> {
    plotforge_studio::update_audio_bible(path, audio_bible)
}

#[tauri::command(rename_all = "snake_case")]
pub fn play_once_project(
    path: String,
    player_input: String,
) -> Result<PlayOnceReport, StudioCommandError> {
    plotforge_studio::play_once_project(path, &player_input)
}

#[tauri::command(rename_all = "snake_case")]
pub fn play_once_project_with_save(
    path: String,
    player_input: String,
    save_id: String,
) -> Result<PlayOnceReport, StudioCommandError> {
    plotforge_studio::play_once_project_with_save(path, &player_input, Some(&save_id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn play_once_project_from_snapshot(
    path: String,
    player_input: String,
    snapshot_id: String,
    save_id: Option<String>,
) -> Result<PlayOnceReport, StudioCommandError> {
    plotforge_studio::play_once_project_from_snapshot(
        path,
        &player_input,
        &snapshot_id,
        save_id.as_deref(),
    )
}

#[tauri::command(rename_all = "snake_case")]
pub fn play_once_project_from_latest_snapshot(
    path: String,
    player_input: String,
    save_id: Option<String>,
) -> Result<PlayOnceReport, StudioCommandError> {
    plotforge_studio::play_once_project_from_latest_snapshot(
        path,
        &player_input,
        save_id.as_deref(),
    )
}

#[tauri::command(rename_all = "snake_case")]
pub fn export_static_project(
    path: String,
    output_dir: String,
) -> Result<StaticExportReport, StudioCommandError> {
    plotforge_studio::export_static_project(path, output_dir)
}

#[tauri::command(rename_all = "snake_case")]
pub fn export_static_project_zip(
    path: String,
    output_dir: String,
    archive_path: String,
) -> Result<StaticExportReport, StudioCommandError> {
    plotforge_studio::export_static_project_zip(path, output_dir, archive_path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_asset_records(path: String) -> Result<Vec<AssetRecord>, StudioCommandError> {
    plotforge_studio::list_asset_records(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_source_files(path: String) -> Result<Vec<SourceFileSummary>, StudioCommandError> {
    plotforge_studio::list_source_files(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn read_source_file(
    path: String,
    relative_path: String,
) -> Result<SourceFileContent, StudioCommandError> {
    plotforge_studio::read_source_file(path, &relative_path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn write_source_file(
    path: String,
    relative_path: String,
    content: String,
) -> Result<SourceFileContent, StudioCommandError> {
    plotforge_studio::write_source_file(path, &relative_path, &content)
}

#[tauri::command(rename_all = "snake_case")]
pub fn pi_agent_run(
    request: PiAgentRunRequest,
) -> Result<PiAgentRunResult, StudioCommandError> {
    plotforge_studio::pi_agent_run(request)
}

#[tauri::command(rename_all = "snake_case")]
pub fn pi_agent_capabilities() -> Result<Vec<PiAgentCapability>, StudioCommandError> {
    plotforge_studio::pi_agent_capabilities()
}
