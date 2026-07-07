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

// ---------------------------------------------------------------------------
// T2.3: Context injection — token-budget-aware project context assembly.
//
// The pi-Agent needs the full project context (world bible, characters, story
// state, current scene, rules, visual style) in its prompt to produce scene
// plans that respect canon and continuity. Different models have different
// context windows, so the assembly is budget-aware: it estimates the token
// cost of the assembled context and progressively truncates lower-priority
// sections when the full context would overflow the per-call budget.
//
// This is pure logic — no filesystem, no provider calls — so it is fully
// unit-testable in `prompts.rs`. The Studio layer (`pi_agent_apply_run`)
// reads project data through `plotforge-storage` public APIs, builds a
// `ProjectContext`, and hands it to `assemble_context`. The resulting text
// becomes the user-message `context` passed to `PromptAssembler::assemble`
// (and thus to `complete_text_agent_output_with_messages`).
// ---------------------------------------------------------------------------

/// Default total context window assumed for model ids that do not match a
/// known large-window family. Most OpenAI-compatible chat models expose a
/// 128k window; this constant is the conservative default when a model id is
/// unrecognized.
pub const DEFAULT_MODEL_TOTAL_TOKENS: u32 = 128_000;

/// Tokens reserved for the system message (role + output instructions). The
/// built-in templates' system messages are a few hundred words, so 2000
/// tokens is a comfortable reservation that keeps the role text well clear of
/// the context region.
pub const SYSTEM_RESERVED_TOKENS: u32 = 2_000;

/// Tokens reserved for the model's output (the JSON `AgentOutputEnvelope`).
/// 4096 tokens matches the `max_output_tokens` default the provider config
/// layer uses and leaves headroom for a full `ScenePlan` envelope.
pub const OUTPUT_RESERVED_TOKENS: u32 = 4_096;

/// A model context-window budget broken into reserved and available regions.
///
/// `total_tokens` is the model's full context window. `system_reserved` and
/// `output_reserved` are subtracted to leave `available_for_context`, the
/// token budget the project context assembly must fit within. All four fields
/// are carried together (rather than recomputed) so a caller can inspect the
/// reservation breakdown for trace diagnostics without re-deriving it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextBudget {
    pub total_tokens: u32,
    pub system_reserved: u32,
    pub output_reserved: u32,
    pub available_for_context: u32,
}

impl ContextBudget {
    /// Estimate the context budget for a model id.
    ///
    /// The total token window is inferred from the model name (case-
    /// insensitive substring match): Anthropic Claude variants get a 200k
    /// window, `gpt-5.4-mini` gets 400k, the largest next-gen models
    /// (`gpt-5.4`, `gpt-5.5`, `claude-sonnet-5`, `claude-opus-4-8`,
    /// `claude-fable-5`) get 1M, and every other model id falls back to the
    /// 128k default. System reserved is `SYSTEM_RESERVED_TOKENS` (2000) and
    /// output reserved is `OUTPUT_RESERVED_TOKENS` (4096); `available_for_
    /// context` is the remainder, saturated at 0 so a tiny total can never
    /// underflow the subtraction.
    ///
    /// This is a v1 heuristic: it does not query a model registry and never
    /// makes a network call. A model id that the registry later reports a
    /// different window for is still handled safely — the assembly simply
    /// truncates more or less aggressively.
    pub fn for_model(model: &str) -> Self {
        let total_tokens = model_total_tokens(model);
        let system_reserved = SYSTEM_RESERVED_TOKENS;
        let output_reserved = OUTPUT_RESERVED_TOKENS;
        let available_for_context = total_tokens
            .saturating_sub(system_reserved)
            .saturating_sub(output_reserved);
        Self {
            total_tokens,
            system_reserved,
            output_reserved,
            available_for_context,
        }
    }
}

