use std::{fs, io::Write, process::Command};

use plotforge_schema::{
    AI_USAGE_MANIFEST_FILE, AiUsageContentKind, AiUsageDisclosure, AiUsageManifest,
    AiUsageSourceKind, DESKTOP_RUNTIME_DRAFT_FILE, ExportProfile, REDACTED_TRACE_SECRET,
    SteamSubmissionKitRequest, WORKSHOP_ITEM_MANIFEST_FILE, WorkshopDraftVisibility,
    WorkshopItemPackage, WorkshopPackageFile,
};
use sha2::{Digest, Sha256};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_plotforge-cli")
}

/// Build a CLI `Command` with deterministic English output.
///
/// The CLI resolves its output language from `PLOTFORGE_LANGUAGE`/`LANG` when
/// `--language` is not passed. Without pinning this, test assertions on English
/// output text become flaky across locales.
fn cli() -> Command {
    let mut cmd = Command::new(bin());
    cmd.env("PLOTFORGE_LANGUAGE", "en");
    cmd
}

#[test]
fn cli_runs_full_demo_flow_in_tempdir() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("starter-project");
    let export = temp.path().join("export");
    let desktop_export = temp.path().join("desktop-export");
    let export_zip = temp.path().join("starter-project-static.zip");
    let unpacked_export = temp.path().join("unpacked-export");

    create_starter_project(&project).assert_success_contains("created project Starter Project");
    run(["check", project.to_str().unwrap()]).assert_success_contains("ok: Starter Project");
    run(["export", "profiles"])
        .assert_success_contains("static-web target=static_web")
        .assert_contains("byo-key-web target=dynamic_web requires_network_at_runtime=true")
        .assert_contains("self-host-backend target=dynamic_web requires_network_at_runtime=true")
        .assert_contains("desktop-runtime target=desktop_bundle")
        .assert_contains("steam-workshop target=steam_workshop")
        .assert_contains("steam-submission-kit target=steam_submission_kit")
        .assert_contains("includes_provider_config=false")
        .assert_contains("includes_private_traces=false")
        .assert_contains("platform_submission_ready=false");
    run(["play", project.to_str().unwrap(), "--once"]).assert_success_contains("choice: continue");
    run([
        "trace",
        "inspect",
        project.join("traces/latest.json").to_str().unwrap(),
    ])
    .assert_success_contains("fallback: false")
    .assert_contains("run seed: 7")
    .assert_contains("prompt version: plotforge-local-mock-prompt-v1")
    .assert_contains("model version: plotforge-local-mock-model-v1")
    .assert_contains("provider config hash: sha256:plotforge-local-mock-provider-config-v1")
    .assert_contains("trace evidence id: trace-000")
    .assert_contains("snapshot evidence id: none")
    .assert_contains("intent: continue")
    .assert_contains("intent choice: continue")
    .assert_contains("intent action: continue")
    .assert_contains("intent matched terms: continue")
    .assert_contains("rule: continue")
    .assert_contains("rule committed: true")
    .assert_contains("planner: none")
    .assert_contains("planner requested: continue")
    .assert_contains("planner fallback: false")
    .assert_contains("story before: scene=opening-scene beat=opening-scene-beat-001 turn=0")
    .assert_contains("story after: scene=opening-scene beat=opening-scene-beat-002 turn=0")
    .assert_contains("diagnostics: 5")
    .assert_contains("diagnostic: InterpretAction Completed")
    .assert_contains("media references: 1")
    .assert_contains(
        "media: Scene opening-scene background_asset -> assets/generated/placeholder.png",
    );
    run([
        "export",
        "static",
        project.to_str().unwrap(),
        "--out",
        export.to_str().unwrap(),
        "--zip",
        export_zip.to_str().unwrap(),
    ])
    .assert_success_contains("exported static zip")
    .assert_contains("exported static player");

    assert!(export.join("index.html").is_file());
    assert!(export.join("game.json").is_file());
    assert!(export.join("assets/generated/placeholder.png").is_file());
    assert!(export_zip.is_file());
    extract_zip(&export_zip, &unpacked_export);
    assert!(unpacked_export.join("index.html").is_file());
    assert!(unpacked_export.join("game.json").is_file());
    assert!(
        unpacked_export
            .join("assets/generated/placeholder.png")
            .is_file()
    );
    assert_export_tree_excludes_private_paths(&unpacked_export);

    run([
        "export",
        "desktop",
        project.to_str().unwrap(),
        "--out",
        desktop_export.to_str().unwrap(),
    ])
    .assert_success_contains("exported desktop runtime draft")
    .assert_contains("desktop-runtime-draft.json")
    .assert_contains("desktop build notes:");

    assert!(desktop_export.join("index.html").is_file());
    assert!(desktop_export.join("game.json").is_file());
    assert!(desktop_export.join(AI_USAGE_MANIFEST_FILE).is_file());
    assert!(desktop_export.join(DESKTOP_RUNTIME_DRAFT_FILE).is_file());
    assert!(desktop_export.join("desktop-build-notes.md").is_file());
    assert!(
        desktop_export
            .join("assets/generated/placeholder.png")
            .is_file()
    );
    assert_export_tree_excludes_private_paths(&desktop_export);
}

