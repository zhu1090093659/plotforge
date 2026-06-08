use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use plotforge_schema::{
    ActionIntent, ReferenceAnalysis, ReferenceRights, ReferenceSource, ReferenceSourceType,
    ReferenceStructureNote, RuntimePlannerResult, RuntimeRuleResult, RuntimeSnapshot, RuntimeTrace,
    RuntimeTraceDiagnostic, RuntimeTraceStage, RuntimeTraceStageStatus, StoryState, WorldDelta,
    WorldState,
};
use plotforge_storage::{
    StorageError, create_demo_project, dynasty_embers_project, load_project,
    read_latest_runtime_snapshot, read_runtime_snapshot, validate_project,
    validate_reference_library, write_reference_analysis, write_runtime_snapshot, write_trace,
};

#[test]
fn create_demo_respects_force_flag() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");

    create_demo_project(&project, false).expect("first create");
    let error = create_demo_project(&project, false).expect_err("force=false should fail");
    assert!(matches!(error, StorageError::ProjectExists(_)));

    create_demo_project(&project, true).expect("force=true recreates");
}

#[test]
fn validate_fails_when_entry_scene_is_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");
    fs::remove_file(project.join("scenes/court-crisis-001.scene.json")).expect("remove scene");

    let error = validate_project(&project).expect_err("missing scene should fail");
    assert!(matches!(error, StorageError::MissingFile(_)));
}

#[test]
fn write_trace_writes_trace_id_and_latest() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");
    let story = StoryState {
        current_scene_key: "court-crisis-001".into(),
        completed_scene_keys: Vec::new(),
        turn: 1,
    };
    let trace = RuntimeTrace {
        id: "trace-test".into(),
        timestamp_ms: 1,
        player_input: Some("test".into()),
        selected_choice: Some("raise-tax".into()),
        action_intent: Some(ActionIntent::supported("raise_tax", vec!["test".into()])),
        rule_result: Some(RuntimeRuleResult {
            action_type: "raise_tax".into(),
            delta_empty: true,
            state_committed: true,
            error: None,
        }),
        planner_result: Some(RuntimePlannerResult {
            requested_action_type: "raise_tax".into(),
            scene_key: Some("court-crisis-002".into()),
            fallback_used: false,
            error: None,
        }),
        diagnostics: vec![RuntimeTraceDiagnostic::new_redacted(
            RuntimeTraceStage::CommitState,
            RuntimeTraceStageStatus::Completed,
            "committed",
        )],
        world_state_before: WorldState::default(),
        world_state_delta: WorldDelta::default(),
        world_state_after: WorldState::default(),
        story_state_before: story.clone(),
        story_state_after: story,
        narrative_review: None,
        media_references: Vec::new(),
        errors: Vec::new(),
        fallback_used: false,
    };

    let trace_path = write_trace(&project, &trace).expect("write trace");

    assert_eq!(trace_path.file_name().unwrap(), "trace-test.json");
    assert!(project.join("traces/trace-test.json").is_file());
    assert!(project.join("traces/latest.json").is_file());
}

#[test]
fn write_runtime_snapshot_roundtrips_snapshot_id_and_latest() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");
    let snapshot = sample_runtime_snapshot("save-001");

    let snapshot_path = write_runtime_snapshot(&project, &snapshot).expect("write snapshot");

    assert_eq!(
        snapshot_path.file_name().unwrap(),
        "save-001.runtime_snapshot.json"
    );
    assert!(
        project
            .join("saves/save-001.runtime_snapshot.json")
            .is_file()
    );
    assert!(project.join("saves/latest.runtime_snapshot.json").is_file());
    assert_eq!(
        read_runtime_snapshot(&project, "save-001").expect("read snapshot"),
        snapshot
    );
    assert_eq!(
        read_latest_runtime_snapshot(&project).expect("read latest"),
        snapshot
    );
}

#[test]
fn write_runtime_snapshot_rejects_unsafe_snapshot_id() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");
    let snapshot = sample_runtime_snapshot("../escape");

    let error = write_runtime_snapshot(&project, &snapshot).expect_err("unsafe snapshot id");

    assert!(matches!(error, StorageError::InvalidRuntimeSnapshotId(_)));
    assert!(!project.join("escape.runtime_snapshot.json").exists());
}

#[test]
fn reference_imports_store_metadata_and_summary_only() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");

    let path =
        write_reference_analysis(&project, &sample_reference_analysis()).expect("write reference");
    validate_reference_library(&project).expect("reference library valid");

    let json = fs::read_to_string(path).expect("reference json");
    assert!(json.contains("\"source\""));
    assert!(json.contains("\"summary\""));
    assert!(json.contains("\"structure_notes\""));
    assert!(!json.contains("raw_text"));
    assert!(!json.contains("copyrighted body"));
}

#[test]
fn reference_library_rejects_large_raw_copyrighted_text_fixture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");
    let raw_path = project.join("references/user_imports/paid_novel_chapter.txt");
    fs::write(&raw_path, "paid novel chapter body ".repeat(400)).expect("write raw text");

    let error = validate_project(&project).expect_err("large raw text should fail validation");

    assert!(matches!(
        error,
        StorageError::ReferenceCompliance { reason, .. } if reason.contains("large raw reference text")
    ));
}

