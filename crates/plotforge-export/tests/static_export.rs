use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use plotforge_export::{
    ExportError, export_desktop_runtime_draft, export_static_web, export_static_web_zip,
};
use plotforge_media::MediaError;
use plotforge_schema::{
    AI_USAGE_MANIFEST_FILE, AiSafetyPolicy, AiUsageContentKind, AiUsageManifest, AssetKind,
    AssetSourceKind, DESKTOP_RUNTIME_DRAFT_FILE, DesktopRuntimeDraft, ExportManifest,
    MediaAssetReference, ProjectCreationRequest, ProjectTemplateId, contains_secret_marker_text,
};
use plotforge_storage::{
    attach_beat_audio_reference, create_project_from_request, load_project, update_ai_safety_policy,
};

mod common;

#[test]
fn export_manifest_contains_entry_scene_and_assets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_starter_project(&project_path);

    let report = export_static_web(&project_path, &output_dir).expect("export");

    let manifest: ExportManifest = serde_json::from_str(
        &fs::read_to_string(output_dir.join("game.json")).expect("read manifest"),
    )
    .expect("parse manifest");
    assert_eq!(manifest.game.title, "Project");
    assert_eq!(manifest.entry_scene, "opening-scene");
    assert_eq!(manifest.profile.id, "static-web");
    assert!(!manifest.profile.requires_network_at_runtime);
    assert!(!manifest.profile.includes_provider_config);
    assert!(!manifest.profile.includes_private_traces);
    assert!(!manifest.profile.platform_submission_ready);
    assert_eq!(manifest.ai_usage_manifest_path, AI_USAGE_MANIFEST_FILE);
    assert!(
        manifest
            .assets
            .contains(&"assets/generated/placeholder.png".into())
    );
    for asset in manifest.assets {
        assert!(output_dir.join(asset).is_file());
    }
    let expected_files = expected_export_files();
    assert_eq!(report.audit.allowed_files, expected_files);
    assert_eq!(report.audit.files_found, expected_files);
}

#[test]
fn export_writes_ai_usage_manifest_without_secrets_or_legal_guarantees() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_starter_project(&project_path);
    let policy = update_ai_safety_policy(
        &project_path,
        AiSafetyPolicy {
            live_generated_content_enabled: true,
            content_kinds: vec![AiUsageContentKind::Text],
            safety_guardrails: vec!["Review generated text before package distribution.".into()],
            user_reporting_path: "studio://moderation-queue".into(),
            moderation_policy: "Creator reviews live-generated text before export.".into(),
            human_review_required: true,
            moderation_queue_enabled: true,
            policy_source_path: None,
            evidence_ids: vec!["export-policy-evidence".into()],
            policy_hash: Some("sha256:export-policy-fixture".into()),
            notices: vec!["Local policy evidence only.".into()],
        },
    )
    .expect("update policy");

    let report = export_static_web(&project_path, &output_dir).expect("export");

    assert_eq!(report.audit.files_found, expected_export_files());
    let usage_text = fs::read_to_string(output_dir.join(AI_USAGE_MANIFEST_FILE)).expect("usage");
    let usage: AiUsageManifest = serde_json::from_str(&usage_text).expect("parse usage");
    assert_eq!(usage.project_id, "project");
    assert_eq!(usage.export_profile.id, "static-web");
    assert!(!usage.external_model_calls_during_export);
    assert!(!usage.provider_credentials_included);
    assert!(!usage.raw_provider_responses_included);
    assert!(!usage.private_traces_included);
    assert_eq!(usage.ai_safety_policy, policy);
    assert!(usage.disclosures.iter().any(|disclosure| {
        disclosure
            .asset_paths
            .contains(&"assets/generated/placeholder.png".into())
    }));
    assert_secret_free(&usage_text);
    assert!(!usage_text.contains("raw_response"));
    assert!(!usage_text.contains("request_id"));
    assert!(usage_text.contains("not a legal compliance guarantee"));
}

