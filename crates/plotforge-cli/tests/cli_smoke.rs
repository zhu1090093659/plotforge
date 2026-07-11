use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

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
fn cli_trace_inspect_reports_reproducibility_and_intent() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");

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
}

#[test]
fn cli_export_workflows_produce_audited_packages() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    let export = temp.path().join("export");
    let desktop_export = temp.path().join("desktop-export");
    let export_zip = temp.path().join("starter-project-static.zip");
    let unpacked_export = temp.path().join("unpacked-export");
    create_starter_project(&project).assert_success_contains("created project Starter Project");

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
fn cli_workshop_validate_and_import() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("workshop-package");
    let library = temp.path().join("workshop-library");
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
}

#[test]
fn cli_workshop_library_list_load_remix() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("workshop-package");
    let library = temp.path().join("workshop-library");
    write_valid_workshop_package(&package);
    import_workshop_package(&library, &package);

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
}

#[test]
fn cli_workshop_report_block_delete() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("workshop-package");
    let library = temp.path().join("workshop-library");
    write_valid_workshop_package(&package);
    import_workshop_package(&library, &package);
    remix_workshop_item(&library, "starter-workshop-draft", "starter-workshop-remix");

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
}

#[test]
fn cli_workshop_publish_draft_and_submission_kit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("workshop-package");
    let publish_out = temp.path().join("publish-draft");
    let kit_out = temp.path().join("submission-kit");
    write_valid_workshop_package(&package);

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
        !result["reproducibility"]["prompt_version"]
            .as_str()
            .unwrap()
            .is_empty(),
        "prompt_version must be present"
    );
    assert!(
        !result["reproducibility"]["model_version"]
            .as_str()
            .unwrap()
            .is_empty(),
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

// ---------------------------------------------------------------------------
// Black-box smoke tests for the studio command groups added when the agent
// configuration, provider registry, prompt template, and skill library
// commands were wired into the CLI dispatcher. Per AGENTS.md these are split
// by command group so a failure in one group does not block the rest, and
// each asserts "call succeeded + output shape" (exit code, JSON parses,
// expected field present) — not the business results already covered by the
// Rust unit tests in `plotforge-studio` / `plotforge-agent` /
// `plotforge-storage`.
//
// User-global state (`~/.plotforge/providers.json`, `~/.plotforge/prompts.json`,
// `~/.plotforge/skill-index.json`, and the external skill scan roots) is
// redirected to a temp `HOME` via `cli_with_home` so these tests never touch
// the real `~/.plotforge/`. See `cli_with_home` for the redirect rationale.
// ---------------------------------------------------------------------------

#[test]
fn cli_studio_pi_agent_apply_run_commits_local_pi_scene_plan() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");

    // The default per-project agent config has model_id="local-pi", so the
    // pi-Agent falls back to the deterministic mock provider (no network, no
    // credential). The request carries the project path + player input; the
    // CLI parses `request` out of the JSON payload and delegates to
    // `pi_agent_apply_run`.
    let result = run_with_stdin(
        ["studio", "pi_agent_apply_run"],
        &serde_json::json!({
            "request": {
                "agent_id": "pi-agent-local",
                "run_seed": 7,
                "project_path": project.to_string_lossy(),
                "player_input": "continue",
            }
        })
        .to_string(),
    )
    .assert_success_contains("\"is_local_pi\":true")
    .stdout_json();

    // Shape only: the envelope echoes the committed scene + trace alongside the
    // redaction-safe pi-Agent run evidence. Business correctness of the
    // committed beat/turn lives in `plotforge-runtime` unit tests.
    assert_eq!(result["run"]["descriptor"]["agent_id"], "pi-agent-local");
    assert_eq!(result["run"]["descriptor"]["is_local_pi"], true);
    assert!(
        result["scene_key"].as_str().is_some_and(|s| !s.is_empty()),
        "scene_key must be present and non-empty"
    );
    assert!(
        result["scene"]["key"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "scene.key must be present and non-empty"
    );
    assert!(
        result["trace"]["id"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "trace.id must be present and non-empty"
    );
    assert!(
        result["trace_path"].as_str().is_some_and(|s| !s.is_empty()),
        "trace_path must be present and non-empty"
    );
    // The committed trace must be written under the temp project (never the
    // real home), proving the dispatch wired the storage layer end to end.
    assert!(project.join("traces/latest.json").is_file());

    // Redaction safety: no secret markers leak into the serialized envelope.
    let raw = run_with_stdin(
        ["studio", "pi_agent_apply_run"],
        &serde_json::json!({
            "request": {
                "agent_id": "pi-agent-local",
                "run_seed": 7,
                "project_path": project.to_string_lossy(),
                "player_input": "continue",
            }
        })
        .to_string(),
    );
    let stdout = String::from_utf8_lossy(&raw.output.stdout);
    assert!(!stdout.contains("raw_provider_response"));
    assert!(!stdout.contains("api_key"));
    assert!(!stdout.contains("sk-"));
    assert!(!stdout.contains("OPENAI_API_KEY"));
}

#[test]
fn cli_studio_pi_agent_apply_run_blocks_flagged_input_before_text_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = hermetic_home();
    let project = check_project_path(&temp);
    create_starter_project_with_home(&home.home, &project)
        .assert_success_contains("created project Starter Project");

    let moderation = MockHttpServer::new(json_http_response(&serde_json::json!({
        "id": "moderation-flagged",
        "model": "omni-moderation-test",
        "results": [{
            "flagged": true,
            "categories": {"violence": true},
            "category_scores": {}
        }]
    })));
    let text = MockHttpServer::new(json_http_response(&openai_scene_plan_body()));
    configure_real_apply(
        &home.home,
        &project,
        &format!("http://{}/v1", moderation.addr()),
        &format!("http://{}/v1", text.addr()),
    );

    let output = run_with_stdin_home(
        &home.home,
        ["studio", "pi_agent_apply_run"],
        &pi_agent_apply_payload(&project, "blocked player input"),
    );
    assert!(
        !output.output.status.success(),
        "flagged apply must fail explicitly"
    );
    let stderr = String::from_utf8_lossy(&output.output.stderr);
    assert!(
        stderr.contains("pi_agent_moderation_flagged"),
        "expected moderation error code, got: {stderr}"
    );
    assert_eq!(moderation.request_count(), 1);
    assert_eq!(
        text.request_count(),
        0,
        "flagged input must never reach the text provider"
    );
}

#[test]
fn cli_studio_pi_agent_apply_run_persists_passing_moderation_hash() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = hermetic_home();
    let project = check_project_path(&temp);
    create_starter_project_with_home(&home.home, &project)
        .assert_success_contains("created project Starter Project");

    let moderation = MockHttpServer::new(json_http_response(&serde_json::json!({
        "id": "moderation-pass",
        "model": "omni-moderation-test",
        "results": [{
            "flagged": false,
            "categories": {"violence": false},
            "category_scores": {}
        }]
    })));
    let text = MockHttpServer::new(json_http_response(&openai_scene_plan_body()));
    configure_real_apply(
        &home.home,
        &project,
        &format!("http://{}/v1", moderation.addr()),
        &format!("http://{}/v1", text.addr()),
    );

    let result = run_with_stdin_home(
        &home.home,
        ["studio", "pi_agent_apply_run"],
        &pi_agent_apply_payload(&project, "safe player input"),
    )
    .stdout_json();

    assert_eq!(moderation.request_count(), 1);
    assert_eq!(text.request_count(), 1);
    assert_eq!(result["moderation_outcome"]["flagged"], false);
    let moderation_hash = result["run"]["reproducibility"]["moderation_config_hash"]
        .as_str()
        .filter(|hash| !hash.is_empty())
        .expect("run moderation hash");
    assert_eq!(
        result["trace"]["reproducibility"]["moderation_config_hash"],
        moderation_hash
    );

    let persisted_trace: serde_json::Value = serde_json::from_slice(
        &fs::read(project.join("traces/latest.json")).expect("persisted latest trace"),
    )
    .expect("persisted trace JSON");
    assert_eq!(
        persisted_trace["reproducibility"]["moderation_config_hash"],
        moderation_hash
    );
}