#[test]
fn committed_fixture_matches_generated_demo_semantics() {
    let committed_fixture = repo_root().join("examples/dynasty-embers");
    let temp = tempfile::tempdir().expect("tempdir");
    let generated_fixture = temp.path().join("dynasty-embers");
    create_demo_project(&generated_fixture, false).expect("generate fixture");

    assert_fixture_files_match_generated_demo(&committed_fixture, &generated_fixture);

    let committed =
        canonical_project(load_project(&committed_fixture).expect("load committed fixture"));
    let generated_loaded =
        canonical_project(load_project(&generated_fixture).expect("load generated fixture"));
    let generated_in_memory = canonical_project(dynasty_embers_project());

    assert_eq!(committed, generated_loaded);
    assert_eq!(committed, generated_in_memory);
}

#[test]
fn committed_fixture_contains_no_generated_trace_json() {
    let fixture = repo_root().join("examples/dynasty-embers");
    let trace_files = trace_json_files(&fixture);
    let trace_file_list = trace_files
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    assert!(
        trace_files.is_empty(),
        "committed fixture contains generated trace json: {trace_file_list}"
    );
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn sample_runtime_snapshot(id: &str) -> RuntimeSnapshot {
    let project = dynasty_embers_project();
    RuntimeSnapshot {
        id: id.into(),
        timestamp_ms: 42,
        project_id: project.game.id,
        project_version: project.game.version,
        story_state: project.story_state,
        world_state: project.world_state,
        scenes: project.scenes,
    }
}

fn assert_fixture_files_match_generated_demo(committed_fixture: &Path, generated_fixture: &Path) {
    let committed_manifest = source_file_manifest(committed_fixture);
    let generated_manifest = source_file_manifest(generated_fixture);
    let committed_paths = committed_manifest.keys().collect::<Vec<_>>();
    let generated_paths = generated_manifest.keys().collect::<Vec<_>>();

    assert_eq!(committed_paths, generated_paths);
    for (relative_path, generated_bytes) in generated_manifest {
        let committed_bytes = committed_manifest
            .get(&relative_path)
            .unwrap_or_else(|| panic!("committed fixture missing {}", relative_path.display()));
        assert!(
            committed_bytes == &generated_bytes,
            "fixture source file drifted from generated demo: {}",
            relative_path.display()
        );
    }
}

fn canonical_project(mut project: plotforge_schema::ProjectData) -> plotforge_schema::ProjectData {
    project
        .resources
        .sort_by(|left, right| left.key.cmp(&right.key));
    project
        .characters
        .sort_by(|left, right| left.id.cmp(&right.id));
    project.rules.sort_by(|left, right| left.id.cmp(&right.id));
    project
        .scenes
        .sort_by(|left, right| left.key.cmp(&right.key));
    project
        .story_craft
        .plot_threads
        .sort_by(|left, right| left.id.cmp(&right.id));
    project
        .story_craft
        .active_promises
        .sort_by(|left, right| left.id.cmp(&right.id));
    project
        .story_craft
        .character_arcs
        .sort_by(|left, right| left.id.cmp(&right.id));
    project
        .story_craft
        .review_notes
        .sort_by(|left, right| left.id.cmp(&right.id));
    project
        .story_craft
        .bible
        .reference_modules
        .sort_by(|left, right| left.id.cmp(&right.id));
    project.story_craft.emotional_arc.sort_by(|left, right| {
        left.scene_key
            .cmp(&right.scene_key)
            .then_with(|| left.target_emotion.cmp(&right.target_emotion))
            .then_with(|| left.intensity.cmp(&right.intensity))
    });
    project
}

fn source_file_manifest(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut manifest = BTreeMap::new();
    collect_source_files(root, root, &mut manifest);
    manifest
}

fn collect_source_files(root: &Path, dir: &Path, manifest: &mut BTreeMap<PathBuf, Vec<u8>>) {
    let mut entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read fixture directory {}: {error}", dir.display()))
        .map(|entry| entry.expect("fixture directory entry").path())
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        if path.is_dir() {
            collect_source_files(root, &path, manifest);
            continue;
        }

        let relative_path = path
            .strip_prefix(root)
            .unwrap_or_else(|error| panic!("strip fixture prefix {}: {error}", path.display()))
            .to_path_buf();
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("read fixture file {}: {error}", path.display()));
        manifest.insert(relative_path, bytes);
    }
}

fn trace_json_files(root: &Path) -> Vec<PathBuf> {
    source_file_manifest(root)
        .into_keys()
        .filter(|path| {
            path.components()
                .any(|component| component.as_os_str() == "traces")
                && path
                    .extension()
                    .is_some_and(|extension| extension == "json")
        })
        .collect()
}

fn sample_reference_analysis() -> ReferenceAnalysis {
    ReferenceAnalysis {
        id: "authorized-crisis-notes".into(),
        title: "Authorized Crisis Notes".into(),
        source: ReferenceSource {
            source_type: ReferenceSourceType::UserImport,
            rights: ReferenceRights::UserAuthorized,
            citation: "User-supplied local notes".into(),
            user_authorized: true,
        },
        summary: "Use scarcity, legitimacy, and faction pressure as structure notes only.".into(),
        structure_notes: vec![ReferenceStructureNote {
            label: "pressure sequence".into(),
            summary: "Start with shortage, then force a legitimacy tradeoff.".into(),
        }],
        tags: vec!["authorized".into(), "structure".into()],
    }
}
