mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::open_project,
            commands::check_project,
            commands::play_once_project,
            commands::export_static_project,
            commands::list_source_files,
            commands::read_source_file,
            commands::write_source_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running PlotForge Studio");
}
