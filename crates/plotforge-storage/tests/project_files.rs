use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use plotforge_schema::{
    ActionIntent, AiSafetyPolicy, AiUsageContentKind, AssetKind, AssetSourceKind, Character,
    CharacterGenerationReport, CharacterPortraitRequest, Effect, GenerationEvidence,
    GenerationStatus, MediaAssetReference, ProjectCreationRequest, ProjectTemplateId,
    ReferenceAnalysis, ReferenceRights, ReferenceSource, ReferenceSourceType,
    ReferenceStructureNote, ReproducibilityMetadata, ResourceDefinition, Rule,
    RuntimePlannerResult, RuntimeRuleResult, RuntimeSnapshot, RuntimeTrace, RuntimeTraceDiagnostic,
    RuntimeTraceStage, RuntimeTraceStageStatus, StoryCraftGenerationReport, StoryState, WorldDelta,
    WorldEditDocument, WorldGenerationReport, WorldState,
};
use plotforge_storage::{
    SQLITE_CACHE_SCHEMA_VERSION, StorageError, apply_character_generation_report,
    apply_story_craft_generation_report, apply_world_generation_report,
    attach_beat_audio_reference, attach_scene_audio_reference, build_character_generation_request,
    build_story_craft_generation_request, build_world_generation_request, create_character,
    create_demo_project, create_project_from_request, create_resource, create_rule,
    dynasty_embers_project, load_project, read_ai_safety_policy, read_character_edit_document,
    read_latest_runtime_snapshot, read_rules_edit_document, read_runtime_snapshot,
    read_sqlite_cache_summary, read_state_variables_edit_document, read_story_craft_edit_document,
    read_world_edit_document, rebuild_sqlite_cache, sqlite_cache_path, update_ai_safety_policy,
    update_character_edit_document, update_rules_edit_document,
    update_state_variables_edit_document, update_story_craft_edit_document,
    update_world_edit_document, validate_project, validate_reference_library,
    write_reference_analysis, write_runtime_snapshot, write_trace,
};
use rusqlite::Connection;

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
fn create_project_from_request_persists_wizard_fields_and_reopens() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("winter-regency");
    let request = sample_project_creation_request();

    let report =
        create_project_from_request(&project, request.clone(), false).expect("create project");
    let loaded = validate_project(&project).expect("validate created project");

    assert_eq!(report.project_path, project.display().to_string());
    assert_eq!(report.template, ProjectTemplateId::HistoricalCrisis);
    assert_eq!(report.concept, request.concept);
    assert_eq!(report.visual_style, request.visual_style);
    assert!(report.voice_enabled);
    assert_eq!(report.initial_scene_request, request.initial_scene_request);
    assert_eq!(report.project.game.id, "winter-regency");
    assert_eq!(loaded.game.title, "Winter Regency");
    assert_eq!(
        loaded.game.description,
        "A regency court must survive a winter coup."
    );
    assert_eq!(
        loaded.story_craft.bible.genre_promise,
        "A regency court must survive a winter coup."
    );
    assert_eq!(
        loaded.story_craft.bible.prose_style_guide.as_deref(),
        Some("ink wash court drama")
    );
    assert!(project.join("game.toml").is_file());
    assert!(project.join("story/story_craft.toml").is_file());
    assert!(project.join("world/forbidden_facts.json").is_file());
    assert!(project.join("safety/ai_safety_policy.toml").is_file());
    assert!(report.files_created.iter().any(|path| path == "game.toml"));
    assert!(
        report
            .files_created
            .iter()
            .any(|path| path == "story/story_craft.toml")
    );
    assert!(
        report
            .files_created
            .iter()
            .any(|path| path == "world/forbidden_facts.json")
    );
    assert!(
        report
            .files_created
            .iter()
            .any(|path| path == "safety/ai_safety_policy.toml")
    );
    assert!(loaded.ai_safety_policy.human_review_required);
    assert_eq!(
        loaded.ai_safety_policy.policy_source_path.as_deref(),
        Some("safety/ai_safety_policy.toml")
    );

    let world = fs::read_to_string(project.join("world/world.md")).expect("world bible");
    let story = fs::read_to_string(project.join("story/story_bible.md")).expect("story bible");
    let style = fs::read_to_string(project.join("story/style_guide.md")).expect("style guide");
    assert!(world.contains("A regency court must survive a winter coup."));
    assert!(story.contains("Open on an empty granary ledger."));
    assert!(style.contains("ink wash court drama"));
    assert!(style.contains("Voice generation requested"));
}

