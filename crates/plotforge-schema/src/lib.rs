use std::collections::BTreeMap;

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

pub type ResourceMap = BTreeMap<String, i32>;
pub type FlagMap = BTreeMap<String, bool>;

pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CONTRACT_SCHEMA_VERSION: u32 = 13;
pub const CONTRACT_GENERATOR: &str = "plotforge-schema";
pub const AI_USAGE_MANIFEST_FILE: &str = "ai-usage.json";
pub const WORKSHOP_ITEM_MANIFEST_FILE: &str = "workshop-item.json";
pub const DESKTOP_RUNTIME_DRAFT_FILE: &str = "desktop-runtime-draft.json";

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameProject {
    pub id: String,
    pub title: String,
    pub version: String,
    pub description: String,
    pub entry_scene: String,
    pub run_seed: u64,
}

#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTemplateId {
    #[default]
    HistoricalCrisis,
    DynastyEmbers,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectCreationRequest {
    pub template: ProjectTemplateId,
    pub concept: String,
    pub visual_style: String,
    pub voice_enabled: bool,
    pub initial_scene_request: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectCreationReport {
    pub project_path: String,
    pub template: ProjectTemplateId,
    pub concept: String,
    pub visual_style: String,
    pub voice_enabled: bool,
    pub initial_scene_request: String,
    pub files_created: Vec<String>,
    pub project: ProjectData,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldEditDocument {
    pub world_bible_markdown: String,
    pub canon_markdown: String,
    pub forbidden_facts: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StoryCraftEditDocument {
    pub story_bible_markdown: String,
    pub style_guide_markdown: String,
    pub story_craft: StoryCraftState,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CharacterEditDocument {
    pub characters: Vec<Character>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StateVariablesEditDocument {
    pub resources: Vec<ResourceDefinition>,
    pub initial_world_state: WorldState,
    pub initial_story_state: StoryState,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RulesEditDocument {
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceDefinition {
    pub key: String,
    pub label: String,
    pub initial: i32,
    pub min: i32,
    pub max: i32,
}

#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldState {
    pub resources: ResourceMap,
    pub flags: FlagMap,
    pub triggered_events: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryState {
    pub current_scene_key: String,
    #[serde(default)]
    pub current_beat_id: Option<String>,
    pub completed_scene_keys: Vec<String>,
    pub turn: u32,
}

#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacingProfile {
    pub escalation_interval_scenes: u8,
    pub target_tension_curve: Vec<u8>,
    pub breather_scene_frequency: Option<u8>,
}

#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookStrategy {
    pub primary_hook: String,
    pub recurring_hook_patterns: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReversalStrategy {
    pub cadence_scenes: u8,
    pub principle: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReferenceModule {
    pub id: String,
    pub title: String,
    pub summary: String,
}

pub const MAX_REFERENCE_SUMMARY_CHARS: usize = 600;
pub const MAX_REFERENCE_STRUCTURE_NOTE_CHARS: usize = 300;

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSource {
    pub source_type: ReferenceSourceType,
    pub rights: ReferenceRights,
    pub citation: String,
    pub user_authorized: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceSourceType {
    UserImport,
    PublicDomain,
    OpenLicense,
    MethodTemplate,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceRights {
    UserOwned,
    UserAuthorized,
    PublicDomain,
    OpenLicense,
    GenericMethod,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceStructureNote {
    pub label: String,
    pub summary: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryPromise {
    pub id: String,
    pub text: String,
    pub status: StoryPromiseStatus,
    pub introduced_at: String,
    pub payoff_hint: Option<String>,
}

#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StoryPromiseStatus {
    #[default]
    Active,
    Complicated,
    PaidOff,
    Dropped,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlotThreadStatus {
    Open,
    Escalating,
    Resolved,
    Deepened,
    PaidOff,
    Abandoned,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmotionalArcPoint {
    pub scene_key: String,
    pub target_emotion: String,
    pub intensity: u8,
}

#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterArc {
    pub id: String,
    pub character_id: String,
    pub desire: String,
    pub pressure: String,
    pub current_state: String,
    pub target_state: String,
    pub status: CharacterArcStatus,
}

#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CharacterArcStatus {
    #[default]
    Setup,
    Pressured,
    Changed,
    Resolved,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct NarrativeReviewNote {
    pub id: String,
    pub scene_key: Option<String>,
    pub severity: Severity,
    pub message: String,
    pub resolved: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub role: String,
    pub traits: Vec<String>,
    pub visual_card: String,
    pub voice_card: String,
    #[serde(default)]
    pub portrait_request: Option<CharacterPortraitRequest>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CharacterPortraitRequest {
    pub prompt_summary: String,
    pub style: String,
    pub target_asset_slot: String,
    pub prompt_hash: String,
    pub provider_config_hash: String,
    #[serde(default)]
    pub reference_asset_ids: Vec<String>,
    #[serde(default)]
    pub fallback_allowed: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Image,
    Audio,
    Voice,
    Data,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AssetSourceKind {
    UserImport,
    Generated,
    Placeholder,
    External,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AssetReferenceKind {
    Project,
    Scene,
    Character,
    ExportProfile,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct AssetReference {
    pub reference_kind: AssetReferenceKind,
    pub reference_id: String,
    pub slot: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetProviderMetadata {
    pub provider: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub prompt_hash: Option<String>,
    #[serde(default)]
    pub fallback_used: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetRecord {
    pub id: String,
    pub kind: AssetKind,
    pub source: AssetSourceKind,
    pub project_path: String,
    pub export_path: String,
    pub content_hash: String,
    pub hash_algorithm: String,
    pub byte_length: u64,
    #[serde(default)]
    pub provider_metadata: Option<AssetProviderMetadata>,
    #[serde(default)]
    pub references: Vec<AssetReference>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MediaAssetReference {
    #[serde(default)]
    pub asset_id: Option<String>,
    pub kind: AssetKind,
    pub source: AssetSourceKind,
    pub project_path: String,
    pub export_path: String,
    pub slot: String,
}

#[derive(Clone, Debug, Default, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualBible {
    #[serde(default)]
    pub style_cards: Vec<VisualStyleCard>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualStyleCard {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub prompt: String,
    #[serde(default)]
    pub palette: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub reference_asset_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AudioBible {
    #[serde(default)]
    pub voice_cards: Vec<AudioVoiceCard>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AudioVoiceCard {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub voice: String,
    pub delivery: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub sample_text: Option<String>,
    #[serde(default)]
    pub reference_asset_ids: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scene {
    pub key: String,
    pub title: String,
    pub location: String,
    pub dramatic_purpose: String,
    pub hook: String,
    pub background_asset: String,
    #[serde(default)]
    pub audio_refs: Vec<MediaAssetReference>,
    pub character_ids: Vec<String>,
    pub plot_thread_updates: BTreeMap<String, String>,
    pub beats: Vec<Beat>,
    #[serde(default)]
    pub entry_beat_id: Option<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct Beat {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub speaker: Option<String>,
    #[serde(default)]
    pub line_delivery: Option<String>,
    #[serde(default)]
    pub audio_refs: Vec<MediaAssetReference>,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub next: BeatNext,
}

#[derive(Clone, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum BeatNext {
    Beat(String),
    Scene,
    End,
    #[default]
    None,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub action_type: String,
    #[serde(default)]
    pub input_terms: Vec<String>,
    pub dramatic_purpose: String,
    pub change_scene: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    StoryArchitect,
    StoryCraftPlanner,
    CharacterDesigner,
    ScenePlanner,
    BeatWriter,
    PlotDoctor,
    ConsistencyChecker,
    DeslopRefiner,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentOutputProposal {
    pub id: String,
    pub agent: AgentRole,
    pub output: AgentProposalPayload,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReproducibilityMetadata {
    pub run_seed: u64,
    pub prompt_version: String,
    pub model_version: String,
    pub provider_config_hash: String,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub snapshot_id: Option<String>,
}

impl Default for ReproducibilityMetadata {
    fn default() -> Self {
        Self::local_mock(0)
    }
}

impl ReproducibilityMetadata {
    pub fn local_mock(run_seed: u64) -> Self {
        Self {
            run_seed,
            prompt_version: "plotforge-local-mock-prompt-v1".into(),
            model_version: "plotforge-local-mock-model-v1".into(),
            provider_config_hash: "sha256:plotforge-local-mock-provider-config-v1".into(),
            trace_id: None,
            snapshot_id: None,
        }
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn with_snapshot_id(mut self, snapshot_id: impl Into<String>) -> Self {
        self.snapshot_id = Some(snapshot_id.into());
        self
    }
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentOutputEnvelope {
    pub id: String,
    pub contract_version: String,
    pub schema_version: u32,
    pub agent: AgentRole,
    pub reproducibility: ReproducibilityMetadata,
    pub proposal: AgentOutputProposal,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AgentProposalPayload {
    WorldExpansion(Box<WorldExpansionProposal>),
    StoryCraftPlan(Box<StoryCraftPlanProposal>),
    CharacterProfile(Box<CharacterProposal>),
    ScenePlan(Box<ScenePlanProposal>),
    BeatDrafts(Box<BeatDraftsProposal>),
    Review(Box<ReviewProposal>),
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldGenerationRequest {
    pub expansion_goal: String,
    pub document: WorldEditDocument,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldExpansionProposal {
    pub world_bible_markdown: String,
    pub canon_markdown: String,
    pub forbidden_facts: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldGenerationReport {
    pub document: WorldEditDocument,
    pub evidence: GenerationEvidence,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StoryCraftGenerationRequest {
    pub concept: String,
    pub world_bible_markdown: String,
    pub canon_markdown: String,
    pub forbidden_facts: Vec<String>,
    pub document: StoryCraftEditDocument,
    pub characters: Vec<Character>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StoryCraftPlanProposal {
    pub story_bible_markdown: String,
    pub style_guide_markdown: String,
    pub story_craft: StoryCraftState,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StoryCraftGenerationReport {
    pub document: StoryCraftEditDocument,
    pub evidence: GenerationEvidence,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CharacterGenerationRequest {
    pub concept: String,
    pub role_hint: String,
    pub world_bible_markdown: String,
    pub story_bible_markdown: String,
    pub existing_characters: Vec<Character>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CharacterProposal {
    pub character: Character,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CharacterGenerationReport {
    pub character: Character,
    pub evidence: GenerationEvidence,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationEvidence {
    pub status: GenerationStatus,
    pub fallback_used: bool,
    #[serde(default)]
    pub error: Option<RuntimeError>,
    pub reproducibility: ReproducibilityMetadata,
    #[serde(default)]
    pub envelopes: Vec<AgentOutputEnvelope>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStatus {
    Succeeded,
    Fallback,
    Failed,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BeatDraftsProposal {
    pub scene_key: String,
    #[serde(default)]
    pub beats: Vec<BeatDraftProposal>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BeatDraftProposal {
    pub id: String,
    pub scene_key: String,
    pub text: String,
    pub choices: Vec<Choice>,
    pub narrative_function: NarrativeFunction,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeFunction {
    Hook,
    Setup,
    Payoff,
    Reversal,
    Choice,
    Cliffhanger,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewProposal {
    pub scene_key: String,
    pub review: NarrativeReview,
    #[serde(default)]
    pub notes: Vec<NarrativeReviewNote>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionIntent {
    pub status: ActionIntentStatus,
    #[serde(default)]
    pub choice_id: Option<String>,
    pub action_type: Option<String>,
    pub matched_terms: Vec<String>,
    pub reason: Option<String>,
}

impl ActionIntent {
    pub fn supported(action_type: impl Into<String>, matched_terms: Vec<String>) -> Self {
        Self {
            status: ActionIntentStatus::Supported,
            choice_id: None,
            action_type: Some(action_type.into()),
            matched_terms,
            reason: None,
        }
    }

    pub fn for_choice(
        choice_id: impl Into<String>,
        action_type: impl Into<String>,
        matched_terms: Vec<String>,
    ) -> Self {
        Self {
            status: ActionIntentStatus::Supported,
            choice_id: Some(choice_id.into()),
            action_type: Some(action_type.into()),
            matched_terms,
            reason: None,
        }
    }

    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            status: ActionIntentStatus::Unsupported,
            choice_id: None,
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
            choice_id: self
                .choice_id
                .as_ref()
                .map(|choice_id| redact_trace_text(choice_id)),
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionIntentStatus {
    Supported,
    Unsupported,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rule {
    pub id: String,
    pub action_type: String,
    pub conditions: Vec<Condition>,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Condition {
    ResourceAtLeast { key: String, value: i32 },
    ResourceAtMost { key: String, value: i32 },
    FlagEquals { key: String, value: bool },
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Effect {
    AddResource { key: String, amount: i32 },
    SetResource { key: String, value: i32 },
    SetFlag { key: String, value: bool },
    TriggerEvent { event: String },
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeRuleResult {
    pub action_type: String,
    pub delta_empty: bool,
    pub state_committed: bool,
    pub error: Option<RuntimeError>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimePlannerResult {
    pub requested_action_type: String,
    pub scene_key: Option<String>,
    pub fallback_used: bool,
    pub error: Option<RuntimeError>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeMediaReference {
    pub reference: AssetReference,
    pub project_path: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTraceStage {
    InterpretAction,
    SelectChoice,
    EvaluateRules,
    PlanScene,
    CommitState,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTraceStageStatus {
    Completed,
    Fallback,
    Error,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTrace {
    pub id: String,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub reproducibility: ReproducibilityMetadata,
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
    #[serde(default)]
    pub media_references: Vec<RuntimeMediaReference>,
    pub errors: Vec<RuntimeError>,
    pub fallback_used: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    pub id: String,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub reproducibility: ReproducibilityMetadata,
    pub project_id: String,
    pub project_version: String,
    pub story_state: StoryState,
    pub world_state: WorldState,
    pub scenes: Vec<Scene>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    TextGeneration,
    ImageGeneration,
    TtsGeneration,
    ExportPackage,
    ReferenceAnalysis,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Canceled,
    TimedOut,
}

#[derive(Clone, Debug, Default, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JobProgress {
    #[serde(default)]
    pub completed_units: u32,
    #[serde(default)]
    pub total_units: u32,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Default, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JobCost {
    #[serde(default)]
    pub estimated_units: u64,
    #[serde(default)]
    pub spent_units: u64,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JobFailure {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JobRecord {
    pub id: String,
    pub kind: JobKind,
    pub status: JobStatus,
    pub attempt: u32,
    pub max_attempts: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(default)]
    pub started_at_ms: Option<u64>,
    #[serde(default)]
    pub finished_at_ms: Option<u64>,
    pub timeout_ms: u64,
    #[serde(default)]
    pub progress: JobProgress,
    #[serde(default)]
    pub cost: JobCost,
    #[serde(default)]
    pub failure: Option<JobFailure>,
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

pub fn contains_secret_marker_text(text: &str) -> bool {
    text.split_whitespace().any(contains_secret_marker)
}

fn contains_secret_marker(token: &str) -> bool {
    let normalized = token.to_ascii_lowercase();
    normalized.starts_with("sk-")
        || [
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

#[derive(Clone, Debug, Default, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AiSafetyPolicy {
    #[serde(default)]
    pub live_generated_content_enabled: bool,
    #[serde(default)]
    pub content_kinds: Vec<AiUsageContentKind>,
    #[serde(default)]
    pub safety_guardrails: Vec<String>,
    #[serde(default)]
    pub user_reporting_path: String,
    #[serde(default)]
    pub moderation_policy: String,
    #[serde(default)]
    pub human_review_required: bool,
    #[serde(default)]
    pub moderation_queue_enabled: bool,
    #[serde(default)]
    pub policy_source_path: Option<String>,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub policy_hash: Option<String>,
    #[serde(default)]
    pub notices: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectData {
    pub game: GameProject,
    pub resources: Vec<ResourceDefinition>,
    pub world_state: WorldState,
    pub story_state: StoryState,
    pub story_craft: StoryCraftState,
    pub characters: Vec<Character>,
    pub rules: Vec<Rule>,
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub visual_bible: VisualBible,
    #[serde(default)]
    pub audio_bible: AudioBible,
    #[serde(default)]
    pub asset_records: Vec<AssetRecord>,
    #[serde(default)]
    pub ai_safety_policy: AiSafetyPolicy,
}

impl ProjectData {
    pub fn scene(&self, key: &str) -> Option<&Scene> {
        self.scenes.iter().find(|scene| scene.key == key)
    }
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportProfileTarget {
    StaticWeb,
    DynamicWeb,
    DesktopBundle,
    SteamWorkshop,
    SteamSubmissionKit,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportProfileCapability {
    LocalHttp,
    NoNetworkPlayer,
    StaticAssets,
    StandalonePackage,
    RuntimeSaveRestore,
    ProviderBackedGeneration,
    PlayerByoKey,
    SelfHostBackend,
    DesktopShell,
    SteamWorkshopMetadata,
    SteamSubmissionEvidence,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportProfile {
    pub id: String,
    pub target: ExportProfileTarget,
    pub intent: String,
    pub capabilities: Vec<ExportProfileCapability>,
    pub requires_network_at_runtime: bool,
    pub includes_provider_config: bool,
    pub includes_private_traces: bool,
    pub platform_submission_ready: bool,
    pub notes: Vec<String>,
}

impl Default for ExportProfile {
    fn default() -> Self {
        Self::static_web()
    }
}

impl ExportProfile {
    pub fn static_web() -> Self {
        Self {
            id: "static-web".into(),
            target: ExportProfileTarget::StaticWeb,
            intent: "Package a local/self-hosted static player with prebaked project content."
                .into(),
            capabilities: vec![
                ExportProfileCapability::LocalHttp,
                ExportProfileCapability::NoNetworkPlayer,
                ExportProfileCapability::StaticAssets,
                ExportProfileCapability::StandalonePackage,
            ],
            requires_network_at_runtime: false,
            includes_provider_config: false,
            includes_private_traces: false,
            platform_submission_ready: false,
            notes: vec![
                "Runs from copied player files and local asset references.".into(),
                "Does not include provider credentials, raw provider responses, or private traces."
                    .into(),
                "This profile is an engineering disclosure surface, not a legal compliance guarantee."
                    .into(),
            ],
        }
    }

    pub fn dynamic_web() -> Self {
        Self {
            id: "byo-key-web".into(),
            target: ExportProfileTarget::DynamicWeb,
            intent: "Describe a future dynamic web player where each player supplies their own provider key."
                .into(),
            capabilities: vec![
                ExportProfileCapability::ProviderBackedGeneration,
                ExportProfileCapability::PlayerByoKey,
                ExportProfileCapability::RuntimeSaveRestore,
            ],
            requires_network_at_runtime: true,
            includes_provider_config: false,
            includes_private_traces: false,
            platform_submission_ready: false,
            notes: vec![
                "Player BYO keys must be entered at runtime and must not be written into export packages.".into(),
                "Raw provider responses are never part of this profile contract.".into(),
                "This is a descriptive profile only; static export remains the runnable MVP output.".into(),
            ],
        }
    }

    pub fn self_host_backend() -> Self {
        Self {
            id: "self-host-backend".into(),
            target: ExportProfileTarget::DynamicWeb,
            intent: "Describe a future dynamic web package backed by a creator-operated service."
                .into(),
            capabilities: vec![
                ExportProfileCapability::ProviderBackedGeneration,
                ExportProfileCapability::SelfHostBackend,
                ExportProfileCapability::RuntimeSaveRestore,
            ],
            requires_network_at_runtime: true,
            includes_provider_config: false,
            includes_private_traces: false,
            platform_submission_ready: false,
            notes: vec![
                "Provider credentials must stay in the creator-operated backend, never in client packages.".into(),
                "The export kit may generate deployment notes, but not backend secrets or live provider config.".into(),
                "This is a descriptive profile only; no hosted backend is generated in the MVP.".into(),
            ],
        }
    }

    pub fn desktop_bundle() -> Self {
        Self {
            id: "desktop-runtime".into(),
            target: ExportProfileTarget::DesktopBundle,
            intent: "Generate a local desktop runtime draft with package evidence and build notes."
                .into(),
            capabilities: vec![
                ExportProfileCapability::DesktopShell,
                ExportProfileCapability::RuntimeSaveRestore,
                ExportProfileCapability::StaticAssets,
                ExportProfileCapability::StandalonePackage,
            ],
            requires_network_at_runtime: false,
            includes_provider_config: false,
            includes_private_traces: false,
            platform_submission_ready: false,
            notes: vec![
                "Desktop runtime state must stay separate from private debug traces.".into(),
                "Provider credentials are not bundled into distributable packages.".into(),
            ],
        }
    }

    pub fn steam_workshop() -> Self {
        Self {
            id: "steam-workshop".into(),
            target: ExportProfileTarget::SteamWorkshop,
            intent: "Describe a future Workshop metadata/package candidate.".into(),
            capabilities: vec![
                ExportProfileCapability::SteamWorkshopMetadata,
                ExportProfileCapability::StaticAssets,
            ],
            requires_network_at_runtime: false,
            includes_provider_config: false,
            includes_private_traces: false,
            platform_submission_ready: false,
            notes: vec![
                "Workshop support is a metadata/package exploration profile only.".into(),
                "This profile does not upload content or promise platform approval.".into(),
            ],
        }
    }

    pub fn steam_submission_kit() -> Self {
        Self {
            id: "steam-submission-kit".into(),
            target: ExportProfileTarget::SteamSubmissionKit,
            intent: "Generate local draft evidence for a creator-owned Steam submission workflow."
                .into(),
            capabilities: vec![
                ExportProfileCapability::SteamSubmissionEvidence,
                ExportProfileCapability::StaticAssets,
                ExportProfileCapability::StandalonePackage,
            ],
            requires_network_at_runtime: false,
            includes_provider_config: false,
            includes_private_traces: false,
            platform_submission_ready: false,
            notes: vec![
                "Submission Kit output is draft support material only.".into(),
                "Creators remain responsible for Steamworks setup, store copy, build upload, content survey, and platform review.".into(),
                "This profile does not call Steamworks APIs or promise approval.".into(),
            ],
        }
    }
}

pub fn supported_export_profiles() -> Vec<ExportProfile> {
    vec![
        ExportProfile::static_web(),
        ExportProfile::dynamic_web(),
        ExportProfile::self_host_backend(),
        ExportProfile::desktop_bundle(),
        ExportProfile::steam_workshop(),
        ExportProfile::steam_submission_kit(),
    ]
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiUsageContentKind {
    Text,
    Image,
    Audio,
    Voice,
    Data,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiUsageSourceKind {
    ProjectSource,
    LocalMockProvider,
    ExternalProvider,
    Placeholder,
    UserImport,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AiUsageDisclosure {
    pub content_kind: AiUsageContentKind,
    pub source_kind: AiUsageSourceKind,
    pub summary: String,
    pub asset_paths: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AiProviderSummary {
    pub provider: String,
    #[serde(default)]
    pub model: Option<String>,
    pub generated_asset_count: u32,
    pub fallback_asset_count: u32,
    pub prompt_hashes: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AiUsageManifest {
    pub manifest_version: String,
    pub project_id: String,
    pub project_version: String,
    pub export_profile: ExportProfile,
    pub generated_by: String,
    pub external_model_calls_during_export: bool,
    pub provider_credentials_included: bool,
    pub raw_provider_responses_included: bool,
    pub private_traces_included: bool,
    pub disclosures: Vec<AiUsageDisclosure>,
    pub provider_summaries: Vec<AiProviderSummary>,
    #[serde(default)]
    pub ai_safety_policy: AiSafetyPolicy,
    pub notices: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesktopRuntimeDraft {
    pub manifest_version: String,
    pub project_id: String,
    pub project_version: String,
    pub export_profile: ExportProfile,
    pub static_manifest_path: String,
    pub ai_usage_manifest_path: String,
    pub build_notes_markdown: String,
    pub package_files: Vec<WorkshopPackageFile>,
    pub runtime_entrypoint: String,
    pub requires_network_at_runtime: bool,
    pub provider_credentials_included: bool,
    pub private_traces_included: bool,
    pub raw_provider_responses_included: bool,
    pub notices: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkshopDraftVisibility {
    PrivateDraft,
    FriendsOnlyDraft,
    UnlistedDraft,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkshopPackageFile {
    pub path: String,
    pub content_hash: String,
    pub hash_algorithm: String,
    pub byte_length: u64,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkshopItemPackage {
    pub manifest_version: String,
    pub package_id: String,
    pub title: String,
    pub description: String,
    pub visibility: WorkshopDraftVisibility,
    pub preview_image: String,
    pub content_root: String,
    pub tags: Vec<String>,
    pub export_profile: ExportProfile,
    pub ai_usage_manifest_path: String,
    pub content_files: Vec<WorkshopPackageFile>,
    pub notices: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkshopPublishDraft {
    pub manifest_version: String,
    pub package_id: String,
    pub title: String,
    pub description: String,
    pub visibility: WorkshopDraftVisibility,
    pub preview_image: String,
    pub content_root: String,
    pub tags: Vec<String>,
    pub ai_usage_manifest_path: String,
    pub package_files: Vec<WorkshopPackageFile>,
    pub generated_by: String,
    pub upload_enabled: bool,
    pub requires_explicit_steamworks_credentials: bool,
    pub steamworks_api_called: bool,
    pub notices: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SteamSubmissionKitRequest {
    pub product_name: String,
    #[serde(default)]
    pub desktop_build_path: Option<String>,
    pub store_short_description: String,
    pub screenshot_paths: Vec<String>,
    pub capsule_asset_paths: Vec<String>,
    pub content_warnings: Vec<String>,
    pub safety_guardrails: Vec<String>,
    pub user_reporting_path: String,
    pub moderation_policy: String,
    pub build_notes: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SteamSubmissionKitDraft {
    pub manifest_version: String,
    pub product_name: String,
    pub workshop_package_id: String,
    pub generated_by: String,
    pub source_workshop_manifest_path: String,
    pub store_copy_markdown: String,
    pub checklist_markdown: String,
    pub ai_disclosure_markdown: String,
    pub content_warnings_markdown: String,
    pub asset_references_markdown: String,
    pub steam_direct_checklist_markdown: String,
    pub content_safety_checklist_markdown: String,
    pub packaging_notes_markdown: String,
    pub official_reference_urls: Vec<String>,
    pub notices: Vec<String>,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportManifest {
    pub game: GameProject,
    pub entry_scene: String,
    pub scenes: Vec<Scene>,
    pub assets: Vec<String>,
    #[serde(default)]
    pub asset_records: Vec<AssetRecord>,
    #[serde(default = "ExportProfile::static_web")]
    pub profile: ExportProfile,
    #[serde(default = "default_ai_usage_manifest_path")]
    pub ai_usage_manifest_path: String,
    pub generated_by: String,
}

fn default_ai_usage_manifest_path() -> String {
    AI_USAGE_MANIFEST_FILE.into()
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractRootSchemas {
    pub project_creation_request: ProjectCreationRequest,
    pub project_creation_report: ProjectCreationReport,
    pub world_edit_document: WorldEditDocument,
    pub story_craft_edit_document: StoryCraftEditDocument,
    pub character_edit_document: CharacterEditDocument,
    pub state_variables_edit_document: StateVariablesEditDocument,
    pub rules_edit_document: RulesEditDocument,
    pub project_data: ProjectData,
    pub runtime_trace: RuntimeTrace,
    pub runtime_snapshot: RuntimeSnapshot,
    pub job_record: JobRecord,
    pub agent_output_proposal: AgentOutputProposal,
    pub agent_output_envelope: AgentOutputEnvelope,
    pub reproducibility_metadata: ReproducibilityMetadata,
    pub generation_evidence: GenerationEvidence,
    pub world_generation_request: WorldGenerationRequest,
    pub world_generation_report: WorldGenerationReport,
    pub story_craft_generation_request: StoryCraftGenerationRequest,
    pub story_craft_generation_report: StoryCraftGenerationReport,
    pub character_generation_request: CharacterGenerationRequest,
    pub character_generation_report: CharacterGenerationReport,
    pub character_portrait_request: CharacterPortraitRequest,
    pub reference_analysis: ReferenceAnalysis,
    pub asset_record: AssetRecord,
    pub media_asset_reference: MediaAssetReference,
    pub visual_bible: VisualBible,
    pub audio_bible: AudioBible,
    pub ai_safety_policy: AiSafetyPolicy,
    pub ai_usage_manifest: AiUsageManifest,
    pub desktop_runtime_draft: DesktopRuntimeDraft,
    pub workshop_item_package: WorkshopItemPackage,
    pub workshop_publish_draft: WorkshopPublishDraft,
    pub steam_submission_kit_request: SteamSubmissionKitRequest,
    pub steam_submission_kit_draft: SteamSubmissionKitDraft,
    pub export_manifest: ExportManifest,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractBundle {
    pub contract_version: String,
    pub schema_version: u32,
    pub generated_by: String,
    pub json_schema: serde_json::Value,
    pub typescript: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContractVersionError {
    UnsupportedContractVersion { expected: String, actual: String },
    UnsupportedSchemaVersion { expected: u32, actual: u32 },
}

pub fn contract_bundle() -> ContractBundle {
    ContractBundle {
        contract_version: CONTRACT_VERSION.into(),
        schema_version: CONTRACT_SCHEMA_VERSION,
        generated_by: CONTRACT_GENERATOR.into(),
        json_schema: contract_json_schema(),
        typescript: contract_typescript(),
    }
}

pub fn contract_json_schema() -> serde_json::Value {
    serde_json::to_value(schema_for!(ContractRootSchemas)).expect("contract schema serializes")
}

pub fn contract_typescript() -> String {
    let mut output = String::new();
    output.push_str("// Generated by plotforge-schema. Do not edit by hand.\n");
    output.push_str(&format!(
        "export const PLOTFORGE_CONTRACT_VERSION = \"{}\" as const;\n",
        CONTRACT_VERSION
    ));
    output.push_str(&format!(
        "export const PLOTFORGE_CONTRACT_SCHEMA_VERSION = {} as const;\n\n",
        CONTRACT_SCHEMA_VERSION
    ));
    output.push_str(
        r#"export type ResourceMap = { [key: string]: number };
export type FlagMap = { [key: string]: boolean };
export type ContractEnvelope<T> = { contract_version: typeof PLOTFORGE_CONTRACT_VERSION; schema_version: typeof PLOTFORGE_CONTRACT_SCHEMA_VERSION; payload: T };
export type ProjectTemplateId = "historical_crisis" | "dynasty_embers";
export interface ProjectCreationRequest { template: ProjectTemplateId; concept: string; visual_style: string; voice_enabled: boolean; initial_scene_request: string; }
export interface ProjectCreationReport { project_path: string; template: ProjectTemplateId; concept: string; visual_style: string; voice_enabled: boolean; initial_scene_request: string; files_created: string[]; project: ProjectData; }
export interface WorldEditDocument { world_bible_markdown: string; canon_markdown: string; forbidden_facts: string[]; }
export interface StoryCraftEditDocument { story_bible_markdown: string; style_guide_markdown: string; story_craft: StoryCraftState; }
export interface CharacterEditDocument { characters: Character[]; }
export interface StateVariablesEditDocument { resources: ResourceDefinition[]; initial_world_state: WorldState; initial_story_state: StoryState; }
export interface RulesEditDocument { rules: Rule[]; }

export interface GameProject { id: string; title: string; version: string; description: string; entry_scene: string; run_seed: number; }
export interface ResourceDefinition { key: string; label: string; initial: number; min: number; max: number; }
export interface WorldState { resources: ResourceMap; flags: FlagMap; triggered_events: string[]; }
export interface WorldDelta { resource_changes: ResourceMap; resource_sets: ResourceMap; flags: FlagMap; triggered_events: string[]; }
export interface StoryState { current_scene_key: string; current_beat_id?: string | null; completed_scene_keys: string[]; turn: number; }

export interface StoryCraftBible { target_audience?: string | null; genre_promise: string; central_question: string; target_emotions: string[]; core_foreshadowing: string[]; emotional_contract: string[]; pacing_profile: PacingProfile; hook_strategy: HookStrategy; reversal_strategy?: ReversalStrategy | null; prose_style_guide?: string | null; banned_cliches: string[]; reference_modules: ReferenceModule[]; }
export interface PacingProfile { escalation_interval_scenes: number; target_tension_curve: number[]; breather_scene_frequency?: number | null; }
export interface HookStrategy { primary_hook: string; recurring_hook_patterns: string[]; }
export interface ReversalStrategy { cadence_scenes: number; principle: string; }
export interface ReferenceModule { id: string; title: string; summary: string; }
export interface ReferenceAnalysis { id: string; title: string; source: ReferenceSource; summary: string; structure_notes: ReferenceStructureNote[]; tags: string[]; }
export interface ReferenceSource { source_type: ReferenceSourceType; rights: ReferenceRights; citation: string; user_authorized: boolean; }
export type ReferenceSourceType = "user_import" | "public_domain" | "open_license" | "method_template";
export type ReferenceRights = "user_owned" | "user_authorized" | "public_domain" | "open_license" | "generic_method";
export interface ReferenceStructureNote { label: string; summary: string; }
export interface StoryPromise { id: string; text: string; status: StoryPromiseStatus; introduced_at: string; payoff_hint?: string | null; }
export type StoryPromiseStatus = "active" | "complicated" | "paid_off" | "dropped";
export interface PlotThread { id: string; title: string; promise: string; thread_type: PlotThreadType; status: PlotThreadStatus; introduced_at: string; expected_payoff?: string | null; related_characters: string[]; related_world_flags: string[]; last_update: string; }
export type PlotThreadType = "mystery" | "foreshadow" | "relationship" | "political" | "survival" | "custom";
export type PlotThreadStatus = "open" | "escalating" | "resolved" | "deepened" | "paid_off" | "abandoned";
export interface EmotionalArcPoint { scene_key: string; target_emotion: string; intensity: number; }
export interface StoryCraftState { bible: StoryCraftBible; active_promises: StoryPromise[]; emotional_arc: EmotionalArcPoint[]; plot_threads: PlotThread[]; character_arcs: CharacterArc[]; pacing_score?: number | null; tension_score?: number | null; ai_slop_risk?: number | null; review_notes: NarrativeReviewNote[]; }
export interface CharacterArc { id: string; character_id: string; desire: string; pressure: string; current_state: string; target_state: string; status: CharacterArcStatus; }
export type CharacterArcStatus = "setup" | "pressured" | "changed" | "resolved";
export interface NarrativeReviewNote { id: string; scene_key?: string | null; severity: Severity; message: string; resolved: boolean; }

export interface CharacterPortraitRequest { prompt_summary: string; style: string; target_asset_slot: string; prompt_hash: string; provider_config_hash: string; reference_asset_ids: string[]; fallback_allowed: boolean; }
export interface Character { id: string; name: string; role: string; traits: string[]; visual_card: string; voice_card: string; portrait_request?: CharacterPortraitRequest | null; }
export type AssetKind = "image" | "audio" | "voice" | "data";
export type AssetSourceKind = "user_import" | "generated" | "placeholder" | "external";
export type AssetReferenceKind = "project" | "scene" | "character" | "export_profile";
export interface AssetReference { reference_kind: AssetReferenceKind; reference_id: string; slot: string; }
export interface AssetProviderMetadata { provider: string; model?: string | null; request_id?: string | null; prompt_hash?: string | null; fallback_used: boolean; }
export interface AssetRecord { id: string; kind: AssetKind; source: AssetSourceKind; project_path: string; export_path: string; content_hash: string; hash_algorithm: string; byte_length: number; provider_metadata?: AssetProviderMetadata | null; references: AssetReference[]; }
export interface MediaAssetReference { asset_id?: string | null; kind: AssetKind; source: AssetSourceKind; project_path: string; export_path: string; slot: string; }
export interface VisualBible { style_cards: VisualStyleCard[]; }
export interface VisualStyleCard { id: string; title: string; summary: string; prompt: string; palette: string[]; tags: string[]; reference_asset_ids: string[]; }
export interface AudioBible { voice_cards: AudioVoiceCard[]; }
export interface AudioVoiceCard { id: string; title: string; summary: string; voice: string; delivery: string; tags: string[]; sample_text?: string | null; reference_asset_ids: string[]; }
export type ExportProfileTarget = "static_web" | "dynamic_web" | "desktop_bundle" | "steam_workshop" | "steam_submission_kit";
export type ExportProfileCapability = "local_http" | "no_network_player" | "static_assets" | "standalone_package" | "runtime_save_restore" | "provider_backed_generation" | "player_byo_key" | "self_host_backend" | "desktop_shell" | "steam_workshop_metadata" | "steam_submission_evidence";
export interface ExportProfile { id: string; target: ExportProfileTarget; intent: string; capabilities: ExportProfileCapability[]; requires_network_at_runtime: boolean; includes_provider_config: boolean; includes_private_traces: boolean; platform_submission_ready: boolean; notes: string[]; }
export type AiUsageContentKind = "text" | "image" | "audio" | "voice" | "data";
export type AiUsageSourceKind = "project_source" | "local_mock_provider" | "external_provider" | "placeholder" | "user_import";
export interface AiUsageDisclosure { content_kind: AiUsageContentKind; source_kind: AiUsageSourceKind; summary: string; asset_paths: string[]; }
export interface AiProviderSummary { provider: string; model?: string | null; generated_asset_count: number; fallback_asset_count: number; prompt_hashes: string[]; }
export interface AiSafetyPolicy { live_generated_content_enabled: boolean; content_kinds: AiUsageContentKind[]; safety_guardrails: string[]; user_reporting_path: string; moderation_policy: string; human_review_required: boolean; moderation_queue_enabled: boolean; policy_source_path?: string | null; evidence_ids: string[]; policy_hash?: string | null; notices: string[]; }
export interface AiUsageManifest { manifest_version: string; project_id: string; project_version: string; export_profile: ExportProfile; generated_by: string; external_model_calls_during_export: boolean; provider_credentials_included: boolean; raw_provider_responses_included: boolean; private_traces_included: boolean; disclosures: AiUsageDisclosure[]; provider_summaries: AiProviderSummary[]; ai_safety_policy: AiSafetyPolicy; notices: string[]; }
export type WorkshopDraftVisibility = "private_draft" | "friends_only_draft" | "unlisted_draft";
export interface WorkshopPackageFile { path: string; content_hash: string; hash_algorithm: string; byte_length: number; }
export interface DesktopRuntimeDraft { manifest_version: string; project_id: string; project_version: string; export_profile: ExportProfile; static_manifest_path: string; ai_usage_manifest_path: string; build_notes_markdown: string; package_files: WorkshopPackageFile[]; runtime_entrypoint: string; requires_network_at_runtime: boolean; provider_credentials_included: boolean; private_traces_included: boolean; raw_provider_responses_included: boolean; notices: string[]; }
export interface WorkshopItemPackage { manifest_version: string; package_id: string; title: string; description: string; visibility: WorkshopDraftVisibility; preview_image: string; content_root: string; tags: string[]; export_profile: ExportProfile; ai_usage_manifest_path: string; content_files: WorkshopPackageFile[]; notices: string[]; }
export interface WorkshopPublishDraft { manifest_version: string; package_id: string; title: string; description: string; visibility: WorkshopDraftVisibility; preview_image: string; content_root: string; tags: string[]; ai_usage_manifest_path: string; package_files: WorkshopPackageFile[]; generated_by: string; upload_enabled: boolean; requires_explicit_steamworks_credentials: boolean; steamworks_api_called: boolean; notices: string[]; }
export interface SteamSubmissionKitRequest { product_name: string; desktop_build_path?: string | null; store_short_description: string; screenshot_paths: string[]; capsule_asset_paths: string[]; content_warnings: string[]; safety_guardrails: string[]; user_reporting_path: string; moderation_policy: string; build_notes: string[]; }
export interface SteamSubmissionKitDraft { manifest_version: string; product_name: string; workshop_package_id: string; generated_by: string; source_workshop_manifest_path: string; store_copy_markdown: string; checklist_markdown: string; ai_disclosure_markdown: string; content_warnings_markdown: string; asset_references_markdown: string; steam_direct_checklist_markdown: string; content_safety_checklist_markdown: string; packaging_notes_markdown: string; official_reference_urls: string[]; notices: string[]; }
export interface Scene { key: string; title: string; location: string; dramatic_purpose: string; hook: string; background_asset: string; audio_refs: MediaAssetReference[]; character_ids: string[]; plot_thread_updates: Record<string, string>; beats: Beat[]; entry_beat_id?: string | null; }
export interface Beat { id: string; text: string; speaker?: string | null; line_delivery?: string | null; audio_refs: MediaAssetReference[]; choices: Choice[]; next?: BeatNext; }
export type BeatNext = { kind: "beat"; payload: string } | { kind: "scene" } | { kind: "end" } | { kind: "none" };
export interface Choice { id: string; label: string; action_type: string; input_terms: string[]; dramatic_purpose: string; change_scene: boolean; }

export type AgentRole = "story_architect" | "story_craft_planner" | "character_designer" | "scene_planner" | "beat_writer" | "plot_doctor" | "consistency_checker" | "deslop_refiner";
export interface AgentOutputProposal { id: string; agent: AgentRole; output: AgentProposalPayload; }
export interface ReproducibilityMetadata { run_seed: number; prompt_version: string; model_version: string; provider_config_hash: string; trace_id?: string | null; snapshot_id?: string | null; }
export interface AgentOutputEnvelope { id: string; contract_version: string; schema_version: number; agent: AgentRole; reproducibility: ReproducibilityMetadata; proposal: AgentOutputProposal; }
export type AgentProposalPayload = { kind: "world_expansion"; payload: WorldExpansionProposal } | { kind: "story_craft_plan"; payload: StoryCraftPlanProposal } | { kind: "character_profile"; payload: CharacterProposal } | { kind: "scene_plan"; payload: ScenePlanProposal } | { kind: "beat_drafts"; payload: BeatDraftsProposal } | { kind: "review"; payload: ReviewProposal };
export interface WorldGenerationRequest { expansion_goal: string; document: WorldEditDocument; }
export interface WorldExpansionProposal { world_bible_markdown: string; canon_markdown: string; forbidden_facts: string[]; }
export interface WorldGenerationReport { document: WorldEditDocument; evidence: GenerationEvidence; }
export interface StoryCraftGenerationRequest { concept: string; world_bible_markdown: string; canon_markdown: string; forbidden_facts: string[]; document: StoryCraftEditDocument; characters: Character[]; }
export interface StoryCraftPlanProposal { story_bible_markdown: string; style_guide_markdown: string; story_craft: StoryCraftState; }
export interface StoryCraftGenerationReport { document: StoryCraftEditDocument; evidence: GenerationEvidence; }
export interface CharacterGenerationRequest { concept: string; role_hint: string; world_bible_markdown: string; story_bible_markdown: string; existing_characters: Character[]; }
export interface CharacterProposal { character: Character; }
export interface CharacterGenerationReport { character: Character; evidence: GenerationEvidence; }
export interface GenerationEvidence { status: GenerationStatus; fallback_used: boolean; error?: RuntimeError | null; reproducibility: ReproducibilityMetadata; envelopes: AgentOutputEnvelope[]; }
export type GenerationStatus = "succeeded" | "fallback" | "failed";
export interface ScenePlanProposal { scene_key: string; title: string; location: string; scene_summary: string; dramatic_purpose: string; hook: string; emotional_goal?: string | null; cast: string[]; entry_beat_id: string; background_asset?: string | null; }
export interface BeatDraftsProposal { scene_key: string; beats: BeatDraftProposal[]; }
export interface BeatDraftProposal { id: string; scene_key: string; text: string; choices: Choice[]; narrative_function: NarrativeFunction; }
export type NarrativeFunction = "hook" | "setup" | "payoff" | "reversal" | "choice" | "cliffhanger";
export interface ReviewProposal { scene_key: string; review: NarrativeReview; notes: NarrativeReviewNote[]; }

export interface ActionIntent { status: ActionIntentStatus; choice_id?: string | null; action_type?: string | null; matched_terms: string[]; reason?: string | null; }
export type ActionIntentStatus = "supported" | "unsupported";
export interface Rule { id: string; action_type: string; conditions: Condition[]; effects: Effect[]; }
export type Condition = { kind: "resource_at_least"; key: string; value: number } | { kind: "resource_at_most"; key: string; value: number } | { kind: "flag_equals"; key: string; value: boolean };
export type Effect = { kind: "add_resource"; key: string; amount: number } | { kind: "set_resource"; key: string; value: number } | { kind: "set_flag"; key: string; value: boolean } | { kind: "trigger_event"; event: string };
export interface NarrativeReview { scene_key: string; score: number; hook_score: number; pacing_score: number; character_consistency_score: number; payoff_score: number; choice_meaningfulness_score: number; ai_slop_risk: number; issues: NarrativeIssue[]; }
export interface NarrativeIssue { kind: NarrativeIssueKind; severity: Severity; message: string; }
export type NarrativeIssueKind = "weak_hook" | "no_progress" | "fake_choice" | "out_of_character" | "broken_plot_thread" | "thread_forgotten" | "world_state_ignored" | "too_much_exposition" | "ai_slop_style";
export type Severity = "info" | "warning" | "error";

export interface RuntimeError { code: string; message: string; }
export interface RuntimeRuleResult { action_type: string; delta_empty: boolean; state_committed: boolean; error?: RuntimeError | null; }
export interface RuntimePlannerResult { requested_action_type: string; scene_key?: string | null; fallback_used: boolean; error?: RuntimeError | null; }
export interface RuntimeMediaReference { reference: AssetReference; project_path: string; }
export interface RuntimeTraceDiagnostic { stage: RuntimeTraceStage; status: RuntimeTraceStageStatus; message: string; }
export type RuntimeTraceStage = "interpret_action" | "select_choice" | "evaluate_rules" | "plan_scene" | "commit_state";
export type RuntimeTraceStageStatus = "completed" | "fallback" | "error";
export interface RuntimeTrace { id: string; timestamp_ms: number; reproducibility: ReproducibilityMetadata; player_input?: string | null; selected_choice?: string | null; action_intent?: ActionIntent | null; rule_result?: RuntimeRuleResult | null; planner_result?: RuntimePlannerResult | null; diagnostics: RuntimeTraceDiagnostic[]; world_state_before: WorldState; world_state_delta: WorldDelta; world_state_after: WorldState; story_state_before: StoryState; story_state_after: StoryState; narrative_review?: NarrativeReview | null; media_references: RuntimeMediaReference[]; errors: RuntimeError[]; fallback_used: boolean; }
export interface RuntimeSnapshot { id: string; timestamp_ms: number; reproducibility: ReproducibilityMetadata; project_id: string; project_version: string; story_state: StoryState; world_state: WorldState; scenes: Scene[]; }

export type JobKind = "text_generation" | "image_generation" | "tts_generation" | "export_package" | "reference_analysis";
export type JobStatus = "queued" | "running" | "succeeded" | "failed" | "canceled" | "timed_out";
export interface JobProgress { completed_units: number; total_units: number; message?: string | null; }
export interface JobCost { estimated_units: number; spent_units: number; }
export interface JobFailure { code: string; message: string; retryable: boolean; }
export interface JobRecord { id: string; kind: JobKind; status: JobStatus; attempt: number; max_attempts: number; created_at_ms: number; updated_at_ms: number; started_at_ms?: number | null; finished_at_ms?: number | null; timeout_ms: number; progress: JobProgress; cost: JobCost; failure?: JobFailure | null; }

export interface ProjectData { game: GameProject; resources: ResourceDefinition[]; world_state: WorldState; story_state: StoryState; story_craft: StoryCraftState; characters: Character[]; rules: Rule[]; scenes: Scene[]; visual_bible: VisualBible; audio_bible: AudioBible; asset_records: AssetRecord[]; ai_safety_policy: AiSafetyPolicy; }
export interface ExportManifest { game: GameProject; entry_scene: string; scenes: Scene[]; assets: string[]; asset_records: AssetRecord[]; profile: ExportProfile; ai_usage_manifest_path: string; generated_by: string; }
export interface ContractRootSchemas { project_creation_request: ProjectCreationRequest; project_creation_report: ProjectCreationReport; world_edit_document: WorldEditDocument; story_craft_edit_document: StoryCraftEditDocument; character_edit_document: CharacterEditDocument; state_variables_edit_document: StateVariablesEditDocument; rules_edit_document: RulesEditDocument; project_data: ProjectData; runtime_trace: RuntimeTrace; runtime_snapshot: RuntimeSnapshot; job_record: JobRecord; agent_output_proposal: AgentOutputProposal; agent_output_envelope: AgentOutputEnvelope; reproducibility_metadata: ReproducibilityMetadata; generation_evidence: GenerationEvidence; world_generation_request: WorldGenerationRequest; world_generation_report: WorldGenerationReport; story_craft_generation_request: StoryCraftGenerationRequest; story_craft_generation_report: StoryCraftGenerationReport; character_generation_request: CharacterGenerationRequest; character_generation_report: CharacterGenerationReport; character_portrait_request: CharacterPortraitRequest; reference_analysis: ReferenceAnalysis; asset_record: AssetRecord; media_asset_reference: MediaAssetReference; visual_bible: VisualBible; audio_bible: AudioBible; ai_safety_policy: AiSafetyPolicy; ai_usage_manifest: AiUsageManifest; desktop_runtime_draft: DesktopRuntimeDraft; workshop_item_package: WorkshopItemPackage; workshop_publish_draft: WorkshopPublishDraft; steam_submission_kit_request: SteamSubmissionKitRequest; steam_submission_kit_draft: SteamSubmissionKitDraft; export_manifest: ExportManifest; }
"#,
    );
    output
}

pub fn validate_contract_version(
    contract_version: &str,
    schema_version: u32,
) -> Result<(), ContractVersionError> {
    if contract_version != CONTRACT_VERSION {
        return Err(ContractVersionError::UnsupportedContractVersion {
            expected: CONTRACT_VERSION.into(),
            actual: contract_version.into(),
        });
    }
    if schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(ContractVersionError::UnsupportedSchemaVersion {
            expected: CONTRACT_SCHEMA_VERSION,
            actual: schema_version,
        });
    }
    Ok(())
}

pub fn validate_contract_bundle(bundle: &ContractBundle) -> Result<(), ContractVersionError> {
    validate_contract_version(&bundle.contract_version, bundle.schema_version)
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
    fn project_creation_contracts_roundtrip_and_reject_unknown_fields() {
        let request = ProjectCreationRequest {
            template: ProjectTemplateId::HistoricalCrisis,
            concept: "A regency court must survive a winter coup.".into(),
            visual_style: "ink wash court drama".into(),
            voice_enabled: true,
            initial_scene_request: "Open on an empty granary ledger.".into(),
        };
        let report = ProjectCreationReport {
            project_path: "/tmp/winter-regency".into(),
            template: request.template.clone(),
            concept: request.concept.clone(),
            visual_style: request.visual_style.clone(),
            voice_enabled: request.voice_enabled,
            initial_scene_request: request.initial_scene_request.clone(),
            files_created: vec!["game.toml".into(), "story/story_craft.toml".into()],
            project: sample_project_data(),
        };

        let request_json = serde_json::to_string_pretty(&request).expect("serialize request");
        let report_json = serde_json::to_string_pretty(&report).expect("serialize report");
        let request_value: serde_json::Value =
            serde_json::from_str(&request_json).expect("request value");

        assert_eq!(
            serde_json::from_str::<ProjectCreationRequest>(&request_json)
                .expect("deserialize request"),
            request
        );
        assert_eq!(
            serde_json::from_str::<ProjectCreationReport>(&report_json)
                .expect("deserialize report"),
            report
        );
        assert_eq!(request_value["template"], "historical_crisis");

        let mut with_unknown = request_value;
        with_unknown["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<ProjectCreationRequest>(with_unknown)
            .expect_err("unknown creation fields should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn structured_edit_contracts_roundtrip_and_reject_unknown_fields() {
        let project = sample_project_data();

        assert_contract_roundtrip_rejects_unknown(WorldEditDocument {
            world_bible_markdown: "# World Bible\n\nA winter court crisis.\n".into(),
            canon_markdown: "# Canon\n\n- Consequences stay visible.\n".into(),
            forbidden_facts: vec!["The emperor is not secretly immortal.".into()],
        });
        assert_contract_roundtrip_rejects_unknown(StoryCraftEditDocument {
            story_bible_markdown: "# Story Bible\n\nThe court cannot buy every ally.\n".into(),
            style_guide_markdown: "# Style Guide\n\nConcrete political pressure.\n".into(),
            story_craft: project.story_craft.clone(),
        });
        assert_contract_roundtrip_rejects_unknown(CharacterEditDocument {
            characters: vec![Character {
                id: "regent".into(),
                name: "Regent".into(),
                role: "Temporary court authority".into(),
                traits: vec!["cautious".into(), "clear".into()],
                visual_card: "ink portrait with winter robes".into(),
                voice_card: "measured court speech".into(),
                portrait_request: None,
            }],
        });
        assert_contract_roundtrip_rejects_unknown(StateVariablesEditDocument {
            resources: project.resources.clone(),
            initial_world_state: project.world_state.clone(),
            initial_story_state: project.story_state.clone(),
        });
        assert_contract_roundtrip_rejects_unknown(RulesEditDocument {
            rules: project.rules.clone(),
        });
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
        assert!(matches!(decoded.output, AgentProposalPayload::ScenePlan(_)));
    }

    #[test]
    fn agent_output_envelope_roundtrips_reproducibility_metadata() {
        let envelope = sample_agent_output_envelope();

        let encoded = serde_json::to_string_pretty(&envelope).expect("serialize envelope");
        let decoded: AgentOutputEnvelope =
            serde_json::from_str(&encoded).expect("deserialize envelope");
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("envelope value");

        assert_eq!(decoded, envelope);
        assert_eq!(value["contract_version"], CONTRACT_VERSION);
        assert_eq!(value["schema_version"], CONTRACT_SCHEMA_VERSION);
        assert_eq!(value["reproducibility"]["run_seed"], 7);
        assert_eq!(
            value["reproducibility"]["prompt_version"],
            "plotforge-agent-text-prompt-v1"
        );
        assert!(value.get("raw_provider_response").is_none());
    }

    #[test]
    fn agent_output_envelope_rejects_raw_provider_fields() {
        let mut envelope = serde_json::to_value(sample_agent_output_envelope()).expect("envelope");
        envelope["raw_provider_response"] =
            serde_json::json!("raw provider body with sk-test-secret-marker");
        let error = serde_json::from_value::<AgentOutputEnvelope>(envelope)
            .expect_err("raw provider response should be rejected");
        assert!(error.to_string().contains("unknown field"));

        let mut nested = serde_json::to_value(sample_agent_output_envelope()).expect("envelope");
        nested["reproducibility"]["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<AgentOutputEnvelope>(nested)
            .expect_err("provider credential field should be rejected");
        assert!(error.to_string().contains("unknown field"));
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
    fn asset_record_roundtrips_json() {
        let record = sample_asset_record();

        let encoded = serde_json::to_string_pretty(&record).expect("serialize asset record");
        let decoded: AssetRecord =
            serde_json::from_str(&encoded).expect("deserialize asset record");
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("asset value");

        assert_eq!(decoded, record);
        assert_eq!(value["kind"], "image");
        assert_eq!(value["source"], "generated");
        assert_eq!(value["references"][0]["reference_kind"], "scene");
        assert_eq!(
            value["provider_metadata"]["provider"],
            "fake-image-provider"
        );
        assert!(value["provider_metadata"].get("raw_response").is_none());
    }

    #[test]
    fn asset_provider_metadata_rejects_raw_response_fields() {
        let mut record = serde_json::to_value(sample_asset_record()).expect("asset record");
        record["provider_metadata"]["raw_response"] =
            serde_json::json!("raw provider body with sk-test-secret-marker");

        let error = serde_json::from_value::<AssetRecord>(record)
            .expect_err("raw provider response should be rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn job_record_roundtrips_json() {
        let record = sample_job_record();

        let encoded = serde_json::to_string_pretty(&record).expect("serialize job");
        let decoded: JobRecord = serde_json::from_str(&encoded).expect("deserialize job");
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("job value");

        assert_eq!(decoded, record);
        assert_eq!(value["kind"], "image_generation");
        assert_eq!(value["status"], "running");
        assert_eq!(value["progress"]["completed_units"], 2);
        assert_eq!(value["cost"]["estimated_units"], 500);
    }

    #[test]
    fn job_record_rejects_unknown_failure_fields() {
        let mut record = serde_json::to_value(sample_job_record()).expect("job record");
        record["failure"] = serde_json::json!({
            "code": "provider_error",
            "message": "provider failed",
            "retryable": true,
            "raw_response": "sk-test-secret-marker"
        });

        let error =
            serde_json::from_value::<JobRecord>(record).expect_err("raw failure field rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn contract_bundle_uses_current_version_envelope() {
        let bundle = contract_bundle();

        assert_eq!(bundle.contract_version, CONTRACT_VERSION);
        assert_eq!(bundle.schema_version, CONTRACT_SCHEMA_VERSION);
        assert_eq!(bundle.generated_by, CONTRACT_GENERATOR);
        validate_contract_bundle(&bundle).expect("current bundle valid");
    }

    #[test]
    fn contract_export_snapshots_are_reproducible() {
        let schema_snapshot: serde_json::Value =
            serde_json::from_str(include_str!("../../../contracts/plotforge.schema.json"))
                .expect("schema snapshot json");
        let typescript_snapshot = include_str!("../../../contracts/plotforge.d.ts");

        assert_eq!(schema_snapshot, contract_json_schema());
        assert_eq!(typescript_snapshot, contract_typescript());
    }

    #[test]
    fn contract_version_validation_rejects_unsupported_versions() {
        let version_error = validate_contract_version("0.0.0", CONTRACT_SCHEMA_VERSION)
            .expect_err("invalid contract version");
        assert!(matches!(
            version_error,
            ContractVersionError::UnsupportedContractVersion { .. }
        ));

        let schema_error = validate_contract_version(CONTRACT_VERSION, CONTRACT_SCHEMA_VERSION + 1)
            .expect_err("invalid schema version");
        assert!(matches!(
            schema_error,
            ContractVersionError::UnsupportedSchemaVersion { .. }
        ));
    }

    #[test]
    fn runtime_trace_roundtrips_json() {
        let story_state = StoryState {
            current_scene_key: "court-crisis-001".into(),
            current_beat_id: Some("court-crisis-001-beat-001".into()),
            completed_scene_keys: Vec::new(),
            turn: 0,
        };
        let trace = RuntimeTrace {
            id: "trace-1".into(),
            timestamp_ms: 1,
            reproducibility: sample_reproducibility_metadata().with_trace_id("trace-1"),
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
            media_references: vec![RuntimeMediaReference {
                reference: AssetReference {
                    reference_kind: AssetReferenceKind::Scene,
                    reference_id: "court-crisis-001".into(),
                    slot: "background_asset".into(),
                },
                project_path: "assets/generated/court-crisis-001.png".into(),
            }],
            errors: Vec::new(),
            fallback_used: false,
        };

        let encoded = serde_json::to_string_pretty(&trace).expect("serialize trace");
        let decoded: RuntimeTrace = serde_json::from_str(&encoded).expect("deserialize trace");
        assert_eq!(decoded, trace);
    }

    #[test]
    fn runtime_snapshot_roundtrips_json() {
        let snapshot = RuntimeSnapshot {
            id: "save-001".into(),
            timestamp_ms: 42,
            reproducibility: sample_reproducibility_metadata().with_snapshot_id("save-001"),
            project_id: "dynasty-embers".into(),
            project_version: "0.1.0".into(),
            story_state: StoryState {
                current_scene_key: "court-crisis-001".into(),
                current_beat_id: Some("court-crisis-001-beat-001".into()),
                completed_scene_keys: vec!["opening-court".into()],
                turn: 1,
            },
            world_state: WorldState {
                resources: BTreeMap::from([("treasury".into(), 52)]),
                flags: BTreeMap::from([("corruption_investigation".into(), true)]),
                triggered_events: vec!["officials_submit_memorials".into()],
            },
            scenes: vec![Scene {
                key: "court-crisis-001".into(),
                title: "The Red Deficit Ledger".into(),
                location: "Qianqing Palace".into(),
                dramatic_purpose: "Keep runtime restore deterministic.".into(),
                hook: "A saved crisis returns exactly where it paused.".into(),
                background_asset: "assets/generated/court-crisis-001.png".into(),
                audio_refs: Vec::new(),
                character_ids: Vec::new(),
                plot_thread_updates: BTreeMap::new(),
                entry_beat_id: Some("court-crisis-001-beat-001".into()),
                beats: vec![Beat {
                    id: "court-crisis-001-beat-001".into(),
                    text: "The court resumes from the saved beat.".into(),
                    speaker: None,
                    line_delivery: None,
                    audio_refs: Vec::new(),
                    choices: vec![Choice {
                        id: "raise-tax".into(),
                        label: "Raise taxes".into(),
                        action_type: "raise_tax".into(),
                        input_terms: vec!["raise".into(), "tax".into()],
                        dramatic_purpose: "Commit a restored scene choice.".into(),
                        change_scene: true,
                    }],
                    next: BeatNext::Scene,
                }],
            }],
        };

        let encoded = serde_json::to_string_pretty(&snapshot).expect("serialize snapshot");
        let decoded: RuntimeSnapshot =
            serde_json::from_str(&encoded).expect("deserialize snapshot");

        assert_eq!(decoded, snapshot);
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
                current_beat_id: Some("court-crisis-001-beat-001".into()),
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
            visual_bible: VisualBible::default(),
            audio_bible: AudioBible::default(),
            asset_records: Vec::new(),
            ai_safety_policy: Default::default(),
        };
        let manifest = ExportManifest {
            game: project.game.clone(),
            entry_scene: project.story_state.current_scene_key.clone(),
            scenes: project.scenes.clone(),
            assets: vec!["assets/generated/placeholder.png".into()],
            asset_records: Vec::new(),
            profile: ExportProfile::static_web(),
            ai_usage_manifest_path: AI_USAGE_MANIFEST_FILE.into(),
            generated_by: "test".into(),
        };
        let ai_usage = sample_ai_usage_manifest(&project.game);
        let desktop_runtime_draft = sample_desktop_runtime_draft(&project.game);

        let project_json = serde_json::to_string(&project).expect("serialize project");
        let manifest_json = serde_json::to_string(&manifest).expect("serialize manifest");
        let ai_usage_json = serde_json::to_string(&ai_usage).expect("serialize ai usage");
        let desktop_runtime_json =
            serde_json::to_string(&desktop_runtime_draft).expect("serialize desktop draft");
        assert_eq!(
            serde_json::from_str::<ProjectData>(&project_json).expect("deserialize project"),
            project
        );
        assert_eq!(
            serde_json::from_str::<ExportManifest>(&manifest_json).expect("deserialize manifest"),
            manifest
        );
        assert_eq!(
            serde_json::from_str::<AiUsageManifest>(&ai_usage_json).expect("deserialize ai usage"),
            ai_usage
        );
        assert_eq!(
            serde_json::from_str::<DesktopRuntimeDraft>(&desktop_runtime_json)
                .expect("deserialize desktop draft"),
            desktop_runtime_draft
        );
    }

    #[test]
    fn legacy_export_manifest_defaults_profile_fields() {
        let legacy = r#"{
            "game": {
                "id": "dynasty-embers",
                "title": "Dynasty Embers",
                "version": "0.1.0",
                "description": "Demo",
                "entry_scene": "court-crisis-001",
                "run_seed": 7
            },
            "entry_scene": "court-crisis-001",
            "scenes": [],
            "assets": ["assets/generated/placeholder.png"],
            "generated_by": "test"
        }"#;

        let decoded: ExportManifest = serde_json::from_str(legacy).expect("legacy export manifest");

        assert_eq!(decoded.profile, ExportProfile::static_web());
        assert_eq!(decoded.ai_usage_manifest_path, AI_USAGE_MANIFEST_FILE);
    }

    #[test]
    fn export_profiles_describe_supported_intents_without_secrets_or_submission_guarantees() {
        let profiles = supported_export_profiles();
        let ids = profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                "static-web",
                "byo-key-web",
                "self-host-backend",
                "desktop-runtime",
                "steam-workshop",
                "steam-submission-kit"
            ]
        );
        assert_eq!(
            profiles
                .iter()
                .find(|profile| profile.id == "steam-submission-kit")
                .expect("submission kit profile")
                .target,
            ExportProfileTarget::SteamSubmissionKit
        );
        for profile in profiles {
            assert!(!profile.includes_provider_config);
            assert!(!profile.includes_private_traces);
            assert!(!profile.platform_submission_ready);
            let serialized = serde_json::to_string(&profile).expect("profile json");
            assert!(!serialized.contains("OPENAI_API_KEY"));
            assert!(!serialized.contains("api_key"));
            assert!(!serialized.contains("secret_key"));
            assert!(!serialized.contains("sk-"));
        }
    }

    #[test]
    fn ai_usage_manifest_rejects_raw_provider_fields() {
        let project = GameProject {
            id: "dynasty-embers".into(),
            title: "Dynasty Embers".into(),
            version: "0.1.0".into(),
            description: "Demo".into(),
            entry_scene: "court-crisis-001".into(),
            run_seed: 7,
        };
        let mut manifest =
            serde_json::to_value(sample_ai_usage_manifest(&project)).expect("usage value");
        manifest["provider_summaries"][0]["raw_response"] =
            serde_json::json!("provider body with sk-test-secret-marker");

        let error = serde_json::from_value::<AiUsageManifest>(manifest)
            .expect_err("raw provider response should be rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn desktop_runtime_draft_roundtrips_and_rejects_private_or_upload_fields() {
        let game = GameProject {
            id: "dynasty-embers".into(),
            title: "Dynasty Embers".into(),
            version: "0.1.0".into(),
            description: "Demo".into(),
            entry_scene: "court-crisis-001".into(),
            run_seed: 7,
        };
        let draft = sample_desktop_runtime_draft(&game);
        assert_contract_roundtrip_rejects_unknown(draft.clone());
        assert_eq!(draft.export_profile, ExportProfile::desktop_bundle());
        assert!(!draft.requires_network_at_runtime);
        assert!(!draft.provider_credentials_included);
        assert!(!draft.private_traces_included);
        assert!(!draft.raw_provider_responses_included);

        let mut value = serde_json::to_value(draft).expect("desktop draft value");
        value["provider_config_path"] = serde_json::json!("providers/config.json");
        let error = serde_json::from_value::<DesktopRuntimeDraft>(value)
            .expect_err("provider config field should be rejected");
        assert!(error.to_string().contains("unknown field"));

        let mut upload = serde_json::json!({
            "manifest_version": "2026-06-08",
            "project_id": "dynasty-embers",
            "project_version": "0.1.0",
            "export_profile": ExportProfile::desktop_bundle(),
            "static_manifest_path": "game.json",
            "ai_usage_manifest_path": AI_USAGE_MANIFEST_FILE,
            "build_notes_markdown": "# Desktop Runtime Draft\n",
            "package_files": [],
            "runtime_entrypoint": "index.html",
            "requires_network_at_runtime": false,
            "provider_credentials_included": false,
            "private_traces_included": false,
            "raw_provider_responses_included": false,
            "notices": [],
            "steam_app_id": "000000"
        });
        let error = serde_json::from_value::<DesktopRuntimeDraft>(upload.take())
            .expect_err("Steam upload fields should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn workshop_item_package_roundtrips_and_rejects_upload_fields() {
        let package = sample_workshop_item_package();
        let encoded = serde_json::to_string_pretty(&package).expect("serialize workshop package");
        let decoded: WorkshopItemPackage =
            serde_json::from_str(&encoded).expect("deserialize workshop package");
        assert_eq!(decoded, package);
        assert_eq!(
            decoded.export_profile.target,
            ExportProfileTarget::SteamWorkshop
        );
        assert_eq!(decoded.ai_usage_manifest_path, AI_USAGE_MANIFEST_FILE);

        let mut value = serde_json::to_value(package).expect("workshop package value");
        value["published_file_id"] = serde_json::json!("1234567890");
        let error = serde_json::from_value::<WorkshopItemPackage>(value)
            .expect_err("upload fields should be rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn workshop_publish_draft_roundtrips_and_rejects_upload_state_fields() {
        let draft = sample_workshop_publish_draft();
        assert_contract_roundtrip_rejects_unknown(draft.clone());
        assert!(!draft.upload_enabled);
        assert!(draft.requires_explicit_steamworks_credentials);
        assert!(!draft.steamworks_api_called);

        let mut value = serde_json::to_value(draft).expect("publish draft value");
        value["published_file_id"] = serde_json::json!("1234567890");
        let error = serde_json::from_value::<WorkshopPublishDraft>(value)
            .expect_err("Steam upload fields should be rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn steam_submission_kit_contracts_roundtrip_and_reject_unknown_fields() {
        let request = sample_steam_submission_kit_request();
        let encoded = serde_json::to_string_pretty(&request).expect("serialize kit request");
        let decoded: SteamSubmissionKitRequest =
            serde_json::from_str(&encoded).expect("deserialize kit request");
        assert_eq!(decoded, request);

        let draft = sample_steam_submission_kit_draft();
        let encoded = serde_json::to_string_pretty(&draft).expect("serialize kit draft");
        let decoded: SteamSubmissionKitDraft =
            serde_json::from_str(&encoded).expect("deserialize kit draft");
        assert_eq!(decoded, draft);

        let mut value = serde_json::to_value(decoded).expect("kit draft value");
        value["steam_app_id"] = serde_json::json!("000000");
        let error = serde_json::from_value::<SteamSubmissionKitDraft>(value)
            .expect_err("unknown Steam upload fields should be rejected");

        assert!(error.to_string().contains("unknown field"));
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

    fn sample_project_data() -> ProjectData {
        ProjectData {
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
                current_beat_id: Some("court-crisis-001-beat-001".into()),
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
            visual_bible: VisualBible::default(),
            audio_bible: AudioBible::default(),
            asset_records: Vec::new(),
            ai_safety_policy: Default::default(),
        }
    }

    fn assert_contract_roundtrip_rejects_unknown<T>(contract: T)
    where
        T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug + PartialEq,
    {
        let encoded = serde_json::to_string_pretty(&contract).expect("serialize contract");
        let decoded: T = serde_json::from_str(&encoded).expect("deserialize contract");
        assert_eq!(decoded, contract);

        let mut value: serde_json::Value = serde_json::from_str(&encoded).expect("contract value");
        value["unknown_field"] = serde_json::json!("not allowed");
        let error = serde_json::from_value::<T>(value).expect_err("unknown field rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    fn sample_scene_plan_output_proposal() -> AgentOutputProposal {
        AgentOutputProposal {
            id: "scene-plan-proposal-001".into(),
            agent: AgentRole::ScenePlanner,
            output: AgentProposalPayload::ScenePlan(Box::new(sample_scene_plan_proposal())),
        }
    }

    fn sample_reproducibility_metadata() -> ReproducibilityMetadata {
        ReproducibilityMetadata {
            run_seed: 7,
            prompt_version: "plotforge-agent-text-prompt-v1".into(),
            model_version: "fake-text-model-v1".into(),
            provider_config_hash: "sha256:fake-text-provider-config".into(),
            trace_id: None,
            snapshot_id: None,
        }
    }

    fn sample_agent_output_envelope() -> AgentOutputEnvelope {
        let proposal = sample_scene_plan_output_proposal();
        AgentOutputEnvelope {
            id: "scene-plan-envelope-001".into(),
            contract_version: CONTRACT_VERSION.into(),
            schema_version: CONTRACT_SCHEMA_VERSION,
            agent: proposal.agent.clone(),
            reproducibility: sample_reproducibility_metadata(),
            proposal,
        }
    }

    fn sample_ai_usage_manifest(game: &GameProject) -> AiUsageManifest {
        AiUsageManifest {
            manifest_version: "2026-06-08".into(),
            project_id: game.id.clone(),
            project_version: game.version.clone(),
            export_profile: ExportProfile::static_web(),
            generated_by: "test".into(),
            external_model_calls_during_export: false,
            provider_credentials_included: false,
            raw_provider_responses_included: false,
            private_traces_included: false,
            disclosures: vec![AiUsageDisclosure {
                content_kind: AiUsageContentKind::Image,
                source_kind: AiUsageSourceKind::LocalMockProvider,
                summary: "Placeholder scene artwork generated by a local fake provider.".into(),
                asset_paths: vec!["assets/generated/placeholder.png".into()],
            }],
            provider_summaries: vec![AiProviderSummary {
                provider: "fake-image-provider".into(),
                model: None,
                generated_asset_count: 1,
                fallback_asset_count: 0,
                prompt_hashes: vec!["sha256:abc".into()],
            }],
            ai_safety_policy: Default::default(),
            notices: vec![
                "No provider credentials, raw provider responses, or private traces are included."
                    .into(),
            ],
        }
    }

    fn sample_workshop_item_package() -> WorkshopItemPackage {
        WorkshopItemPackage {
            manifest_version: "2026-06-08".into(),
            package_id: "dynasty-embers-workshop-draft".into(),
            title: "Dynasty Embers".into(),
            description: "Offline Workshop package draft for local validation.".into(),
            visibility: WorkshopDraftVisibility::PrivateDraft,
            preview_image: "preview.png".into(),
            content_root: "content".into(),
            tags: vec!["story-game".into(), "strategy".into()],
            export_profile: ExportProfile::steam_workshop(),
            ai_usage_manifest_path: AI_USAGE_MANIFEST_FILE.into(),
            content_files: vec![WorkshopPackageFile {
                path: "content/game.json".into(),
                content_hash: "sha256:abc".into(),
                hash_algorithm: "sha256".into(),
                byte_length: 42,
            }],
            notices: vec![
                "Local package validation only; no upload integration is included.".into(),
                "This package does not promise platform approval or release readiness.".into(),
            ],
        }
    }

    fn sample_workshop_publish_draft() -> WorkshopPublishDraft {
        WorkshopPublishDraft {
            manifest_version: "2026-06-09".into(),
            package_id: "dynasty-embers-workshop-draft".into(),
            title: "Dynasty Embers".into(),
            description: "Offline Workshop package draft for local validation.".into(),
            visibility: WorkshopDraftVisibility::PrivateDraft,
            preview_image: "preview.png".into(),
            content_root: "content".into(),
            tags: vec!["story-game".into(), "strategy".into()],
            ai_usage_manifest_path: AI_USAGE_MANIFEST_FILE.into(),
            package_files: vec![WorkshopPackageFile {
                path: "content/game.json".into(),
                content_hash: "sha256:abc".into(),
                hash_algorithm: "sha256".into(),
                byte_length: 42,
            }],
            generated_by: "plotforge-workshop 0.1.0".into(),
            upload_enabled: false,
            requires_explicit_steamworks_credentials: true,
            steamworks_api_called: false,
            notices: vec![
                "Local publish draft only; no Steamworks API call was made.".into(),
                "A future upload adapter must require explicit credentials and configuration."
                    .into(),
            ],
        }
    }

    fn sample_desktop_runtime_draft(game: &GameProject) -> DesktopRuntimeDraft {
        DesktopRuntimeDraft {
            manifest_version: "2026-06-08".into(),
            project_id: game.id.clone(),
            project_version: game.version.clone(),
            export_profile: ExportProfile::desktop_bundle(),
            static_manifest_path: "game.json".into(),
            ai_usage_manifest_path: AI_USAGE_MANIFEST_FILE.into(),
            build_notes_markdown: "# Desktop Runtime Draft\n".into(),
            package_files: vec![WorkshopPackageFile {
                path: "game.json".into(),
                content_hash: "sha256:abc".into(),
                hash_algorithm: "sha256".into(),
                byte_length: 42,
            }],
            runtime_entrypoint: "index.html".into(),
            requires_network_at_runtime: false,
            provider_credentials_included: false,
            private_traces_included: false,
            raw_provider_responses_included: false,
            notices: vec![
                "Local desktop runtime draft only; no installer or platform submission is generated."
                    .into(),
                "No provider credentials, raw provider responses, or private traces are included."
                    .into(),
            ],
        }
    }

    fn sample_steam_submission_kit_request() -> SteamSubmissionKitRequest {
        SteamSubmissionKitRequest {
            product_name: "Dynasty Embers".into(),
            desktop_build_path: Some("builds/dynasty-embers-desktop.zip".into()),
            store_short_description: "A branching court drama built with PlotForge.".into(),
            screenshot_paths: vec!["media/screenshots/court-crisis.png".into()],
            capsule_asset_paths: vec!["media/capsules/header.png".into()],
            content_warnings: vec!["Political conflict".into(), "Textual violence".into()],
            safety_guardrails: vec![
                "No live-generated AI content is included in this package.".into(),
                "Moderation review is required before store submission.".into(),
            ],
            user_reporting_path: "support@example.invalid".into(),
            moderation_policy: "Review player-visible text and imagery before release.".into(),
            build_notes: vec![
                "Desktop build must be tested on supported operating systems.".into(),
            ],
        }
    }

    fn sample_steam_submission_kit_draft() -> SteamSubmissionKitDraft {
        SteamSubmissionKitDraft {
            manifest_version: "2026-06-08".into(),
            product_name: "Dynasty Embers".into(),
            workshop_package_id: "dynasty-embers-workshop-draft".into(),
            generated_by: "plotforge-workshop 0.1.0".into(),
            source_workshop_manifest_path: "workshop-item.json".into(),
            store_copy_markdown: "# Store Copy Draft\n".into(),
            checklist_markdown: "# Steam Submission Checklist Draft\n".into(),
            ai_disclosure_markdown: "# Steam AI Disclosure Draft\n".into(),
            content_warnings_markdown: "# Content Warnings Draft\n".into(),
            asset_references_markdown: "# Screenshots And Capsule References\n".into(),
            steam_direct_checklist_markdown: "# Steam Direct Checklist Draft\n".into(),
            content_safety_checklist_markdown: "# Content Safety Checklist Draft\n".into(),
            packaging_notes_markdown: "# Packaging Notes\n".into(),
            official_reference_urls: vec![
                "https://partner.steamgames.com/doc/gettingstarted/contentsurvey".into(),
                "https://partner.steamgames.com/doc/store/review_process".into(),
            ],
            notices: vec![
                "Draft support material only; it does not submit content to Steam.".into(),
                "Responsible developers must review current Steamworks requirements.".into(),
            ],
        }
    }

    fn sample_beat_drafts_output_proposal() -> AgentOutputProposal {
        AgentOutputProposal {
            id: "beat-drafts-proposal-001".into(),
            agent: AgentRole::BeatWriter,
            output: AgentProposalPayload::BeatDrafts(Box::new(BeatDraftsProposal {
                scene_key: "court-crisis-002".into(),
                beats: vec![sample_beat_draft_proposal()],
            })),
        }
    }

    fn sample_review_output_proposal() -> AgentOutputProposal {
        AgentOutputProposal {
            id: "review-proposal-001".into(),
            agent: AgentRole::PlotDoctor,
            output: AgentProposalPayload::Review(Box::new(sample_review_proposal())),
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
                input_terms: vec!["inspect".into(), "corruption".into()],
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

    fn sample_asset_record() -> AssetRecord {
        AssetRecord {
            id: "asset-image-abcdef0123456789".into(),
            kind: AssetKind::Image,
            source: AssetSourceKind::Generated,
            project_path: "assets/generated/court-crisis-001.png".into(),
            export_path: "assets/generated/court-crisis-001.png".into(),
            content_hash: "a".repeat(64),
            hash_algorithm: "sha256".into(),
            byte_length: 256,
            provider_metadata: Some(AssetProviderMetadata {
                provider: "fake-image-provider".into(),
                model: Some("placeholder-v1".into()),
                request_id: Some("request-1".into()),
                prompt_hash: Some("prompt-hash".into()),
                fallback_used: false,
            }),
            references: vec![AssetReference {
                reference_kind: AssetReferenceKind::Scene,
                reference_id: "court-crisis-001".into(),
                slot: "background_asset".into(),
            }],
        }
    }

    fn sample_job_record() -> JobRecord {
        JobRecord {
            id: "job-000001".into(),
            kind: JobKind::ImageGeneration,
            status: JobStatus::Running,
            attempt: 1,
            max_attempts: 3,
            created_at_ms: 100,
            updated_at_ms: 150,
            started_at_ms: Some(125),
            finished_at_ms: None,
            timeout_ms: 10_000,
            progress: JobProgress {
                completed_units: 2,
                total_units: 5,
                message: Some("rendering".into()),
            },
            cost: JobCost {
                estimated_units: 500,
                spent_units: 100,
            },
            failure: None,
        }
    }
}
