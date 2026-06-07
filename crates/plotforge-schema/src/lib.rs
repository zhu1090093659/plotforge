use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub type ResourceMap = BTreeMap<String, i32>;
pub type FlagMap = BTreeMap<String, bool>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameProject {
    pub id: String,
    pub title: String,
    pub version: String,
    pub description: String,
    pub entry_scene: String,
    pub run_seed: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceDefinition {
    pub key: String,
    pub label: String,
    pub initial: i32,
    pub min: i32,
    pub max: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldState {
    pub resources: ResourceMap,
    pub flags: FlagMap,
    pub triggered_events: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldDelta {
    pub resource_changes: ResourceMap,
    pub resource_sets: ResourceMap,
    pub flags: FlagMap,
    pub triggered_events: Vec<String>,
}

impl WorldDelta {
    pub fn is_empty(&self) -> bool {
        self.resource_changes.is_empty()
            && self.resource_sets.is_empty()
            && self.flags.is_empty()
            && self.triggered_events.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryState {
    pub current_scene_key: String,
    pub completed_scene_keys: Vec<String>,
    pub turn: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryCraftBible {
    pub genre_promise: String,
    pub central_question: String,
    pub target_emotions: Vec<String>,
    pub core_foreshadowing: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlotThread {
    pub id: String,
    pub title: String,
    pub promise: String,
    pub status: PlotThreadStatus,
    pub last_update: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlotThreadStatus {
    Open,
    Escalating,
    Resolved,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmotionalArcPoint {
    pub scene_key: String,
    pub target_emotion: String,
    pub intensity: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryCraftState {
    pub bible: StoryCraftBible,
    pub emotional_arc: Vec<EmotionalArcPoint>,
    pub plot_threads: Vec<PlotThread>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub role: String,
    pub traits: Vec<String>,
    pub visual_card: String,
    pub voice_card: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scene {
    pub key: String,
    pub title: String,
    pub location: String,
    pub dramatic_purpose: String,
    pub hook: String,
    pub background_asset: String,
    pub character_ids: Vec<String>,
    pub plot_thread_updates: BTreeMap<String, String>,
    pub beats: Vec<Beat>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Beat {
    pub id: String,
    pub text: String,
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub action_type: String,
    pub dramatic_purpose: String,
    pub change_scene: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionIntent {
    pub status: ActionIntentStatus,
    pub action_type: Option<String>,
    pub matched_terms: Vec<String>,
    pub reason: Option<String>,
}

impl ActionIntent {
    pub fn supported(action_type: impl Into<String>, matched_terms: Vec<String>) -> Self {
        Self {
            status: ActionIntentStatus::Supported,
            action_type: Some(action_type.into()),
            matched_terms,
            reason: None,
        }
    }

    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            status: ActionIntentStatus::Unsupported,
            action_type: None,
            matched_terms: Vec::new(),
            reason: Some(reason.into()),
        }
    }

    pub fn action_type(&self) -> Option<&str> {
        self.action_type.as_deref()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionIntentStatus {
    Supported,
    Unsupported,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rule {
    pub id: String,
    pub action_type: String,
    pub conditions: Vec<Condition>,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Condition {
    ResourceAtLeast { key: String, value: i32 },
    ResourceAtMost { key: String, value: i32 },
    FlagEquals { key: String, value: bool },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Effect {
    AddResource { key: String, amount: i32 },
    SetResource { key: String, value: i32 },
    SetFlag { key: String, value: bool },
    TriggerEvent { event: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NarrativeReview {
    pub scene_key: String,
    pub score: u8,
    pub issues: Vec<NarrativeIssue>,
}

impl NarrativeReview {
    pub fn passes(&self) -> bool {
        self.issues
            .iter()
            .all(|issue| issue.severity != Severity::Error)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NarrativeIssue {
    pub kind: NarrativeIssueKind,
    pub severity: Severity,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeIssueKind {
    WeakHook,
    NoProgress,
    FakeChoice,
    OutOfCharacter,
    BrokenPlotThread,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTrace {
    pub id: String,
    pub timestamp_ms: u64,
    pub player_input: Option<String>,
    pub selected_choice: Option<String>,
    pub world_state_before: WorldState,
    pub world_state_delta: WorldDelta,
    pub world_state_after: WorldState,
    pub story_state_before: StoryState,
    pub story_state_after: StoryState,
    pub narrative_review: Option<NarrativeReview>,
    pub errors: Vec<RuntimeError>,
    pub fallback_used: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectData {
    pub game: GameProject,
    pub resources: Vec<ResourceDefinition>,
    pub world_state: WorldState,
    pub story_state: StoryState,
    pub story_craft: StoryCraftState,
    pub characters: Vec<Character>,
    pub rules: Vec<Rule>,
    pub scenes: Vec<Scene>,
}

impl ProjectData {
    pub fn scene(&self, key: &str) -> Option<&Scene> {
        self.scenes.iter().find(|scene| scene.key == key)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportManifest {
    pub game: GameProject,
    pub entry_scene: String,
    pub scenes: Vec<Scene>,
    pub assets: Vec<String>,
    pub generated_by: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_project_roundtrips_json() {
        let game = GameProject {
            id: "dynasty-embers".into(),
            title: "Dynasty Embers".into(),
            version: "0.1.0".into(),
            description: "A historical crisis simulation.".into(),
            entry_scene: "court-crisis-001".into(),
            run_seed: 7,
        };

        let encoded = serde_json::to_string(&game).expect("serialize game");
        let decoded: GameProject = serde_json::from_str(&encoded).expect("deserialize game");
        assert_eq!(decoded, game);
    }

    #[test]
    fn action_intent_roundtrips_json() {
        let intent = ActionIntent::supported("raise_tax", vec!["加征".into(), "辽饷".into()]);

        let encoded = serde_json::to_string(&intent).expect("serialize intent");
        let decoded: ActionIntent = serde_json::from_str(&encoded).expect("deserialize intent");

        assert_eq!(decoded, intent);
        assert_eq!(decoded.status, ActionIntentStatus::Supported);
        assert_eq!(decoded.action_type(), Some("raise_tax"));
    }

    #[test]
    fn runtime_trace_roundtrips_json() {
        let story_state = StoryState {
            current_scene_key: "court-crisis-001".into(),
            completed_scene_keys: Vec::new(),
            turn: 0,
        };
        let trace = RuntimeTrace {
            id: "trace-1".into(),
            timestamp_ms: 1,
            player_input: Some("raise taxes".into()),
            selected_choice: Some("raise_tax".into()),
            world_state_before: WorldState::default(),
            world_state_delta: WorldDelta::default(),
            world_state_after: WorldState::default(),
            story_state_before: story_state.clone(),
            story_state_after: story_state,
            narrative_review: None,
            errors: Vec::new(),
            fallback_used: false,
        };

        let encoded = serde_json::to_string_pretty(&trace).expect("serialize trace");
        let decoded: RuntimeTrace = serde_json::from_str(&encoded).expect("deserialize trace");
        assert_eq!(decoded, trace);
    }

    #[test]
    fn project_data_and_export_manifest_roundtrip_json() {
        let project = ProjectData {
            game: GameProject {
                id: "dynasty-embers".into(),
                title: "Dynasty Embers".into(),
                version: "0.1.0".into(),
                description: "Demo".into(),
                entry_scene: "court-crisis-001".into(),
                run_seed: 7,
            },
            resources: vec![ResourceDefinition {
                key: "treasury".into(),
                label: "Treasury".into(),
                initial: 40,
                min: 0,
                max: 100,
            }],
            world_state: WorldState::default(),
            story_state: StoryState {
                current_scene_key: "court-crisis-001".into(),
                completed_scene_keys: Vec::new(),
                turn: 0,
            },
            story_craft: StoryCraftState {
                bible: StoryCraftBible {
                    genre_promise: "history".into(),
                    central_question: "survive?".into(),
                    target_emotions: vec!["pressure".into()],
                    core_foreshadowing: vec!["ledger".into()],
                },
                emotional_arc: Vec::new(),
                plot_threads: Vec::new(),
            },
            characters: Vec::new(),
            rules: vec![Rule {
                id: "raise-tax".into(),
                action_type: "raise_tax".into(),
                conditions: vec![Condition::ResourceAtMost {
                    key: "treasury".into(),
                    value: 80,
                }],
                effects: vec![Effect::TriggerEvent {
                    event: "local_tax_resistance".into(),
                }],
            }],
            scenes: Vec::new(),
        };
        let manifest = ExportManifest {
            game: project.game.clone(),
            entry_scene: project.story_state.current_scene_key.clone(),
            scenes: project.scenes.clone(),
            assets: vec!["assets/generated/placeholder.png".into()],
            generated_by: "test".into(),
        };

        let project_json = serde_json::to_string(&project).expect("serialize project");
        let manifest_json = serde_json::to_string(&manifest).expect("serialize manifest");
        assert_eq!(
            serde_json::from_str::<ProjectData>(&project_json).expect("deserialize project"),
            project
        );
        assert_eq!(
            serde_json::from_str::<ExportManifest>(&manifest_json).expect("deserialize manifest"),
            manifest
        );
    }

    #[test]
    fn tagged_rule_enums_use_snake_case_contracts() {
        let condition = serde_json::to_value(Condition::FlagEquals {
            key: "tax_resistance".into(),
            value: true,
        })
        .expect("serialize condition");
        let effect = serde_json::to_value(Effect::SetResource {
            key: "treasury".into(),
            value: 12,
        })
        .expect("serialize effect");

        assert_eq!(condition["kind"], "flag_equals");
        assert_eq!(effect["kind"], "set_resource");
    }
}