#[test]
fn structured_edit_documents_roundtrip_and_persist_source_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");

    let mut world = read_world_edit_document(&project).expect("read world edit");
    world
        .world_bible_markdown
        .push_str("\n## Winter Court\n\nSnow blocks the passes.\n");
    world
        .forbidden_facts
        .push("The emperor cannot secretly be immortal.".into());
    update_world_edit_document(&project, world.clone()).expect("update world edit");
    assert_eq!(
        read_world_edit_document(&project).expect("read world again"),
        world
    );
    assert!(
        fs::read_to_string(project.join("world/forbidden_facts.json"))
            .expect("forbidden facts")
            .contains("immortal")
    );

    let mut story = read_story_craft_edit_document(&project).expect("read story edit");
    story.story_craft.bible.genre_promise = "A winter court crisis with visible tradeoffs.".into();
    story
        .story_craft
        .bible
        .core_foreshadowing
        .push("frozen granary locks".into());
    story
        .story_bible_markdown
        .push_str("\nThe frozen granary locks matter.\n");
    update_story_craft_edit_document(&project, story.clone()).expect("update story edit");
    let loaded = load_project(&project).expect("load story update");
    assert_eq!(
        loaded.story_craft.bible.genre_promise,
        story.story_craft.bible.genre_promise
    );
    assert!(
        fs::read_to_string(project.join("story/story_bible.md"))
            .expect("story bible")
            .contains("frozen granary locks")
    );

    let mut characters = read_character_edit_document(&project).expect("read characters");
    characters.characters.push(sample_character("regent"));
    update_character_edit_document(&project, characters).expect("update characters");
    assert!(project.join("characters/regent.character.toml").is_file());
    create_character(&project, sample_character("tax-envoy")).expect("create character");
    assert!(
        read_character_edit_document(&project)
            .expect("read created characters")
            .characters
            .iter()
            .any(|character| character.id == "tax-envoy")
    );

    let mut state = read_state_variables_edit_document(&project).expect("read state");
    state.resources.push(ResourceDefinition {
        key: "legitimacy".into(),
        label: "Legitimacy".into(),
        initial: 60,
        min: 0,
        max: 100,
    });
    state
        .initial_world_state
        .resources
        .insert("legitimacy".into(), 60);
    update_state_variables_edit_document(&project, state).expect("update state");
    create_resource(
        &project,
        ResourceDefinition {
            key: "grain".into(),
            label: "Grain".into(),
            initial: 30,
            min: 0,
            max: 100,
        },
    )
    .expect("create resource");
    let loaded = load_project(&project).expect("load state update");
    assert!(
        loaded
            .resources
            .iter()
            .any(|resource| resource.key == "grain")
    );
    assert_eq!(loaded.world_state.resources.get("grain"), Some(&30));

    let mut rules = read_rules_edit_document(&project).expect("read rules");
    rules.rules.push(sample_rule("spend-grain", "grain"));
    update_rules_edit_document(&project, rules).expect("update rules");
    create_rule(&project, sample_rule("restore-legitimacy", "legitimacy")).expect("create rule");
    let loaded = load_project(&project).expect("load rules update");
    assert!(
        loaded
            .rules
            .iter()
            .any(|rule| rule.id == "restore-legitimacy")
    );
}

