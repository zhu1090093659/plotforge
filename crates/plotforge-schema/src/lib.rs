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
}
