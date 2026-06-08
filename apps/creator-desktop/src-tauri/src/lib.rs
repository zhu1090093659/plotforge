mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::create_project,
            commands::open_project,
            commands::check_project,
            commands::list_export_profiles,
            commands::read_world_edit_document,
            commands::update_world_edit_document,
            commands::read_story_craft_edit_document,
            commands::update_story_craft_edit_document,
            commands::read_character_edit_document,
            commands::update_character_edit_document,
            commands::create_character,
            commands::read_state_variables_edit_document,
            commands::update_state_variables_edit_document,
            commands::create_resource,
            commands::read_rules_edit_document,
            commands::update_rules_edit_document,
            commands::create_rule,
            commands::generate_world_expansion,
            commands::generate_story_craft,
            commands::generate_character,
            commands::read_ai_safety_policy,
            commands::update_ai_safety_policy,
            commands::read_visual_bible,
            commands::update_visual_bible,
            commands::read_audio_bible,
            commands::update_audio_bible,
            commands::play_once_project,
            commands::play_once_project_with_save,
            commands::play_once_project_from_snapshot,
            commands::play_once_project_from_latest_snapshot,
            commands::export_static_project,
            commands::export_static_project_zip,
            commands::list_asset_records,
            commands::list_source_files,
            commands::read_source_file,
            commands::write_source_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running PlotForge Studio");
}
