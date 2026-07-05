//! Prompt template schema contracts.
//!
//! Prompt templates are reusable, redaction-safe prompt bodies scoped to
//! either the user-global library (`~/.plotforge/prompts.json`, shared across
//! projects) or a single project (`<project>/.plotforge/prompts.json`). They
//! never carry provider credentials, raw provider responses, or secret
//! markers. Project-scoped templates stay under `.plotforge/` and are blocked
//! from export packages by the export allowlist and the workshop denylist.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The scope a prompt template lives in. `User` templates are shared across
/// every project on this machine; `Project` templates are private to one
/// project's `.plotforge/prompts.json`.
#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromptScope {
    #[default]
    User,
    Project,
}

/// A reusable prompt template. `body_markdown` is the prompt body the
/// pi-Agent splices into its system prompt; it must not contain secret
/// markers (the export layer scans for them).
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PromptTemplate {
    pub id: String,
    pub label: String,
    pub scope: PromptScope,
    pub body_markdown: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_role_hint: Option<String>,
}

/// The persisted file shape for both user-global and project-scoped prompt
/// template stores.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct PromptTemplateFile {
    pub templates: Vec<PromptTemplate>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_scope_renames_snake_case() {
        assert_eq!(
            serde_json::to_string(&PromptScope::User).unwrap(),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&PromptScope::Project).unwrap(),
            "\"project\""
        );
        let decoded: PromptScope = serde_json::from_str("\"project\"").unwrap();
        assert_eq!(decoded, PromptScope::Project);
    }

    #[test]
    fn prompt_template_roundtrips_json() {
        let template = PromptTemplate {
            id: "scene-pacing".into(),
            label: "Scene pacing guardrails".into(),
            scope: PromptScope::User,
            body_markdown: "Prefer concrete beats over exposition dumps.".into(),
            default_role_hint: Some("scene_planner".into()),
        };
        let encoded = serde_json::to_string_pretty(&template).expect("serialize template");
        let decoded: PromptTemplate = serde_json::from_str(&encoded).expect("deserialize template");
        assert_eq!(decoded, template);
    }

    #[test]
    fn prompt_template_rejects_secret_fields() {
        let template = PromptTemplate {
            id: "leaky".into(),
            label: "Leaky".into(),
            scope: PromptScope::Project,
            body_markdown: "benign body".into(),
            default_role_hint: None,
        };
        let mut value = serde_json::to_value(&template).expect("template value");
        value["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<PromptTemplate>(value)
            .expect_err("api_key field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn prompt_template_file_roundtrips_json() {
        let file = PromptTemplateFile {
            templates: vec![
                PromptTemplate {
                    id: "user-a".into(),
                    label: "A".into(),
                    scope: PromptScope::User,
                    body_markdown: "body a".into(),
                    default_role_hint: None,
                },
                PromptTemplate {
                    id: "proj-b".into(),
                    label: "B".into(),
                    scope: PromptScope::Project,
                    body_markdown: "body b".into(),
                    default_role_hint: Some("story_craft_planner".into()),
                },
            ],
        };
        let encoded = serde_json::to_string_pretty(&file).expect("serialize file");
        let decoded: PromptTemplateFile = serde_json::from_str(&encoded).expect("deserialize file");
        assert_eq!(decoded, file);
        assert_eq!(decoded.templates.len(), 2);
    }

    #[test]
    fn prompt_template_file_default_is_empty() {
        assert!(PromptTemplateFile::default().templates.is_empty());
    }
}