#[test]
fn cli_runs_workshop_local_flow_in_tempdir() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("workshop-package");
    let library = temp.path().join("workshop-library");
    let publish_out = temp.path().join("publish-draft");
    let kit_out = temp.path().join("submission-kit");
    write_valid_workshop_package(&package);

    run(["workshop", "validate", package.to_str().unwrap()])
        .assert_success_contains("workshop package ok: starter-workshop-draft")
        .assert_contains("files=2");

    run([
        "workshop",
        "import",
        library.to_str().unwrap(),
        package.to_str().unwrap(),
    ])
    .assert_success_contains("imported workshop item starter-workshop-draft")
    .assert_contains("validated imported package");

    run(["workshop", "list", library.to_str().unwrap()])
        .assert_success_contains("workshop library items: 1")
        .assert_contains("starter-workshop-draft title=Starter Project blocked=false reports=0");

    run([
        "workshop",
        "load",
        library.to_str().unwrap(),
        "starter-workshop-draft",
    ])
    .assert_success_contains("loaded workshop item starter-workshop-draft")
    .assert_contains("validated loaded package");

    run([
        "workshop",
        "remix",
        library.to_str().unwrap(),
        "starter-workshop-draft",
        "--new-id",
        "starter-workshop-remix",
        "--title",
        "Starter Workshop Sample Remix",
    ])
    .assert_success_contains(
        "remixed workshop item starter-workshop-draft -> starter-workshop-remix",
    )
    .assert_contains("validated remixed package");

    run([
        "workshop",
        "report",
        library.to_str().unwrap(),
        "starter-workshop-remix",
        "--reason",
        "Needs local creator review.",
    ])
    .assert_success_contains("reported workshop item starter-workshop-remix reports=1");

    run([
        "workshop",
        "block",
        library.to_str().unwrap(),
        "starter-workshop-remix",
        "--reason",
        "Blocked in local library.",
    ])
    .assert_success_contains("blocked workshop item starter-workshop-remix")
    .assert_contains("reason=Blocked in local library.");

    let blocked_load = cli()
        .args([
            "workshop",
            "load",
            library.to_str().unwrap(),
            "starter-workshop-remix",
        ])
        .output()
        .expect("run command");
    assert!(!blocked_load.status.success());
    let stderr = String::from_utf8_lossy(&blocked_load.stderr);
    assert!(stderr.contains("workshop library item is blocked"));

    run([
        "workshop",
        "delete",
        library.to_str().unwrap(),
        "starter-workshop-remix",
    ])
    .assert_success_contains("deleted workshop item starter-workshop-remix");

    run([
        "workshop",
        "publish-draft",
        package.to_str().unwrap(),
        "--out",
        publish_out.to_str().unwrap(),
    ])
    .assert_success_contains("wrote workshop publish draft for starter-workshop-draft")
    .assert_contains("upload_enabled=false steamworks_api_called=false");
    let publish_draft = publish_out.join("workshop-publish-draft.json");
    assert!(publish_draft.is_file());
    let publish_json = fs::read_to_string(&publish_draft).expect("publish draft");
    assert!(publish_json.contains("\"requires_explicit_steamworks_credentials\": true"));
    assert!(!publish_json.contains("steam_app_id"));
    assert!(!publish_json.contains("published_file_id"));

    let disabled_upload = cli()
        .args([
            "workshop",
            "upload-draft",
            package.to_str().unwrap(),
            publish_draft.to_str().unwrap(),
        ])
        .output()
        .expect("run command");
    assert!(!disabled_upload.status.success());
    let stderr = String::from_utf8_lossy(&disabled_upload.stderr);
    assert!(stderr.contains("Steamworks upload is disabled"));

    let request = sample_submission_kit_request();
    run([
        "workshop",
        "submission-kit",
        package.to_str().unwrap(),
        "--batch",
        "--out",
        kit_out.to_str().unwrap(),
        "--product-name",
        &request.product_name,
        "--desktop-build-path",
        request.desktop_build_path.as_deref().unwrap(),
        "--store-short-description",
        &request.store_short_description,
        "--screenshot",
        &request.screenshot_paths[0],
        "--capsule-asset",
        &request.capsule_asset_paths[0],
        "--content-warning",
        &request.content_warnings[0],
        "--safety-guardrail",
        &request.safety_guardrails[0],
        "--user-reporting-path",
        &request.user_reporting_path,
        "--moderation-policy",
        &request.moderation_policy,
        "--build-note",
        &request.build_notes[0],
    ])
    .assert_success_contains("wrote Steam Submission Kit drafts for starter-workshop-draft")
    .assert_contains("(8 files)");

    for file in [
        "steam-store-copy-draft.md",
        "steam-submission-checklist.md",
        "steam-ai-disclosure-draft.md",
        "steam-content-warnings.md",
        "steam-asset-references.md",
        "steam-direct-checklist.md",
        "steam-content-safety-checklist.md",
        "steam-packaging-notes.md",
    ] {
        assert!(kit_out.join(file).is_file(), "missing {file}");
    }
}