#[test]
fn audio_references_attach_persist_and_rebuild_asset_records() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");
    let audio_path = "assets/generated/audio/court-crisis-001-beat-001.wav";
    fs::create_dir_all(project.join("assets/generated/audio")).expect("audio dir");
    fs::write(project.join(audio_path), b"fake beat wav bytes").expect("write audio");
    let reference = MediaAssetReference {
        asset_id: None,
        kind: AssetKind::Audio,
        source: AssetSourceKind::Generated,
        project_path: audio_path.into(),
        export_path: audio_path.into(),
        slot: "narration".into(),
    };

    let updated = attach_beat_audio_reference(
        &project,
        "court-crisis-001",
        "court-crisis-001-beat-001",
        reference.clone(),
    )
    .expect("attach beat audio");

    assert_eq!(updated.beats[0].audio_refs, vec![reference.clone()]);
    let loaded = load_project(&project).expect("reload project");
    assert_eq!(loaded.scenes[0].beats[0].audio_refs, vec![reference]);
    let audio_record = loaded
        .asset_records
        .iter()
        .find(|record| record.kind == AssetKind::Audio)
        .expect("audio asset record");
    assert_eq!(audio_record.project_path, audio_path);
    assert_eq!(audio_record.export_path, audio_path);
    assert_eq!(
        audio_record.byte_length,
        b"fake beat wav bytes".len() as u64
    );
    assert!(audio_record.references.iter().any(|record_reference| {
        record_reference.reference_id == "court-crisis-001"
            && record_reference.slot == "beat_audio:court-crisis-001-beat-001:narration"
    }));
}

#[test]
fn audio_reference_attach_rejects_non_audio_external_and_unsafe_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");

    let mut reference = MediaAssetReference {
        asset_id: None,
        kind: AssetKind::Image,
        source: AssetSourceKind::Generated,
        project_path: "assets/generated/not-audio.png".into(),
        export_path: "assets/generated/not-audio.png".into(),
        slot: "narration".into(),
    };
    let error = attach_beat_audio_reference(
        &project,
        "court-crisis-001",
        "court-crisis-001-beat-001",
        reference.clone(),
    )
    .expect_err("image audio ref should fail");
    assert!(matches!(
        error,
        StorageError::InvalidStructuredEdit { surface, field, .. }
            if surface == "beat_audio" && field == "audio_refs.kind"
    ));

    reference.kind = AssetKind::Audio;
    reference.source = AssetSourceKind::External;
    let error = attach_scene_audio_reference(&project, "court-crisis-001", reference.clone())
        .expect_err("external audio ref should fail");
    assert!(matches!(
        error,
        StorageError::InvalidStructuredEdit { surface, field, .. }
            if surface == "scene_audio" && field == "audio_refs.source"
    ));

    reference.source = AssetSourceKind::Generated;
    reference.project_path = "../outside.wav".into();
    reference.export_path = "assets/generated/not-audio.wav".into();
    let error = attach_scene_audio_reference(&project, "court-crisis-001", reference)
        .expect_err("unsafe audio path should fail");
    assert!(matches!(
        error,
        StorageError::InvalidStructuredEdit { surface, field, .. }
            if surface == "scene_audio" && field == "audio_refs.project_path"
    ));
}

#[test]
fn ai_safety_policy_defaults_when_absent_and_roundtrips() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");

    let loaded = load_project(&project).expect("load project");
    let policy = read_ai_safety_policy(&project).expect("read policy");
    assert_eq!(loaded.ai_safety_policy, policy);
    assert!(project.join("safety/ai_safety_policy.toml").is_file());
    assert!(!policy.live_generated_content_enabled);
    assert!(policy.human_review_required);

    fs::remove_file(project.join("safety/ai_safety_policy.toml")).expect("remove policy");
    let defaulted = load_project(&project).expect("load default policy");
    assert_eq!(
        defaulted.ai_safety_policy.policy_source_path.as_deref(),
        Some("safety/ai_safety_policy.toml")
    );
    assert!(defaulted.ai_safety_policy.human_review_required);

    let updated = update_ai_safety_policy(
        &project,
        AiSafetyPolicy {
            live_generated_content_enabled: true,
            content_kinds: vec![AiUsageContentKind::Text],
            safety_guardrails: vec!["Queue every generated passage for review.".into()],
            user_reporting_path: "studio://moderation-queue".into(),
            moderation_policy: "Creator reviews generated text before export.".into(),
            human_review_required: true,
            moderation_queue_enabled: true,
            policy_source_path: None,
            evidence_ids: vec!["policy-evidence-001".into()],
            policy_hash: Some("sha256:policy-fixture".into()),
            notices: vec!["Local policy only.".into()],
        },
    )
    .expect("update policy");
    assert_eq!(
        updated.policy_source_path.as_deref(),
        Some("safety/ai_safety_policy.toml")
    );
    assert_eq!(
        read_ai_safety_policy(&project).expect("read updated policy"),
        updated
    );

    let error = update_ai_safety_policy(
        &project,
        AiSafetyPolicy {
            moderation_policy: "OPENAI_API_KEY=sk-test-secret-marker".into(),
            ..updated
        },
    )
    .expect_err("secret marker policy should fail");
    assert!(matches!(error, StorageError::InvalidAiSafetyPolicy { .. }));
}

