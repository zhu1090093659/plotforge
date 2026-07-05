//! Skill library schema contracts.
//!
//! Skills are modular, self-contained folders that extend the pi-Agent with
//! specialized knowledge, workflows, and bundled resources. PlotForge adopts
//! the Claude Code / Codex SKILL.md format: every skill is a directory with a
//! `SKILL.md` (YAML frontmatter `name`+`description` + Markdown body) and
//! optional `scripts/`, `references/`, `assets/`, and `agents/openai.yaml`
//! resources.
//!
//! These contracts describe the *metadata* surface only — the index that
//! lives at `~/.plotforge/skill-index.json` after a scan. Skill bodies are
//! loaded on demand (progressive disclosure) and are never stored in the
//! index, in traces, or in export packages. The index never carries
//! credentials or raw provider responses.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The parsed YAML frontmatter of a `SKILL.md`. Parsing is deliberately
/// lenient: external skills may carry `version`, `license`, `references`, or
/// other fields we do not yet recognize; those are retained in `metadata`
/// rather than rejected.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
}

/// The `interface` block of an `agents/openai.yaml`, surfaced as UI metadata
/// for skill chips and lists. All fields are optional; missing skills fall
/// back to the frontmatter `name`/`description`.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct SkillInterface {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_small: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_large: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand_color: Option<String>,
}

/// Which external agent ecosystem a skill was discovered from. `PlotForgeUser`
/// is the user's own `~/.plotforge/skills/` library; `PlotForgeProject` is a
/// project-local skills directory. The other variants name external agent
/// roots that PlotForge auto-discovers but never mutates. `Other` is an
/// untagged escape hatch so new external roots can be carried without a
/// schema-breaking change; it serializes as a plain string.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillOrigin {
    PlotForgeUser,
    PlotForgeProject,
    ClaudeCode,
    Codex,
    ZCode,
    Cursor,
    Copilot,
    Hanako,
    OpenClaw,
    Workbuddy,
    Redbox,
    #[serde(untagged)]
    Other(String),
}

/// Where a skill lives on disk. `root_path` is the scanned root (e.g.
/// `~/.claude/skills`); `rel_path` is the skill folder relative to that
/// root (e.g. `my-skill`).
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SkillSource {
    pub origin: SkillOrigin,
    pub root_path: String,
    pub rel_path: String,
}

/// The metadata snapshot for one discovered skill. Only paths and frontmatter
/// are stored — the body and bundled resources are loaded on demand by the
/// agent crate. `id` is the frontmatter `name` (the canonical skill id).
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SkillManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: SkillSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<SkillInterface>,
    pub body_path: String,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub references: Vec<String>,
    #[serde(default)]
    pub assets: Vec<String>,
}

/// The persisted scan cache at `~/.plotforge/skill-index.json`. `scanned_at`
/// is an ISO-8601 timestamp; the agent crate refreshes the cache lazily when
/// it is missing or stale.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct SkillIndex {
    pub version: String,
    pub skills: Vec<SkillManifest>,
    pub scanned_at: String,
}

impl SkillIndex {
    /// An empty index used when no cache exists yet. The scan is opt-in via
    /// `refresh_skill_index`, so the first `list_skills` call returns this.
    pub fn empty() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_frontmatter_roundtrips_json() {
        let fm = SkillFrontmatter {
            name: "skill-creator".into(),
            description: "Guide for creating effective skills.".into(),
            version: Some("1.0".into()),
            metadata: Some({
                let mut m = BTreeMap::new();
                m.insert(
                    "short-description".into(),
                    "Create or update a skill".into(),
                );
                m
            }),
        };
        let encoded = serde_json::to_string_pretty(&fm).expect("serialize frontmatter");
        let decoded: SkillFrontmatter =
            serde_json::from_str(&encoded).expect("deserialize frontmatter");
        assert_eq!(decoded, fm);
    }

