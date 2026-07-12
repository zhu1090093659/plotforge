use std::path::PathBuf;

use plotforge_schema::{AI_USAGE_MANIFEST_FILE, DESKTOP_RUNTIME_DRAFT_FILE};

pub fn expected_export_files() -> Vec<PathBuf> {
    vec![
        PathBuf::from(AI_USAGE_MANIFEST_FILE),
        PathBuf::from("game.json"),
        PathBuf::from("index.html"),
        PathBuf::from("player-audio.js"),
        PathBuf::from("player-core.js"),
        PathBuf::from("player-i18n.js"),
        PathBuf::from("player-save.js"),
        PathBuf::from("player-types.js"),
        PathBuf::from("player.js"),
        PathBuf::from("styles.css"),
    ]
}

pub fn expected_export_files_with_build_notes() -> Vec<PathBuf> {
    let mut files = expected_export_files();
    files.insert(1, PathBuf::from("desktop-build-notes.md"));
    files
}

pub fn expected_desktop_export_files() -> Vec<PathBuf> {
    let mut files = expected_export_files_with_build_notes();
    files.insert(2, PathBuf::from(DESKTOP_RUNTIME_DRAFT_FILE));
    files
}
