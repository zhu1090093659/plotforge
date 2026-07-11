//! Text model provider port, configured client, and deterministic fake.
//!
//! Holds the `TextModelProvider` trait and its supporting types, the
//! local `ConfiguredTextModelProvider` adapter (credential resolution +
//! config validation + redaction), and `FakeTextModelProvider` with its
//! deterministic agent-output envelopes. Image/TTS providers live in
//! their own modules; the scene image and TTS pipelines are not text
//! concerns.

use std::env;

use plotforge_schema::{
    AgentOutputEnvelope, AgentOutputProposal, AgentProposalPayload, AgentRole, BeatDraftProposal,
    BeatDraftsProposal, CONTRACT_SCHEMA_VERSION, CONTRACT_VERSION, Character,
    CharacterPortraitRequest, CharacterProposal, Choice, EmotionalArcPoint, NarrativeFunction,
    NarrativeReview, PlotThread, PlotThreadStatus, PlotThreadType, ReproducibilityMetadata,
    ReviewProposal, ScenePlanProposal, StoryCraftPlanProposal, UsageInfo, WorldExpansionProposal,
    contains_secret_marker_text, redact_trace_text,
};

use crate::prompts::ChatMessage;
use crate::shared::{
    FAKE_TEXT_MODEL_VERSION, FAKE_TEXT_PROVIDER_CONFIG_HASH, TEXT_PROMPT_VERSION,
    choice_input_terms, stable_sha256_hash,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextModelRequest {
    pub call_id: String,
    pub agent: AgentRole,
    pub scene_key: String,
    pub run_seed: u64,
    pub prompt_version: String,
    pub model_version: String,
    pub provider_config_hash: String,
    pub prompt: String,
    /// Optional structured chat-message list assembled by the prompt template
    /// system (`crate::prompts::PromptAssembler`). `None` (the default for all
    /// legacy constructors) means the caller is using the single `prompt`
    /// string and the request behaves exactly as before this field was added.
    /// `Some(messages)` carries the assembled system + user messages so a
    /// future provider adapter can send a chat-completions body instead of a
    /// single-prompt completion. This field is strictly additive: the
    /// `TextModelClient` trait and the three HTTP `complete()` signatures are
    /// NOT modified, and `None` preserves the current behaviour.
    pub messages: Option<Vec<ChatMessage>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextModelResponse {
    pub raw_json: String,
    /// Redaction-safe provider token counts for this completion. Legacy,
    /// local-fake, and JSON-only response paths leave this as `None`.
    pub usage: Option<UsageInfo>,
}

impl TextModelResponse {
    pub fn json(raw_json: impl Into<String>) -> Self {
        Self {
            raw_json: raw_json.into(),
            usage: None,
        }
    }

    pub fn json_with_usage(raw_json: impl Into<String>, usage: Option<UsageInfo>) -> Self {
        Self {
            raw_json: raw_json.into(),
            usage,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextModelProviderErrorKind {
    Provider,
    Timeout,
    /// Upstream returned a 429 (Too Many Requests). `retry_after_ms` carries
    /// the server-advised delay parsed from the `Retry-After` header (seconds
    /// or HTTP-date), in milliseconds, when present. It is `None` if the
    /// header was absent or unparseable — the retry loop then falls back to
    /// its own backoff.
    RateLimit {
        retry_after_ms: Option<u64>,
    },
    /// Upstream flagged the response as content-policy-filtered. Carries the
    /// provider-reported `finish_reason` / `stop_reason` string (a short
    /// enumerated token, not user content, so it is trace-safe).
    /// Non-retryable: retrying with the same prompt reproduces the filter.
    ContentFiltered {
        finish_reason: String,
    },
    /// Upstream truncated the output at the model's max-token limit. Carries
    /// the provider-reported output token count when available. Non-retryable:
    /// retrying with the same prompt/limit reproduces the truncation.
    OutputTruncated {
        tokens_generated: Option<u64>,
    },
}

/// Whether a provider error kind is worth retrying with the same request.
/// Rate-limit (429), provider 5xx, and timeout are transient; content-filter
/// and truncation are deterministic outcomes of the prompt/limit and must not
/// be retried (retrying would reproduce the same result and burn quota).
impl TextModelProviderErrorKind {
    pub(crate) fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimit { .. } | Self::Provider | Self::Timeout
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextModelProviderError {
    pub kind: TextModelProviderErrorKind,
    pub code: String,
    pub message: String,
}

impl TextModelProviderError {
    pub fn provider(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: TextModelProviderErrorKind::Provider,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            kind: TextModelProviderErrorKind::Timeout,
            code: "text_provider_timeout".into(),
            message: message.into(),
        }
    }

    /// 429 / rate-limit error. `retry_after_ms` is the parsed `Retry-After`
    /// header value (in ms) when the server supplied one. The code is a
    /// distinct, redacted stable identifier so callers and traces can branch
    /// on it.
    pub fn rate_limit(retry_after_ms: Option<u64>, message: impl Into<String>) -> Self {
        Self {
            kind: TextModelProviderErrorKind::RateLimit { retry_after_ms },
            code: "text_provider_rate_limit".into(),
            message: message.into(),
        }
    }

    /// Content-policy-filtered response. Non-retryable. `finish_reason` is the
    /// short provider-reported stop reason (e.g. `"content_filter"`); it is a
    /// fixed enumerated token, not user content, so it is trace-safe.
    pub fn content_filtered(finish_reason: impl Into<String>) -> Self {
        Self {
            kind: TextModelProviderErrorKind::ContentFiltered {
                finish_reason: finish_reason.into(),
            },
            code: "text_provider_content_filtered".into(),
            message: "content policy triggered; modify prompt".into(),
        }
    }

    /// Output truncated at the model's max-token limit. Non-retryable.
    /// `tokens_generated` is the provider-reported output token count, when
    /// available.
    pub fn output_truncated(tokens_generated: Option<u64>) -> Self {
        Self {
            kind: TextModelProviderErrorKind::OutputTruncated { tokens_generated },
            code: "text_provider_output_truncated".into(),
            message: "output truncated at max_tokens; increase max_output_tokens or reduce prompt"
                .into(),
        }
    }
}

impl std::fmt::Display for TextModelProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for TextModelProviderError {}

pub trait TextModelProvider {
    fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
        ReproducibilityMetadata::local_mock(run_seed)
    }

    fn complete(
        &self,
        request: &TextModelRequest,
    ) -> Result<TextModelResponse, TextModelProviderError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextProviderConfig {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub endpoint_url: Option<String>,
    pub credential_env_var: String,
    /// Optional override for the provider's output token cap. `None` lets each
    /// HTTP client use its own default (4096 for OpenAI variants, 8192 for
    /// Anthropic Messages). `Some(n)` is propagated into the request body so
    /// the configured value replaces the hardcoded default. Included in the
    /// provider-config hash so reproducibility distinguishes runs that differ
    /// only by the output token cap.
    pub max_output_tokens: Option<u32>,
    /// When `true`, the HTTP clients use provider-native JSON Schema mode to
    /// constrain output to a valid `AgentOutputEnvelope` shape:
    /// OpenAI-compatible/Responses send `response_format:
    /// {"type":"json_schema",...}`; Anthropic Messages uses a forced
    /// `tool_use` block with the envelope as the tool input schema. When
    /// `false`, OpenAI-compatible keeps the `json_object` response format and
    /// Anthropic sends plain messages with no tool constraint (the pre-T2.4
    /// shapes). Deliberately excluded from `provider_config_hash` — it is a
    /// client-side capability flag, not part of provider identity.
    pub supports_json_schema: bool,
}

impl TextProviderConfig {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            provider: "disabled".into(),
            model: "disabled".into(),
            endpoint_url: None,
            credential_env_var: "PLOTFORGE_TEXT_PROVIDER_TOKEN".into(),
            max_output_tokens: None,
            supports_json_schema: false,
        }
    }

    pub fn openai_compatible(
        provider: impl Into<String>,
        model: impl Into<String>,
        endpoint_url: impl Into<String>,
        credential_env_var: impl Into<String>,
    ) -> Self {
        Self {
            enabled: true,
            provider: provider.into(),
            model: model.into(),
            endpoint_url: Some(endpoint_url.into()),
            credential_env_var: credential_env_var.into(),
            max_output_tokens: None,
            supports_json_schema: false,
        }
    }

    /// Builder-style setter for JSON Schema output constraint support. The
    /// registry calls this after constructing the base config to enable
    /// provider-native structured-output mode.
    pub fn with_json_schema_support(mut self, enabled: bool) -> Self {
        self.supports_json_schema = enabled;
        self
    }

    pub fn validate(&self) -> Result<(), TextProviderConfigError> {
        require_safe_config_field("provider", &self.provider)?;
        require_safe_config_field("model", &self.model)?;
        if let Some(endpoint_url) = self.endpoint_url.as_ref() {
            require_safe_config_field("endpoint_url", endpoint_url)?;
            if !endpoint_url.starts_with("https://") && !endpoint_url.starts_with("http://") {
                return Err(TextProviderConfigError::InvalidField {
                    field: "endpoint_url",
                    reason: "must start with http:// or https://".into(),
                });
            }
            // Reject credentials embedded in the URL query string (e.g.
            // `https://host/v1?key=sk-realkey`). The secret-marker scanner
            // only flags `sk-` as a token-prefix and a few `api_key`/`token=`
            // substrings, so a `?key=...` query string can slip through. A
            // provider endpoint must never carry a credential in the URL.
            if url_has_query_credential(endpoint_url) {
                return Err(TextProviderConfigError::InvalidField {
                    field: "endpoint_url",
                    reason: "must not carry credentials in the query string".into(),
                });
            }
        }
        // `credential_env_var` may be empty for local no-auth endpoints
        // (Ollama, vLLM without a token) — the `OptionalEnvCredentialResolver`
        // path in `build_text_provider` returns an empty credential and the
        // HTTP clients omit the auth header. Auth-required providers use the
        // strict `EnvCredentialResolver`, which surfaces `pi_agent_missing_credential`
        // when the env var is unset/empty. Only the character-set check runs
        // unconditionally so a malformed env-var name cannot reach the resolver.
        if !self.credential_env_var.trim().is_empty()
            && !self
                .credential_env_var
                .chars()
                .all(|value| value.is_ascii_uppercase() || value.is_ascii_digit() || value == '_')
        {
            return Err(TextProviderConfigError::InvalidField {
                field: "credential_env_var",
                reason: "must contain only uppercase ASCII letters, digits, or underscores".into(),
            });
        }

        Ok(())
    }

    pub fn provider_config_hash(&self) -> String {
        let endpoint_url = self.endpoint_url.as_deref().unwrap_or("");
        let max_output_tokens = match self.max_output_tokens {
            Some(value) => value.to_string(),
            None => "None".into(),
        };
        format!(
            "sha256:{}",
            stable_sha256_hash(&format!(
                "enabled={}\nprovider={}\nmodel={}\nendpoint_url={}\ncredential_env_var={}\nmax_output_tokens={}\n",
                self.enabled,
                self.provider,
                self.model,
                endpoint_url,
                self.credential_env_var,
                max_output_tokens
            ))
        )
    }

    pub fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
        ReproducibilityMetadata {
            run_seed,
            prompt_version: TEXT_PROMPT_VERSION.into(),
            model_version: self.model.clone(),
            provider_config_hash: self.provider_config_hash(),
            mcp_tool_call_hash: None,
            trace_id: None,
            snapshot_id: None,
        }
    }
}

fn require_safe_config_field(
    field: &'static str,
    value: &str,
) -> Result<(), TextProviderConfigError> {
    if value.trim().is_empty() {
        return Err(TextProviderConfigError::InvalidField {
            field,
            reason: "must not be empty".into(),
        });
    }
    if contains_secret_marker_text(value) {
        return Err(TextProviderConfigError::InvalidField {
            field,
            reason: "must not contain secret markers".into(),
        });
    }

    Ok(())
}

/// Returns true when `url` carries a query string with a credential-looking
/// key. Used by `TextProviderConfig::validate` to reject endpoint URLs like
/// `https://host/v1?key=sk-realkey` before they are persisted to the registry
/// or used as the request URL. The credential must always come through the
/// `credential_env_var` name + env resolver, never embedded in the URL.
pub(crate) fn url_has_query_credential(url: &str) -> bool {
    let Some(query_start) = url.find('?') else {
        return false;
    };
    for pair in url[query_start + 1..].split('&') {
        let key = pair.split('=').next().unwrap_or("").to_lowercase();
        if matches!(
            key.as_str(),
            "key" | "api_key" | "apikey" | "access_token" | "secret" | "token" | "auth"
        ) {
            return true;
        }
    }
    false
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextProviderConfigError {
    InvalidField { field: &'static str, reason: String },
}

impl std::fmt::Display for TextProviderConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidField { field, reason } => {
                write!(
                    formatter,
                    "invalid text provider config field {field}: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for TextProviderConfigError {}

fn redact_text_provider_error(error: TextModelProviderError) -> TextModelProviderError {
    TextModelProviderError {
        kind: error.kind,
        code: redact_trace_text(&error.code),
        message: redact_trace_text(&error.message),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderCredentialError {
    Missing { env_var: String },
    Empty { env_var: String },
}

impl std::fmt::Display for ProviderCredentialError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { env_var } => {
                write!(
                    formatter,
                    "missing provider credential in env var `{env_var}`"
                )
            }
            Self::Empty { env_var } => {
                write!(
                    formatter,
                    "provider credential env var `{env_var}` is empty"
                )
            }
        }
    }
}

impl std::error::Error for ProviderCredentialError {}

pub trait ProviderCredentialResolver {
    fn resolve(&self, env_var: &str) -> Result<String, ProviderCredentialError>;
}

#[derive(Clone, Debug, Default)]
pub struct EnvCredentialResolver;

impl ProviderCredentialResolver for EnvCredentialResolver {
    fn resolve(&self, env_var: &str) -> Result<String, ProviderCredentialError> {
        match env::var(env_var) {
            Ok(value) if !value.trim().is_empty() => Ok(value),
            Ok(_) => Err(ProviderCredentialError::Empty {
                env_var: env_var.into(),
            }),
            Err(_) => Err(ProviderCredentialError::Missing {
                env_var: env_var.into(),
            }),
        }
    }
}

pub struct TextModelClientRequest<'a> {
    pub config: &'a TextProviderConfig,
    pub credential: &'a str,
    pub request: &'a TextModelRequest,
}

pub trait TextModelClient {
    fn complete(
        &self,
        request: TextModelClientRequest<'_>,
    ) -> Result<TextModelResponse, TextModelProviderError>;
}

/// Blanket impl so a boxed `dyn TextModelClient` can be plugged into a
/// `ConfiguredTextModelProvider<Box<dyn TextModelClient>, _>`. The box
/// delegates to the inner client. This lets the registry hand back a
/// type-erased client without the caller knowing the concrete HTTP shape.
impl TextModelClient for Box<dyn TextModelClient> {
    fn complete(
        &self,
        request: TextModelClientRequest<'_>,
    ) -> Result<TextModelResponse, TextModelProviderError> {
        (**self).complete(request)
    }
}

#[derive(Clone, Debug)]
pub struct ConfiguredTextModelProvider<C, R> {
    config: TextProviderConfig,
    client: C,
    credential_resolver: R,
}

impl<C, R> ConfiguredTextModelProvider<C, R> {
    pub fn new(config: TextProviderConfig, client: C, credential_resolver: R) -> Self {
        Self {
            config,
            client,
            credential_resolver,
        }
    }

    pub fn config(&self) -> &TextProviderConfig {
        &self.config
    }
}

impl<C, R> TextModelProvider for ConfiguredTextModelProvider<C, R>
where
    C: TextModelClient,
    R: ProviderCredentialResolver,
{
    fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
        self.config.reproducibility_metadata(run_seed)
    }

    fn complete(
        &self,
        request: &TextModelRequest,
    ) -> Result<TextModelResponse, TextModelProviderError> {
        self.config.validate().map_err(|error| {
            TextModelProviderError::provider("text_provider_config", error.to_string())
        })?;
        if !self.config.enabled {
            return Err(TextModelProviderError::provider(
                "text_provider_disabled",
                "text provider is disabled by local configuration",
            ));
        }
        if contains_secret_marker_text(&request.prompt) {
            return Err(TextModelProviderError::provider(
                "text_provider_prompt_secret",
                "text model prompt contained a secret marker",
            ));
        }
        // T2.2: the additive `messages` field carries structured chat-message
        // contents that also leave the process via the provider request body,
        // so each message content is scanned for a secret marker at the same
        // redaction boundary. `None` (the legacy single-prompt path) skips this
        // loop — its content is already covered by the `prompt` scan above.
        if let Some(messages) = request.messages.as_ref() {
            for message in messages {
                if contains_secret_marker_text(&message.content) {
                    return Err(TextModelProviderError::provider(
                        "text_provider_prompt_secret",
                        "text model prompt message contained a secret marker",
                    ));
                }
            }
        }
        let credential = self
            .credential_resolver
            .resolve(&self.config.credential_env_var)
            .map_err(|error| {
                TextModelProviderError::provider(
                    "text_provider_missing_credential",
                    error.to_string(),
                )
            })?;
        let response = self
            .client
            .complete(TextModelClientRequest {
                config: &self.config,
                credential: &credential,
                request,
            })
            .map_err(redact_text_provider_error)?;

        Ok(response)
    }
}

#[derive(Clone, Debug, Default)]
pub struct FakeTextModelProvider {
    failure: Option<FakeTextModelFailure>,
    reproducibility: FakeTextReproducibility,
}

/// Reproducibility identity used by `FakeTextModelProvider`. The generic mock
/// (`success()`) uses the shared `FAKE_TEXT_*` constants; the pi-Agent local
/// provider (`local_pi()`) overrides them with pi-Agent-specific versions so
/// the two surfaces are distinguishable in traces.
#[derive(Clone, Debug)]
struct FakeTextReproducibility {
    prompt_version: &'static str,
    model_version: &'static str,
    provider_config_hash: &'static str,
}

impl Default for FakeTextReproducibility {
    fn default() -> Self {
        Self {
            prompt_version: TEXT_PROMPT_VERSION,
            model_version: FAKE_TEXT_MODEL_VERSION,
            provider_config_hash: FAKE_TEXT_PROVIDER_CONFIG_HASH,
        }
    }
}

/// pi-Agent local provider reproducibility identity. Kept redaction-safe: the
/// config hash is a fixed descriptive string, never a credential.
const PI_AGENT_PROMPT_VERSION: &str = "plotforge-pi-agent-prompt-v1";
const PI_AGENT_MODEL_VERSION: &str = "plotforge-pi-agent-model-v1";
const PI_AGENT_PROVIDER_CONFIG_HASH: &str = "sha256:plotforge-pi-agent-local-config-v1";

impl FakeTextModelProvider {
    pub fn success() -> Self {
        Self::default()
    }

    /// Construct a deterministic local pi-Agent text provider. Distinct from
    /// the generic mock: it carries pi-Agent-appropriate prompt/model versions
    /// and a pi-Agent-local provider config hash so traces distinguish the
    /// pi-Agent surface from the generic fake. It performs no network calls
    /// and stores no credentials or raw provider responses.
    pub fn local_pi() -> Self {
        Self {
            failure: None,
            reproducibility: FakeTextReproducibility {
                prompt_version: PI_AGENT_PROMPT_VERSION,
                model_version: PI_AGENT_MODEL_VERSION,
                provider_config_hash: PI_AGENT_PROVIDER_CONFIG_HASH,
            },
        }
    }

    pub fn provider_error(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::ProviderError)
    }

    pub fn timeout(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::Timeout)
    }

    pub fn invalid_json(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::InvalidJson)
    }

    pub fn invalid_schema(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::InvalidSchema)
    }

    pub fn wrapped_json(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::WrappedJson)
    }

    pub fn secret_marker(agent: AgentRole) -> Self {
        Self::with_failure(agent, FakeTextModelFailureKind::SecretMarker)
    }

    fn with_failure(agent: AgentRole, kind: FakeTextModelFailureKind) -> Self {
        Self {
            failure: Some(FakeTextModelFailure { agent, kind }),
            reproducibility: FakeTextReproducibility::default(),
        }
    }
}