    #[test]
    fn skill_frontmatter_accepts_unknown_fields() {
        // External skills carry version/license/references we may not model.
        // Lenient parsing must keep those without failing.
        let raw = r#"{"name":"x","description":"d","license":"MIT","references":["a"]}"#;
        let fm: SkillFrontmatter = serde_json::from_str(raw).expect("lenient parse");
        assert_eq!(fm.name, "x");
        assert_eq!(fm.description, "d");
        assert!(fm.version.is_none());
    }

    #[test]
    fn skill_interface_roundtrips_json() {
        let interface = SkillInterface {
            display_name: Some("Skill Creator".into()),
            short_description: Some("Create or update a skill".into()),
            default_prompt: Some("Use $skill-creator to ...".into()),
            icon_small: None,
            icon_large: None,
            brand_color: None,
        };
        let encoded = serde_json::to_string_pretty(&interface).expect("serialize interface");
        let decoded: SkillInterface =
            serde_json::from_str(&encoded).expect("deserialize interface");
        assert_eq!(decoded, interface);
    }

    #[test]
    fn skill_origin_renames_snake_case() {
        assert_eq!(
            serde_json::to_string(&SkillOrigin::ClaudeCode).unwrap(),
            "\"claude_code\""
        );
        assert_eq!(
            serde_json::to_string(&SkillOrigin::PlotForgeUser).unwrap(),
            "\"plot_forge_user\""
        );
        assert_eq!(
            serde_json::to_string(&SkillOrigin::Other("custom".into())).unwrap(),
            "\"custom\""
        );
        let decoded: SkillOrigin = serde_json::from_str("\"cursor\"").unwrap();
        assert_eq!(decoded, SkillOrigin::Cursor);
        let decoded: SkillOrigin = serde_json::from_str("\"some_new_agent\"").unwrap();
        assert_eq!(decoded, SkillOrigin::Other("some_new_agent".into()));
    }

    #[test]
    fn skill_manifest_roundtrips_json() {
        let manifest = SkillManifest {
            id: "frontend-design".into(),
            name: "frontend-design".into(),
            description: "Frontend design guardrails.".into(),
            source: SkillSource {
                origin: SkillOrigin::ClaudeCode,
                root_path: "/Users/x/.claude/skills".into(),
                rel_path: "frontend-design".into(),
            },
            interface: Some(SkillInterface {
                display_name: Some("Frontend Design".into()),
                short_description: Some("Frontend design".into()),
                default_prompt: None,
                icon_small: None,
                icon_large: None,
                brand_color: None,
            }),
            body_path: "frontend-design/SKILL.md".into(),
            scripts: Vec::new(),
            references: vec!["frontend-design/reference/typography.md".into()],
            assets: Vec::new(),
        };
        let encoded = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
        let decoded: SkillManifest = serde_json::from_str(&encoded).expect("deserialize manifest");
        assert_eq!(decoded, manifest);
    }

    #[test]
    fn skill_manifest_rejects_secret_fields() {
        let manifest = SkillManifest {
            id: "x".into(),
            name: "x".into(),
            description: "d".into(),
            source: SkillSource {
                origin: SkillOrigin::PlotForgeUser,
                root_path: "/tmp".into(),
                rel_path: "x".into(),
            },
            interface: None,
            body_path: "x/SKILL.md".into(),
            scripts: Vec::new(),
            references: Vec::new(),
            assets: Vec::new(),
        };
        let mut value = serde_json::to_value(&manifest).expect("manifest value");
        value["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<SkillManifest>(value)
            .expect_err("api_key field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn skill_index_roundtrips_json() {
        let index = SkillIndex {
            version: "1".into(),
            skills: Vec::new(),
            scanned_at: "2026-07-05T00:00:00Z".into(),
        };
        let encoded = serde_json::to_string_pretty(&index).expect("serialize index");
        let decoded: SkillIndex = serde_json::from_str(&encoded).expect("deserialize index");
        assert_eq!(decoded, index);
    }
}