#[test]
fn generation_requests_build_from_project_source_and_reports_apply() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");

    let world_request =
        build_world_generation_request(&project, "Expand the northern border crisis.")
            .expect("build world request");
    assert_eq!(
        world_request.expansion_goal,
        "Expand the northern border crisis."
    );
    assert!(
        world_request
            .document
            .world_bible_markdown
            .contains("dynasty")
    );

    let mut world_document = world_request.document.clone();
    world_document
        .forbidden_facts
        .push("The northern border cannot be solved off-screen.".into());
    apply_world_generation_report(
        &project,
        WorldGenerationReport {
            document: world_document.clone(),
            evidence: sample_generation_evidence(GenerationStatus::Succeeded),
        },
    )
    .expect("apply world report");
    assert_eq!(
        read_world_edit_document(&project)
            .expect("read applied world")
            .forbidden_facts,
        world_document.forbidden_facts
    );

    let story_request =
        build_story_craft_generation_request(&project, "Tighten the winter crisis.")
            .expect("build story request");
    assert!(story_request.world_bible_markdown.contains("dynasty"));
    assert!(
        story_request
            .forbidden_facts
            .contains(&"The northern border cannot be solved off-screen.".into())
    );
    assert!(!story_request.characters.is_empty());

    let mut story_document = story_request.document.clone();
    story_document
        .style_guide_markdown
        .push_str("\nKeep court reversals concrete.\n");
    apply_story_craft_generation_report(
        &project,
        StoryCraftGenerationReport {
            document: story_document.clone(),
            evidence: sample_generation_evidence(GenerationStatus::Fallback),
        },
    )
    .expect("apply fallback story report");
    assert_eq!(
        read_story_craft_edit_document(&project).expect("read story report"),
        story_document
    );

    let character_request =
        build_character_generation_request(&project, "Design a grain envoy.", "court envoy")
            .expect("build character request");
    assert_eq!(character_request.role_hint, "court envoy");
    assert!(character_request.story_bible_markdown.contains("throne"));
    assert!(!character_request.existing_characters.is_empty());

    apply_character_generation_report(
        &project,
        CharacterGenerationReport {
            character: sample_character("grain-envoy"),
            evidence: sample_generation_evidence(GenerationStatus::Succeeded),
        },
    )
    .expect("apply character report");
    assert!(
        read_character_edit_document(&project)
            .expect("read characters")
            .characters
            .iter()
            .any(|character| character.id == "grain-envoy")
    );
}

#[test]
fn failed_generation_report_does_not_mutate_project_source() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");

    let original = read_world_edit_document(&project).expect("read original");
    let mut rejected = original.clone();
    rejected
        .world_bible_markdown
        .push_str("\nThis failed report must not persist.\n");

    let error = apply_world_generation_report(
        &project,
        WorldGenerationReport {
            document: rejected,
            evidence: sample_generation_evidence(GenerationStatus::Failed),
        },
    )
    .expect_err("failed generation report should not apply");

    assert!(matches!(
        error,
        StorageError::InvalidGenerationReport { surface, .. } if surface == "world"
    ));
    assert_eq!(
        read_world_edit_document(&project).expect("read after failed report"),
        original
    );
}

