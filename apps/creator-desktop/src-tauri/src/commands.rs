use plotforge_studio::{
    PlayOnceReport, ProjectCheckReport, ProjectData, StaticExportReport, StudioCommandError,
};

#[tauri::command(rename_all = "snake_case")]
pub fn open_project(path: String) -> Result<ProjectData, StudioCommandError> {
    plotforge_studio::open_project(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn check_project(path: String) -> Result<ProjectCheckReport, StudioCommandError> {
    plotforge_studio::check_project(path)
}

#[tauri::command(rename_all = "snake_case")]
pub fn play_once_project(
    path: String,
    player_input: String,
) -> Result<PlayOnceReport, StudioCommandError> {
    plotforge_studio::play_once_project(path, &player_input)
}

#[tauri::command(rename_all = "snake_case")]
pub fn export_static_project(
    path: String,
    output_dir: String,
) -> Result<StaticExportReport, StudioCommandError> {
    plotforge_studio::export_static_project(path, output_dir)
}
