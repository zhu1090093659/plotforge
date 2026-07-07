//! Prompt template system and version management.
//!
//! This module owns the structured prompt templates that assemble system
//! messages, context, and output instructions for each agent role
//! (ScenePlanner, BeatWriter, PlotDoctor). Templates are versioned for
//! reproducibility: every template carries a `version` string that flows into
//! `ReproducibilityMetadata.prompt_version` so a generated envelope can be
//! traced back to the exact prompt template that produced it.
//!
//! ## Additive design
//!
//! The template system is strictly additive with respect to the existing
//! text-output pipeline. The `TextModelRequest` struct gains an optional
//! `messages: Option<Vec<ChatMessage>>` field; when it is `None` the request
//! behaves exactly as before (single `prompt` string). The
//! `TextModelClient` trait and the three HTTP client `complete()` signatures
//! are NOT modified. `complete_text_agent_output` keeps its current path; the
//! new `complete_text_agent_output_with_messages` function constructs a
//! `TextModelRequest` with `messages: Some(messages)` and a `prompt_version`
//! derived from the template, feeding it into `ReproducibilityMetadata`.
//!
//! The legacy `TEXT_PROMPT_VERSION` constant (`plotforge-agent-text-prompt-v1`)
//! is retained for backward compatibility: existing callers that do not pass a
//! template continue to use it. The built-in templates below carry their own
//! per-role version strings (`scene_planner_v1`, `beat_writer_v1`,
//! `plot_doctor_v1`) so reproducibility metadata distinguishes runs that used
//! the structured template path from runs that used the legacy single-prompt
//! path.

use plotforge_schema::AgentRole;

/// The role of a chat message within a multi-turn prompt.
///
/// Mirrors the OpenAI / Anthropic chat-message role vocabulary. `System` is the
/// role/instruction message, `User` carries the per-call context, and
/// `Assistant` is reserved for prior model turns (used by future multi-turn
/// flows). The derive set matches `TextModelRequest`'s derive so a
/// `Vec<ChatMessage>` can be embedded in the request struct without dropping
/// the `PartialEq`/`Eq` bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl MessageRole {
    /// Returns the lowercase wire string for the role (`"system"`, `"user"`,
    /// `"assistant"`). This is the value serialized into chat-completions
    /// request bodies; it is a fixed enumerated token, not user content, so it
    /// is trace-safe.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

/// A single chat message: a role plus content. The content is the prompt text
/// (system message, user context, or a prior assistant turn). It is never used
/// to carry credentials or secret markers; the existing
/// `contains_secret_marker_text` scan in the provider pipeline covers the
/// assembled prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    /// Construct a message with the given role and content.
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Convenience constructor for a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content)
    }

    /// Convenience constructor for a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content)
    }
}

/// A versioned prompt template for an agent role.
///
/// `id` is a stable, human-readable identifier for the template (e.g.
/// `"scene_planner"`); `version` is the reproducibility version string that
/// flows into `ReproducibilityMetadata.prompt_version`. `system_message` is the
/// role/instruction text; `output_instructions` describes the JSON output
/// contract the model must follow. All fields are `&'static str` so a template
/// is a compile-time constant with no allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptTemplate {
    pub id: &'static str,
    pub version: &'static str,
    pub system_message: &'static str,
    pub output_instructions: &'static str,
}

/// Built-in ScenePlanner template (v1).
///
/// The system message defines the ScenePlanner role: produce a scene plan as a
/// JSON `AgentOutputEnvelope` whose `proposal.output` is a `ScenePlan`. The
/// output instructions pin the JSON envelope shape so the downstream repair +
/// validation path can parse the response.
pub const SCENE_PLANNER_V1: PromptTemplate = PromptTemplate {
    id: "scene_planner",
    version: "scene_planner_v1",
    system_message: "You are the ScenePlanner agent for PlotForge. Your job is to \
propose the next scene as a structured scene plan. Given the current story state, \
world bible, canon, and player intent, choose a scene key, title, location, summary, \
dramatic purpose, hook, and entry beat. Keep the player in control: every scene must \
introduce visible pressure and a concrete consequence, never a hidden prophecy or a \
cost-free resolution. Do not invent facts that contradict the canon or the forbidden \
facts. Return your result as a JSON AgentOutputEnvelope whose proposal.output is a \
ScenePlan payload.",
    output_instructions: "Respond with a single JSON object matching the \
AgentOutputEnvelope schema: { id, contract_version, schema_version, agent, \
reproducibility, proposal }. The proposal.output must be a ScenePlan payload with \
non-empty scene_key, title, location, scene_summary, dramatic_purpose, hook, and \
entry_beat_id. Output only the JSON object — no prose, no markdown fences, no \
commentary.",
};

