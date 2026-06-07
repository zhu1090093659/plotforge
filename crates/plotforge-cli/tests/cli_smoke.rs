use std::{fs, process::Command};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_plotforge-cli")
}

#[test]
fn cli_runs_full_demo_flow_in_tempdir() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    let export = temp.path().join("export");

    run([
        "new",
        "demo",
        "--path",
        project.to_str().unwrap(),
        "--force",
    ])
    .assert_success_contains("created Dynasty Embers");
    run(["check", project.to_str().unwrap()]).assert_success_contains("ok: Dynasty Embers");
    run(["play", project.to_str().unwrap(), "--once"])
        .assert_success_contains("choice: raise-tax")
        .assert_contains("treasury: +12");
    run([
        "trace",
        "inspect",
        project.join("traces/latest.json").to_str().unwrap(),
    ])
    .assert_success_contains("fallback: false")
    .assert_contains("review issues: 0");
    run([
        "export",
        "static",
        project.to_str().unwrap(),
        "--out",
        export.to_str().unwrap(),
    ])
    .assert_success_contains("exported static player");

    assert!(export.join("index.html").is_file());
    assert!(export.join("game.json").is_file());
    assert!(
        export
            .join("assets/generated/court-crisis-001.png")
            .is_file()
    );
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