#[test]
fn cli_submission_kit_wizard_requires_tty_without_batch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("workshop-package");
    write_valid_workshop_package(&package);

    let output = cli()
        .args(["workshop", "submission-kit", package.to_str().unwrap()])
        .output()
        .expect("run command");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("interactive wizard requires a TTY"),
        "expected TTY guard error, got: {stderr}"
    );
}

#[test]
fn cli_creates_project_from_wizard_fields_and_reopens_it() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("winter-regency");

    run([
        "new",
        "project",
        "--path",
        project.to_str().unwrap(),
        "--template",
        "historical-crisis",
        "--concept",
        "A regency court must survive a winter coup.",
        "--visual-style",
        "ink wash court drama",
        "--voice-enabled",
        "--initial-scene",
        "Open on an empty granary ledger.",
    ])
    .assert_success_contains("created project Winter Regency");

    run(["check", project.to_str().unwrap()]).assert_success_contains("ok: Winter Regency");

    let world = fs::read_to_string(project.join("world/world.md")).expect("world bible");
    let story = fs::read_to_string(project.join("story/story_bible.md")).expect("story bible");
    let style = fs::read_to_string(project.join("story/style_guide.md")).expect("style guide");
    assert!(world.contains("A regency court must survive a winter coup."));
    assert!(story.contains("Open on an empty granary ledger."));
    assert!(style.contains("ink wash court drama"));
    assert!(style.contains("Voice generation requested"));
}