/// Built-in BeatWriter template (v1).
///
/// The system message defines the BeatWriter role: write the beats (narrative
/// text + choices) for a scene plan as a JSON `AgentOutputEnvelope` whose
/// `proposal.output` is a `BeatDrafts` payload. The output instructions pin the
/// JSON envelope shape and require at least one beat with a choice.
pub const BEAT_WRITER_V1: PromptTemplate = PromptTemplate {
    id: "beat_writer",
    version: "beat_writer_v1",
    system_message: "You are the BeatWriter agent for PlotForge. Your job is to write \
the beats for a planned scene. Given the scene plan, world bible, style guide, and \
current story state, write the narrative beats and the choices the player can take. \
Each beat must be specific and consequence-first; each choice must have a clear \
dramatic purpose and an action_type. Keep prose tight and avoid vague grandeur or \
convenient prophecy. Do not contradict canon, forbidden facts, or the scene plan. \
Return your result as a JSON AgentOutputEnvelope whose proposal.output is a \
BeatDrafts payload.",
    output_instructions: "Respond with a single JSON object matching the \
AgentOutputEnvelope schema: { id, contract_version, schema_version, agent, \
reproducibility, proposal }. The proposal.output must be a BeatDrafts payload with \
the scene's scene_key and a non-empty beats list. Each beat needs a unique id, the \
scene_key, narrative text, at least one choice, and a narrative_function. Output only \
the JSON object — no prose, no markdown fences, no commentary.",
};

/// Built-in PlotDoctor template (v1).
///
/// The system message defines the PlotDoctor role: review the narrative and
/// return a `NarrativeReview` inside a JSON `AgentOutputEnvelope` whose
/// `proposal.output` is a `Review` payload. The output instructions pin the
/// JSON envelope shape and require the review scores to be present.
pub const PLOT_DOCTOR_V1: PromptTemplate = PromptTemplate {
    id: "plot_doctor",
    version: "plot_doctor_v1",
    system_message: "You are the PlotDoctor agent for PlotForge. Your job is to review \
the narrative quality of a scene and its beats. Given the scene plan, beats, world \
bible, and style guide, score the scene on hook, pacing, character consistency, \
payoff, and choice meaningfulness; estimate the AI-slop risk; and list concrete issues \
to fix. Be honest and specific: do not inflate scores, and do not rubber-stamp vague \
prose. Return your result as a JSON AgentOutputEnvelope whose proposal.output is a \
Review payload carrying a NarrativeReview.",
    output_instructions: "Respond with a single JSON object matching the \
AgentOutputEnvelope schema: { id, contract_version, schema_version, agent, \
reproducibility, proposal }. The proposal.output must be a Review payload with a \
NarrativeReview that has the scene's scene_key and all score fields populated \
(hook_score, pacing_score, character_consistency_score, payoff_score, \
choice_meaningfulness_score, score, ai_slop_risk). Output only the JSON object — no \
prose, no markdown fences, no commentary.",
};

/// Assembles a chat-message list from a prompt template and per-call context.
///
/// The returned list is exactly two messages:
/// 1. a `System` message whose content is `template.system_message` followed
///    by `"\n\n"` and `template.output_instructions`, so the model sees the
///    role definition and the output contract together;
/// 2. a `User` message whose content is the per-call `context`.
///
/// This shape is what `complete_text_agent_output_with_messages` packs into a
/// `TextModelRequest.messages`. The caller is responsible for redaction
/// (`contains_secret_marker_text`) of the `context` before assembly; the
/// provider pipeline re-scans the assembled prompt as a defence-in-depth
/// measure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PromptAssembler;

impl PromptAssembler {
    /// Assemble the two-message chat list for `template` + `context`.
    pub fn assemble(template: &PromptTemplate, context: &str) -> Vec<ChatMessage> {
        let system_content = format!(
            "{}\n\n{}",
            template.system_message, template.output_instructions
        );
        vec![
            ChatMessage::system(system_content),
            ChatMessage::user(context),
        ]
    }
}