#[test]
fn export_package_includes_player_web_surface_without_network_urls() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_starter_project(&project_path);

    let report = export_static_web(&project_path, &output_dir).expect("export");

    assert_eq!(report.audit.files_found, expected_export_files());
    let index = fs::read_to_string(output_dir.join("index.html")).expect("index");
    let player = fs::read_to_string(output_dir.join("player.js")).expect("player");
    let player_core = fs::read_to_string(output_dir.join("player-core.js")).expect("player core");
    let player_save = fs::read_to_string(output_dir.join("player-save.js")).expect("player save");
    let player_i18n = fs::read_to_string(output_dir.join("player-i18n.js")).expect("player i18n");
    let player_audio =
        fs::read_to_string(output_dir.join("player-audio.js")).expect("player audio");
    let player_types =
        fs::read_to_string(output_dir.join("player-types.js")).expect("player types");
    let styles = fs::read_to_string(output_dir.join("styles.css")).expect("styles");
    assert!(index.contains("data-player-root"));
    assert!(index.contains("src=\"./player.js\""));
    assert!(index.contains("href=\"./styles.css\""));
    assert!(index.contains("data-field=\"progress\""));
    assert!(index.contains("data-field=\"outcome\""));
    assert!(player.contains("bootPlayer"));
    assert!(player_core.contains("fetch(\"./game.json\""));
    assert!(player_core.contains("resolveChoiceTransition"));
    assert!(player_save.contains("createSaveStore"));
    assert!(player_i18n.contains("createPlayerI18n"));
    assert!(player_audio.contains("renderAudio"));
    for file in [
        index,
        player,
        player_core,
        player_save,
        player_i18n,
        player_audio,
        player_types,
        styles,
    ] {
        assert!(!file.contains("https://"));
        assert!(!file.contains("http://"));
        assert!(!file.contains("//cdn."));
        assert!(!file.contains("//unpkg."));
        assert!(!file.contains("//fonts."));
    }
}

#[test]
fn export_rejects_secret_markers_in_manifest_data() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_starter_project(&project_path);
    let game_path = project_path.join("game.toml");
    let mut game = fs::read_to_string(&game_path).expect("read game");
    game = game.replace(
        "A local starter project for export tests.",
        "Authorization: bearer token=value",
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
    create_starter_project(&project_path);
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
        project_path.join("provider_config.local.toml"),
        r#"api_key = "sk-test-secret-marker""#,
    )
    .expect("write local provider config");
    fs::write(
        project_path.join("agents/raw_responses/scene.json"),
        r#"{"raw":"RAW_PROVIDER_BODY_SHOULD_NOT_EXPORT"}"#,
    )
    .expect("write raw response");

    let report = export_static_web(&project_path, &output_dir).expect("export");

    assert!(!output_dir.join("traces/latest.json").exists());
    assert!(!output_dir.join("providers/config.json").exists());
    assert!(!output_dir.join("provider_config.local.toml").exists());
    assert!(!output_dir.join("agents/raw_responses/scene.json").exists());
    assert_eq!(
        report
            .audit
            .files_found
            .into_iter()
            .collect::<BTreeSet<_>>(),
        expected_export_files().into_iter().collect::<BTreeSet<_>>()
    );
    let exported_manifest = fs::read_to_string(output_dir.join("game.json")).expect("manifest");
    let exported_usage =
        fs::read_to_string(output_dir.join(AI_USAGE_MANIFEST_FILE)).expect("usage");
    assert!(!exported_manifest.contains("sk-test-secret-marker"));
    assert!(!exported_manifest.contains("RAW_PROVIDER_BODY_SHOULD_NOT_EXPORT"));
    assert!(!exported_usage.contains("sk-test-secret-marker"));
    assert!(!exported_usage.contains("RAW_PROVIDER_BODY_SHOULD_NOT_EXPORT"));
}