#[test]
fn cli_studio_json_invokes_real_studio_commands() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");

    let check = run_with_stdin(
        ["studio", "check_project"],
        &serde_json::json!({ "path": project.to_string_lossy() }).to_string(),
    )
    .assert_success_contains("\"title\":\"Starter Project\"")
    .stdout_json();
    assert_eq!(check["entry_scene"], "opening-scene");
    assert_eq!(check["scene_count"], 1);

    let play = run_with_stdin(
        ["studio", "play_once_project"],
        &serde_json::json!({
            "path": check_project_path(&temp).to_string_lossy(),
            "player_input": "continue"
        })
        .to_string(),
    )
    .assert_success_contains("\"trace_path\"")
    .stdout_json();
    assert_eq!(play["scene"]["key"], "opening-scene");
    assert_eq!(play["trace"]["id"], "trace-000");
    assert_eq!(play["trace"]["selected_choice"], "continue");
    assert!(
        check_project_path(&temp)
            .join("traces/latest.json")
            .is_file()
    );
}

#[test]
fn cli_studio_pi_agent_run_returns_redaction_safe_envelope() {
    let result = run_with_stdin(
        ["studio", "pi_agent_run"],
        &serde_json::json!({
            "request": {
                "agent_id": "pi-agent-local",
                "run_seed": 7,
                "prompt_summary": "Generate a validated scene plan proposal.",
                "prompt_hash": "sha256:cli-smoke-prompt"
            }
        })
        .to_string(),
    )
    .assert_success_contains("\"is_local_pi\":true")
    .stdout_json();

    // The descriptor must identify the local pi-Agent and expose capabilities.
    assert_eq!(result["descriptor"]["agent_id"], "pi-agent-local");
    assert_eq!(result["descriptor"]["is_local_pi"], true);
    assert!(
        result["descriptor"]["capabilities"].is_array(),
        "capabilities must be a list"
    );
    assert!(
        !result["descriptor"]["capabilities"]
            .as_array()
            .unwrap()
            .is_empty(),
        "capabilities must not be empty"
    );

    // Reproducibility metadata must be present and complete.
    assert_eq!(result["reproducibility"]["run_seed"], 7);
    assert!(
        result["reproducibility"]["provider_config_hash"]
            .as_str()
            .unwrap()
            .starts_with("sha256:"),
        "provider_config_hash must be present"
    );
    assert!(
        !result["reproducibility"]["prompt_version"].as_str().unwrap().is_empty(),
        "prompt_version must be present"
    );
    assert!(
        !result["reproducibility"]["model_version"].as_str().unwrap().is_empty(),
        "model_version must be present"
    );

    // The serialized output must be redaction-safe: no raw provider responses
    // or secret markers anywhere in the envelope.
    let raw_run = run_with_stdin(
        ["studio", "pi_agent_run"],
        &serde_json::json!({
            "request": {
                "agent_id": "pi-agent-local",
                "run_seed": 7,
                "prompt_summary": "Generate a validated scene plan proposal.",
                "prompt_hash": "sha256:cli-smoke-prompt"
            }
        })
        .to_string(),
    );
    let stdout = String::from_utf8_lossy(&raw_run.output.stdout);
    assert!(!stdout.contains("raw_provider_response"));
    assert!(!stdout.contains("api_key"));
    assert!(!stdout.contains("sk-"));
    assert!(!stdout.contains("OPENAI_API_KEY"));
}

#[test]
fn cli_studio_pi_agent_capabilities_lists_wired_capability() {
    let capabilities = run_with_stdin(
        ["studio", "pi_agent_capabilities"],
        &serde_json::json!({}).to_string(),
    )
    .assert_success_contains("\"status\":\"wired\"")
    .stdout_json();

    assert!(capabilities.is_array(), "capabilities must be a list");
    let list = capabilities.as_array().unwrap();
    assert!(!list.is_empty(), "capabilities must not be empty");
    let wired = list
        .iter()
        .filter(|capability| capability["status"] == "wired")
        .count();
    assert_ne!(wired, 0, "at least one capability should be wired");

    // Capability evidence must be redaction-safe.
    let raw_capabilities = run_with_stdin(
        ["studio", "pi_agent_capabilities"],
        &serde_json::json!({}).to_string(),
    );
    let stdout = String::from_utf8_lossy(&raw_capabilities.output.stdout);
    assert!(!stdout.contains("sk-"));
    assert!(!stdout.contains("OPENAI_API_KEY"));
}

