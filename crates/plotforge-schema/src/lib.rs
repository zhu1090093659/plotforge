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

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryCraftBible {
    #[serde(default)]
    pub target_audience: Option<String>,
    pub genre_promise: String,
    pub central_question: String,
    pub target_emotions: Vec<String>,
    pub core_foreshadowing: Vec<String>,
    #[serde(default)]
    pub emotional_contract: Vec<String>,
    #[serde(default)]
    pub pacing_profile: PacingProfile,
    #[serde(default)]
    pub hook_strategy: HookStrategy,
    #[serde(default)]
    pub reversal_strategy: Option<ReversalStrategy>,
    #[serde(default)]
    pub prose_style_guide: Option<String>,
    #[serde(default)]
    pub banned_cliches: Vec<String>,
    #[serde(default)]
    pub reference_modules: Vec<ReferenceModule>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacingProfile {
    pub escalation_interval_scenes: u8,
    pub target_tension_curve: Vec<u8>,
    pub breather_scene_frequency: Option<u8>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookStrategy {
    pub primary_hook: String,
    pub recurring_hook_patterns: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReversalStrategy {
    pub cadence_scenes: u8,
    pub principle: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReferenceModule {
    pub id: String,
    pub title: String,
    pub summary: String,
}

pub const MAX_REFERENCE_SUMMARY_CHARS: usize = 600;
pub const MAX_REFERENCE_STRUCTURE_NOTE_CHARS: usize = 300;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceAnalysis {
    pub id: String,
    pub title: String,
    pub source: ReferenceSource,
    pub summary: String,
    #[serde(default)]
    pub structure_notes: Vec<ReferenceStructureNote>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSource {
    pub source_type: ReferenceSourceType,
    pub rights: ReferenceRights,
    pub citation: String,
    pub user_authorized: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceSourceType {
    UserImport,
    PublicDomain,
    OpenLicense,
    MethodTemplate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceRights {
    UserOwned,
    UserAuthorized,
    PublicDomain,
    OpenLicense,
    GenericMethod,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceStructureNote {
    pub label: String,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryPromise {
    pub id: String,
    pub text: String,
    pub status: StoryPromiseStatus,
    pub introduced_at: String,
    pub payoff_hint: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StoryPromiseStatus {
    #[default]
    Active,
    Complicated,
    PaidOff,
    Dropped,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlotThread {
    pub id: String,
    pub title: String,
    pub promise: String,
    #[serde(default)]
    pub thread_type: PlotThreadType,
    pub status: PlotThreadStatus,
    #[serde(default)]
    pub introduced_at: String,
    #[serde(default)]
    pub expected_payoff: Option<String>,
    #[serde(default)]
    pub related_characters: Vec<String>,
    #[serde(default)]
    pub related_world_flags: Vec<String>,
    pub last_update: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlotThreadType {
    Mystery,
    Foreshadow,
    Relationship,
    Political,
    Survival,
    #[default]
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlotThreadStatus {
    Open,
    Escalating,
    Resolved,
    Deepened,
    PaidOff,
    Abandoned,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmotionalArcPoint {
    pub scene_key: String,
    pub target_emotion: String,
    pub intensity: u8,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryCraftState {
    pub bible: StoryCraftBible,
    #[serde(default)]
    pub active_promises: Vec<StoryPromise>,
    pub emotional_arc: Vec<EmotionalArcPoint>,
    pub plot_threads: Vec<PlotThread>,
    #[serde(default)]
    pub character_arcs: Vec<CharacterArc>,
    #[serde(default)]
    pub pacing_score: Option<u8>,
    #[serde(default)]
    pub tension_score: Option<u8>,
    #[serde(default)]
    pub ai_slop_risk: Option<u8>,
    #[serde(default)]
    pub review_notes: Vec<NarrativeReviewNote>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterArc {
    pub id: String,
    pub character_id: String,
    pub desire: String,
    pub pressure: String,
    pub current_state: String,
    pub target_state: String,
    pub status: CharacterArcStatus,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CharacterArcStatus {
    #[default]
    Setup,
    Pressured,
    Changed,
    Resolved,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NarrativeReviewNote {
    pub id: String,
    pub scene_key: Option<String>,
    pub severity: Severity,
    pub message: String,
    pub resolved: bool,
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
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    StoryArchitect,
    StoryCraftPlanner,
    ScenePlanner,
    BeatWriter,
    PlotDoctor,
    ConsistencyChecker,
    DeslopRefiner,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentOutputProposal {
    pub id: String,
    pub agent: AgentRole,
    pub output: AgentProposalPayload,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AgentProposalPayload {
    ScenePlan(ScenePlanProposal),
    BeatDrafts(BeatDraftsProposal),
    Review(ReviewProposal),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenePlanProposal {
    pub scene_key: String,
    pub title: String,
    pub location: String,
    pub scene_summary: String,
    pub dramatic_purpose: String,
    pub hook: String,
    pub emotional_goal: Option<String>,
    pub cast: Vec<String>,
    pub entry_beat_id: String,
    pub background_asset: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BeatDraftsProposal {
    pub scene_key: String,
    #[serde(default)]
    pub beats: Vec<BeatDraftProposal>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BeatDraftProposal {
    pub id: String,
    pub scene_key: String,
    pub text: String,
    pub choices: Vec<Choice>,
    pub narrative_function: NarrativeFunction,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeFunction {
    Hook,
    Setup,
    Payoff,
    Reversal,
    Choice,
    Cliffhanger,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewProposal {
    pub scene_key: String,
    pub review: NarrativeReview,
    #[serde(default)]
    pub notes: Vec<NarrativeReviewNote>,
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

    pub fn redacted(&self) -> Self {
        Self {
            status: self.status.clone(),
            action_type: self
                .action_type
                .as_ref()
                .map(|action_type| redact_trace_text(action_type)),
            matched_terms: self
                .matched_terms
                .iter()
                .map(|term| redact_trace_text(term))
                .collect(),
            reason: self.reason.as_ref().map(|reason| redact_trace_text(reason)),
        }
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
    #[serde(default = "default_review_score")]
    pub hook_score: u8,
    #[serde(default = "default_review_score")]
    pub pacing_score: u8,
    #[serde(default = "default_review_score")]
    pub character_consistency_score: u8,
    #[serde(default = "default_review_score")]
    pub payoff_score: u8,
    #[serde(default = "default_review_score")]
    pub choice_meaningfulness_score: u8,
    #[serde(default)]
    pub ai_slop_risk: u8,
    pub issues: Vec<NarrativeIssue>,
}

impl NarrativeReview {
    pub fn passes(&self) -> bool {
        self.issues
            .iter()
            .all(|issue| issue.severity != Severity::Error)
    }

    pub fn redacted(&self) -> Self {
        Self {
            scene_key: redact_trace_text(&self.scene_key),
            score: self.score,
            hook_score: self.hook_score,
            pacing_score: self.pacing_score,
            character_consistency_score: self.character_consistency_score,
            payoff_score: self.payoff_score,
            choice_meaningfulness_score: self.choice_meaningfulness_score,
            ai_slop_risk: self.ai_slop_risk,
            issues: self.issues.iter().map(NarrativeIssue::redacted).collect(),
        }
    }
}

fn default_review_score() -> u8 {
    100
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NarrativeIssue {
    pub kind: NarrativeIssueKind,
    pub severity: Severity,
    pub message: String,
}

impl NarrativeIssue {
    pub fn redacted(&self) -> Self {
        Self {
            kind: self.kind.clone(),
            severity: self.severity.clone(),
            message: redact_trace_text(&self.message),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeIssueKind {
    WeakHook,
    NoProgress,
    FakeChoice,
    OutOfCharacter,
    BrokenPlotThread,
    ThreadForgotten,
    WorldStateIgnored,
    TooMuchExposition,
    AiSlopStyle,
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

impl RuntimeError {
    pub fn redacted(code: impl Into<String>, message: impl Into<String>) -> Self {
        let code = code.into();
        let message = message.into();
        Self {
            code: redact_trace_text(&code),
            message: redact_trace_text(&message),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeRuleResult {
    pub action_type: String,
    pub delta_empty: bool,
    pub state_committed: bool,
    pub error: Option<RuntimeError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimePlannerResult {
    pub requested_action_type: String,
    pub scene_key: Option<String>,
    pub fallback_used: bool,
    pub error: Option<RuntimeError>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTraceDiagnostic {
    pub stage: RuntimeTraceStage,
    pub status: RuntimeTraceStageStatus,
    pub message: String,
}

impl RuntimeTraceDiagnostic {
    pub fn new_redacted(
        stage: RuntimeTraceStage,
        status: RuntimeTraceStageStatus,
        message: impl Into<String>,
    ) -> Self {
        let message = message.into();
        Self {
            stage,
            status,
            message: redact_trace_text(&message),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTraceStage {
    InterpretAction,
    SelectChoice,
    EvaluateRules,
    PlanScene,
    CommitState,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTraceStageStatus {
    Completed,
    Fallback,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTrace {
    pub id: String,
    pub timestamp_ms: u64,
    pub player_input: Option<String>,
    pub selected_choice: Option<String>,
    #[serde(default)]
    pub action_intent: Option<ActionIntent>,
    #[serde(default)]
    pub rule_result: Option<RuntimeRuleResult>,
    #[serde(default)]
    pub planner_result: Option<RuntimePlannerResult>,
    #[serde(default)]
    pub diagnostics: Vec<RuntimeTraceDiagnostic>,
    pub world_state_before: WorldState,
    pub world_state_delta: WorldDelta,
    pub world_state_after: WorldState,
    pub story_state_before: StoryState,
    pub story_state_after: StoryState,
    pub narrative_review: Option<NarrativeReview>,
    pub errors: Vec<RuntimeError>,
    pub fallback_used: bool,
}

pub const REDACTED_TRACE_SECRET: &str = "[REDACTED_SECRET]";

pub fn redact_trace_text(text: &str) -> String {
    text.split_whitespace()
        .map(|token| {
            if contains_secret_marker(token) {
                REDACTED_TRACE_SECRET.to_string()
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_secret_marker(token: &str) -> bool {
    let normalized = token.to_ascii_lowercase();
    [
        "sk-",
        "api_key",
        "secret_key",
        "openai_api_key",
        "authorization:",
        "bearer",
        "token=",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
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
    fn agent_output_proposals_roundtrip_json() {
        let proposals = vec![
            sample_scene_plan_output_proposal(),
            sample_beat_drafts_output_proposal(),
            sample_review_output_proposal(),
        ];

        for proposal in proposals {
            let encoded = serde_json::to_string_pretty(&proposal).expect("serialize proposal");
            let decoded: AgentOutputProposal =
                serde_json::from_str(&encoded).expect("deserialize proposal");

            assert_eq!(decoded, proposal);
        }

        let encoded =
            serde_json::to_string_pretty(&sample_scene_plan_output_proposal()).expect("serialize");
        let decoded: AgentOutputProposal =
            serde_json::from_str(&encoded).expect("deserialize proposal");
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("proposal value");

        assert_eq!(value["agent"], "scene_planner");
        assert_eq!(value["output"]["kind"], "scene_plan");
        assert!(matches!(
            decoded.output,
            AgentProposalPayload::ScenePlan(ScenePlanProposal { .. })
        ));
    }

    #[test]
    fn agent_output_proposal_rejects_unknown_state_commit_fields() {
        let mut proposal =
            serde_json::to_value(sample_scene_plan_output_proposal()).expect("proposal");
        proposal["world_state_delta"] = serde_json::json!({"treasury": 10});
        let error = serde_json::from_value::<AgentOutputProposal>(proposal)
            .expect_err("top-level state patch should be rejected");
        assert!(error.to_string().contains("unknown field"));

        let mut output =
            serde_json::to_value(sample_scene_plan_output_proposal()).expect("proposal");
        output["output"]["world_state_delta"] = serde_json::json!({"treasury": 10});
        let _error = serde_json::from_value::<AgentOutputProposal>(output)
            .expect_err("output state patch should be rejected");

        let mut nested =
            serde_json::to_value(sample_scene_plan_output_proposal()).expect("proposal");
        nested["output"]["payload"]["story_state_patch"] = serde_json::json!({"turn": 2});
        let error = serde_json::from_value::<AgentOutputProposal>(nested)
            .expect_err("scene plan state patch should be rejected");
        assert!(error.to_string().contains("unknown field"));

        let mut beat =
            serde_json::to_value(sample_beat_drafts_output_proposal()).expect("proposal");
        beat["output"]["payload"]["beats"][0]["world_state_delta"] =
            serde_json::json!({"treasury": 10});
        let error = serde_json::from_value::<AgentOutputProposal>(beat)
            .expect_err("beat draft state patch should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn reference_analysis_roundtrips_summary_only_contract() {
        let analysis = sample_reference_analysis();

        let encoded = serde_json::to_string_pretty(&analysis).expect("serialize reference");
        let decoded: ReferenceAnalysis =
            serde_json::from_str(&encoded).expect("deserialize reference");
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("reference value");

        assert_eq!(decoded, analysis);
        assert_eq!(value["source"]["source_type"], "user_import");
        assert_eq!(value["source"]["rights"], "user_authorized");
        assert!(value.get("raw_text").is_none());
        assert!(value.get("body").is_none());
    }

    #[test]
    fn reference_analysis_rejects_raw_text_fields() {
        let mut analysis = serde_json::to_value(sample_reference_analysis()).expect("reference");
        analysis["raw_text"] = serde_json::json!("large copyrighted body");
        let error = serde_json::from_value::<ReferenceAnalysis>(analysis)
            .expect_err("raw text should be rejected");

        assert!(error.to_string().contains("unknown field"));
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
            action_intent: Some(ActionIntent::supported("raise_tax", vec!["tax".into()])),
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
                RuntimeTraceStage::InterpretAction,
                RuntimeTraceStageStatus::Completed,
                "action matched",
            )],
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
    fn story_craft_contract_roundtrips_prd_fields() {
        let state = StoryCraftState {
            bible: StoryCraftBible {
                target_audience: Some("political sim players".into()),
                genre_promise: "court intrigue".into(),
                central_question: "survive?".into(),
                target_emotions: vec!["pressure".into()],
                core_foreshadowing: vec!["ledger".into()],
                emotional_contract: vec!["lonely authority".into()],
                pacing_profile: PacingProfile {
                    escalation_interval_scenes: 2,
                    target_tension_curve: vec![70, 85, 95],
                    breather_scene_frequency: Some(4),
                },
                hook_strategy: HookStrategy {
                    primary_hook: "Open with a concrete pressure.".into(),
                    recurring_hook_patterns: vec!["sealed memorial".into()],
                },
                reversal_strategy: Some(ReversalStrategy {
                    cadence_scenes: 3,
                    principle: "Relief exposes a new cost.".into(),
                }),
                prose_style_guide: Some("concrete political pressure".into()),
                banned_cliches: vec!["empty prophecy".into()],
                reference_modules: vec![ReferenceModule {
                    id: "pacing-template".into(),
                    title: "Pacing Template".into(),
                    summary: "Escalate every two scenes.".into(),
                }],
            },
            active_promises: vec![StoryPromise {
                id: "survival-cost".into(),
                text: "Every survival move has a visible cost.".into(),
                status: StoryPromiseStatus::Active,
                introduced_at: "court-crisis-001".into(),
                payoff_hint: Some("force a legitimacy tradeoff".into()),
            }],
            emotional_arc: vec![EmotionalArcPoint {
                scene_key: "court-crisis-001".into(),
                target_emotion: "pressure".into(),
                intensity: 80,
            }],
            plot_threads: vec![PlotThread {
                id: "border-payroll".into(),
                title: "Border payroll".into(),
                promise: "Soldiers stay loyal while paid.".into(),
                thread_type: PlotThreadType::Survival,
                status: PlotThreadStatus::Open,
                introduced_at: "court-crisis-001".into(),
                expected_payoff: Some("army paid or defects".into()),
                related_characters: vec!["war-minister".into()],
                related_world_flags: vec!["border_army_paid".into()],
                last_update: "ledger arrives".into(),
            }],
            character_arcs: vec![CharacterArc {
                id: "minister-pressure".into(),
                character_id: "war-minister".into(),
                desire: "pay the army".into(),
                pressure: "empty treasury".into(),
                current_state: "urgent".into(),
                target_state: "openly defies court delay".into(),
                status: CharacterArcStatus::Setup,
            }],
            pacing_score: Some(80),
            tension_score: Some(85),
            ai_slop_risk: Some(10),
            review_notes: vec![NarrativeReviewNote {
                id: "opening-hook".into(),
                scene_key: Some("court-crisis-001".into()),
                severity: Severity::Info,
                message: "Opening hook is concrete.".into(),
                resolved: true,
            }],
        };

        let encoded = serde_json::to_string_pretty(&state).expect("serialize story craft");
        let decoded: StoryCraftState =
            serde_json::from_str(&encoded).expect("deserialize story craft");

        assert_eq!(decoded, state);
        assert_eq!(decoded.bible.pacing_profile.escalation_interval_scenes, 2);
        assert_eq!(
            decoded.plot_threads[0].thread_type,
            PlotThreadType::Survival
        );
        assert_eq!(decoded.character_arcs[0].status, CharacterArcStatus::Setup);
    }

    #[test]
    fn legacy_story_craft_defaults_new_prd_fields() {
        let legacy = r#"{
            "bible": {
                "genre_promise": "history",
                "central_question": "survive?",
                "target_emotions": ["pressure"],
                "core_foreshadowing": ["ledger"]
            },
            "emotional_arc": [],
            "plot_threads": [{
                "id": "border-payroll",
                "title": "Border payroll",
                "promise": "Soldiers stay loyal while paid.",
                "status": "open",
                "last_update": "ledger arrives"
            }]
        }"#;

        let decoded: StoryCraftState =
            serde_json::from_str(legacy).expect("deserialize legacy story craft");

        assert!(decoded.active_promises.is_empty());
        assert!(decoded.character_arcs.is_empty());
        assert!(decoded.review_notes.is_empty());
        assert_eq!(decoded.bible.pacing_profile, PacingProfile::default());
        assert_eq!(decoded.bible.hook_strategy, HookStrategy::default());
        assert_eq!(decoded.plot_threads[0].thread_type, PlotThreadType::Custom);
        assert!(decoded.plot_threads[0].related_characters.is_empty());
    }

    #[test]
    fn trace_redaction_removes_secret_markers() {
        let text = "朕决定加征辽饷 OPENAI_API_KEY=sk-test-secret-marker bearer token=value";

        let redacted = redact_trace_text(text);
        let error = RuntimeError::redacted(text, text);
        let diagnostic = RuntimeTraceDiagnostic::new_redacted(
            RuntimeTraceStage::PlanScene,
            RuntimeTraceStageStatus::Error,
            text,
        );
        let review = NarrativeReview {
            scene_key: text.into(),
            score: 1,
            hook_score: 1,
            pacing_score: 1,
            character_consistency_score: 1,
            payoff_score: 1,
            choice_meaningfulness_score: 1,
            ai_slop_risk: 99,
            issues: vec![NarrativeIssue {
                kind: NarrativeIssueKind::WeakHook,
                severity: Severity::Warning,
                message: text.into(),
            }],
        }
        .redacted();

        assert!(redacted.contains(REDACTED_TRACE_SECRET));
        assert!(redacted.contains("朕决定加征辽饷"));
        assert!(!redacted.contains("OPENAI_API_KEY"));
        assert!(!redacted.contains("sk-test-secret-marker"));
        assert!(!error.code.contains("OPENAI_API_KEY"));
        assert!(!error.message.contains("token=value"));
        assert!(!diagnostic.message.contains("bearer"));
        assert!(!review.scene_key.contains("sk-test-secret-marker"));
        assert!(!review.issues[0].message.contains("token=value"));
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
                    ..StoryCraftBible::default()
                },
                emotional_arc: Vec::new(),
                plot_threads: Vec::new(),
                ..StoryCraftState::default()
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

    fn sample_scene_plan_output_proposal() -> AgentOutputProposal {
        AgentOutputProposal {
            id: "scene-plan-proposal-001".into(),
            agent: AgentRole::ScenePlanner,
            output: AgentProposalPayload::ScenePlan(sample_scene_plan_proposal()),
        }
    }

    fn sample_beat_drafts_output_proposal() -> AgentOutputProposal {
        AgentOutputProposal {
            id: "beat-drafts-proposal-001".into(),
            agent: AgentRole::BeatWriter,
            output: AgentProposalPayload::BeatDrafts(BeatDraftsProposal {
                scene_key: "court-crisis-002".into(),
                beats: vec![sample_beat_draft_proposal()],
            }),
        }
    }

    fn sample_review_output_proposal() -> AgentOutputProposal {
        AgentOutputProposal {
            id: "review-proposal-001".into(),
            agent: AgentRole::PlotDoctor,
            output: AgentProposalPayload::Review(sample_review_proposal()),
        }
    }

    fn sample_scene_plan_proposal() -> ScenePlanProposal {
        ScenePlanProposal {
            scene_key: "court-crisis-002".into(),
            title: "Tax Resistance Memorials".into(),
            location: "Qianqing Palace".into(),
            scene_summary: "The levy creates immediate provincial resistance.".into(),
            dramatic_purpose: "Show the cost of emergency revenue.".into(),
            hook: "Three memorials arrive with broken tax seals.".into(),
            emotional_goal: Some("consequence".into()),
            cast: vec!["grand-secretary".into()],
            entry_beat_id: "court-crisis-002-beat-001".into(),
            background_asset: Some("assets/generated/court-crisis-002.png".into()),
        }
    }

    fn sample_beat_draft_proposal() -> BeatDraftProposal {
        BeatDraftProposal {
            id: "court-crisis-002-beat-001".into(),
            scene_key: "court-crisis-002".into(),
            text: "The court reads three provincial reports in silence.".into(),
            choices: vec![Choice {
                id: "inspect-corruption".into(),
                label: "Investigate the collectors".into(),
                action_type: "inspect_corruption".into(),
                dramatic_purpose: "Trade court stability for cleaner revenue.".into(),
                change_scene: true,
            }],
            narrative_function: NarrativeFunction::Hook,
        }
    }

    fn sample_review_proposal() -> ReviewProposal {
        ReviewProposal {
            scene_key: "court-crisis-002".into(),
            review: NarrativeReview {
                scene_key: "court-crisis-002".into(),
                score: 95,
                hook_score: 95,
                pacing_score: 95,
                character_consistency_score: 100,
                payoff_score: 90,
                choice_meaningfulness_score: 95,
                ai_slop_risk: 5,
                issues: Vec::new(),
            },
            notes: vec![NarrativeReviewNote {
                id: "proposal-review-note".into(),
                scene_key: Some("court-crisis-002".into()),
                severity: Severity::Info,
                message: "Proposal advances tax disorder visibly.".into(),
                resolved: true,
            }],
        }
    }

    fn sample_reference_analysis() -> ReferenceAnalysis {
        ReferenceAnalysis {
            id: "authorized-crisis-structure".into(),
            title: "Authorized Crisis Structure Notes".into(),
            source: ReferenceSource {
                source_type: ReferenceSourceType::UserImport,
                rights: ReferenceRights::UserAuthorized,
                citation: "User supplied notes, local import".into(),
                user_authorized: true,
            },
            summary:
                "Escalate the visible cost of every court decision without copying source prose."
                    .into(),
            structure_notes: vec![ReferenceStructureNote {
                label: "pressure ladder".into(),
                summary: "Start with material scarcity, then make legitimacy and loyalty collide."
                    .into(),
            }],
            tags: vec!["political".into(), "pacing".into()],
        }
    }
}
