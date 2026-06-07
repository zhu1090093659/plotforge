use std::fs;

use plotforge_export::{ExportError, export_static_web};
use plotforge_schema::ExportManifest;
use plotforge_storage::create_demo_project;

#[test]
fn export_manifest_contains_entry_scene_and_assets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_demo_project(&project_path, false).expect("create");

    export_static_web(&project_path, &output_dir).expect("export");

    let manifest: ExportManifest = serde_json::from_str(
        &fs::read_to_string(output_dir.join("game.json")).expect("read manifest"),
    )
    .expect("parse manifest");
    assert_eq!(manifest.game.title, "Dynasty Embers");
    assert_eq!(manifest.entry_scene, "court-crisis-001");
    assert!(
        manifest
            .assets
            .contains(&"assets/generated/court-crisis-001.png".into())
    );
    for asset in manifest.assets {
        assert!(output_dir.join(asset).is_file());
    }
}

#[test]
fn export_rejects_secret_markers_in_manifest_data() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_demo_project(&project_path, false).expect("create");
    let game_path = project_path.join("game.toml");
    let mut game = fs::read_to_string(&game_path).expect("read game");
    game = game.replace(
        "Historical crisis simulation about a collapsing dynasty.",
        "sk-test-secret-marker",
    );
    fs::write(&game_path, game).expect("write game");

    let error = export_static_web(&project_path, &output_dir).expect_err("secret marker");

    assert!(matches!(error, ExportError::SecretMarker(_)));
}