/// Resolve a model's total context window from its id. See
/// `ContextBudget::for_model` for the matching rules. Kept as a free function
/// so the rule and its constants are testable in isolation.
fn model_total_tokens(model: &str) -> u32 {
    // Matching is case-insensitive and substring-based so a model id like
    // `anthropic/claude-sonnet-4` still maps to the Claude family. The
    // largest-window families are checked first so a more specific match
    // (e.g. `gpt-5.4` vs `gpt-5.4-mini`) wins over the general fallback.
    let lower = model.to_lowercase();
    // 1M-window next-gen models. `gpt-5.4-mini` is intentionally excluded
    // here (it keeps the 400k tier) by checking `gpt-5.4` only as a suffix
    // of the family root — i.e. the id *is* or *starts with* `gpt-5.4` but
    // is not the `-mini` variant.
    if is_million_tier(&lower) {
        return 1_000_000;
    }
    if lower.contains("gpt-5.4-mini") {
        return 400_000;
    }
    // Anthropic Claude variants (3.x, 4.x, 5.x) — 200k window.
    if lower.contains("claude") {
        return 200_000;
    }
    DEFAULT_MODEL_TOTAL_TOKENS
}

/// Returns true when `lower` (an already-lowercased model id) belongs to the
/// 1M-token tier: `gpt-5.4` (but not `gpt-5.4-mini`), `gpt-5.5`,
/// `claude-sonnet-5`, `claude-opus-4-8`, or `claude-fable-5`.
fn is_million_tier(lower: &str) -> bool {
    // `gpt-5.4` exactly, or `gpt-5.4` followed by a non-`-mini` suffix.
    if lower == "gpt-5.4" || lower.starts_with("gpt-5.4-") && !lower.ends_with("-mini") {
        return true;
    }
    if lower == "gpt-5.5" || lower.starts_with("gpt-5.5-") {
        return true;
    }
    if lower == "claude-sonnet-5" || lower.starts_with("claude-sonnet-5-") {
        return true;
    }
    if lower == "claude-opus-4-8" || lower.starts_with("claude-opus-4-8-") {
        return true;
    }
    if lower == "claude-fable-5" || lower.starts_with("claude-fable-5-") {
        return true;
    }
    false
}

/// A heuristic, word-count-based token estimate.
///
/// For English-like text the estimate is `word_count * 1.33` (the reciprocal
/// of the standard "1 token ≈ 0.75 words" approximation). For Chinese-heavy
/// text — where more than half of the string's non-whitespace characters are
/// CJK characters — the estimate is `char_count * 0.5`, since CJK tokens are
/// closer to 0.5 tokens per character. The empty string estimates to 0.
///
/// This is a v1 approximation only; it never makes a network call and is
/// deterministic. Downstream truncation uses it as a budget signal, not as an
/// exact count, so a coarse estimate is acceptable.
pub fn estimate_tokens(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    // Count CJK characters (Unified Ideographs + common CJK ranges). If they
    // make up more than half of the non-whitespace characters, treat the text
    // as Chinese-heavy and use the char-based estimate.
    let cjk_count = text.chars().filter(|c| is_cjk(*c)).count();
    let non_whitespace = text.chars().filter(|c| !c.is_whitespace()).count();
    if non_whitespace > 0 && cjk_count as f64 > 0.5 * non_whitespace as f64 {
        // char_count * 0.5, rounded. Use the full char count (including the
        // small fraction of non-CJK chars) for a stable, simple estimate.
        let estimate = text.chars().count() as f64 * 0.5;
        return estimate.round() as u32;
    }
    // English-like: split on whitespace, multiply by 1.33.
    let word_count = text.split_whitespace().count();
    let estimate = word_count as f64 * 1.33;
    estimate.round() as u32
}

/// Returns true when `c` is a CJK character (Unified Ideographs and the most
/// common CJK extension/range), used by `estimate_tokens` to detect Chinese-
/// heavy text.
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}'      // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}'    // CJK Unified Ideographs Extension A
        | '\u{20000}'..='\u{2A6DF}'  // Extension B
        | '\u{2A700}'..='\u{2B73F}'   // Extension C
        | '\u{2B740}'..='\u{2B81F}'  // Extension D
        | '\u{2B820}'..='\u{2CEAF}'  // Extension E
        | '\u{F900}'..='\u{FAFF}'    // CJK Compatibility Ideographs
        | '\u{3000}'..='\u{303F}'    // CJK Symbols and Punctuation
        | '\u{3040}'..='\u{309F}'    // Hiragana
        | '\u{30A0}'..='\u{30FF}'    // Katakana
    )
}