#[test]
fn structured_edit_documents_reject_invalid_data_explicitly() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");

    let error = update_world_edit_document(
        &project,
        WorldEditDocument {
            world_bible_markdown: "OPENAI_API_KEY=sk-test-secret-marker".into(),
            canon_markdown: "# Canon\n".into(),
            forbidden_facts: Vec::new(),
        },
    )
    .expect_err("secret marker should fail");
    assert!(matches!(
        error,
        StorageError::InvalidStructuredEdit { surface, field, reason }
            if surface == "world"
                && field == "world_bible_markdown"
                && reason.contains("secret markers")
    ));

    let mut state = read_state_variables_edit_document(&project).expect("read state");
    state.resources[0].initial = state.resources[0].max + 1;
    let error = update_state_variables_edit_document(&project, state)
        .expect_err("resource bounds should fail");
    assert!(matches!(
        error,
        StorageError::InvalidStructuredEdit { surface, field, .. }
            if surface == "state_variables" && field == "resources.initial"
    ));

    let error = create_character(&project, sample_character("../escape"))
        .expect_err("unsafe character id should fail");
    assert!(matches!(
        error,
        StorageError::InvalidStructuredEdit { surface, field, .. }
            if surface == "characters" && field == "characters.id"
    ));

    let mut character = sample_character("unsafe-portrait");
    character.portrait_request = Some(CharacterPortraitRequest {
        prompt_summary: "Authorization: bearer token=value".into(),
        style: "court portrait".into(),
        target_asset_slot: "portrait".into(),
        prompt_hash: "sha256:portrait".into(),
        provider_config_hash: "sha256:provider".into(),
        reference_asset_ids: Vec::new(),
        fallback_allowed: true,
    });
    let error = create_character(&project, character)
        .expect_err("secret marker portrait request should fail");
    assert!(matches!(
        error,
        StorageError::InvalidStructuredEdit { surface, field, reason }
            if surface == "characters"
                && field == "characters.portrait_request.prompt_summary"
                && reason.contains("secret markers")
    ));

    let mut rules = read_rules_edit_document(&project).expect("read rules");
    rules
        .rules
        .push(sample_rule("bad-rule", "missing_resource"));
    let error =
        update_rules_edit_document(&project, rules).expect_err("unknown resource should fail");
    assert!(matches!(
        error,
        StorageError::InvalidStructuredEdit { surface, field, reason }
            if surface == "rules"
                && field == "rules.effects.key"
                && reason.contains("unknown resource key")
    ));

    assert!(!project.join("characters/../escape.character.toml").exists());
}

#[test]
fn create_project_from_request_rejects_existing_path_without_force() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("winter-regency");

    create_project_from_request(&project, sample_project_creation_request(), false)
        .expect("first create");
    let error = create_project_from_request(&project, sample_project_creation_request(), false)
        .expect_err("force=false should reject existing project");

    assert!(matches!(error, StorageError::ProjectExists(_)));
}

#[test]
fn create_project_from_request_rejects_secret_markers() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("winter-regency");
    let mut request = sample_project_creation_request();
    request.concept = "A court drama OPENAI_API_KEY=sk-test-secret-marker".into();

    let error = create_project_from_request(&project, request, false)
        .expect_err("secret marker should fail explicitly");

    assert!(matches!(
        error,
        StorageError::InvalidProjectCreationRequest { field, reason }
            if field == "concept" && reason.contains("secret markers")
    ));
    assert!(!project.join("game.toml").exists());
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
        current_beat_id: Some("court-crisis-001-beat-001".into()),
        completed_scene_keys: Vec::new(),
        turn: 1,
    };
    let trace = RuntimeTrace {
        id: "trace-test".into(),
        timestamp_ms: 1,
        reproducibility: ReproducibilityMetadata::local_mock(7).with_trace_id("trace-test"),
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
fn sqlite_cache_migrates_and_rebuilds_from_folder_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");
    write_trace(&project, &sample_runtime_trace("trace-cache-001")).expect("write trace");

    let summary = rebuild_sqlite_cache(&project).expect("rebuild cache");

    assert_eq!(summary.schema_version, SQLITE_CACHE_SCHEMA_VERSION);
    assert_eq!(summary.project_id, "dynasty-embers");
    assert_eq!(summary.title, "Dynasty Embers");
    assert_eq!(summary.scene_count, 1);
    assert_eq!(summary.asset_count, 1);
    assert_eq!(summary.trace_count, 1);
    assert!(summary.source_file_count > 0);
    assert_eq!(
        read_sqlite_cache_summary(&project).expect("read cache summary"),
        Some(summary)
    );

    let connection = Connection::open(sqlite_cache_path(&project)).expect("open sqlite");
    let user_version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("user version");
    let migration_count: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            [SQLITE_CACHE_SCHEMA_VERSION],
            |row| row.get(0),
        )
        .expect("migration count");
    let source_file_count: u32 = connection
        .query_row("SELECT COUNT(*) FROM source_files", [], |row| row.get(0))
        .expect("source file count");
    let image_asset_count: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM assets WHERE kind = 'image' AND source = 'generated'",
            [],
            |row| row.get(0),
        )
        .expect("asset count");

    assert_eq!(user_version, SQLITE_CACHE_SCHEMA_VERSION);
    assert_eq!(migration_count, 1);
    assert!(source_file_count > 0);
    assert_eq!(image_asset_count, 1);
}