/// Maps an `AgentRole` to the version string of its built-in prompt template.
///
/// Only the three roles with built-in templates (ScenePlanner, BeatWriter,
/// PlotDoctor) map to their per-role version. Every other role maps to the
/// legacy shared `TEXT_PROMPT_VERSION` (`plotforge-agent-text-prompt-v1`) so
/// existing callers that have not yet adopted the structured-template path keep
/// the same reproducibility identity they had before this module landed. This
/// is the additive boundary: a role without a template does not error and does
/// not silently pick a wrong template — it reports the legacy version so
/// reproducibility metadata stays truthful about which prompt path produced the
/// envelope.
pub fn prompt_version_for_role(role: &AgentRole) -> &'static str {
    match role {
        AgentRole::ScenePlanner => SCENE_PLANNER_V1.version,
        AgentRole::BeatWriter => BEAT_WRITER_V1.version,
        AgentRole::PlotDoctor => PLOT_DOCTOR_V1.version,
        // Roles without a built-in template fall back to the legacy shared
        // prompt version so their reproducibility metadata stays stable.
        AgentRole::StoryArchitect
        | AgentRole::StoryCraftPlanner
        | AgentRole::CharacterDesigner
        | AgentRole::ConsistencyChecker
        | AgentRole::DeslopRefiner => crate::shared::TEXT_PROMPT_VERSION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotforge_schema::AgentRole;

    #[test]
    fn message_role_as_str_returns_wire_tokens() {
        assert_eq!(MessageRole::System.as_str(), "system");
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Assistant.as_str(), "assistant");
    }

    #[test]
    fn chat_message_derives_clone_copy_debug_eq() {
        // MessageRole is Copy; ChatMessage is Clone (content is String).
        let role = MessageRole::User;
        let copied_role = role;
        assert_eq!(role, copied_role);

        let msg = ChatMessage::system("hello");
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
        // Debug formatting works (exercises the Debug derive).
        let debug = format!("{msg:?}");
        assert!(debug.contains("ChatMessage"));
        assert!(debug.contains("System"));
    }

    #[test]
    fn chat_message_constructors_set_role_and_content() {
        let sys = ChatMessage::system("system-text");
        assert_eq!(sys.role, MessageRole::System);
        assert_eq!(sys.content, "system-text");

        let usr = ChatMessage::user("user-text");
        assert_eq!(usr.role, MessageRole::User);
        assert_eq!(usr.content, "user-text");

        let general = ChatMessage::new(MessageRole::Assistant, "assistant-text");
        assert_eq!(general.role, MessageRole::Assistant);
        assert_eq!(general.content, "assistant-text");
    }

    #[test]
    fn assembler_returns_system_then_user() {
        let template = PromptTemplate {
            id: "test",
            version: "test_v1",
            system_message: "ROLE",
            output_instructions: "OUTPUT",
        };
        let messages = PromptAssembler::assemble(&template, "CONTEXT");
        assert_eq!(messages.len(), 2, "assemble must produce exactly 2 messages");
        assert_eq!(messages[0].role, MessageRole::System);
        assert_eq!(messages[1].role, MessageRole::User);
    }

    #[test]
    fn assembler_system_message_concatenates_system_and_output_instructions() {
        let template = PromptTemplate {
            id: "test",
            version: "test_v1",
            system_message: "ROLE-TEXT",
            output_instructions: "OUTPUT-TEXT",
        };
        let messages = PromptAssembler::assemble(&template, "ignored-context");
        assert_eq!(
            messages[0].content,
            "ROLE-TEXT\n\nOUTPUT-TEXT",
            "system message must be system_message + \"\\n\\n\" + output_instructions"
        );
        assert!(
            messages[0].content.contains("ROLE-TEXT"),
            "system message must contain template.system_message"
        );
        assert!(
            messages[0].content.contains("OUTPUT-TEXT"),
            "system message must contain template.output_instructions"
        );
    }

    #[test]
    fn assembler_user_message_contains_context() {
        let template = PromptTemplate {
            id: "test",
            version: "test_v1",
            system_message: "ROLE",
            output_instructions: "OUTPUT",
        };
        let messages = PromptAssembler::assemble(&template, "the per-call context");
        assert_eq!(messages[1].role, MessageRole::User);
        assert_eq!(
            messages[1].content, "the per-call context",
            "user message must be the context string verbatim"
        );
    }

    #[test]
    fn assembler_with_empty_context_still_emits_user_message() {
        // Empty context is valid (e.g. a role-only prompt); the user message
        // must still be present so downstream providers see a two-message
        // turn and do not drop the user role.
        let template = PromptTemplate {
            id: "test",
            version: "test_v1",
            system_message: "ROLE",
            output_instructions: "OUTPUT",
        };
        let messages = PromptAssembler::assemble(&template, "");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].role, MessageRole::User);
        assert_eq!(messages[1].content, "");
    }

    #[test]
    fn built_in_templates_have_distinct_versions_and_ids() {
        // The three built-in templates must each have a unique id and version
        // so reproducibility metadata distinguishes them.
        assert_eq!(SCENE_PLANNER_V1.id, "scene_planner");
        assert_eq!(SCENE_PLANNER_V1.version, "scene_planner_v1");
        assert_eq!(BEAT_WRITER_V1.id, "beat_writer");
        assert_eq!(BEAT_WRITER_V1.version, "beat_writer_v1");
        assert_eq!(PLOT_DOCTOR_V1.id, "plot_doctor");
        assert_eq!(PLOT_DOCTOR_V1.version, "plot_doctor_v1");

        let versions = [
            SCENE_PLANNER_V1.version,
            BEAT_WRITER_V1.version,
            PLOT_DOCTOR_V1.version,
        ];
        let unique: std::collections::HashSet<&str> = versions.iter().copied().collect();
        assert_eq!(unique.len(), 3, "built-in template versions must be distinct");
    }

    #[test]
    fn built_in_templates_have_non_empty_system_and_output() {
        for template in [SCENE_PLANNER_V1, BEAT_WRITER_V1, PLOT_DOCTOR_V1] {
            assert!(
                !template.system_message.trim().is_empty(),
                "template {} must have a non-empty system_message",
                template.id
            );
            assert!(
                !template.output_instructions.trim().is_empty(),
                "template {} must have non-empty output_instructions",
                template.id
            );
        }
    }

    #[test]
    fn scene_planner_template_mentions_json_envelope_and_scene_plan() {
        // The template must instruct the model to return a JSON envelope with a
        // ScenePlan payload; this is the contract the repair + validation path
        // relies on.
        let combined = format!(
            "{}\n\n{}",
            SCENE_PLANNER_V1.system_message, SCENE_PLANNER_V1.output_instructions
        );
        let lower = combined.to_lowercase();
        assert!(lower.contains("agentoutputenvelope") || lower.contains("agent output envelope") || lower.contains("json"));
        assert!(lower.contains("sceneplan") || lower.contains("scene plan"));
    }

    #[test]
    fn beat_writer_template_mentions_beats() {
        let combined = format!(
            "{}\n\n{}",
            BEAT_WRITER_V1.system_message, BEAT_WRITER_V1.output_instructions
        );
        let lower = combined.to_lowercase();
        assert!(lower.contains("beatdrafts") || lower.contains("beat drafts") || lower.contains("beats"));
    }

    #[test]
    fn plot_doctor_template_mentions_review() {
        let combined = format!(
            "{}\n\n{}",
            PLOT_DOCTOR_V1.system_message, PLOT_DOCTOR_V1.output_instructions
        );
        let lower = combined.to_lowercase();
        assert!(lower.contains("narrativereview") || lower.contains("narrative review") || lower.contains("review"));
    }

    #[test]
    fn prompt_version_for_role_maps_scene_planner_correctly() {
        assert_eq!(
            prompt_version_for_role(&AgentRole::ScenePlanner),
            SCENE_PLANNER_V1.version
        );
    }

    #[test]
    fn prompt_version_for_role_maps_beat_writer_correctly() {
        assert_eq!(
            prompt_version_for_role(&AgentRole::BeatWriter),
            BEAT_WRITER_V1.version
        );
    }

    #[test]
    fn prompt_version_for_role_maps_plot_doctor_correctly() {
        assert_eq!(
            prompt_version_for_role(&AgentRole::PlotDoctor),
            PLOT_DOCTOR_V1.version
        );
    }

    #[test]
    fn prompt_version_for_role_falls_back_to_legacy_for_other_roles() {
        // Roles without a built-in template fall back to the legacy shared
        // prompt version so their reproducibility metadata stays stable.
        let legacy = crate::shared::TEXT_PROMPT_VERSION;
        for role in [
            AgentRole::StoryArchitect,
            AgentRole::StoryCraftPlanner,
            AgentRole::CharacterDesigner,
            AgentRole::ConsistencyChecker,
            AgentRole::DeslopRefiner,
        ] {
            assert_eq!(
                prompt_version_for_role(&role),
                legacy,
                "role {role:?} must fall back to the legacy prompt version"
            );
        }
    }

    #[test]
    fn prompt_version_for_role_distinct_for_templated_roles() {
        // The three templated roles must each report their own version, not the
        // legacy shared one.
        assert_ne!(
            prompt_version_for_role(&AgentRole::ScenePlanner),
            crate::shared::TEXT_PROMPT_VERSION
        );
        assert_ne!(
            prompt_version_for_role(&AgentRole::BeatWriter),
            crate::shared::TEXT_PROMPT_VERSION
        );
        assert_ne!(
            prompt_version_for_role(&AgentRole::PlotDoctor),
            crate::shared::TEXT_PROMPT_VERSION
        );
    }

    #[test]
    fn prompt_template_is_copy() {
        // PromptTemplate is Copy (&'static str fields); passing by value must
        // not move out of the const. This guards the derive against an
        // accidental drop of the Copy bound.
        let template = SCENE_PLANNER_V1;
        let copied = template;
        assert_eq!(template, copied);
    }
}