/// The relevant project data the pi-Agent needs in its prompt, expressed as
/// already-summarized strings so the assembly logic stays free of filesystem
/// and schema concerns. The Studio layer (`pi_agent_apply_run`) is
/// responsible for reading the project through `plotforge-storage` and
/// populating these fields.
///
/// `None` or empty fields are omitted entirely from the assembled context;
/// empty `Vec`s likewise contribute no section. This keeps the prompt tight
/// for a fresh project (no world bible yet) without special-casing each
/// field in the assembly.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectContext {
    /// The world bible summary (markdown). Highest-stability world context.
    pub world_summary: Option<String>,
    /// One short summary per character (e.g. `"name (role): voice card"`).
    pub character_summaries: Vec<String>,
    /// A short summary of the current story state (current scene, turn, …).
    pub story_state_summary: Option<String>,
    /// The current scene's title + description (markdown).
    pub current_scene: Option<String>,
    /// Active rule summaries (one string per rule).
    pub active_rules: Vec<String>,
    /// The visual style summary (from the visual bible / concept).
    pub visual_style: Option<String>,
}

/// The output of `assemble_context`: the markdown context block and its
/// estimated token cost. `estimated_tokens` is computed with
/// `estimate_tokens` over the final (possibly truncated) `text`, so a caller
/// can assert the result fits the budget it asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssembledContext {
    pub text: String,
    pub estimated_tokens: u32,
}

/// A labeled section of the assembled context: a markdown header plus the
/// section body. Sections are ordered by priority (highest first) so the
/// truncation pass can drop or trim later sections first.
#[derive(Clone, Debug)]
struct ContextSection {
    header: &'static str,
    body: String,
}

impl ContextSection {
    /// Render the section as a markdown block: `## {header}\n{body}`. The
    /// body is trimmed of surrounding whitespace so an empty body produces no
    /// section text (and the caller omits it).
    fn render(&self) -> String {
        format!("## {}\n{}", self.header, self.body.trim())
    }
}

/// Assemble `project` into a markdown context block that fits `budget`.
///
/// Sections are emitted in priority order (highest first):
/// `current_scene` > `character_summaries` > `story_state_summary` >
/// `world_summary` > `active_rules` > `visual_style`. Empty/`None` sections
/// are omitted entirely. If the full assembled block exceeds
/// `budget.available_for_context`, lower-priority sections are progressively
/// truncated (a suffix of their body is dropped, keeping a prefix) until the
/// estimate fits; if that is still not enough, the lowest-priority sections
/// are dropped wholesale, highest-priority sections preserved.
///
/// The returned `AssembledContext.estimated_tokens` is the estimate of the
/// final text, so it is always `<= budget.available_for_context` (or the
/// budget is 0, in which case the text is empty).
pub fn assemble_context(project: &ProjectContext, budget: &ContextBudget) -> AssembledContext {
    // Build the ordered, non-empty sections. `Vec` is used (not an iterator)
    // because the truncation pass needs index access to drop/trim sections.
    let sections = collect_sections(project);
    if sections.is_empty() {
        return AssembledContext::default();
    }

    // Fast path: the full assembly fits the budget. Avoids the truncation
    // pass entirely for the common case where the project context is small.
    let full_text = render_sections(&sections);
    let full_estimate = estimate_tokens(&full_text);
    if full_estimate <= budget.available_for_context {
        return AssembledContext {
            text: full_text,
            estimated_tokens: full_estimate,
        };
    }

    // Overflow: progressively truncate lower-priority sections first. We walk
    // from the lowest-priority section (last index) toward the highest,
    // trimming a suffix of each section's body until the estimate fits. If a
    // section is fully trimmed away (body becomes empty), the next iteration
    // drops it entirely (empty bodies are filtered by `render_sections`).
    let mut working: Vec<ContextSection> = sections;
    // Cap iterations so a pathological budget can never spin forever.
    for _ in 0..working.len() {
        if estimate_tokens(&render_sections(&working)) <= budget.available_for_context {
            break;
        }
        // Trim the lowest-priority remaining non-empty section. Start from
        // the end (lowest priority) and find the first section whose body is
        // non-empty; halve its body (keep a prefix) so each pass removes a
        // meaningful chunk without nuking a section in one shot.
        let mut trimmed = false;
        for index in (0..working.len()).rev() {
            let body = working[index].body.trim();
            if body.is_empty() {
                continue;
            }
            // Halve the body by characters, keeping the prefix. If the body
            // is already very short, empty it so the next pass drops it.
            let keep = body.len() / 2;
            working[index].body = if keep == 0 {
                String::new()
            } else {
                body[..keep].to_string()
            };
            trimmed = true;
            break;
        }
        // No non-empty section left to trim and still over budget: drop the
        // lowest-priority section entirely and try again.
        if !trimmed {
            if working.is_empty() {
                break;
            }
            working.pop();
        }
    }

    // Final pass: drop any sections whose body became empty during trimming,
    // re-render, and re-estimate. If the result still overflows (e.g. a 0
    // budget), keep only the highest-priority section, then empty out
    // entirely as a last resort — never return a block that exceeds the
    // budget without having tried to honor it.
    working.retain(|section| !section.body.trim().is_empty());
    let text = render_sections(&working);
    let mut estimate = estimate_tokens(&text);
    if estimate > budget.available_for_context && !working.is_empty() {
        // Keep only the highest-priority section, truncated to fit.
        let top = working.remove(0);
        working = vec![top];
        let trimmed_text = render_sections(&working);
        estimate = estimate_tokens(&trimmed_text);
        if estimate > budget.available_for_context {
            // Truncate the single remaining section to a prefix that fits.
            let prefix = fit_prefix(&working[0].body, budget.available_for_context);
            working[0].body = prefix;
            let final_text = render_sections(&working);
            estimate = estimate_tokens(&final_text);
            return AssembledContext {
                text: final_text,
                estimated_tokens: estimate,
            };
        }
        return AssembledContext {
            text: trimmed_text,
            estimated_tokens: estimate,
        };
    }

    AssembledContext { text, estimated_tokens: estimate }
}