#[test]
fn cli_new_project_rejects_secret_markers() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("winter-regency");

    let output = cli()
        .args([
            "new",
            "project",
            "--path",
            project.to_str().unwrap(),
            "--concept",
            "A court drama OPENAI_API_KEY=sk-test-secret-marker",
            "--visual-style",
            "ink wash",
            "--initial-scene",
            "Open in court.",
        ])
        .output()
        .expect("run command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid project creation request field concept"));
    assert!(!project.join("game.toml").exists());
}

#[test]
fn cli_rejects_interactive_play_for_now() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");

    let output = cli()
        .args(["play", project.to_str().unwrap()])
        .output()
        .expect("run command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("interactive play is not implemented"));
}

#[test]
fn cli_rejects_unsupported_play_input_without_trace() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");

    let output = cli()
        .args([
            "play",
            project.to_str().unwrap(),
            "--once",
            "--input",
            "朕今日只想题诗赏月",
        ])
        .output()
        .expect("run command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unsupported player action"));
    assert!(!project.join("traces/latest.json").exists());
}

#[test]
fn cli_trace_redacts_secret_markers_from_play_input() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");

    run([
        "play",
        project.to_str().unwrap(),
        "--once",
        "--input",
        "continue OPENAI_API_KEY=sk-test-secret-marker bearer token=value",
    ])
    .assert_success_contains("choice: continue");

    let trace_path = project.join("traces/latest.json");
    let trace_json = fs::read_to_string(&trace_path).expect("read trace");
    assert!(trace_json.contains(REDACTED_TRACE_SECRET));
    assert!(!trace_json.contains("OPENAI_API_KEY"));
    assert!(!trace_json.contains("sk-test-secret-marker"));
    assert!(!trace_json.contains("token=value"));

    run(["trace", "inspect", trace_path.to_str().unwrap()])
        .assert_success_contains("intent: continue")
        .assert_contains("rule: continue")
        .assert_contains("planner: none");
}

#[test]
fn cli_play_can_save_and_restore_runtime_snapshot() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");

    run([
        "play",
        project.to_str().unwrap(),
        "--once",
        "--input",
        "continue",
        "--save-id",
        "save-001",
    ])
    .assert_success_contains("snapshot:")
    .assert_contains("save-001.runtime_snapshot.json");
    assert!(
        project
            .join("saves/save-001.runtime_snapshot.json")
            .is_file()
    );
    assert!(project.join("saves/latest.runtime_snapshot.json").is_file());

    run([
        "play",
        project.to_str().unwrap(),
        "--once",
        "--restore-id",
        "save-001",
        "--input",
        "continue",
        "--save-id",
        "save-002",
    ])
    .assert_success_contains("choice: continue")
    .assert_contains("snapshot:");

    let restored_trace =
        fs::read_to_string(project.join("traces/latest.json")).expect("latest trace");
    assert!(restored_trace.contains("\"current_beat_id\": \"opening-scene-beat-003\""));
    assert!(
        project
            .join("saves/save-002.runtime_snapshot.json")
            .is_file()
    );
}

#[test]
fn cli_play_rejects_corrupted_runtime_snapshot_explicitly() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");
    fs::write(
        project.join("saves/corrupt.runtime_snapshot.json"),
        "{not-json\n",
    )
    .expect("corrupt save");

    let output = cli()
        .args([
            "play",
            project.to_str().unwrap(),
            "--once",
            "--restore-id",
            "corrupt",
        ])
        .output()
        .expect("run command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("read runtime snapshot `corrupt`"));
    assert!(stderr.contains("json error"));
    assert!(!project.join("traces/latest.json").exists());
}