#[test]
fn export_excludes_project_prompt_templates_and_skills_under_plotforge() {
    // Project-scoped prompt templates and skills live under `.plotforge/`,
    // which the export allowlist omits and the workshop denylist blocks.
    // This test guards against a regression that would leak them into an
    // export package. It also confirms a secret marker in a prompt body is
    // never persisted to the export (the storage writer rejects it before
    // write, but export is the defence-in-depth gate).
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_starter_project(&project_path);
    fs::create_dir_all(project_path.join(".plotforge")).expect("plotforge dir");
    fs::write(
        project_path.join(".plotforge").join("prompts.json"),
        r#"{"templates":[{"id":"leaky","label":"Leaky","scope":"project","body_markdown":"benign"}]}"#,
    )
    .expect("write project prompts");
    fs::create_dir_all(
        project_path
            .join(".plotforge")
            .join("skills")
            .join("my-skill"),
    )
    .expect("skills dir");
    fs::write(
        project_path
            .join(".plotforge")
            .join("skills")
            .join("my-skill")
            .join("SKILL.md"),
        "---\nname: my-skill\ndescription: A skill.\n---\nbody\n",
    )
    .expect("write skill");

    let report = export_static_web(&project_path, &output_dir).expect("export");

    assert!(!output_dir.join(".plotforge").join("prompts.json").exists());
    assert!(
        !output_dir
            .join(".plotforge")
            .join("skills")
            .join("my-skill")
            .join("SKILL.md")
            .exists()
    );
    // The audit allowlist never admits anything under `.plotforge/`.
    assert_eq!(
        report
            .audit
            .files_found
            .into_iter()
            .collect::<BTreeSet<_>>(),
        expected_export_files().into_iter().collect::<BTreeSet<_>>()
    );
    let exported_manifest = fs::read_to_string(output_dir.join("game.json")).expect("manifest");
    assert!(!exported_manifest.contains("my-skill"));
    assert!(!exported_manifest.contains("Leaky"));
}

#[test]
fn export_copies_only_referenced_media_registry_assets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_starter_project(&project_path);
    fs::write(
        project_path.join("assets/generated/unused-generated.png"),
        b"unused generated image bytes",
    )
    .expect("write unused asset");
    let source_bytes =
        fs::read(project_path.join("assets/generated/placeholder.png")).expect("source png");

    let report = export_static_web(&project_path, &output_dir).expect("export");

    assert_eq!(
        fs::read(output_dir.join("assets/generated/placeholder.png")).expect("exported png"),
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
fn export_copies_referenced_audio_assets_and_records_them_in_manifest() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_starter_project(&project_path);
    let audio_path = "assets/generated/opening-scene-beat-001.wav";
    fs::write(project_path.join(audio_path), b"fake wav bytes").expect("write audio");
    attach_beat_audio_reference(
        &project_path,
        "opening-scene",
        "opening-scene-beat-001",
        MediaAssetReference {
            asset_id: None,
            kind: AssetKind::Audio,
            source: AssetSourceKind::Generated,
            project_path: audio_path.into(),
            export_path: audio_path.into(),
            slot: "narration".into(),
        },
    )
    .expect("attach audio reference");
    assert_eq!(
        load_project(&project_path)
            .expect("load project before export")
            .scenes[0]
            .beats[0]
            .audio_refs,
        vec![MediaAssetReference {
            asset_id: None,
            kind: AssetKind::Audio,
            source: AssetSourceKind::Generated,
            project_path: audio_path.into(),
            export_path: audio_path.into(),
            slot: "narration".into(),
        }]
    );

    export_static_web(&project_path, &output_dir).expect("export");

    let manifest: ExportManifest = serde_json::from_str(
        &fs::read_to_string(output_dir.join("game.json")).expect("read manifest"),
    )
    .expect("manifest");
    assert!(
        output_dir
            .join("assets/generated/opening-scene-beat-001.wav")
            .is_file()
    );
    assert!(
        manifest
            .assets
            .contains(&"assets/generated/opening-scene-beat-001.wav".into())
    );
    let audio_record = manifest
        .asset_records
        .iter()
        .find(|record| record.kind == AssetKind::Audio)
        .expect("audio asset record");
    assert_eq!(
        audio_record.export_path,
        "assets/generated/opening-scene-beat-001.wav"
    );
    assert!(audio_record.references.iter().any(|reference| {
        reference.reference_kind == plotforge_schema::AssetReferenceKind::Scene
            && reference.reference_id == "opening-scene"
            && reference.slot == "beat_audio:opening-scene-beat-001:narration"
    }));
    assert_eq!(
        load_project(&project_path)
            .expect("load project")
            .asset_records
            .iter()
            .filter(|record| record.kind == AssetKind::Audio)
            .count(),
        1
    );
}