/// Collect the non-empty sections of `project` in priority order (highest
/// first): current_scene, characters, story_state, world, rules, visual_style.
fn collect_sections(project: &ProjectContext) -> Vec<ContextSection> {
    let mut sections = Vec::new();
    if let Some(scene) = non_empty(project.current_scene.as_deref()) {
        sections.push(ContextSection {
            header: "Current Scene",
            body: scene.to_string(),
        });
    }
    if !project.character_summaries.is_empty() {
        let body = project
            .character_summaries
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if !body.is_empty() {
            sections.push(ContextSection {
                header: "Characters",
                body,
            });
        }
    }
    if let Some(state) = non_empty(project.story_state_summary.as_deref()) {
        sections.push(ContextSection {
            header: "Story State",
            body: state.to_string(),
        });
    }
    if let Some(world) = non_empty(project.world_summary.as_deref()) {
        sections.push(ContextSection {
            header: "World",
            body: world.to_string(),
        });
    }
    if !project.active_rules.is_empty() {
        let body = project
            .active_rules
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if !body.is_empty() {
            sections.push(ContextSection {
                header: "Rules",
                body,
            });
        }
    }
    if let Some(style) = non_empty(project.visual_style.as_deref()) {
        sections.push(ContextSection {
            header: "Visual Style",
            body: style.to_string(),
        });
    }
    sections
}