#[test]
fn cli_play_rejects_invalid_save_id_before_trace_write() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");

    let output = cli()
        .args([
            "play",
            project.to_str().unwrap(),
            "--once",
            "--save-id",
            "../escape",
        ])
        .output()
        .expect("run command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid runtime snapshot id"));
    assert!(!project.join("traces/latest.json").exists());
    assert!(!project.join("saves/latest.runtime_snapshot.json").exists());
}

#[test]
fn cli_check_supports_chinese_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");

    run(["--language", "zh", "check", project.to_str().unwrap()])
        .assert_success_contains("通过：Starter Project")
        .assert_contains("1 个场景");
}

fn run<I, S>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = cli().args(args).output().expect("run command");
    CommandOutput { output }
}

fn run_with_stdin<I, S>(args: I, stdin: &str) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut child = cli()
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn command");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait command");
    CommandOutput { output }
}

fn check_project_path(temp: &tempfile::TempDir) -> std::path::PathBuf {
    temp.path().join("starter-project")
}

fn create_starter_project(project: &std::path::Path) -> CommandOutput {
    run([
        "new",
        "project",
        "--path",
        project.to_str().unwrap(),
        "--force",
        "--concept",
        "A local starter project for tests.",
        "--visual-style",
        "clear readable test style",
        "--initial-scene",
        "A creator opens a fresh PlotForge project.",
    ])
}

fn extract_zip(archive_path: &std::path::Path, output_dir: &std::path::Path) {
    fs::create_dir_all(output_dir).expect("unpack dir");
    let status = Command::new("unzip")
        .args([
            "-q",
            archive_path.to_str().unwrap(),
            "-d",
            output_dir.to_str().unwrap(),
        ])
        .status()
        .expect("unzip command");
    assert!(status.success(), "unzip failed with {status:?}");
}

fn assert_export_tree_excludes_private_paths(export_dir: &std::path::Path) {
    assert!(!export_dir.join("traces").exists());
    assert!(!export_dir.join("providers").exists());
    assert!(
        !export_tree_paths(export_dir)
            .iter()
            .any(|path| path.contains("raw_responses")),
        "export contains raw_responses path"
    );
}

fn export_tree_paths(root: &std::path::Path) -> Vec<String> {
    let mut paths = Vec::new();
    collect_export_tree_paths(root, root, &mut paths);
    paths
}

fn collect_export_tree_paths(
    root: &std::path::Path,
    current: &std::path::Path,
    paths: &mut Vec<String>,
) {
    for entry in fs::read_dir(current).expect("read export dir") {
        let entry = entry.expect("export entry");
        let path = entry.path();
        let relative_path = path.strip_prefix(root).expect("relative export path");
        paths.push(relative_path.to_string_lossy().replace('\\', "/"));
        if path.is_dir() {
            collect_export_tree_paths(root, &path, paths);
        }
    }
}

fn write_valid_workshop_package(package_dir: &std::path::Path) {
    let game = br#"{"title":"Starter Project"}"#;
    let preview = b"preview-image\n";

    fs::create_dir_all(package_dir.join("content")).expect("content dir");
    fs::write(package_dir.join("content/game.json"), game).expect("game json");
    fs::write(package_dir.join("preview.png"), preview).expect("preview");
    fs::write(
        package_dir.join(AI_USAGE_MANIFEST_FILE),
        serde_json::to_string_pretty(&sample_ai_usage_manifest()).expect("ai usage json") + "\n",
    )
    .expect("ai usage manifest");
    fs::write(
        package_dir.join(WORKSHOP_ITEM_MANIFEST_FILE),
        serde_json::to_string_pretty(&sample_workshop_package(game, preview))
            .expect("workshop manifest json")
            + "\n",
    )
    .expect("workshop manifest");
}

