use plotforge_studio::{
    AgentSessionConfig, AiSafetyPolicy, AssetRecord, AudioBible, Character, CharacterDraft,
    CharacterEditDocument, CharacterGenerationReport, ExportProfile, GitBranchInfo,
    GitSwitchResult, ImageProviderEntry, McpServerEntry, McpServerTestResult,
    McpToolCallRequest, McpToolCallResult, McpToolManifest, ModelOption, PiAgentApplyRequest,
    PiAgentApplyResult, PiAgentCapability, PiAgentRunRequest, PiAgentRunResult, PlayOnceReport,
    ProjectCheckReport, ProjectCreationReport, ProjectCreationRequest, ProjectData, ProviderEntry,
    ProviderTestResult, PromptTemplate, RemoteModelInfo, ResourceDefinition, Rule, RuleDraft,
    RulesEditDocument, SkillIndex, SkillManifest, SourceFileContent, SourceFileSummary,
    StateVariablesEditDocument, StaticExportReport, StoryCraftEditDocument,
    StoryCraftGenerationReport, StudioCommandError, TtsProviderEntry, VisualBible,
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
pub fn create_character_from_draft(
    path: String,
    draft: CharacterDraft,
) -> Result<CharacterEditDocument, StudioCommandError> {
    plotforge_studio::create_character_from_draft(path, draft)
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_rule_from_draft(
    path: String,
    draft: RuleDraft,
) -> Result<RulesEditDocument, StudioCommandError> {
    plotforge_studio::create_rule_from_draft(path, draft)
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

#[tauri::command(rename_all = "snake_case")]
pub fn git_current_branch(path: String) -> Result<String, StudioCommandError> {
    plotforge_studio::git_current_branch(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn git_list_branches(path: String) -> Result<Vec<GitBranchInfo>, StudioCommandError> {
    plotforge_studio::git_list_branches(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn git_switch_branch(
    path: String,
    branch: String,
) -> Result<GitSwitchResult, StudioCommandError> {
    plotforge_studio::git_switch_branch(path, &branch)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_available_models() -> Result<Vec<ModelOption>, StudioCommandError> {
    plotforge_studio::list_available_models()
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_agent_session_config(path: String) -> Result<AgentSessionConfig, StudioCommandError> {
    plotforge_studio::get_agent_session_config(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_agent_session_config(
    path: String,
    config: AgentSessionConfig,
) -> Result<AgentSessionConfig, StudioCommandError> {
    plotforge_studio::set_agent_session_config(path, &config)
}

#[tauri::command(rename_all = "snake_case")]
pub fn pi_agent_apply_run(
    request: PiAgentApplyRequest,
) -> Result<PiAgentApplyResult, StudioCommandError> {
    plotforge_studio::pi_agent_apply_run(request)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_providers() -> Result<Vec<ProviderEntry>, StudioCommandError> {
    plotforge_studio::list_providers()
}

#[tauri::command(rename_all = "snake_case")]
pub fn upsert_provider(entry: ProviderEntry) -> Result<ProviderEntry, StudioCommandError> {
    plotforge_studio::upsert_provider(entry)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_provider(id: String) -> Result<ProviderEntry, StudioCommandError> {
    plotforge_studio::delete_provider(id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn test_provider_connection(id: String) -> Result<ProviderTestResult, StudioCommandError> {
    plotforge_studio::test_provider_connection(id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_remote_models(provider_id: String) -> Result<Vec<RemoteModelInfo>, StudioCommandError> {
    plotforge_studio::list_remote_models(provider_id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_image_providers() -> Result<Vec<ImageProviderEntry>, StudioCommandError> {
    plotforge_studio::list_image_providers()
}

#[tauri::command(rename_all = "snake_case")]
pub fn upsert_image_provider(
    entry: ImageProviderEntry,
) -> Result<ImageProviderEntry, StudioCommandError> {
    plotforge_studio::upsert_image_provider(entry)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_image_provider(id: String) -> Result<ImageProviderEntry, StudioCommandError> {
    plotforge_studio::delete_image_provider(id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn test_image_provider(id: String) -> Result<ProviderTestResult, StudioCommandError> {
    plotforge_studio::test_image_provider(id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_tts_providers() -> Result<Vec<TtsProviderEntry>, StudioCommandError> {
    plotforge_studio::list_tts_providers()
}

#[tauri::command(rename_all = "snake_case")]
pub fn upsert_tts_provider(
    entry: TtsProviderEntry,
) -> Result<TtsProviderEntry, StudioCommandError> {
    plotforge_studio::upsert_tts_provider(entry)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_tts_provider(id: String) -> Result<TtsProviderEntry, StudioCommandError> {
    plotforge_studio::delete_tts_provider(id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn test_tts_provider(id: String) -> Result<ProviderTestResult, StudioCommandError> {
    plotforge_studio::test_tts_provider(id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_user_prompt_templates() -> Result<Vec<PromptTemplate>, StudioCommandError> {
    plotforge_studio::list_user_prompt_templates()
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_project_prompt_templates(
    project_path: String,
) -> Result<Vec<PromptTemplate>, StudioCommandError> {
    plotforge_studio::list_project_prompt_templates(project_path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn upsert_user_prompt_template(
    template: PromptTemplate,
) -> Result<PromptTemplate, StudioCommandError> {
    plotforge_studio::upsert_user_prompt_template(template)
}

#[tauri::command(rename_all = "snake_case")]
pub fn upsert_project_prompt_template(
    project_path: String,
    template: PromptTemplate,
) -> Result<PromptTemplate, StudioCommandError> {
    plotforge_studio::upsert_project_prompt_template(project_path, template)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_user_prompt_template(id: String) -> Result<(), StudioCommandError> {
    plotforge_studio::delete_user_prompt_template(id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_project_prompt_template(
    project_path: String,
    id: String,
) -> Result<(), StudioCommandError> {
    plotforge_studio::delete_project_prompt_template(project_path, id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_skills() -> Result<Vec<SkillManifest>, StudioCommandError> {
    plotforge_studio::list_skills()
}

#[tauri::command(rename_all = "snake_case")]
pub fn refresh_skill_index() -> Result<SkillIndex, StudioCommandError> {
    plotforge_studio::refresh_skill_index()
}

#[tauri::command(rename_all = "snake_case")]
pub fn import_skill(skill_id: String) -> Result<SkillManifest, StudioCommandError> {
    plotforge_studio::import_skill(skill_id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn read_skill_body(skill_id: String) -> Result<String, StudioCommandError> {
    plotforge_studio::read_skill_body(skill_id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn enable_skill_for_project(
    project_path: String,
    skill_id: String,
    enabled: bool,
) -> Result<AgentSessionConfig, StudioCommandError> {
    plotforge_studio::enable_skill_for_project(project_path, skill_id, enabled)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_mcp_servers() -> Result<Vec<McpServerEntry>, StudioCommandError> {
    plotforge_studio::list_mcp_servers()
}

#[tauri::command(rename_all = "snake_case")]
pub fn upsert_mcp_server(entry: McpServerEntry) -> Result<McpServerEntry, StudioCommandError> {
    plotforge_studio::upsert_mcp_server(entry)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_mcp_server(id: String) -> Result<McpServerEntry, StudioCommandError> {
    plotforge_studio::delete_mcp_server(id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn test_mcp_server(id: String) -> Result<McpServerTestResult, StudioCommandError> {
    plotforge_studio::test_mcp_server(id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_mcp_tools(server_id: String) -> Result<Vec<McpToolManifest>, StudioCommandError> {
    plotforge_studio::list_mcp_tools(server_id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn invoke_mcp_tool(
    request: McpToolCallRequest,
) -> Result<McpToolCallResult, StudioCommandError> {
    plotforge_studio::invoke_mcp_tool(request)
}

#[tauri::command(rename_all = "snake_case")]
pub fn enable_mcp_server_for_project(
    project_path: String,
    server_id: String,
    enabled: bool,
) -> Result<AgentSessionConfig, StudioCommandError> {
    plotforge_studio::enable_mcp_server_for_project(project_path, server_id, enabled)
}