#[test]
fn project_loads_without_sqlite_cache() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");

    assert!(!sqlite_cache_path(&project).exists());
    assert_eq!(
        read_sqlite_cache_summary(&project).expect("cache miss"),
        None
    );
    assert_eq!(
        load_project(&project)
            .expect("load without cache")
            .game
            .title,
        "Dynasty Embers"
    );
}

#[test]
fn folder_source_takes_precedence_over_stale_sqlite_cache() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("dynasty-embers");
    create_demo_project(&project, false).expect("create");
    let original = rebuild_sqlite_cache(&project).expect("initial rebuild");
    assert_eq!(original.title, "Dynasty Embers");

    let game_path = project.join("game.toml");
    let game_toml = fs::read_to_string(&game_path).expect("read game");
    fs::write(
        &game_path,
        game_toml.replace("title = \"Dynasty Embers\"", "title = \"Cache Ignored\""),
    )
    .expect("write game");

    assert_eq!(
        load_project(&project).expect("load project").game.title,
        "Cache Ignored"
    );
    assert_eq!(
        read_sqlite_cache_summary(&project)
            .expect("read stale cache")
            .expect("stale cache exists")
            .title,
        "Dynasty Embers"
    );
    assert_eq!(
        rebuild_sqlite_cache(&project).expect("rebuild cache").title,
        "Cache Ignored"
    );
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
        reproducibility: ReproducibilityMetadata::local_mock(project.game.run_seed)
            .with_snapshot_id(id),
        project_id: project.game.id,
        project_version: project.game.version,
        story_state: project.story_state,
        world_state: project.world_state,
        scenes: project.scenes,
    }
}

fn sample_runtime_trace(id: &str) -> RuntimeTrace {
    let story = StoryState {
        current_scene_key: "court-crisis-001".into(),
        current_beat_id: Some("court-crisis-001-beat-001".into()),
        completed_scene_keys: Vec::new(),
        turn: 1,
    };
    RuntimeTrace {
        id: id.into(),
        timestamp_ms: 1,
        reproducibility: ReproducibilityMetadata::local_mock(7).with_trace_id(id),
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
    }
}

fn sample_generation_evidence(status: GenerationStatus) -> GenerationEvidence {
    let fallback_used = matches!(status, GenerationStatus::Fallback);
    GenerationEvidence {
        status,
        fallback_used,
        error: None,
        reproducibility: ReproducibilityMetadata::local_mock(7),
        envelopes: Vec::new(),
    }
}

fn assert_fixture_files_match_generated_demo(committed_fixture: &Path, generated_fixture: &Path) {
    let committed_manifest = source_file_manifest(committed_fixture);
    let mut generated_manifest = source_file_manifest(generated_fixture);
    for optional_path in [PathBuf::from("safety/ai_safety_policy.toml")] {
        if !committed_manifest.contains_key(&optional_path) {
            generated_manifest.remove(&optional_path);
        }
    }
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

fn sample_project_creation_request() -> ProjectCreationRequest {
    ProjectCreationRequest {
        template: ProjectTemplateId::HistoricalCrisis,
        concept: "A regency court must survive a winter coup.".into(),
        visual_style: "ink wash court drama".into(),
        voice_enabled: true,
        initial_scene_request: "Open on an empty granary ledger.".into(),
    }
}

fn sample_character(id: &str) -> Character {
    Character {
        id: id.into(),
        name: "Regent".into(),
        role: "Temporary court authority".into(),
        traits: vec!["cautious".into(), "clear".into()],
        visual_card: "ink portrait with winter robes".into(),
        voice_card: "measured court speech".into(),
        portrait_request: None,
    }
}

fn sample_rule(id: &str, resource_key: &str) -> Rule {
    Rule {
        id: id.into(),
        action_type: id.replace('-', "_"),
        conditions: Vec::new(),
        effects: vec![Effect::AddResource {
            key: resource_key.into(),
            amount: -3,
        }],
    }
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
