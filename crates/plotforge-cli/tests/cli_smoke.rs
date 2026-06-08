use std::{fs, process::Command};

use plotforge_schema::{AI_USAGE_MANIFEST_FILE, DESKTOP_RUNTIME_DRAFT_FILE, REDACTED_TRACE_SECRET};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_plotforge-cli")
}

#[test]
fn cli_runs_full_demo_flow_in_tempdir() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    let export = temp.path().join("export");
    let desktop_export = temp.path().join("desktop-export");
    let export_zip = temp.path().join("dynasty-embers-static.zip");
    let unpacked_export = temp.path().join("unpacked-export");

    run([
        "new",
        "demo",
        "--path",
        project.to_str().unwrap(),
        "--force",
    ])
    .assert_success_contains("created Dynasty Embers");
    run(["check", project.to_str().unwrap()]).assert_success_contains("ok: Dynasty Embers");
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
    run(["play", project.to_str().unwrap(), "--once"])
        .assert_success_contains("choice: raise-tax")
        .assert_contains("treasury: +12");
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
    .assert_contains("trace evidence id: trace-001")
    .assert_contains("snapshot evidence id: none")
    .assert_contains("intent: raise_tax")
    .assert_contains("intent choice: raise-tax")
    .assert_contains("intent action: raise_tax")
    .assert_contains("intent matched terms: 加征")
    .assert_contains("rule: raise_tax")
    .assert_contains("rule committed: true")
    .assert_contains("planner: court-crisis-001")
    .assert_contains("planner requested: raise_tax")
    .assert_contains("planner fallback: false")
    .assert_contains("story before: scene=court-crisis-001 beat=court-crisis-001-beat-001 turn=0")
    .assert_contains("story after: scene=court-crisis-001 beat=court-crisis-001-beat-001 turn=1")
    .assert_contains("diagnostics: 5")
    .assert_contains("diagnostic: InterpretAction Completed")
    .assert_contains("media references: 1")
    .assert_contains(
        "media: Scene court-crisis-001 background_asset -> assets/generated/court-crisis-001.png",
    )
    .assert_contains("review scene: court-crisis-001")
    .assert_contains("review issues: 0");
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
    assert!(
        export
            .join("assets/generated/court-crisis-001.png")
            .is_file()
    );
    assert!(export_zip.is_file());
    extract_zip(&export_zip, &unpacked_export);
    assert!(unpacked_export.join("index.html").is_file());
    assert!(unpacked_export.join("game.json").is_file());
    assert!(
        unpacked_export
            .join("assets/generated/court-crisis-001.png")
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
            .join("assets/generated/court-crisis-001.png")
            .is_file()
    );
    assert_export_tree_excludes_private_paths(&desktop_export);
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
fn cli_new_project_rejects_secret_markers() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("winter-regency");

    let output = Command::new(bin())
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
    let project = temp.path().join("dynasty-embers");
    run([
        "new",
        "demo",
        "--path",
        project.to_str().unwrap(),
        "--force",
    ])
    .assert_success_contains("created Dynasty Embers");

    let output = Command::new(bin())
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
    let project = temp.path().join("dynasty-embers");
    run([
        "new",
        "demo",
        "--path",
        project.to_str().unwrap(),
        "--force",
    ])
    .assert_success_contains("created Dynasty Embers");

    let output = Command::new(bin())
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
    let project = temp.path().join("dynasty-embers");
    run([
        "new",
        "demo",
        "--path",
        project.to_str().unwrap(),
        "--force",
    ])
    .assert_success_contains("created Dynasty Embers");

    run([
        "play",
        project.to_str().unwrap(),
        "--once",
        "--input",
        "朕决定加征辽饷 OPENAI_API_KEY=sk-test-secret-marker bearer token=value",
    ])
    .assert_success_contains("choice: raise-tax");

    let trace_path = project.join("traces/latest.json");
    let trace_json = fs::read_to_string(&trace_path).expect("read trace");
    assert!(trace_json.contains(REDACTED_TRACE_SECRET));
    assert!(!trace_json.contains("OPENAI_API_KEY"));
    assert!(!trace_json.contains("sk-test-secret-marker"));
    assert!(!trace_json.contains("token=value"));

    run(["trace", "inspect", trace_path.to_str().unwrap()])
        .assert_success_contains("intent: raise_tax")
        .assert_contains("rule: raise_tax")
        .assert_contains("planner: court-crisis-001");
}

#[test]
fn cli_play_can_save_and_restore_runtime_snapshot() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    run([
        "new",
        "demo",
        "--path",
        project.to_str().unwrap(),
        "--force",
    ])
    .assert_success_contains("created Dynasty Embers");

    run([
        "play",
        project.to_str().unwrap(),
        "--once",
        "--input",
        "朕决定加征辽饷",
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
        "先拨内帑稳住边军军饷",
        "--save-id",
        "save-002",
    ])
    .assert_success_contains("choice: pay-army")
    .assert_contains("army_morale: +12")
    .assert_contains("snapshot:");

    let restored_trace =
        fs::read_to_string(project.join("traces/latest.json")).expect("latest trace");
    assert!(restored_trace.contains("\"turn\": 2"));
    assert!(
        project
            .join("saves/save-002.runtime_snapshot.json")
            .is_file()
    );
}

#[test]
fn cli_play_rejects_corrupted_runtime_snapshot_explicitly() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    run([
        "new",
        "demo",
        "--path",
        project.to_str().unwrap(),
        "--force",
    ])
    .assert_success_contains("created Dynasty Embers");
    fs::write(
        project.join("saves/corrupt.runtime_snapshot.json"),
        "{not-json\n",
    )
    .expect("corrupt save");

    let output = Command::new(bin())
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
    let project = temp.path().join("dynasty-embers");
    run([
        "new",
        "demo",
        "--path",
        project.to_str().unwrap(),
        "--force",
    ])
    .assert_success_contains("created Dynasty Embers");

    let output = Command::new(bin())
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
fn committed_example_fixture_is_cli_valid() {
    let root = repo_root();
    let fixture = root.join("examples/dynasty-embers");

    run(["check", fixture.to_str().unwrap()]).assert_success_contains("ok: Dynasty Embers");
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn run<I, S>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new(bin())
        .args(args)
        .output()
        .expect("run command");
    CommandOutput { output }
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
}

#[test]
fn fixture_trace_directory_stays_generated_only() {
    let root = repo_root();
    let traces = root.join("examples/dynasty-embers/traces");
    let committed_trace_count = if traces.is_dir() {
        fs::read_dir(traces)
            .expect("traces directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            .count()
    } else {
        0
    };
    assert_eq!(committed_trace_count, 0);
}