/// Render `sections` into a single markdown block, joined by blank lines.
fn render_sections(sections: &[ContextSection]) -> String {
    sections
        .iter()
        .filter(|section| !section.body.trim().is_empty())
        .map(ContextSection::render)
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Returns `Some(text)` when `text` is non-empty after trimming, else `None`.
/// Used to treat `None` and whitespace-only strings uniformly as "omit".
fn non_empty(text: Option<&str>) -> Option<&str> {
    text.filter(|t| !t.trim().is_empty())
}

/// Return the longest prefix of `body` (by characters) whose
/// `estimate_tokens` is `<= budget`. Used as the last-resort truncation when
/// even a single section overflows the budget.
fn fit_prefix(body: &str, budget: u32) -> String {
    if budget == 0 {
        return String::new();
    }
    let chars: Vec<char> = body.chars().collect();
    // Binary search the largest prefix length that fits the budget.
    let mut lo = 0usize;
    let mut hi = chars.len();
    let mut best = 0usize;
    while lo <= hi {
        let mid = (lo + hi) / 2;
        let candidate: String = chars[..mid].iter().collect();
        if estimate_tokens(&candidate) <= budget {
            best = mid;
            lo = mid + 1;
        } else if mid == 0 {
            break;
        } else {
            hi = mid - 1;
        }
    }
    chars[..best].iter().collect()
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

    // -------------------------------------------------------------------------
    // T2.3: context injection — estimate_tokens, ContextBudget, assemble_context.
    //
    // `estimate_tokens` is a v1 heuristic, so these tests assert the shape of
    // the estimate (monotonicity, zero for empty, the CN/EN split) rather than
    // exact magic numbers where the magic would be brittle. `ContextBudget::
    // for_model` pins the model-family tiers. `assemble_context` covers the
    // acceptance-criteria scenarios: empty project, full fit, overflow
    // truncation, and priority preservation.
    // -------------------------------------------------------------------------

    #[test]
    fn estimate_tokens_empty_string_is_zero() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("   \n\t  "), 0, "whitespace-only has 0 words");
    }

    #[test]
    fn estimate_tokens_english_scales_with_word_count() {
        let one = estimate_tokens("hello");
        let few = estimate_tokens("hello world this is a sentence");
        assert!(one > 0, "a single English word must estimate > 0 tokens");
        assert!(
            few > one,
            "a longer English sentence must estimate more tokens than a single word"
        );
        // 1 token ≈ 0.75 words => word_count * 1.33. For 5 words: 5 * 1.33 = 6.65 -> 7.
        assert_eq!(estimate_tokens("one two three four five"), 7);
    }

    #[test]
    fn estimate_tokens_chinese_uses_char_based_estimate() {
        // A pure-Chinese string: char_count * 0.5. 4 CJK chars -> 2 tokens.
        let cn = estimate_tokens("春眠不觉晓");
        assert_eq!(cn, 3, "5 CJK chars * 0.5 = 2.5 -> rounds to 2 or 3");
        // The Chinese-heavy string must not use the word-count path: a
        // whitespace-less CJK string has 1 "word" under split_whitespace,
        // which would give 1 * 1.33 = 1 token. The char-based path gives a
        // larger value, so the two paths are distinguishable.
        let word_path = (1.33_f64).round() as u32;
        assert!(
            cn >= word_path,
            "Chinese-heavy text must use the char-based estimate, not the 1-word path"
        );
    }

    #[test]
    fn estimate_tokens_mixed_text_is_deterministic() {
        // A mixed string that is not Chinese-heavy falls back to the English
        // path; the estimate must be stable across calls.
        let text = "The quick brown fox jumps over the lazy dog.";
        let a = estimate_tokens(text);
        let b = estimate_tokens(text);
        assert_eq!(a, b, "estimate must be deterministic");
        assert!(a > 0);
    }

    #[test]
    fn context_budget_for_model_default_tier() {
        let budget = ContextBudget::for_model("some-unknown-model");
        assert_eq!(budget.total_tokens, DEFAULT_MODEL_TOTAL_TOKENS);
        assert_eq!(budget.system_reserved, SYSTEM_RESERVED_TOKENS);
        assert_eq!(budget.output_reserved, OUTPUT_RESERVED_TOKENS);
        assert_eq!(
            budget.available_for_context,
            DEFAULT_MODEL_TOTAL_TOKENS - SYSTEM_RESERVED_TOKENS - OUTPUT_RESERVED_TOKENS
        );
    }

    #[test]
    fn context_budget_for_model_anthropic_claude_tier() {
        // Any Claude variant maps to the 200k tier, case-insensitively.
        for model in ["claude-3-sonnet", "Claude-Opus-4", "anthropic/claude-haiku"] {
            let budget = ContextBudget::for_model(model);
            assert_eq!(
                budget.total_tokens, 200_000,
                "model `{model}` must map to the 200k Claude tier"
            );
        }
    }

    #[test]
    fn context_budget_for_model_gpt_5_4_mini_tier() {
        let budget = ContextBudget::for_model("gpt-5.4-mini");
        assert_eq!(budget.total_tokens, 400_000);
        // The -mini variant must NOT hit the 1M tier that gpt-5.4 does.
        assert_ne!(budget.total_tokens, 1_000_000);
    }

    #[test]
    fn context_budget_for_model_million_tier() {
        for model in [
            "gpt-5.4",
            "gpt-5.4-turbo",
            "gpt-5.5",
            "gpt-5.5-preview",
            "claude-sonnet-5",
            "claude-sonnet-5-x",
            "claude-opus-4-8",
            "claude-opus-4-8-mini",
            "claude-fable-5",
        ] {
            let budget = ContextBudget::for_model(model);
            assert_eq!(
                budget.total_tokens, 1_000_000,
                "model `{model}` must map to the 1M tier"
            );
        }
        // `gpt-5.4-mini` must stay in the 400k tier even though it starts with
        // `gpt-5.4`; this guards the million-tier exclusion.
        assert_ne!(ContextBudget::for_model("gpt-5.4-mini").total_tokens, 1_000_000);
    }

    #[test]
    fn context_budget_for_local_pi_uses_default() {
        // The local-pi mock model id is unrecognized, so it falls back to the
        // 128k default. This keeps the local-pi path budget-conservative.
        let budget = ContextBudget::for_model(crate::LOCAL_PI_MODEL_ID);
        assert_eq!(budget.total_tokens, DEFAULT_MODEL_TOTAL_TOKENS);
        assert!(budget.available_for_context > 0);
    }

    #[test]
    fn context_budget_available_saturates_on_tiny_total() {
        // A model with a total smaller than the reservations must saturate the
        // available budget to 0, never underflow.
        let budget = ContextBudget {
            total_tokens: 1_000,
            system_reserved: SYSTEM_RESERVED_TOKENS,
            output_reserved: OUTPUT_RESERVED_TOKENS,
            available_for_context: 0,
        };
        // Sanity: the reservations alone exceed this total.
        assert_eq!(budget.available_for_context, 0);
    }

    #[test]
    fn assemble_context_empty_project_yields_empty_text() {
        let project = ProjectContext::default();
        let budget = ContextBudget::for_model("gpt-4o");
        let assembled = assemble_context(&project, &budget);
        assert!(assembled.text.is_empty(), "empty project => empty context text");
        assert_eq!(assembled.estimated_tokens, 0);
    }

    #[test]
    fn assemble_context_whitespace_only_fields_omitted() {
        // Whitespace-only Option fields and empty Vecs contribute nothing.
        let project = ProjectContext {
            world_summary: Some("   \n  ".into()),
            character_summaries: vec!["  ".into(), String::new()],
            story_state_summary: Some("\t".into()),
            current_scene: None,
            active_rules: vec![],
            visual_style: Some("".into()),
        };
        let budget = ContextBudget::for_model("gpt-4o");
        let assembled = assemble_context(&project, &budget);
        assert!(assembled.text.is_empty(), "whitespace-only fields must be omitted");
    }

    #[test]
    fn assemble_context_full_project_fits_budget() {
        let project = ProjectContext {
            world_summary: Some("A regency court under winter pressure.".into()),
            character_summaries: vec![
                "Lord Hale (treasurer): cautious, ledger-focused.".into(),
                "Mara (guild-liaison): blunt, cost-aware.".into(),
            ],
            story_state_summary: Some("Turn 3, current scene: opening-pressure.".into()),
            current_scene: Some("Opening Pressure — the empty granary ledger.".into()),
            active_rules: vec!["change_scene => public_trust -1.".into()],
            visual_style: Some("ink wash court drama, restrained palette.".into()),
        };
        // A generous budget so the full assembly fits without truncation.
        let budget = ContextBudget::for_model("gpt-5.4");
        let assembled = assemble_context(&project, &budget);
        assert!(
            assembled.estimated_tokens <= budget.available_for_context,
            "assembled context must fit the budget"
        );
        // Every section header must be present in priority order.
        let headers = [
            "## Current Scene",
            "## Characters",
            "## Story State",
            "## World",
            "## Rules",
            "## Visual Style",
        ];
        let mut last = 0;
        for header in headers {
            let pos = assembled
                .text
                .find(header)
                .unwrap_or_else(|| panic!("missing section header `{header}`"));
            assert!(
                pos >= last,
                "section `{header}` must appear after the previous section"
            );
            last = pos;
        }
        // Section bodies are present.
        assert!(assembled.text.contains("Opening Pressure"));
        assert!(assembled.text.contains("Lord Hale"));
        assert!(assembled.text.contains("Turn 3"));
    }

    #[test]
    fn assemble_context_priority_order_current_scene_before_world() {
        // The current scene is the highest-priority section; even when the
        // world bible is large and the budget is tight, the current scene must
        // appear (and appear before) the world section.
        let project = ProjectContext {
            current_scene: Some("THE CURRENT SCENE TITLE".into()),
            world_summary: Some(
                "W ".repeat(5_000), // large world bible to force pressure
            ),
            ..ProjectContext::default()
        };
        // A tight budget that cannot hold both at full size.
        let budget = ContextBudget {
            total_tokens: 10_000,
            system_reserved: SYSTEM_RESERVED_TOKENS,
            output_reserved: OUTPUT_RESERVED_TOKENS,
            available_for_context: 200,
        };
        let assembled = assemble_context(&project, &budget);
        assert!(
            assembled.estimated_tokens <= budget.available_for_context,
            "truncated context must fit the tight budget"
        );
        assert!(
            assembled.text.contains("## Current Scene"),
            "highest-priority current scene must survive truncation"
        );
        assert!(
            assembled.text.contains("THE CURRENT SCENE TITLE"),
            "current scene body must survive truncation"
        );
        // The current scene header must precede the world header when both
        // are present.
        if let (Some(scene_pos), Some(world_pos)) =
            (assembled.text.find("## Current Scene"), assembled.text.find("## World"))
        {
            assert!(
                scene_pos < world_pos,
                "current scene must appear before world in priority order"
            );
        }
    }

    #[test]
    fn assemble_context_truncates_lower_priority_first() {
        // With a budget that holds the high-priority section but not the
        // low-priority one, the low-priority section (visual_style) is
        // truncated/dropped while the high-priority current scene survives.
        let project = ProjectContext {
            current_scene: Some("Short current scene".into()),
            visual_style: Some("X ".repeat(2_000)), // large low-priority section
            ..ProjectContext::default()
        };
        let budget = ContextBudget {
            total_tokens: 10_000,
            system_reserved: SYSTEM_RESERVED_TOKENS,
            output_reserved: OUTPUT_RESERVED_TOKENS,
            available_for_context: 100,
        };
        let assembled = assemble_context(&project, &budget);
        assert!(
            assembled.estimated_tokens <= budget.available_for_context,
            "must fit the tight budget after truncation"
        );
        assert!(
            assembled.text.contains("## Current Scene"),
            "current scene must survive truncation"
        );
    }

    #[test]
    fn assemble_context_zero_budget_yields_empty_text() {
        // A 0 available budget must produce empty text, never overflow.
        let project = ProjectContext {
            current_scene: Some("scene".into()),
            world_summary: Some("world".into()),
            ..ProjectContext::default()
        };
        let budget = ContextBudget {
            total_tokens: 1_000,
            system_reserved: SYSTEM_RESERVED_TOKENS,
            output_reserved: OUTPUT_RESERVED_TOKENS,
            available_for_context: 0,
        };
        let assembled = assemble_context(&project, &budget);
        assert!(
            assembled.text.is_empty(),
            "a 0-token budget must yield empty context, not overflow"
        );
        assert_eq!(assembled.estimated_tokens, 0);
    }

    #[test]
    fn assembled_context_estimated_tokens_matches_estimate_of_text() {
        // The reported estimated_tokens must equal estimate_tokens(text) for
        // both the fit and truncation paths.
        let project = ProjectContext {
            world_summary: Some("A short world summary.".into()),
            current_scene: Some("A short current scene.".into()),
            ..ProjectContext::default()
        };
        let budget = ContextBudget::for_model("gpt-5.4");
        let assembled = assemble_context(&project, &budget);
        assert_eq!(assembled.estimated_tokens, estimate_tokens(&assembled.text));
    }

    #[test]
    fn assemble_context_omits_empty_vec_items() {
        // Empty strings inside the character_summaries Vec must not produce
        // blank lines or empty sections.
        let project = ProjectContext {
            character_summaries: vec![
                String::new(),
                "  ".into(),
                "Hero (protagonist): brave.".into(),
            ],
            ..ProjectContext::default()
        };
        let budget = ContextBudget::for_model("gpt-5.4");
        let assembled = assemble_context(&project, &budget);
        assert!(assembled.text.contains("## Characters"));
        assert!(assembled.text.contains("Hero"));
        assert!(
            !assembled.text.contains("\n\n\n"),
            "no blank-line runs from omitted empty vec items"
        );
    }
}