#[test]
fn export_static_zip_contains_only_audited_package_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    let archive_path = temp.path().join("starter-project-static.zip");
    create_starter_project(&project_path);
    fs::create_dir_all(project_path.join("traces")).expect("traces dir");
    fs::write(project_path.join("traces/latest.json"), "{}").expect("private trace");

    let report =
        export_static_web_zip(&project_path, &output_dir, &archive_path).expect("export zip");

    assert_eq!(
        report.source_report.audit.files_found,
        expected_export_files()
    );
    assert_eq!(report.archived_files, expected_export_files());
    let names = zip_file_names(&archive_path);
    assert_eq!(
        names,
        expected_export_files()
            .into_iter()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>()
    );
    assert!(!names.iter().any(|name| name.starts_with("traces/")));
    assert!(!names.iter().any(|name| name.starts_with("providers/")));
    assert!(!names.iter().any(|name| name.contains("raw_responses")));
    let manifest = zip_file_text(&archive_path, "game.json");
    let usage = zip_file_text(&archive_path, AI_USAGE_MANIFEST_FILE);
    assert_secret_free(&manifest);
    assert_secret_free(&usage);
}

#[test]
fn export_desktop_runtime_draft_writes_local_package_evidence_without_private_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("desktop-export");
    create_starter_project(&project_path);
    fs::create_dir_all(project_path.join("traces")).expect("traces dir");
    fs::create_dir_all(project_path.join("providers")).expect("providers dir");
    fs::create_dir_all(project_path.join("agents/raw_responses")).expect("raw dir");
    fs::write(project_path.join("traces/latest.json"), "{}").expect("private trace");
    fs::write(project_path.join("providers/config.json"), "{}").expect("provider config");
    fs::write(project_path.join("agents/raw_responses/scene.json"), "{}").expect("raw response");

    let report = export_desktop_runtime_draft(&project_path, &output_dir).expect("desktop export");

    let expected_files = expected_desktop_export_files();
    assert_eq!(report.audit.allowed_files, expected_files);
    assert_eq!(report.audit.files_found, expected_files);
    assert!(output_dir.join("index.html").is_file());
    assert!(output_dir.join("game.json").is_file());
    assert!(output_dir.join(AI_USAGE_MANIFEST_FILE).is_file());
    assert!(output_dir.join(DESKTOP_RUNTIME_DRAFT_FILE).is_file());
    assert!(output_dir.join("desktop-build-notes.md").is_file());
    assert!(
        output_dir
            .join("assets/generated/placeholder.png")
            .is_file()
    );
    assert!(!output_dir.join("traces/latest.json").exists());
    assert!(!output_dir.join("providers/config.json").exists());
    assert!(!output_dir.join("agents/raw_responses/scene.json").exists());

    let manifest: ExportManifest = serde_json::from_str(
        &fs::read_to_string(output_dir.join("game.json")).expect("read manifest"),
    )
    .expect("manifest");
    assert_eq!(manifest.profile.id, "desktop-runtime");
    assert!(!manifest.profile.requires_network_at_runtime);
    assert_eq!(manifest.ai_usage_manifest_path, AI_USAGE_MANIFEST_FILE);

    let ai_usage_text = fs::read_to_string(output_dir.join(AI_USAGE_MANIFEST_FILE)).expect("usage");
    let ai_usage: AiUsageManifest = serde_json::from_str(&ai_usage_text).expect("usage json");
    assert_eq!(ai_usage.export_profile.id, "desktop-runtime");
    assert!(!ai_usage.provider_credentials_included);
    assert!(!ai_usage.raw_provider_responses_included);
    assert!(!ai_usage.private_traces_included);
    assert!(ai_usage_text.contains("Desktop runtime draft packages project content"));

    let draft_text =
        fs::read_to_string(output_dir.join(DESKTOP_RUNTIME_DRAFT_FILE)).expect("draft");
    let draft: DesktopRuntimeDraft = serde_json::from_str(&draft_text).expect("draft json");
    assert_eq!(draft.project_id, "project");
    assert_eq!(draft.export_profile.id, "desktop-runtime");
    assert_eq!(draft.static_manifest_path, "game.json");
    assert_eq!(draft.ai_usage_manifest_path, AI_USAGE_MANIFEST_FILE);
    assert_eq!(draft.runtime_entrypoint, "index.html");
    assert!(!draft.requires_network_at_runtime);
    assert!(!draft.provider_credentials_included);
    assert!(!draft.private_traces_included);
    assert!(!draft.raw_provider_responses_included);
    assert!(
        draft
            .build_notes_markdown
            .contains("# Desktop Runtime Draft")
    );
    assert!(
        draft
            .build_notes_markdown
            .contains("does not include a Tauri build")
    );
    assert_eq!(
        draft
            .package_files
            .iter()
            .map(|file| PathBuf::from(&file.path))
            .collect::<Vec<_>>(),
        expected_export_files_with_build_notes()
    );
    for file in &draft.package_files {
        let path = output_dir.join(&file.path);
        let bytes = fs::read(&path).expect("package file");
        assert_eq!(file.hash_algorithm, "sha256");
        assert_eq!(file.byte_length, bytes.len() as u64);
        assert_eq!(file.content_hash.len(), 64);
    }
    assert_secret_free(&draft_text);
    assert!(!draft_text.contains("steam_app_id"));
    assert!(!draft_text.contains("published_file_id"));
}

