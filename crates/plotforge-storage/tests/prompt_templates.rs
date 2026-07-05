use plotforge_schema::{PromptScope, PromptTemplate};
use plotforge_storage::{
    read_project_prompt_templates, read_user_prompt_templates_from, write_project_prompt_templates,
    write_user_prompt_templates, write_user_prompt_templates_to,
};
use tempfile::tempdir;

#[test]
fn project_prompt_templates_roundtrip_under_plotforge_dir() {
    let temp = tempdir().expect("tempdir");
    let templates = vec![
        PromptTemplate {
            id: "scene-pacing".into(),
            label: "Scene pacing".into(),
            scope: PromptScope::Project,
            body_markdown: "Prefer concrete beats over exposition dumps.".into(),
            default_role_hint: Some("scene_planner".into()),
        },
        PromptTemplate {
            id: "voice-card".into(),
            label: "Voice card".into(),
            scope: PromptScope::Project,
            body_markdown: "Keep the witness anxious.".into(),
            default_role_hint: None,
        },
    ];

    // Fresh project returns an empty list, not an error.
    let initial = read_project_prompt_templates(temp.path()).expect("initial read");
    assert!(initial.is_empty());

    write_project_prompt_templates(temp.path(), &templates).expect("write project prompts");
    let prompts_path = temp.path().join(".plotforge").join("prompts.json");
    assert!(
        prompts_path.exists(),
        "prompts.json should be written under .plotforge/"
    );

    let loaded = read_project_prompt_templates(temp.path()).expect("read back");
    assert_eq!(loaded, templates);
    assert_eq!(loaded.len(), 2);

    // Redaction safety: the persisted file must not contain secret markers
    // even if a template body somehow leaked one — the writer scans and
    // rejects before persisting.
    let content = std::fs::read_to_string(&prompts_path).expect("read file content");
    assert!(!content.contains("api_key"));
    assert!(!content.contains("sk-"));
}

#[test]
fn project_prompt_templates_reject_secret_marker_body() {
    let temp = tempdir().expect("tempdir");
    let templates = vec![PromptTemplate {
        id: "leaky".into(),
        label: "Leaky".into(),
        scope: PromptScope::Project,
        body_markdown: "Use api_key sk-test-secret-marker here.".into(),
        default_role_hint: None,
    }];
    let error = write_project_prompt_templates(temp.path(), &templates)
        .expect_err("secret marker should be rejected");
    assert!(matches!(
        error,
        plotforge_storage::StorageError::SecretMarker { .. }
    ));
    // The .plotforge/prompts.json file must not have been written.
    assert!(!temp.path().join(".plotforge").join("prompts.json").exists());
}

#[test]
fn user_prompt_templates_reject_secret_marker_body_without_writing() {
    // The user-global writer resolves the real user config dir; we cannot
    // safely redirect it in a unit test without env mutation (unsafe in
    // edition 2024). What we *can* assert is the secret-marker scan runs
    // before any filesystem touch: a body containing a secret marker must
    // error out of `write_user_prompt_templates` before opening any file.
    let templates = vec![PromptTemplate {
        id: "leaky".into(),
        label: "Leaky".into(),
        scope: PromptScope::User,
        body_markdown: "sk-test-secret-marker leak".into(),
        default_role_hint: None,
    }];
    let error = write_user_prompt_templates(&templates)
        .expect_err("secret marker should be rejected before any write");
    assert!(matches!(
        error,
        plotforge_storage::StorageError::SecretMarker { .. }
    ));
}

// R4: the positive user-global write/roundtrip path. The path-injected
// `_to`/`_from` variants let us exercise the real writer (secret-marker
// scan + create_dir_all + JSON write + read back) without mutating the
// real user config dir or setting `HOME` (unsafe under parallel tests).
#[test]
fn user_prompt_templates_roundtrip_via_path_injected_variants() {
    let temp = tempdir().expect("tempdir");
    let prompts_path = temp.path().join("prompts.json");
    let templates = vec![
        PromptTemplate {
            id: "user-scene-planner".into(),
            label: "User scene planner".into(),
            scope: PromptScope::User,
            body_markdown: "# Scene planner prompt\nPlan a scene with visible pressure.".into(),
            default_role_hint: Some("scene_planner".into()),
        },
        PromptTemplate {
            id: "user-voice".into(),
            label: "User voice".into(),
            scope: PromptScope::User,
            body_markdown: "Keep voices terse and specific.".into(),
            default_role_hint: None,
        },
    ];
    // Fresh path returns empty, not an error.
    let initial = read_user_prompt_templates_from(&prompts_path).expect("initial read");
    assert!(initial.is_empty());

    write_user_prompt_templates_to(&prompts_path, &templates).expect("write user prompts");
    assert!(prompts_path.exists(), "prompts.json must be written");

    let loaded = read_user_prompt_templates_from(&prompts_path).expect("read back");
    assert_eq!(loaded, templates);
    assert_eq!(loaded.len(), 2);

    // Redaction safety: no secret markers in the persisted file.
    let content = std::fs::read_to_string(&prompts_path).expect("read file content");
    assert!(!content.contains("api_key"));
    assert!(!content.contains("sk-"));
}