#[test]
fn cli_studio_lists_providers_on_fresh_home() {
    // `list_providers` reads the user-global registry. A fresh install has no
    // `providers.json`, so the list must be an empty JSON array (not an error).
    // Running against a temp HOME keeps the real registry untouched.
    let home = hermetic_home();
    let providers = run_with_stdin_home(
        &home.home,
        ["studio", "list_providers"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    assert!(
        providers.is_array(),
        "list_providers must return a JSON array"
    );
    assert!(
        providers.as_array().unwrap().is_empty(),
        "fresh home must have no providers"
    );
}

#[test]
fn cli_usage_command_group_reports_empty_fresh_home() {
    let home = hermetic_home();

    run_with_home(&home.home, ["usage", "summary"])
        .assert_success_contains("usage summary")
        .assert_contains("input tokens: 0")
        .assert_contains("cost units: 0")
        .assert_contains("no usage recorded");

    let summary = run_with_home(&home.home, ["usage", "summary", "--json"])
        .assert_no_ansi()
        .stdout_json();
    assert_eq!(summary["total_input_tokens"], 0);
    assert_eq!(summary["total_output_tokens"], 0);
    assert_eq!(summary["total_spent_cost_units"], 0);
    assert!(summary["by_provider"].as_object().unwrap().is_empty());

    run_with_home(&home.home, ["usage", "provider", "--id", "provider-a"])
        .assert_success_contains("provider usage: provider-a")
        .assert_contains("text calls: 0")
        .assert_contains("moderation calls: 0")
        .assert_contains("cost units: 0");

    let provider = run_with_home(
        &home.home,
        ["usage", "provider", "--id", "provider-a", "--json"],
    )
    .assert_no_ansi()
    .stdout_json();
    assert_eq!(provider["provider_id"], "provider-a");
    assert_eq!(provider["text_calls"], 0);
    assert_eq!(provider["moderation_calls"], 0);
    assert_eq!(provider["spent_cost_units"], 0);
}

#[test]
fn cli_studio_usage_dispatch_returns_contract_shapes() {
    let home = hermetic_home();

    let summary = run_with_stdin_home(
        &home.home,
        ["studio", "get_usage_summary"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    assert_eq!(summary["total_input_tokens"], 0);
    assert!(summary["by_provider"].is_object());

    let report = run_with_stdin_home(
        &home.home,
        ["studio", "get_provider_cost_report"],
        &serde_json::json!({ "provider_id": "provider-a" }).to_string(),
    )
    .stdout_json();
    assert_eq!(report["provider_id"], "provider-a");
    assert_eq!(report["moderation_calls"], 0);
    assert_eq!(report["spent_cost_units"], 0);
}

#[test]
fn cli_studio_provider_crud_round_trip_against_temp_home() {
    // upsert/delete/test_provider_connection all mutate the user-global
    // registry. Pointing HOME at a temp dir keeps every write under that temp
    // dir, so the real `~/.plotforge/providers.json` is never touched. The
    // provider is created `enabled: false` so `test_provider_connection` never
    // issues a real HTTP call — it returns a redaction-safe `ok: false`
    // envelope instead, keeping the smoke test offline and deterministic.
    let home = hermetic_home();

    let entry = run_with_stdin_home(
        &home.home,
        ["studio", "upsert_provider"],
        &serde_json::json!({
            "entry": {
                "id": "cli-smoke-prov",
                "kind": "openai_compatible",
                "label": "CLI Smoke Provider",
                "endpoint_url": "https://example.invalid/v1",
                "model": "cli-smoke-model",
                "credential_env_var": "CLI_SMOKE_UNUSED_KEY",
                "enabled": false,
            }
        })
        .to_string(),
    )
    .stdout_json();
    assert_eq!(entry["id"], "cli-smoke-prov");
    assert_eq!(entry["enabled"], false);

    let listed = run_with_stdin_home(
        &home.home,
        ["studio", "list_providers"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    let list = listed.as_array().expect("providers list");
    assert_eq!(list.len(), 1, "upserted provider must appear in the list");
    assert_eq!(list[0]["id"], "cli-smoke-prov");

    // test_provider_connection against a disabled provider returns a
    // redaction-safe `ok: false` without making any network call, so exit 0 +
    // JSON shape is the right smoke assertion.
    let test = run_with_stdin_home(
        &home.home,
        ["studio", "test_provider_connection"],
        &serde_json::json!({ "id": "cli-smoke-prov" }).to_string(),
    )
    .stdout_json();
    assert_eq!(test["ok"], false);
    assert!(
        test["message"].as_str().is_some_and(|s| !s.is_empty()),
        "test_provider_connection must carry a redaction-safe message"
    );

    let removed = run_with_stdin_home(
        &home.home,
        ["studio", "delete_provider"],
        &serde_json::json!({ "id": "cli-smoke-prov" }).to_string(),
    )
    .stdout_json();
    assert_eq!(removed["id"], "cli-smoke-prov");

    let after_delete = run_with_stdin_home(
        &home.home,
        ["studio", "list_providers"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    assert!(
        after_delete.as_array().unwrap().is_empty(),
        "registry must be empty after delete"
    );
}

#[test]
fn cli_studio_delete_provider_reports_missing_id_explicitly() {
    // Per AGENTS.md "no silent fallback": deleting an unknown id must surface
    // an explicit `provider_not_found` error rather than no-op. Asserting the
    // error path here confirms the dispatch surfaces studio errors (exit
    // non-zero + code in stderr) without a real registry fixture.
    let home = hermetic_home();
    let output = run_with_stdin_home(
        &home.home,
        ["studio", "delete_provider"],
        &serde_json::json!({ "id": "does-not-exist" }).to_string(),
    );
    assert!(!output.output.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&output.output.stderr);
    assert!(
        stderr.contains("provider_not_found"),
        "expected provider_not_found error, got: {stderr}"
    );
}

#[test]
fn cli_studio_image_provider_commands_round_trip_against_temp_home() {
    let home = hermetic_home();
    let entry = run_with_stdin_home(
        &home.home,
        ["studio", "upsert_image_provider"],
        &serde_json::json!({
            "entry": {
                "id": "cli-smoke-image",
                "endpoint_url": "https://example.invalid/v1",
                "model": "gpt-image-test",
                "credential_env_var": "PLOTFORGE_CLI_SMOKE_IMAGE_KEY_9F3C7A",
                "enabled": false,
                "default_size": "256x256",
                "default_quality": "low"
            }
        })
        .to_string(),
    )
    .stdout_json();
    assert!(entry.is_object(), "upsert must return a JSON object");
    assert!(
        entry["id"].is_string(),
        "upsert object must carry string id"
    );

    let listed = run_with_stdin_home(
        &home.home,
        ["studio", "list_image_providers"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    assert!(listed.is_array(), "list must return a JSON array");

    let tested = run_with_stdin_home(
        &home.home,
        ["studio", "test_image_provider"],
        &serde_json::json!({ "id": "cli-smoke-image" }).to_string(),
    )
    .stdout_json();
    assert!(tested.is_object(), "test must return a JSON object");
    assert!(tested["ok"].is_boolean(), "test object must carry bool ok");
    assert!(
        tested["message"].is_string(),
        "test object must carry string message"
    );

    let removed = run_with_stdin_home(
        &home.home,
        ["studio", "delete_image_provider"],
        &serde_json::json!({ "id": "cli-smoke-image" }).to_string(),
    )
    .stdout_json();
    assert!(removed.is_object(), "delete must return a JSON object");
    assert!(
        removed["id"].is_string(),
        "delete object must carry string id"
    );
}

#[test]
fn cli_studio_tts_provider_commands_round_trip_against_temp_home() {
    let home = hermetic_home();
    let entry = run_with_stdin_home(
        &home.home,
        ["studio", "upsert_tts_provider"],
        &serde_json::json!({
            "entry": {
                "id": "cli-smoke-tts",
                "endpoint_url": "https://example.invalid/v1",
                "model": "gpt-tts-test",
                "credential_env_var": "PLOTFORGE_CLI_SMOKE_TTS_KEY_9F3C7A",
                "enabled": false,
                "voice": "coral",
                "format": "mp3"
            }
        })
        .to_string(),
    )
    .stdout_json();
    assert!(entry.is_object(), "upsert must return a JSON object");
    assert!(
        entry["id"].is_string(),
        "upsert object must carry string id"
    );

    let listed = run_with_stdin_home(
        &home.home,
        ["studio", "list_tts_providers"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    assert!(listed.is_array(), "list must return a JSON array");

    let tested = run_with_stdin_home(
        &home.home,
        ["studio", "test_tts_provider"],
        &serde_json::json!({ "id": "cli-smoke-tts" }).to_string(),
    )
    .stdout_json();
    assert!(tested.is_object(), "test must return a JSON object");
    assert!(tested["ok"].is_boolean(), "test object must carry bool ok");
    assert!(
        tested["message"].is_string(),
        "test object must carry string message"
    );

    let removed = run_with_stdin_home(
        &home.home,
        ["studio", "delete_tts_provider"],
        &serde_json::json!({ "id": "cli-smoke-tts" }).to_string(),
    )
    .stdout_json();
    assert!(removed.is_object(), "delete must return a JSON object");
    assert!(
        removed["id"].is_string(),
        "delete object must carry string id"
    );
}

#[test]
fn cli_studio_moderation_provider_commands_round_trip_against_temp_home() {
    let home = hermetic_home();
    let entry = run_with_stdin_home(
        &home.home,
        ["studio", "upsert_moderation_provider"],
        &serde_json::json!({
            "entry": {
                "id": "cli-smoke-moderation",
                "endpoint_url": "https://example.invalid/v1",
                "model": "omni-moderation-test",
                "credential_env_var": "PLOTFORGE_CLI_SMOKE_MODERATION_KEY_9F3C7A",
                "enabled": false
            }
        })
        .to_string(),
    )
    .stdout_json();
    assert!(entry.is_object(), "upsert must return a JSON object");
    assert_eq!(entry["id"], "cli-smoke-moderation");

    let listed = run_with_stdin_home(
        &home.home,
        ["studio", "list_moderation_providers"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    assert_eq!(listed.as_array().map(Vec::len), Some(1));

    let tested = run_with_stdin_home(
        &home.home,
        ["studio", "test_moderation_provider"],
        &serde_json::json!({ "id": "cli-smoke-moderation" }).to_string(),
    )
    .stdout_json();
    assert_eq!(tested["ok"], false);
    assert!(tested["message"].is_string());

    let removed = run_with_stdin_home(
        &home.home,
        ["studio", "delete_moderation_provider"],
        &serde_json::json!({ "id": "cli-smoke-moderation" }).to_string(),
    )
    .stdout_json();
    assert_eq!(removed["id"], "cli-smoke-moderation");
}

#[test]
fn cli_studio_prompt_templates_user_and_project_round_trip() {
    // User-global templates live at `~/.plotforge/prompts.json`; project
    // templates live at `<project>/.plotforge/prompts.json`. Redirecting HOME
    // keeps the user-global store hermetic; the project store stays under the
    // temp project dir. Both round-trip upsert -> list -> delete.
    let home = hermetic_home();
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");

    // User-global store starts empty.
    let user_before = run_with_stdin_home(
        &home.home,
        ["studio", "list_user_prompt_templates"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    assert!(
        user_before.as_array().unwrap().is_empty(),
        "fresh home must have no user prompt templates"
    );

    // Upsert a user-global template; the persisted store lands under the temp
    // home, never the real `~/.plotforge/prompts.json`.
    let upserted = run_with_stdin_home(
        &home.home,
        ["studio", "upsert_user_prompt_template"],
        &serde_json::json!({
            "template": {
                "id": "cli-smoke-pacing",
                "label": "CLI Smoke Pacing",
                "scope": "user",
                "body_markdown": "Prefer concrete beats over exposition dumps.",
            }
        })
        .to_string(),
    )
    .stdout_json();
    assert_eq!(upserted["id"], "cli-smoke-pacing");
    assert_eq!(upserted["scope"], "user");

    let user_after = run_with_stdin_home(
        &home.home,
        ["studio", "list_user_prompt_templates"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    let user_list = user_after.as_array().expect("user templates list");
    assert_eq!(user_list.len(), 1);
    assert_eq!(user_list[0]["id"], "cli-smoke-pacing");

    // Project-scoped store: list starts empty, upsert writes under the temp
    // project's `.plotforge/prompts.json`, delete removes it.
    let proj_before = run_with_stdin_home(
        &home.home,
        ["studio", "list_project_prompt_templates"],
        &serde_json::json!({ "path": project.to_string_lossy() }).to_string(),
    )
    .stdout_json();
    assert!(
        proj_before.as_array().unwrap().is_empty(),
        "fresh project must have no project prompt templates"
    );

    let proj_upserted = run_with_stdin_home(
        &home.home,
        ["studio", "upsert_project_prompt_template"],
        &serde_json::json!({
            "path": project.to_string_lossy(),
            "template": {
                "id": "cli-smoke-proj-pacing",
                "label": "CLI Smoke Project Pacing",
                "scope": "project",
                "body_markdown": "Keep project pacing tight.",
            }
        })
        .to_string(),
    )
    .stdout_json();
    assert_eq!(proj_upserted["id"], "cli-smoke-proj-pacing");
    assert_eq!(proj_upserted["scope"], "project");

    let proj_after = run_with_stdin_home(
        &home.home,
        ["studio", "list_project_prompt_templates"],
        &serde_json::json!({ "path": project.to_string_lossy() }).to_string(),
    )
    .stdout_json();
    let proj_list = proj_after.as_array().expect("project templates list");
    assert_eq!(proj_list.len(), 1);
    assert_eq!(proj_list[0]["id"], "cli-smoke-proj-pacing");
    // The project store must land under the temp project, never the home dir.
    assert!(project.join(".plotforge/prompts.json").is_file());

    // Delete both and confirm the lists go back to empty.
    run_with_stdin_home(
        &home.home,
        ["studio", "delete_user_prompt_template"],
        &serde_json::json!({ "id": "cli-smoke-pacing" }).to_string(),
    )
    .stdout_json();
    let user_final = run_with_stdin_home(
        &home.home,
        ["studio", "list_user_prompt_templates"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    assert!(user_final.as_array().unwrap().is_empty());

    run_with_stdin_home(
        &home.home,
        ["studio", "delete_project_prompt_template"],
        &serde_json::json!({
            "path": project.to_string_lossy(),
            "id": "cli-smoke-proj-pacing"
        })
        .to_string(),
    )
    .stdout_json();
    let proj_final = run_with_stdin_home(
        &home.home,
        ["studio", "list_project_prompt_templates"],
        &serde_json::json!({ "path": project.to_string_lossy() }).to_string(),
    )
    .stdout_json();
    assert!(proj_final.as_array().unwrap().is_empty());
}

#[test]
fn cli_studio_skills_list_refresh_and_read_body_dispatch() {
    // The skill index lives at `~/.plotforge/skill-index.json` and the external
    // scan roots resolve from `HOME`. A temp HOME with no skill folders yields
    // an empty (but well-formed) index, so `list_skills` / `refresh_skill_index`
    // exercise the dispatch + JSON shape without needing a committed skill
    // fixture. `refresh_skill_index` returns the full `SkillIndex` envelope;
    // `list_skills` returns just the `skills` array.
    let home = hermetic_home();

    let skills_before = run_with_stdin_home(
        &home.home,
        ["studio", "list_skills"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    assert!(
        skills_before.is_array(),
        "list_skills must return a JSON array"
    );

    let index = run_with_stdin_home(
        &home.home,
        ["studio", "refresh_skill_index"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    // SkillIndex envelope shape: { version, skills[], scanned_at }.
    assert!(
        index["version"].as_str().is_some_and(|s| !s.is_empty()),
        "refresh_skill_index must return a versioned index"
    );
    assert!(index["skills"].is_array(), "index.skills must be an array");
    assert!(
        index["scanned_at"].as_str().is_some_and(|s| !s.is_empty()),
        "refresh_skill_index must stamp a scanned_at timestamp"
    );

    // After a refresh the cache file must exist under the temp home so a
    // subsequent `list_skills` reads from the cache (not a re-scan).
    let skills_after = run_with_stdin_home(
        &home.home,
        ["studio", "list_skills"],
        &serde_json::json!({}).to_string(),
    )
    .stdout_json();
    assert!(skills_after.is_array());
    assert_eq!(skills_after.as_array().unwrap().len(), 0);

    // import_skill / read_skill_body mutate the user library and need a real
    // skill on disk for a happy path; with an empty index they must surface an
    // explicit `skill_not_found` error (no silent fallback). Asserting the
    // error path confirms the dispatch is wired without a fixture.
    let import_err = run_with_stdin_home(
        &home.home,
        ["studio", "import_skill"],
        &serde_json::json!({ "skill_id": "nonexistent" }).to_string(),
    );
    assert!(!import_err.output.status.success());
    let stderr = String::from_utf8_lossy(&import_err.output.stderr);
    assert!(
        stderr.contains("skill_not_found"),
        "expected skill_not_found error, got: {stderr}"
    );

    let body_err = run_with_stdin_home(
        &home.home,
        ["studio", "read_skill_body"],
        &serde_json::json!({ "skill_id": "nonexistent" }).to_string(),
    );
    assert!(!body_err.output.status.success());
    let stderr = String::from_utf8_lossy(&body_err.output.stderr);
    assert!(
        stderr.contains("skill_not_found"),
        "expected skill_not_found error, got: {stderr}"
    );
}

#[test]
fn cli_studio_enable_skill_for_project_persists_under_temp_project() {
    // enable_skill_for_project writes the per-project agent config under
    // `<project>/.plotforge/agent-config.json` — it never touches the
    // user-global registry, so only the temp project dir is exercised. The
    // HOME override is still applied for consistency / isolation.
    let home = hermetic_home();
    let temp = tempfile::tempdir().expect("tempdir");
    let project = check_project_path(&temp);
    create_starter_project(&project).assert_success_contains("created project Starter Project");

    let enabled = run_with_stdin_home(
        &home.home,
        ["studio", "enable_skill_for_project"],
        &serde_json::json!({
            "path": project.to_string_lossy(),
            "skill_id": "cli-smoke-skill",
            "enabled": true,
        })
        .to_string(),
    )
    .stdout_json();
    // Shape only: the persisted AgentSessionConfig echoes back with the skill
    // id added to enabled_skills. Business validation of the config lives in
    // `plotforge-studio` unit tests.
    assert_eq!(enabled["model_id"], "local-pi");
    assert!(
        enabled["enabled_skills"]
            .as_array()
            .is_some_and(|s| s.iter().any(|v| v == "cli-smoke-skill")),
        "enabled_skills must contain the toggled skill id"
    );
    assert!(
        project.join(".plotforge/agent-config.json").is_file(),
        "agent config must be written under the temp project"
    );

    // Toggling back off must remove the id.
    let disabled = run_with_stdin_home(
        &home.home,
        ["studio", "enable_skill_for_project"],
        &serde_json::json!({
            "path": project.to_string_lossy(),
            "skill_id": "cli-smoke-skill",
            "enabled": false,
        })
        .to_string(),
    )
    .stdout_json();
    assert!(
        disabled["enabled_skills"]
            .as_array()
            .is_some_and(|s| !s.iter().any(|v| v == "cli-smoke-skill")),
        "enabled_skills must not contain the disabled skill id"
    );
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

/// MCP command-group smoke test (Phase 5, P5.3). Split into its own test so a
/// failure here does not block the other command groups. Uses `cli_with_home`
/// so the user-global `~/.plotforge/mcp.json` is redirected to a temp dir —
/// the test never touches the real registry. Asserts exit codes + output
/// shape only; business correctness lives in `plotforge-studio` unit tests.
#[test]
fn cli_mcp_command_group_round_trips_server_registry() {
    let home = hermetic_home();

    // `mcp list` on a fresh install → empty list, exit 0.
    run_with_home(&home.home, ["mcp", "list"]).assert_success_contains("mcp servers: 0");

    // `mcp add --batch` adds a stdio server entry. The entry points at a
    // non-existent command (the test never spawns it); it only asserts the
    // registry write succeeds.
    run_with_home(
        &home.home,
        [
            "mcp",
            "add",
            "--batch",
            "--id",
            "test-fs",
            "--kind",
            "stdio",
            "--label",
            "Test FS MCP",
            "--command",
            "mcp-server-fs-test",
            "--credential-env-var",
            "MCP_TEST_TOKEN",
        ],
    )
    .assert_success_contains("added mcp server test-fs");

    // `mcp list` now shows 1 server.
    run_with_home(&home.home, ["mcp", "list"])
        .assert_success_contains("mcp servers: 1")
        .assert_contains("test-fs");

    // `mcp test` may fail (the command binary doesn't exist) — assert the
    // command runs and produces a test result line, not that it succeeds.
    let test_output = run_with_home(&home.home, ["mcp", "test", "test-fs"]);
    let test_stdout = String::from_utf8_lossy(&test_output.output.stdout);
    let test_stderr = String::from_utf8_lossy(&test_output.output.stderr);
    assert!(
        test_stdout.contains("mcp test test-fs:") || test_stderr.contains("test-fs"),
        "mcp test must produce a result line; stdout={test_stdout}\nstderr={test_stderr}"
    );

    // `mcp remove` removes the entry, exit 0.
    run_with_home(&home.home, ["mcp", "remove", "test-fs"])
        .assert_success_contains("removed mcp server test-fs");

    // Back to empty.
    run_with_home(&home.home, ["mcp", "list"]).assert_success_contains("mcp servers: 0");

    // No secret markers leak through any MCP command output.
    assert!(!test_stdout.contains("sk-"));
    assert!(!test_stdout.contains("api_key"));
}

fn run<I, S>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = cli().args(args).output().expect("run command");
    CommandOutput { output }
}

/// Like `run`, but redirects `HOME` to a temp dir so user-global config
/// (`~/.plotforge/mcp.json`, providers, skills, prompts) never touches the
/// real home directory. Used by the MCP command-group smoke test.
fn run_with_home<I, S>(home: &std::path::Path, args: I) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = cli_with_home(home)
        .args(args)
        .output()
        .expect("run command with home");
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

/// Build a CLI `Command` that resolves the user-global PlotForge config
/// directory (`~/.plotforge/providers.json`, `~/.plotforge/prompts.json`,
/// `~/.plotforge/skill-index.json`, and the external skill scan roots) against
/// a temp `HOME` instead of the real user home. This is the hermetic test hook
/// for the studio commands that read/write user-global state: the binary
/// already links `dirs::config_dir()`, which on macOS resolves to
/// `$HOME/Library/Application Support` and on Linux to `$HOME/.config`, both of
/// which honor an overridden `HOME`. The external skill roots (`~/.claude`,
/// `~/.codex`, ...) likewise resolve from `HOME`. With `HOME` pointed at a
/// fresh temp dir, none of these paths exist, so the commands see a clean
/// install and any writes land under the temp dir (never the real
/// `~/.plotforge/`). This keeps provider/prompt/skill upsert + delete tests
/// hermetic and safe to run on a developer's real machine.
fn cli_with_home(home: &std::path::Path) -> Command {
    let mut cmd = cli();
    cmd.env("HOME", home);
    cmd
}

fn run_with_stdin_home<I, S>(home: &std::path::Path, args: I, stdin: &str) -> CommandOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut child = cli_with_home(home)
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

/// A fresh temp `HOME` paired with its owning tempdir so the directory lives
/// as long as the test needs it. The `home` path is what gets passed to the
/// CLI subprocess; the `temp` field is intentionally never read — its only
/// job is to keep the temp dir (and therefore `home`) alive until the struct
/// drops at the end of the test.
struct HermeticHome {
    #[allow(dead_code)]
    temp: tempfile::TempDir,
    home: std::path::PathBuf,
}

fn hermetic_home() -> HermeticHome {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("pfhome");
    fs::create_dir_all(&home).expect("create temp home");
    HermeticHome { temp, home }
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

fn create_starter_project_with_home(
    home: &std::path::Path,
    project: &std::path::Path,
) -> CommandOutput {
    run_with_home(
        home,
        [
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
        ],
    )
}

fn configure_real_apply(
    home: &std::path::Path,
    project: &std::path::Path,
    moderation_endpoint: &str,
    text_endpoint: &str,
) {
    fs::create_dir_all(project.join(".plotforge")).expect("project config dir");
    fs::write(
        project.join(".plotforge/agent-config.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "model_id": "cli-e2e-text",
            "permission_level": "ask_every_time",
            "thinking_level": "medium",
            "enabled_skills": [],
            "enabled_mcp_servers": []
        }))
        .expect("agent config JSON"),
    )
    .expect("write agent config");

    run_with_stdin_home(
        home,
        ["studio", "upsert_provider"],
        &serde_json::json!({
            "entry": {
                "id": "cli-e2e-text",
                "kind": "openai_compatible",
                "label": "CLI E2E text",
                "endpoint_url": text_endpoint,
                "model": "cli-e2e-model",
                "credential_env_var": "",
                "enabled": true
            }
        })
        .to_string(),
    )
    .stdout_json();
    run_with_stdin_home(
        home,
        ["studio", "upsert_moderation_provider"],
        &serde_json::json!({
            "entry": {
                "id": "cli-e2e-moderation",
                "endpoint_url": moderation_endpoint,
                "model": "omni-moderation-test",
                "credential_env_var": "",
                "enabled": true
            }
        })
        .to_string(),
    )
    .stdout_json();
}

fn pi_agent_apply_payload(project: &std::path::Path, player_input: &str) -> String {
    serde_json::json!({
        "request": {
            "agent_id": "scene-planner",
            "run_seed": 99,
            "project_path": project.to_string_lossy(),
            "player_input": player_input
        }
    })
    .to_string()
}

fn openai_scene_plan_body() -> serde_json::Value {
    let envelope = serde_json::json!({
        "id": "cli-e2e-envelope",
        "contract_version": plotforge_schema::CONTRACT_VERSION,
        "schema_version": plotforge_schema::CONTRACT_SCHEMA_VERSION,
        "agent": "scene_planner",
        "reproducibility": {
            "run_seed": 99,
            "prompt_version": "provider-emitted",
            "model_version": "cli-e2e-model",
            "provider_config_hash": "sha256:provider-emitted"
        },
        "proposal": {
            "id": "cli-e2e-scene-plan",
            "agent": "scene_planner",
            "output": {
                "kind": "scene_plan",
                "payload": {
                    "scene_key": "cli-e2e-scene",
                    "title": "CLI E2E Scene",
                    "location": "Mock provider",
                    "scene_summary": "A safe scene returned by the local TCP mock.",
                    "dramatic_purpose": "Exercise the public CLI apply path.",
                    "hook": "The moderation hash survives into the trace.",
                    "emotional_goal": null,
                    "cast": [],
                    "entry_beat_id": "cli-e2e-scene-beat-001",
                    "background_asset": null
                }
            }
        }
    })
    .to_string();
    serde_json::json!({
        "choices": [{
            "message": {"content": envelope},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 20}
    })
}

fn json_http_response(body: &serde_json::Value) -> Vec<u8> {
    let body = body.to_string();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

struct MockHttpServer {
    addr: std::net::SocketAddr,
    stop: Arc<AtomicBool>,
    request_count: Arc<AtomicUsize>,
    handle: Option<thread::JoinHandle<()>>,
}

impl MockHttpServer {
    fn new(reply: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock HTTP server");
        listener
            .set_nonblocking(true)
            .expect("nonblocking mock HTTP server");
        let addr = listener.local_addr().expect("mock HTTP server addr");
        let stop = Arc::new(AtomicBool::new(false));
        let request_count = Arc::new(AtomicUsize::new(0));
        let thread_stop = stop.clone();
        let thread_request_count = request_count.clone();
        let handle = thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        thread_request_count.fetch_add(1, Ordering::AcqRel);
                        stream
                            .set_read_timeout(Some(Duration::from_secs(1)))
                            .expect("mock HTTP read timeout");
                        let mut request = vec![0; 65_536];
                        let _ = stream.read(&mut request);
                        stream.write_all(&reply).expect("mock HTTP response");
                        stream.flush().expect("flush mock HTTP response");
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("accept mock HTTP request: {error}"),
                }
            }
        });
        Self {
            addr,
            stop,
            request_count,
            handle: Some(handle),
        }
    }

    fn addr(&self) -> std::net::SocketAddr {
        self.addr
    }

    fn request_count(&self) -> usize {
        self.request_count.load(Ordering::Acquire)
    }
}

impl Drop for MockHttpServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let result = handle.join();
            if !thread::panicking() {
                result.expect("join mock HTTP server");
            }
        }
    }
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

fn import_workshop_package(library: &std::path::Path, package: &std::path::Path) {
    run([
        "workshop",
        "import",
        library.to_str().unwrap(),
        package.to_str().unwrap(),
    ])
    .assert_success_contains("imported workshop item starter-workshop-draft");
}

fn remix_workshop_item(library: &std::path::Path, source_id: &str, new_id: &str) {
    run([
        "workshop",
        "remix",
        library.to_str().unwrap(),
        source_id,
        "--new-id",
        new_id,
        "--title",
        "Starter Workshop Sample Remix",
    ])
    .assert_success_contains(&format!("remixed workshop item {source_id} -> {new_id}"));
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

    fn assert_no_ansi(self) -> Self {
        let stdout = String::from_utf8_lossy(&self.output.stdout);
        assert!(
            !stdout.contains("\x1b["),
            "machine-readable stdout must not contain ANSI escapes: {stdout}"
        );
        self
    }
}