fn sample_workshop_package(game: &[u8], preview: &[u8]) -> WorkshopItemPackage {
    WorkshopItemPackage {
        manifest_version: "2026-06-09".into(),
        package_id: "starter-workshop-draft".into(),
        title: "Starter Project".into(),
        description: "Offline Workshop package draft for local validation.".into(),
        visibility: WorkshopDraftVisibility::PrivateDraft,
        preview_image: "preview.png".into(),
        content_root: "content".into(),
        tags: vec!["story-game".into(), "strategy".into()],
        export_profile: ExportProfile::steam_workshop(),
        ai_usage_manifest_path: AI_USAGE_MANIFEST_FILE.into(),
        content_files: vec![
            workshop_file_record("content/game.json", game),
            workshop_file_record("preview.png", preview),
        ],
        notices: vec![
            "Local package validation only; no external platform action is included.".into(),
            "Creator review remains responsible for distribution decisions.".into(),
        ],
    }
}

fn sample_ai_usage_manifest() -> AiUsageManifest {
    AiUsageManifest {
        manifest_version: "2026-06-09".into(),
        project_id: "starter-project".into(),
        project_version: "0.1.0".into(),
        export_profile: ExportProfile::steam_workshop(),
        generated_by: "plotforge-cli-smoke-test".into(),
        external_model_calls_during_export: false,
        provider_credentials_included: false,
        raw_provider_responses_included: false,
        private_traces_included: false,
        disclosures: vec![
            AiUsageDisclosure {
                content_kind: AiUsageContentKind::Text,
                source_kind: AiUsageSourceKind::ProjectSource,
                summary: "Story text is exported from canonical project source files.".into(),
                asset_paths: Vec::new(),
            },
            AiUsageDisclosure {
                content_kind: AiUsageContentKind::Image,
                source_kind: AiUsageSourceKind::LocalMockProvider,
                summary: "Preview art is represented by a local test asset.".into(),
                asset_paths: vec!["preview.png".into()],
            },
        ],
        provider_summaries: Vec::new(),
        ai_safety_policy: Default::default(),
        notices: vec!["No provider credentials or raw provider responses included.".into()],
    }
}

fn sample_submission_kit_request() -> SteamSubmissionKitRequest {
    SteamSubmissionKitRequest {
        product_name: "Starter Project".into(),
        desktop_build_path: Some("builds/starter-project-desktop.zip".into()),
        store_short_description: "A branching civic drama built with PlotForge.".into(),
        screenshot_paths: vec!["media/screenshots/civic-crisis.png".into()],
        capsule_asset_paths: vec!["media/capsules/header.png".into()],
        content_warnings: vec!["Political conflict".into()],
        safety_guardrails: vec![
            "Keep provider-backed runtime services disabled for this draft.".into(),
        ],
        user_reporting_path: "support@example.invalid".into(),
        moderation_policy: "Human review of player-visible text and images before distribution."
            .into(),
        build_notes: vec!["Test launch, save-data creation, and offline play.".into()],
    }
}

fn workshop_file_record(path: impl Into<String>, bytes: &[u8]) -> WorkshopPackageFile {
    WorkshopPackageFile {
        path: path.into(),
        content_hash: sha256_hex(bytes),
        hash_algorithm: "sha256".into(),
        byte_length: bytes.len() as u64,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

struct CommandOutput {
    output: std::process::Output,
}

impl CommandOutput {
    fn assert_success_contains(self, expected: &str) -> Self {
        assert!(
            self.output.status.success(),
            "expected success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
            self.output.status.code(),
            String::from_utf8_lossy(&self.output.stdout),
            String::from_utf8_lossy(&self.output.stderr)
        );
        self.assert_contains(expected)
    }

    fn assert_contains(self, expected: &str) -> Self {
        let stdout = String::from_utf8_lossy(&self.output.stdout);
        let stderr = String::from_utf8_lossy(&self.output.stderr);
        assert!(
            stdout.contains(expected) || stderr.contains(expected),
            "missing `{expected}`\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        self
    }

    fn stdout_json(self) -> serde_json::Value {
        assert!(
            self.output.status.success(),
            "expected success before parsing json, got status {:?}\nstdout:\n{}\nstderr:\n{}",
            self.output.status.code(),
            String::from_utf8_lossy(&self.output.stdout),
            String::from_utf8_lossy(&self.output.stderr)
        );
        serde_json::from_slice(&self.output.stdout).expect("stdout json")
    }
}
