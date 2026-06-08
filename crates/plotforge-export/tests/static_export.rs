use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use plotforge_export::{ExportError, export_static_web};
use plotforge_media::MediaError;
use plotforge_schema::ExportManifest;
use plotforge_storage::create_demo_project;

#[test]
fn export_manifest_contains_entry_scene_and_assets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_demo_project(&project_path, false).expect("create");

    let report = export_static_web(&project_path, &output_dir).expect("export");

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
    let expected_files = vec![
        PathBuf::from("assets/generated/court-crisis-001.png"),
        PathBuf::from("game.json"),
        PathBuf::from("index.html"),
    ];
    assert_eq!(report.audit.allowed_files, expected_files);
    assert_eq!(report.audit.files_found, expected_files);
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

#[test]
fn export_excludes_project_traces_provider_config_and_raw_responses() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_demo_project(&project_path, false).expect("create");
    fs::create_dir_all(project_path.join("traces")).expect("traces dir");
    fs::create_dir_all(project_path.join("providers")).expect("providers dir");
    fs::create_dir_all(project_path.join("agents/raw_responses")).expect("raw dir");
    fs::write(
        project_path.join("traces/latest.json"),
        r#"{"private":"sk-test-secret-marker"}"#,
    )
    .expect("write trace");
    fs::write(
        project_path.join("providers/config.json"),
        r#"{"api_key":"sk-test-secret-marker"}"#,
    )
    .expect("write provider config");
    fs::write(
        project_path.join("agents/raw_responses/scene.json"),
        r#"{"raw":"provider response"}"#,
    )
    .expect("write raw response");

    let report = export_static_web(&project_path, &output_dir).expect("export");

    assert!(!output_dir.join("traces/latest.json").exists());
    assert!(!output_dir.join("providers/config.json").exists());
    assert!(!output_dir.join("agents/raw_responses/scene.json").exists());
    assert_eq!(
        report
            .audit
            .files_found
            .into_iter()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            PathBuf::from("assets/generated/court-crisis-001.png"),
            PathBuf::from("game.json"),
            PathBuf::from("index.html"),
        ])
    );
    let exported_manifest = fs::read_to_string(output_dir.join("game.json")).expect("manifest");
    assert!(!exported_manifest.contains("sk-test-secret-marker"));
    assert!(!exported_manifest.contains("provider response"));
}

#[test]
fn export_copies_only_referenced_media_registry_assets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_demo_project(&project_path, false).expect("create");
    fs::write(
        project_path.join("assets/generated/unused-generated.png"),
        b"unused generated image bytes",
    )
    .expect("write unused asset");
    let source_bytes =
        fs::read(project_path.join("assets/generated/court-crisis-001.png")).expect("source png");

    let report = export_static_web(&project_path, &output_dir).expect("export");

    assert_eq!(
        fs::read(output_dir.join("assets/generated/court-crisis-001.png")).expect("exported png"),
        source_bytes
    );
    assert!(
        !output_dir
            .join("assets/generated/unused-generated.png")
            .exists()
    );
    assert!(
        !report
            .audit
            .files_found
            .contains(&PathBuf::from("assets/generated/unused-generated.png"))
    );
}

#[test]
fn export_rejects_stale_disallowed_files_in_output_package() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_demo_project(&project_path, false).expect("create");
    fs::create_dir_all(output_dir.join("traces")).expect("output traces");
    fs::write(output_dir.join("traces/latest.json"), "{}").expect("stale trace");

    let error = export_static_web(&project_path, &output_dir).expect_err("disallowed output file");

    assert!(matches!(
        error,
        ExportError::DisallowedPackageFile(path)
            if path.as_path() == Path::new("traces/latest.json")
    ));
}

#[test]
fn export_rejects_unsafe_asset_paths_before_writing_assets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_demo_project(&project_path, false).expect("create");
    let scene_path = project_path.join("scenes/court-crisis-001.scene.json");
    let scene = fs::read_to_string(&scene_path)
        .expect("read scene")
        .replace(
            "assets/generated/court-crisis-001.png",
            "../traces/latest.json",
        );
    fs::write(&scene_path, scene).expect("write scene");

    let error = export_static_web(&project_path, &output_dir).expect_err("unsafe asset");

    assert!(matches!(
        error,
        ExportError::Media(MediaError::UnsafeAssetPath(_))
    ));
    assert!(!temp.path().join("traces/latest.json").exists());
}