#[test]
fn export_desktop_runtime_draft_rejects_stale_private_output_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("desktop-export");
    create_starter_project(&project_path);
    fs::create_dir_all(output_dir.join("providers")).expect("output providers");
    fs::write(output_dir.join("providers/config.json"), "{}").expect("stale config");

    let error = export_desktop_runtime_draft(&project_path, &output_dir)
        .expect_err("stale output should fail");

    assert!(matches!(
        error,
        ExportError::DisallowedPackageFile(path)
            if path.as_path() == Path::new("providers/config.json")
    ));
}

#[test]
fn export_static_zip_refuses_to_include_stale_output_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    let archive_path = temp.path().join("starter-project-static.zip");
    create_starter_project(&project_path);
    fs::create_dir_all(output_dir.join("providers")).expect("output providers");
    fs::write(output_dir.join("providers/config.json"), "{}").expect("stale config");

    let error = export_static_web_zip(&project_path, &output_dir, &archive_path)
        .expect_err("stale output should fail");

    assert!(matches!(
        error,
        ExportError::DisallowedPackageFile(path)
            if path.as_path() == Path::new("providers/config.json")
    ));
    assert!(!archive_path.exists());
}

#[test]
fn export_rejects_stale_disallowed_files_in_output_package() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_path = temp.path().join("project");
    let output_dir = temp.path().join("export");
    create_starter_project(&project_path);
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
    create_starter_project(&project_path);
    let scene_path = project_path.join("scenes/opening-scene.scene.json");
    let scene = fs::read_to_string(&scene_path)
        .expect("read scene")
        .replace("assets/generated/placeholder.png", "../traces/latest.json");
    fs::write(&scene_path, scene).expect("write scene");

    let error = export_static_web(&project_path, &output_dir).expect_err("unsafe asset");

    assert!(matches!(
        error,
        ExportError::Media(MediaError::UnsafeAssetPath(_))
    ));
    assert!(!temp.path().join("traces/latest.json").exists());
}

fn create_starter_project(project_path: &Path) {
    create_project_from_request(
        project_path,
        ProjectCreationRequest {
            template: ProjectTemplateId::HistoricalCrisis,
            concept: "A local starter project for export tests.".into(),
            visual_style: "clear readable test style".into(),
            voice_enabled: false,
            initial_scene_request: "A creator opens a fresh PlotForge project.".into(),
        },
        false,
    )
    .expect("create starter project");
}

fn expected_export_files() -> Vec<PathBuf> {
    common::expected_export_files()
}

fn expected_export_files_with_build_notes() -> Vec<PathBuf> {
    common::expected_export_files_with_build_notes()
}

fn expected_desktop_export_files() -> Vec<PathBuf> {
    common::expected_desktop_export_files()
}

fn assert_secret_free(text: &str) {
    assert!(!contains_secret_marker_text(text));
}

fn zip_file_names(archive_path: &Path) -> Vec<String> {
    let archive = fs::File::open(archive_path).expect("open zip");
    let mut zip = zip::ZipArchive::new(archive).expect("read zip");
    let mut names = (0..zip.len())
        .map(|index| zip.by_index(index).expect("zip entry").name().to_string())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn zip_file_text(archive_path: &Path, name: &str) -> String {
    let archive = fs::File::open(archive_path).expect("open zip");
    let mut zip = zip::ZipArchive::new(archive).expect("read zip");
    let mut file = zip.by_name(name).expect("zip entry");
    let mut text = String::new();
    file.read_to_string(&mut text).expect("read zip text");
    text
}