impl TextModelProvider for FakeTextModelProvider {
    fn reproducibility_metadata(&self, run_seed: u64) -> ReproducibilityMetadata {
        ReproducibilityMetadata {
            run_seed,
            prompt_version: self.reproducibility.prompt_version.into(),
            model_version: self.reproducibility.model_version.into(),
            provider_config_hash: self.reproducibility.provider_config_hash.into(),
            mcp_tool_call_hash: None,
            trace_id: None,
            snapshot_id: None,
        }
    }

    fn complete(
        &self,
        request: &TextModelRequest,
    ) -> Result<TextModelResponse, TextModelProviderError> {
        if let Some(failure) = &self.failure
            && failure.agent == request.agent
        {
            return match failure.kind {
                FakeTextModelFailureKind::ProviderError => Err(TextModelProviderError::provider(
                    "text_provider_error",
                    format!("{:?} fake provider failure", request.agent),
                )),
                FakeTextModelFailureKind::Timeout => Err(TextModelProviderError::timeout(format!(
                    "{:?} fake provider timeout",
                    request.agent
                ))),
                FakeTextModelFailureKind::InvalidJson => Ok(TextModelResponse::json("{")),
                FakeTextModelFailureKind::InvalidSchema => fake_invalid_schema_response(request),
                FakeTextModelFailureKind::WrappedJson => fake_wrapped_json_response(request),
                FakeTextModelFailureKind::SecretMarker => Ok(TextModelResponse::json(
                    "```json\n{\"api_key\":\"sk-test-secret-marker\"}\n```",
                )),
            };
        }

        fake_success_response(request)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FakeTextModelFailure {
    agent: AgentRole,
    kind: FakeTextModelFailureKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FakeTextModelFailureKind {
    ProviderError,
    Timeout,
    InvalidJson,
    InvalidSchema,
    WrappedJson,
    SecretMarker,
}

pub(crate) fn fake_success_response(
    request: &TextModelRequest,
) -> Result<TextModelResponse, TextModelProviderError> {
    let proposal = match request.agent {
        AgentRole::StoryArchitect => AgentOutputProposal {
            id: format!("{}-world-expansion", request.call_id),
            agent: AgentRole::StoryArchitect,
            output: AgentProposalPayload::WorldExpansion(Box::new(WorldExpansionProposal {
                world_bible_markdown: "# World Bible\n\nThe generated world bible turns the initial premise into concrete factions, resources, and visible consequences.\n\n## Pressure Model\n\nEvery player order should change at least one faction expectation or world resource.\n".into(),
                canon_markdown: "# Canon Rules\n\n- Canon changes must preserve visible consequences.\n- The player can revise strategy, but not erase already revealed costs.\n- Generated facts must not contradict forbidden facts.\n".into(),
                forbidden_facts: vec![
                    "Do not reveal a hidden prophecy that solves the crisis.".into(),
                    "Do not make the player immune to resource consequences.".into(),
                ],
            })),
        },
        AgentRole::StoryCraftPlanner => AgentOutputProposal {
            id: format!("{}-story-craft", request.call_id),
            agent: AgentRole::StoryCraftPlanner,
            output: AgentProposalPayload::StoryCraftPlan(Box::new(StoryCraftPlanProposal {
                story_bible_markdown:
                    "# Story Bible\n\nThe opening arc converts a broad premise into three concrete promises: resource pressure, faction cost, and player reputation.\n"
                        .into(),
                style_guide_markdown:
                    "# Style Guide\n\nUse specific pressure, short sentences, and consequence-first choices. Avoid vague grandeur and convenient prophecy.\n"
                        .into(),
                story_craft: generated_story_craft_state(),
            })),
        },
        AgentRole::CharacterDesigner => AgentOutputProposal {
            id: format!("{}-character", request.call_id),
            agent: AgentRole::CharacterDesigner,
            output: AgentProposalPayload::CharacterProfile(Box::new(CharacterProposal {
                character: generated_character(&request.prompt, &request.provider_config_hash),
            })),
        },
        AgentRole::ScenePlanner => AgentOutputProposal {
            id: format!("{}-scene-plan", request.call_id),
            agent: AgentRole::ScenePlanner,
            output: AgentProposalPayload::ScenePlan(Box::new(ScenePlanProposal {
                scene_key: request.scene_key.clone(),
                title: "Provider Planned Scene".into(),
                location: "Civic Hall".into(),
                scene_summary: "A fake text model proposes the next civic crisis.".into(),
                dramatic_purpose:
                    "Exercise the provider-backed scene pipeline without external network calls."
                        .into(),
                hook: "The fake provider returns a schema-checked scene plan.".into(),
                emotional_goal: Some("controlled generation".into()),
                cast: vec!["city-treasurer".into(), "guild-liaison".into()],
                entry_beat_id: format!("{}-beat-001", request.scene_key),
                background_asset: Some(format!("assets/generated/{}.png", request.scene_key)),
            })),
        },
        AgentRole::BeatWriter => AgentOutputProposal {
            id: format!("{}-beats", request.call_id),
            agent: AgentRole::BeatWriter,
            output: AgentProposalPayload::BeatDrafts(Box::new(BeatDraftsProposal {
                scene_key: request.scene_key.clone(),
                beats: vec![BeatDraftProposal {
                    id: format!("{}-beat-001", request.scene_key),
                    scene_key: request.scene_key.clone(),
                    text: "The fake provider writes a beat that keeps the player in control."
                        .into(),
                    choices: vec![Choice {
                        id: "continue-council".into(),
                        label: "Continue".into(),
                        action_type: "continue".into(),
                        input_terms: choice_input_terms("continue"),
                        dramatic_purpose: "Let the engine continue from provider output.".into(),
                        change_scene: false,
                    }],
                    narrative_function: NarrativeFunction::Hook,
                }],
            })),
        },
        AgentRole::PlotDoctor => AgentOutputProposal {
            id: format!("{}-review", request.call_id),
            agent: AgentRole::PlotDoctor,
            output: AgentProposalPayload::Review(Box::new(ReviewProposal {
                scene_key: request.scene_key.clone(),
                review: NarrativeReview {
                    scene_key: request.scene_key.clone(),
                    score: 96,
                    hook_score: 95,
                    pacing_score: 96,
                    character_consistency_score: 100,
                    payoff_score: 94,
                    choice_meaningfulness_score: 95,
                    ai_slop_risk: 5,
                    issues: Vec::new(),
                },
                notes: Vec::new(),
            })),
        },
        _ => {
            return Err(TextModelProviderError::provider(
                "fake_provider_unsupported_agent",
                format!(
                    "fake provider has no output contract for {:?}",
                    request.agent
                ),
            ));
        }
    };

    encode_fake_response(&proposal, request)
}

fn fake_invalid_schema_response(
    request: &TextModelRequest,
) -> Result<TextModelResponse, TextModelProviderError> {
    let proposal = match request.agent {
        AgentRole::StoryArchitect => AgentOutputProposal {
            id: format!("{}-invalid-world-expansion", request.call_id),
            agent: AgentRole::StoryArchitect,
            output: AgentProposalPayload::WorldExpansion(Box::new(WorldExpansionProposal {
                world_bible_markdown: String::new(),
                canon_markdown: "Invalid canon".into(),
                forbidden_facts: Vec::new(),
            })),
        },
        AgentRole::StoryCraftPlanner => AgentOutputProposal {
            id: format!("{}-invalid-story-craft", request.call_id),
            agent: AgentRole::StoryCraftPlanner,
            output: AgentProposalPayload::StoryCraftPlan(Box::new(StoryCraftPlanProposal {
                story_bible_markdown: String::new(),
                style_guide_markdown: "Invalid style".into(),
                story_craft: generated_story_craft_state(),
            })),
        },
        AgentRole::CharacterDesigner => AgentOutputProposal {
            id: format!("{}-invalid-character", request.call_id),
            agent: AgentRole::CharacterDesigner,
            output: AgentProposalPayload::CharacterProfile(Box::new(CharacterProposal {
                character: Character {
                    id: String::new(),
                    name: "Invalid Character".into(),
                    role: "Invalid".into(),
                    traits: Vec::new(),
                    visual_card: "Invalid visual".into(),
                    voice_card: "Invalid voice".into(),
                    portrait_request: None,
                },
            })),
        },
        AgentRole::ScenePlanner => AgentOutputProposal {
            id: format!("{}-invalid-scene-plan", request.call_id),
            agent: AgentRole::ScenePlanner,
            output: AgentProposalPayload::ScenePlan(Box::new(ScenePlanProposal {
                scene_key: String::new(),
                title: "Invalid Scene Plan".into(),
                location: "Civic Hall".into(),
                scene_summary: "This payload is structurally valid JSON but fails validation."
                    .into(),
                dramatic_purpose: "Test schema validation failure.".into(),
                hook: "A missing scene key should fail.".into(),
                emotional_goal: None,
                cast: Vec::new(),
                entry_beat_id: "missing-entry".into(),
                background_asset: None,
            })),
        },
        AgentRole::BeatWriter => AgentOutputProposal {
            id: format!("{}-invalid-beats", request.call_id),
            agent: AgentRole::BeatWriter,
            output: AgentProposalPayload::BeatDrafts(Box::new(BeatDraftsProposal {
                scene_key: request.scene_key.clone(),
                beats: Vec::new(),
            })),
        },
        AgentRole::PlotDoctor => AgentOutputProposal {
            id: format!("{}-invalid-review", request.call_id),
            agent: AgentRole::PlotDoctor,
            output: AgentProposalPayload::Review(Box::new(ReviewProposal {
                scene_key: request.scene_key.clone(),
                review: NarrativeReview {
                    scene_key: "other-scene".into(),
                    score: 96,
                    hook_score: 95,
                    pacing_score: 96,
                    character_consistency_score: 100,
                    payoff_score: 94,
                    choice_meaningfulness_score: 95,
                    ai_slop_risk: 5,
                    issues: Vec::new(),
                },
                notes: Vec::new(),
            })),
        },
        _ => {
            return Err(TextModelProviderError::provider(
                "fake_provider_unsupported_agent",
                format!(
                    "fake provider has no invalid schema output for {:?}",
                    request.agent
                ),
            ));
        }
    };

    encode_fake_response(&proposal, request)
}

fn fake_wrapped_json_response(
    request: &TextModelRequest,
) -> Result<TextModelResponse, TextModelProviderError> {
    let response = fake_success_response(request)?;
    Ok(TextModelResponse::json(format!(
        "Provider draft:\n```json\n{}\n```\nEnd of draft.",
        response.raw_json
    )))
}

fn encode_fake_response(
    proposal: &AgentOutputProposal,
    request: &TextModelRequest,
) -> Result<TextModelResponse, TextModelProviderError> {
    let envelope = AgentOutputEnvelope {
        id: format!("{}-envelope", proposal.id),
        contract_version: CONTRACT_VERSION.into(),
        schema_version: CONTRACT_SCHEMA_VERSION,
        agent: proposal.agent.clone(),
        reproducibility: ReproducibilityMetadata {
            run_seed: request.run_seed,
            prompt_version: request.prompt_version.clone(),
            model_version: request.model_version.clone(),
            provider_config_hash: request.provider_config_hash.clone(),
            mcp_tool_call_hash: None,
            trace_id: None,
            snapshot_id: None,
        },
        proposal: proposal.clone(),
    };
    serde_json::to_string(&envelope)
        .map(TextModelResponse::json)
        .map_err(|error| {
            TextModelProviderError::provider("fake_provider_serialization", error.to_string())
        })
}

fn generated_story_craft_state() -> plotforge_schema::StoryCraftState {
    let mut story_craft = plotforge_storycraft::sample_story_craft_state();
    story_craft.bible.genre_promise = "A pressure-driven interactive civic drama.".into();
    story_craft.bible.central_question =
        "Can the player preserve legitimacy while every survival choice has a cost?".into();
    story_craft.emotional_arc = vec![
        EmotionalArcPoint {
            scene_key: "opening-pressure".into(),
            target_emotion: "urgent responsibility".into(),
            intensity: 72,
        },
        EmotionalArcPoint {
            scene_key: "first-reversal".into(),
            target_emotion: "earned consequence".into(),
            intensity: 84,
        },
        EmotionalArcPoint {
            scene_key: "public-reckoning".into(),
            target_emotion: "costly agency".into(),
            intensity: 92,
        },
    ];
    story_craft.plot_threads = vec![
        PlotThread {
            id: "resource-legitimacy".into(),
            title: "Resource legitimacy".into(),
            promise: "Every emergency resource move changes public trust.".into(),
            thread_type: PlotThreadType::Political,
            status: PlotThreadStatus::Open,
            introduced_at: "opening-pressure".into(),
            expected_payoff: Some(
                "A later scene forces a choice between reserves and legitimacy.".into(),
            ),
            related_characters: Vec::new(),
            related_world_flags: vec!["public_trust_tested".into()],
            last_update: "Generated from StoryCraft planner.".into(),
        },
        PlotThread {
            id: "hidden-civic-cost".into(),
            title: "Hidden civic cost".into(),
            promise: "A useful ally demands a future compromise.".into(),
            thread_type: PlotThreadType::Mystery,
            status: PlotThreadStatus::Open,
            introduced_at: "opening-pressure".into(),
            expected_payoff: Some(
                "The ally's price becomes visible after the first success.".into(),
            ),
            related_characters: Vec::new(),
            related_world_flags: vec!["ally_price_unpaid".into()],
            last_update: "Generated from StoryCraft planner.".into(),
        },
        PlotThread {
            id: "player-style-mirror".into(),
            title: "Player style mirror".into(),
            promise: "Repeated choices teach factions what the ruler values.".into(),
            thread_type: PlotThreadType::Relationship,
            status: PlotThreadStatus::Open,
            introduced_at: "opening-pressure".into(),
            expected_payoff: Some("A faction copies or punishes the player's pattern.".into()),
            related_characters: Vec::new(),
            related_world_flags: vec!["ruling_pattern_noticed".into()],
            last_update: "Generated from StoryCraft planner.".into(),
        },
    ];
    story_craft.character_arcs = Vec::new();
    story_craft
}

fn generated_character(prompt: &str, provider_config_hash: &str) -> Character {
    let prompt_hash = format!("sha256:{}", stable_sha256_hash(prompt));
    Character {
        id: "generated-character".into(),
        name: "Generated Envoy".into(),
        role: "Pressure-bearing story catalyst".into(),
        traits: vec!["observant".into(), "cost-aware".into(), "direct".into()],
        visual_card: "Historically grounded portrait, restrained clothing, alert posture.".into(),
        voice_card: "Concise, tactful, and specific about consequences.".into(),
        portrait_request: Some(CharacterPortraitRequest {
            prompt_summary: "Portrait for a pressure-bearing story catalyst.".into(),
            style: "grounded historical character card".into(),
            target_asset_slot: "portrait".into(),
            prompt_hash,
            provider_config_hash: provider_config_hash.into(),
            reference_asset_ids: Vec::new(),
            fallback_allowed: true,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> TextProviderConfig {
        TextProviderConfig::openai_compatible("p", "m", "https://x", "ENV")
    }

    #[test]
    fn text_model_response_usage_defaults_none() {
        let response = TextModelResponse::json("{}");
        assert_eq!(response.usage, None);

        let request = base_request_with_messages(None);
        let fake_response = FakeTextModelProvider::success()
            .complete(&request)
            .expect("fake response");
        assert_eq!(fake_response.usage, None);
    }

    #[test]
    fn text_model_response_json_with_usage_preserves_usage() {
        let usage = UsageInfo {
            input_tokens: Some(10),
            output_tokens: Some(20),
        };
        let response = TextModelResponse::json_with_usage("{}", Some(usage.clone()));

        assert_eq!(response.raw_json, "{}");
        assert_eq!(response.usage, Some(usage));
    }

    #[test]
    fn openai_compatible_constructor_defaults_max_output_tokens_to_none() {
        let config = base_config();
        assert_eq!(config.max_output_tokens, None);
    }

    #[test]
    fn disabled_constructor_defaults_max_output_tokens_to_none() {
        let config = TextProviderConfig::disabled();
        assert_eq!(config.max_output_tokens, None);
    }

    #[test]
    fn provider_config_hash_changes_with_max_output_tokens() {
        // The hash must distinguish runs that differ only by the output token
        // cap so reproducibility metadata keeps them apart.
        let mut a = base_config();
        a.max_output_tokens = None;
        let mut b = base_config();
        b.max_output_tokens = Some(8192);
        let mut c = base_config();
        c.max_output_tokens = Some(4096);
        assert_ne!(a.provider_config_hash(), b.provider_config_hash());
        assert_ne!(b.provider_config_hash(), c.provider_config_hash());
        assert_ne!(a.provider_config_hash(), c.provider_config_hash());
    }

    #[test]
    fn provider_config_hash_stable_for_same_max_output_tokens() {
        let mut a = base_config();
        a.max_output_tokens = Some(2048);
        let mut b = base_config();
        b.max_output_tokens = Some(2048);
        assert_eq!(a.provider_config_hash(), b.provider_config_hash());
    }

    // ---------------------------------------------------------------------
    // T2.2: message-content secret-marker scan in
    // `ConfiguredTextModelProvider::complete`. The additive `messages` field
    // carries structured chat-message contents that leave the process via the
    // provider request body, so each message content is scanned at the same
    // redaction boundary as `prompt`. A marker in any message must surface as
    // `text_provider_prompt_secret` and must never reach the underlying client.
    // ---------------------------------------------------------------------

    /// A `TextModelClient` that records whether `complete()` was called. Used
    /// to assert the adapter's secret-marker scan short-circuits before the
    /// client sees the request.
    #[derive(Default)]
    struct CallCountingClient {
        calls: std::cell::Cell<usize>,
    }

    impl TextModelClient for CallCountingClient {
        fn complete(
            &self,
            _request: TextModelClientRequest<'_>,
        ) -> Result<TextModelResponse, TextModelProviderError> {
            self.calls.set(self.calls.get() + 1);
            Ok(TextModelResponse::json("{}"))
        }
    }

    /// A credential resolver that always returns a fixed token, so the adapter
    /// reaches the message-scan step (it runs after credential resolution).
    #[derive(Clone)]
    struct StaticCredential {
        token: &'static str,
    }

    impl ProviderCredentialResolver for StaticCredential {
        fn resolve(&self, _env_var: &str) -> Result<String, ProviderCredentialError> {
            Ok(self.token.into())
        }
    }

    fn base_request_with_messages(
        messages: Option<Vec<crate::prompts::ChatMessage>>,
    ) -> TextModelRequest {
        TextModelRequest {
            call_id: "call-1".into(),
            agent: plotforge_schema::AgentRole::ScenePlanner,
            scene_key: "scene-1".into(),
            run_seed: 1,
            prompt_version: "v1".into(),
            model_version: "m".into(),
            provider_config_hash: "sha256:x".into(),
            prompt: String::new(),
            messages,
        }
    }

    #[test]
    fn complete_rejects_secret_marker_in_message_before_client_call() {
        use crate::prompts::{ChatMessage, MessageRole};
        let client = CallCountingClient::default();
        let provider = ConfiguredTextModelProvider::new(
            base_config(),
            client,
            StaticCredential { token: "tok" },
        );
        let request = base_request_with_messages(Some(vec![
            ChatMessage::new(MessageRole::System, "you are the scene planner"),
            ChatMessage::new(
                MessageRole::User,
                "Authorization: bearer sk-leak-in-message",
            ),
        ]));
        let error = provider
            .complete(&request)
            .expect_err("secret marker must error");
        assert_eq!(error.code, "text_provider_prompt_secret");
        assert!(error.message.contains("secret marker"));
        assert!(
            !error.message.contains("sk-leak-in-message"),
            "error message must not echo the secret marker value, got: {error}"
        );
        // The adapter must short-circuit at the scan: the client must NOT have
        // been called (no request body with the marker leaves the process).
        assert_eq!(
            provider.client.calls.get(),
            0,
            "client must not be called when a message carries a secret marker"
        );
    }

    #[test]
    fn complete_accepts_messages_without_secret_marker() {
        use crate::prompts::{ChatMessage, MessageRole};
        let client = CallCountingClient::default();
        let provider = ConfiguredTextModelProvider::new(
            base_config(),
            client,
            StaticCredential { token: "tok" },
        );
        let request = base_request_with_messages(Some(vec![
            ChatMessage::new(MessageRole::System, "you are the scene planner"),
            ChatMessage::new(MessageRole::User, "plan the next scene"),
        ]));
        let response = provider.complete(&request).expect("clean messages succeed");
        assert_eq!(response.raw_json, "{}");
        assert_eq!(
            provider.client.calls.get(),
            1,
            "client must be called exactly once for clean messages"
        );
    }
}
