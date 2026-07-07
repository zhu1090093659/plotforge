//! Real HTTP text-provider clients for the three API formats PlotForge
//! supports: OpenAI-compatible Chat Completions, OpenAI Responses, and
//! Anthropic Messages.
//!
//! Each client implements `TextModelClient` and is constructed from a
//! `TextProviderConfig` plus a resolved credential string. Credentials are
//! injected by `ConfiguredTextModelProvider` through a
//! `ProviderCredentialResolver` — they never live as struct fields, never
//! enter traces, never enter provider-config hashes, and never enter the
//! persisted registry. When the resolved credential is empty (e.g. a local
//! Ollama endpoint with no auth), the client omits the auth header rather
//! than failing.
//!
//! The response is always surfaced as `TextModelResponse { raw_json }`,
//! where `raw_json` is the model's text content. The shared
//! `complete_text_agent_output` pipeline then JSON-repairs, validates, and
//! re-derives the `AgentOutputEnvelope` from that text. Network failures map
//! to `TextModelProviderError::provider` (with a redacted code/message) and
//! timeouts map to `TextModelProviderError::timeout`. No fallback is ever
//! silent: a missing credential or non-2xx response is an explicit error.

use std::time::Duration;

use plotforge_schema::redact_trace_text;
use reqwest::StatusCode;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, RETRY_AFTER};

use crate::prompts::{ChatMessage, MessageRole};
use crate::providers_text::{
    TextModelClient, TextModelClientRequest, TextModelProviderError, TextModelResponse,
};

/// The maximum time a single provider HTTP call may take before it is
/// treated as a timeout. Tuned for the slowest supported API (Anthropic
/// Messages) on a long prompt; short enough that a hung endpoint surfaces
/// to the user instead of hanging the rail.
const HTTP_TIMEOUT: Duration = Duration::from_secs(60);
/// A short connect-only timeout so unreachable or misconfigured endpoints
/// fail fast instead of holding a Tauri worker for the full 60s. The overall
/// `HTTP_TIMEOUT` still bounds the whole request (connect + send + body).
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Token-usage breakdown extracted from a provider response, when the API
/// surfaces one. Both fields are optional because not every provider reports
/// both legs of usage on every response (e.g. some OpenAI-compatible gateways
/// report only a total). Values are redaction-safe counts, never prompt or
/// completion text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UsageInfo {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

/// The decoded payload shared by every provider's `extract_*_content` step.
/// `content` is the model's text (the field `complete()` wraps in
/// `TextModelResponse::json`). `finish_reason` and `usage` are surfaced so
/// the pipeline can detect content-filter / truncation *before* attempting
/// JSON repair — otherwise a content-filtered or max-tokens-truncated
/// partial body would masquerade as opaque `text_provider_invalid_json`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExtractedResponse {
    pub content: String,
    pub finish_reason: Option<String>,
    pub usage: Option<UsageInfo>,
}

/// A process-wide shared `reqwest::blocking::Client`. Connection pooling keeps
/// keep-alive and TLS sessions across turns of the same provider, instead of
/// building a fresh client (and forcing a new TLS handshake) per call. Built
/// once on first use; the blocking client is `Send + Sync` so this is safe to
/// share across Tauri worker threads.
static SHARED_HTTP_CLIENT: std::sync::OnceLock<Client> = std::sync::OnceLock::new();

/// Returns the shared blocking `reqwest::Client` with the standard PlotForge
/// timeouts. All three providers share this client; only the headers and body
/// differ per API.
///
/// Redirect policy: `Policy::none()` (H4). reqwest defaults to following up to
/// 10 redirects and only strips its built-in sensitive headers
/// (`authorization`, `cookie`, `www-authenticate`) on cross-host redirects.
/// The Anthropic client sends `x-api-key` and `anthropic-version` — both
/// custom headers that reqwest would replay to a redirect target. A hijacked
/// or misconfigured `endpoint_url` that 302s to an attacker- or
/// internal-controlled host would leak the API key in `x-api-key`. Disabling
/// redirects surfaces a redirect as an explicit provider error instead
/// (AGENTS.md: no silent fallback).
fn shared_http_client() -> Result<&'static Client, TextModelProviderError> {
    if let Some(client) = SHARED_HTTP_CLIENT.get() {
        return Ok(client);
    }
    let client = Client::builder()
        .timeout(HTTP_TIMEOUT)
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| {
            TextModelProviderError::provider(
                "text_provider_http_client",
                redact_trace_text(&error.to_string()),
            )
        })?;
    // `set` succeeds only on the first call; a concurrent caller that also
    // built a client just discards its copy (the shared one wins). Either way
    // `get` now returns the canonical client.
    let _ = SHARED_HTTP_CLIENT.set(client);
    Ok(SHARED_HTTP_CLIENT
        .get()
        .expect("shared client was just set or already present"))
}

/// Returns a clone of the shared blocking `reqwest::Client` with the standard
/// PlotForge timeouts. Exposed so other modules (e.g. registry model
/// discovery) can reuse the same pooled client instead of constructing a
/// duplicate. Errors are redaction-safe `TextModelProviderError`s.
pub fn shared_blocking_client() -> Result<Client, TextModelProviderError> {
    shared_http_client().cloned()
}

/// Adds a bearer auth header to `headers` only when `credential` is
/// non-empty. Empty credentials (local no-auth endpoints) skip the header.
///
/// A credential containing a byte illegal in an HTTP header value (non-ASCII,
/// control char) surfaces as an explicit error rather than being silently
/// dropped (L1) — otherwise the request would go out with no Authorization
/// header and the upstream would return a confusing 401/403 instead of a
/// clear "credential contains illegal characters" error.
fn push_bearer_auth(
    headers: &mut HeaderMap,
    credential: &str,
) -> Result<(), TextModelProviderError> {
    if credential.trim().is_empty() {
        return Ok(());
    }
    let value = HeaderValue::from_str(&format!("Bearer {credential}")).map_err(|_| {
        TextModelProviderError::provider(
            "text_provider_credential_header",
            "provider credential contains bytes illegal in an HTTP header value",
        )
    })?;
    headers.insert("Authorization", value);
    Ok(())
}

/// Adds an `x-api-key` header to `headers` only when `credential` is
/// non-empty. Anthropic uses this header instead of `Authorization`. Same
/// explicit-error-on-illegal-bytes discipline as `push_bearer_auth` (L1).
fn push_x_api_key(headers: &mut HeaderMap, credential: &str) -> Result<(), TextModelProviderError> {
    if credential.trim().is_empty() {
        return Ok(());
    }
    let value = HeaderValue::from_str(credential).map_err(|_| {
        TextModelProviderError::provider(
            "text_provider_credential_header",
            "provider credential contains bytes illegal in an HTTP header value",
        )
    })?;
    headers.insert("x-api-key", value);
    Ok(())
}

/// Executes a `RequestBuilder`, mapping network/decode failures into
/// redaction-safe provider errors. Timeouts and connection failures map to
/// `TextModelProviderError::timeout`; everything else maps to `provider`.
///
/// On a 429 response the `Retry-After` header (seconds or HTTP-date) is parsed
/// and surfaced as the `RateLimit` kind so the retry loop can honour the
/// server-advised backoff. 5xx responses stay on the `Provider` kind, which
/// the retry policy treats as retryable. Other 4xx stay on `Provider` and are
/// non-retryable (a 401/403 will not fix itself on a blind retry).
fn execute(request: RequestBuilder) -> Result<String, TextModelProviderError> {
    let response = request.send().map_err(|error| {
        if error.is_timeout() {
            TextModelProviderError::timeout(redact_trace_text(&error.to_string()))
        } else {
            TextModelProviderError::provider(
                "text_provider_http_send",
                redact_trace_text(&error.to_string()),
            )
        }
    })?;
    let status = response.status();
    if !status.is_success() {
        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after_ms = parse_retry_after(response.headers().get(RETRY_AFTER));
            return Err(TextModelProviderError::rate_limit(
                retry_after_ms,
                format!("provider returned HTTP {status}"),
            ));
        }
        return Err(http_status_error(status));
    }
    let text = response.text().map_err(|error| {
        TextModelProviderError::provider(
            "text_provider_http_body",
            redact_trace_text(&error.to_string()),
        )
    })?;
    Ok(text)
}

fn http_status_error(status: StatusCode) -> TextModelProviderError {
    TextModelProviderError::provider(
        "text_provider_http_status",
        format!("provider returned HTTP {status}"),
    )
}

/// Parses an HTTP `Retry-After` header value into milliseconds. Supports both
/// the delta-seconds form (`"120"`) and the HTTP-date form
/// (`"Wed, 21 Oct 2026 07:28:00 GMT"`). Returns `None` when the header is
/// absent or unparseable — the caller then falls back to its own backoff.
/// The parsed value is redaction-safe (a duration, not user content).
fn parse_retry_after(header: Option<&HeaderValue>) -> Option<u64> {
    let value = header?;
    let value = value.to_str().ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Delta-seconds form: a non-negative integer.
    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Some(seconds.saturating_mul(1000));
    }
    // HTTP-date form. `rfc2822` conversion: trim to a fixed-length timestamp
    // the std parser accepts. We compute the delay relative to now.
    let date = httpdate_to_system_time(trimmed)?;
    let now = std::time::SystemTime::now();
    match date.duration_since(now) {
        Ok(duration) => Some(duration.as_millis().try_into().ok()?),
        // A past date means "retry now"; surface a zero delay so the retry loop
        // honours the header but does not stall.
        Err(_) => Some(0),
    }
}

/// Parses an RFC 7231 HTTP-date into a `SystemTime`. Hand-rolled to avoid a
/// new dependency: the only formats we accept are the three IMF-fixdate /
/// RFC 850 / asctime forms, but in practice providers send IMF-fixdate
/// (`Wed, 21 Oct 2026 07:28:00 GMT`). Falls back to `chrono`-free parsing of
/// that canonical form; anything else returns `None` (caller falls back to
/// its own backoff — no silent fallback, the retry loop still runs).
fn httpdate_to_system_time(value: &str) -> Option<std::time::SystemTime> {
    // IMF-fixdate: "Wed, 21 Oct 2026 07:28:00 GMT"
    // Pull the time fields out positionally; this matches the dominant form.
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() != 6 {
        return None;
    }
    // parts: [weekday, day, month, year, time, "GMT"]
    let day: u32 = parts[1].parse().ok()?;
    let month = month_index(parts[2])?;
    let year: i32 = parts[3].parse().ok()?;
    let time_parts: Vec<&str> = parts[4].split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour: u32 = time_parts[0].parse().ok()?;
    let minute: u32 = time_parts[1].parse().ok()?;
    let second: u32 = time_parts[2].parse().ok()?;
    if parts[5] != "GMT" {
        return None;
    }
    let epoch_seconds = days_from_civil(year, month, day)? as i64 * 86_400
        + (hour as i64 * 3600)
        + (minute as i64 * 60)
        + second as i64;
    let duration = std::time::Duration::from_secs(epoch_seconds.max(0) as u64);
    Some(std::time::SystemTime::UNIX_EPOCH + duration)
}

fn month_index(name: &str) -> Option<u32> {
    match name {
        "Jan" => Some(1),
        "Feb" => Some(2),
        "Mar" => Some(3),
        "Apr" => Some(4),
        "May" => Some(5),
        "Jun" => Some(6),
        "Jul" => Some(7),
        "Aug" => Some(8),
        "Sep" => Some(9),
        "Oct" => Some(10),
        "Nov" => Some(11),
        "Dec" => Some(12),
        _ => None,
    }
}

/// Howard Hinnant's days-from-civil algorithm. Returns `None` for an invalid
/// month (1-12). Produces the count of days since 1970-01-01 for the given
/// (year, month, day), supporting the HTTP-date epoch conversion above.
fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m = month as i32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe as i64 * 365 + yoe as i64 / 4 - yoe as i64 / 100 + doy as i64;
    Some(era as i64 * 146_097 + doe - 719_468)
}

// ---------------------------------------------------------------------------
// Multi-message request body helpers (T2.2).
//
// `TextModelRequest.messages: Option<Vec<ChatMessage>>` is the additive,
// opt-in multi-message surface (T2.1). When `Some`, the HTTP clients below
// send the structured `{role, content}` conversation verbatim per the
// provider's wire format; when `None`, they fall back to wrapping `prompt`
// as a single user message — the pre-T2.2 behaviour. The helpers below are
// extracted out of the `complete()` methods so the body construction is
// unit-testable without a TCP server, and so the three providers share one
// redaction-safe secret-marker scan (`scan_messages_for_secret_markers`).
// ---------------------------------------------------------------------------

/// Builds the OpenAI-compatible Chat Completions `"messages"` array.
///
/// When `request.messages` is `Some`, every message is serialized as
/// `{"role": <role>, "content": <content>}` in order, mapping
/// `MessageRole::System` → `"system"`, `User` → `"user"`,
/// `Assistant` → `"assistant"`. When `None`, the single `prompt` is wrapped
/// as one user message (the pre-T2.2 shape). The returned array is meant to
/// be inserted into the request body as the `"messages"` field.
fn openai_chat_messages(request: &crate::providers_text::TextModelRequest) -> serde_json::Value {
    match request.messages.as_ref() {
        Some(messages) => serde_json::Value::Array(
            messages
                .iter()
                .map(chat_message_to_json)
                .collect(),
        ),
        None => serde_json::json!([
            { "role": "user", "content": request.prompt }
        ]),
    }
}

/// Builds the OpenAI Responses API split: `instructions` (system) and `input`
/// (concatenated user content). The Responses API takes a single `input`
/// string, so multiple user messages are joined with `"\n"`; assistant
/// messages are dropped from `input` (they carry no new instruction for the
/// single-input shape) but a system message is still surfaced as
/// `instructions`. When `request.messages` is `None`, `instructions` is
/// `None` and `input` is the `prompt` (the pre-T2.2 shape). Returns
/// `(instructions, input)`; `instructions` is `None` when no system message
/// is present.
fn openai_responses_instructions_and_input(
    request: &crate::providers_text::TextModelRequest,
) -> (Option<String>, String) {
    match request.messages.as_ref() {
        Some(messages) => {
            let instructions = messages
                .iter()
                .filter(|message| message.role == MessageRole::System)
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let instructions = if instructions.is_empty() {
                None
            } else {
                Some(instructions)
            };
            let user_input = messages
                .iter()
                .filter(|message| message.role == MessageRole::User)
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            (instructions, user_input)
        }
        None => (None, request.prompt.clone()),
    }
}

/// Builds the Anthropic Messages API split: the top-level `system` string
/// (concatenation of all system messages) and the `"messages"` array
/// (user + assistant messages only — Anthropic does not allow `system` inside
/// the `messages` array). When `request.messages` is `None`, `system` is
/// `None` and `messages` is `[{role:"user", content: prompt}]` (the pre-T2.2
/// shape). Returns `(system, messages)`; `system` is `None` when no system
/// message is present.
fn anthropic_system_and_messages(
    request: &crate::providers_text::TextModelRequest,
) -> (Option<String>, serde_json::Value) {
    match request.messages.as_ref() {
        Some(messages) => {
            let system = messages
                .iter()
                .filter(|message| message.role == MessageRole::System)
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let system = if system.is_empty() { None } else { Some(system) };
            let body_messages = serde_json::Value::Array(
                messages
                    .iter()
                    .filter(|message| message.role != MessageRole::System)
                    .map(chat_message_to_json)
                    .collect(),
            );
            (system, body_messages)
        }
        None => (
            None,
            serde_json::json!([
                { "role": "user", "content": request.prompt }
            ]),
        ),
    }
}

/// Serializes a single `ChatMessage` to `{"role": <role>, "content":
/// <content>}` using the lowercase wire label from `MessageRole::as_str`.
/// Shared by the OpenAI-compatible and Anthropic `messages` arrays (Anthropic
/// uses the same role labels inside its `messages` array; its `system` is a
/// separate top-level param, handled by the callers).
fn chat_message_to_json(message: &ChatMessage) -> serde_json::Value {
    serde_json::json!({
        "role": message.role.as_str(),
        "content": message.content,
    })
}

// ---------------------------------------------------------------------------
// OpenAI-compatible Chat Completions client.
//
// Covers OpenAI, DeepSeek, ZAI GLM, local Ollama / vLLM, and any endpoint
// that mirrors the `/chat/completions` shape. Sends `Authorization: Bearer
// <credential>` when a credential is present.
// ---------------------------------------------------------------------------

/// A `TextModelClient` that talks to an OpenAI-compatible `/chat/completions`
/// endpoint.
#[derive(Clone, Debug)]
pub struct OpenAiCompatibleClient {
    client: Client,
}

impl OpenAiCompatibleClient {
    pub fn new() -> Result<Self, TextModelProviderError> {
        Ok(Self {
            client: shared_http_client()?.clone(),
        })
    }
}

impl Default for OpenAiCompatibleClient {
    fn default() -> Self {
        Self::new().expect("default reqwest blocking client builds without external state")
    }
}

impl TextModelClient for OpenAiCompatibleClient {
    fn complete(
        &self,
        request: TextModelClientRequest<'_>,
    ) -> Result<TextModelResponse, TextModelProviderError> {
        let endpoint_url = request.config.endpoint_url.as_ref().ok_or_else(|| {
            TextModelProviderError::provider(
                "text_provider_missing_endpoint",
                "openai_compatible provider has no endpoint_url",
            )
        })?;
        let chat_url = join_endpoint(endpoint_url, "chat/completions");

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        push_bearer_auth(&mut headers, request.credential)?;

        // `max_tokens` is sent only when the user configured an explicit
        // override. When `None`, the OpenAI-compatible default (4096) is left
        // up to the upstream server rather than pinned here, preserving the
        // previous behaviour.
        //
        // `messages` honours the T2.2 additive multi-message surface: when
        // `request.request.messages` is `Some`, the structured conversation is
        // sent verbatim; when `None`, the single `prompt` is wrapped as one
        // user message (the pre-T2.2 shape). The secret-marker scan on
        // message content runs in `ConfiguredTextModelProvider::complete`
        // (the adapter that wraps every client), so it is not repeated here.
        let mut body = serde_json::json!({
            "model": request.config.model,
            "messages": openai_chat_messages(request.request),
            "response_format": { "type": "json_object" },
        });
        if let Some(max_tokens) = request.config.max_output_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }

        let builder = self.client.post(chat_url).headers(headers).json(&body);
        let text = execute(builder)?;
        let extracted = extract_openai_chat_content(&text)?;
        Ok(TextModelResponse::json(extracted.content))
    }
}

/// Pulls `choices[0].message.content` out of a Chat Completions response body.
/// Also inspects `choices[0].finish_reason`: `"content_filter"` surfaces as a
/// `ContentFiltered` error and `"length"` surfaces as `OutputTruncated`, both
/// *before* the caller attempts JSON repair — otherwise a filtered/truncated
/// partial body would masquerade as opaque `text_provider_invalid_json`.
/// Token usage from the `usage` object is forwarded when present.
fn extract_openai_chat_content(body: &str) -> Result<ExtractedResponse, TextModelProviderError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|error| {
        TextModelProviderError::provider(
            "text_provider_http_decode",
            redact_trace_text(&format!("openai_compatible response was not JSON: {error}")),
        )
    })?;
    let choice = value
        .get("choices")
        .and_then(|choices| choices.get(0))
        .ok_or_else(|| {
            TextModelProviderError::provider(
                "text_provider_http_decode",
                "openai_compatible response missing choices[0]",
            )
        })?;
    let finish_reason = choice
        .get("finish_reason")
        .and_then(|reason| reason.as_str())
        .map(str::to_string);
    // Detect content-filter / truncation from finish_reason before extracting
    // content: a filtered or length-capped response may carry an empty or
    // partial `content` that must not reach the JSON-repair stage.
    if let Some(reason) = finish_reason.as_deref() {
        match reason {
            "content_filter" => {
                return Err(TextModelProviderError::content_filtered(reason.to_string()));
            }
            "length" => {
                return Err(TextModelProviderError::output_truncated(
                    output_tokens_from_usage(&value),
                ));
            }
            _ => {}
        }
    }
    let content = choice
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_str())
        .ok_or_else(|| {
            TextModelProviderError::provider(
                "text_provider_http_decode",
                "openai_compatible response missing choices[0].message.content",
            )
        })?;
    Ok(ExtractedResponse {
        content: content.to_string(),
        finish_reason,
        usage: parse_openai_usage(value.get("usage")),
    })
}

// ---------------------------------------------------------------------------
// OpenAI Responses API client.
//
// The newer `/responses` surface. Uses the same Bearer auth. The response
// shape is `output[0].content[0].text`.
// ---------------------------------------------------------------------------

/// A `TextModelClient` that talks to the OpenAI Responses API (`/responses`).
#[derive(Clone, Debug)]
pub struct OpenAiResponsesClient {
    client: Client,
}

impl OpenAiResponsesClient {
    pub fn new() -> Result<Self, TextModelProviderError> {
        Ok(Self {
            client: shared_http_client()?.clone(),
        })
    }
}

impl Default for OpenAiResponsesClient {
    fn default() -> Self {
        Self::new().expect("default reqwest blocking client builds without external state")
    }
}

impl TextModelClient for OpenAiResponsesClient {
    fn complete(
        &self,
        request: TextModelClientRequest<'_>,
    ) -> Result<TextModelResponse, TextModelProviderError> {
        let endpoint_url = request.config.endpoint_url.as_ref().ok_or_else(|| {
            TextModelProviderError::provider(
                "text_provider_missing_endpoint",
                "openai_responses provider has no endpoint_url",
            )
        })?;
        let responses_url = join_endpoint(endpoint_url, "responses");

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        push_bearer_auth(&mut headers, request.credential)?;

        // The Responses API takes a single `input` string and an optional
        // `instructions` string. T2.2 additive multi-message support: when
        // `request.request.messages` is `Some`, system messages become
        // `instructions` (concatenated with `\n` when multiple) and user
        // messages become `input` (concatenated with `\n` when multiple);
        // assistant messages carry no new instruction for the single-input
        // shape and are dropped from `input`. When `None`, `input` is the
        // single `prompt` and `instructions` is omitted (the pre-T2.2 shape).
        let (instructions, input) = openai_responses_instructions_and_input(request.request);
        let mut body = serde_json::json!({
            "model": request.config.model,
            "input": input,
        });
        if let Some(instructions) = instructions {
            body["instructions"] = serde_json::json!(instructions);
        }
        if let Some(max_output_tokens) = request.config.max_output_tokens {
            body["max_output_tokens"] = serde_json::json!(max_output_tokens);
        }

        let builder = self.client.post(responses_url).headers(headers).json(&body);
        let text = execute(builder)?;
        let extracted = extract_openai_responses_content(&text)?;
        Ok(TextModelResponse::json(extracted.content))
    }
}

/// Pulls `output[0].content[0].text` out of a Responses API response body.
/// Detects refusal indicators (a `"refusal"` part or an `incomplete` status
/// with a refusal reason) and surfaces them as `ContentFiltered` before JSON
/// repair. `incomplete` due to `max_output_tokens` surfaces as
/// `OutputTruncated`. Token usage from the top-level `usage` object is
/// forwarded when present.
fn extract_openai_responses_content(body: &str) -> Result<ExtractedResponse, TextModelProviderError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|error| {
        TextModelProviderError::provider(
            "text_provider_http_decode",
            redact_trace_text(&format!("openai_responses response was not JSON: {error}")),
        )
    })?;
    // The Responses API carries a top-level `status` (`completed`,
    // `incomplete`, `failed`) and an `incomplete_details` object whose
    // `reason` is `max_output_tokens` or `content_filter`. Check these before
    // extracting text so a refusal/truncation never reaches JSON repair.
    if let Some(status) = value.get("status").and_then(|status| status.as_str()) {
        match status {
            "incomplete" => {
                let reason = value
                    .get("incomplete_details")
                    .and_then(|details| details.get("reason"))
                    .and_then(|reason| reason.as_str())
                    .unwrap_or("incomplete");
                if reason == "content_filter" {
                    return Err(TextModelProviderError::content_filtered(
                        reason.to_string(),
                    ));
                }
                return Err(TextModelProviderError::output_truncated(
                    output_tokens_from_usage(&value),
                ));
            }
            "failed" => {
                // A failed response is not content-filter or truncation; fall
                // through to the generic decode error if no text is present.
            }
            _ => {}
        }
    }
    // Refusal parts: the Responses API may emit an output item of type
    // `"refusal"` instead of text when content policy triggers.
    let has_refusal = value
        .get("output")
        .and_then(|output| output.as_array())
        .map(|items| {
            items
                .iter()
                .any(|item| item.get("type").and_then(|t| t.as_str()) == Some("refusal"))
        })
        .unwrap_or(false);
    if has_refusal {
        return Err(TextModelProviderError::content_filtered("refusal"));
    }
    let content = value
        .get("output")
        .and_then(|output| output.get(0))
        .and_then(|item| item.get("content"))
        .and_then(|content| content.get(0))
        .and_then(|part| part.get("text"))
        .and_then(|text| text.as_str())
        .ok_or_else(|| {
            TextModelProviderError::provider(
                "text_provider_http_decode",
                "openai_responses response missing output[0].content[0].text",
            )
        })?;
    Ok(ExtractedResponse {
        content: content.to_string(),
        finish_reason: value
            .get("status")
            .and_then(|status| status.as_str())
            .map(str::to_string),
        usage: parse_openai_usage(value.get("usage")),
    })
}

// ---------------------------------------------------------------------------
// Anthropic Messages API client.
//
// Uses `x-api-key` + `anthropic-version` headers (not Bearer). The response
// shape is `content[0].text`. Sends the prompt as a single user message and
// the agent role/instructions as the `system` field.
// ---------------------------------------------------------------------------

pub(crate) const ANTHROPIC_VERSION: &str = "2023-06-01";

/// A `TextModelClient` that talks to the Anthropic Messages API
/// (`/v1/messages`).
#[derive(Clone, Debug)]
pub struct AnthropicMessagesClient {
    client: Client,
}

impl AnthropicMessagesClient {
    pub fn new() -> Result<Self, TextModelProviderError> {
        Ok(Self {
            client: shared_http_client()?.clone(),
        })
    }
}

impl Default for AnthropicMessagesClient {
    fn default() -> Self {
        Self::new().expect("default reqwest blocking client builds without external state")
    }
}

impl TextModelClient for AnthropicMessagesClient {
    fn complete(
        &self,
        request: TextModelClientRequest<'_>,
    ) -> Result<TextModelResponse, TextModelProviderError> {
        let endpoint_url = request.config.endpoint_url.as_ref().ok_or_else(|| {
            TextModelProviderError::provider(
                "text_provider_missing_endpoint",
                "anthropic_messages provider has no endpoint_url",
            )
        })?;
        let messages_url = join_endpoint(endpoint_url, "v1/messages");

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        push_x_api_key(&mut headers, request.credential)?;

        // `max_tokens` is required by the Anthropic Messages API and caps the
        // output length. When the user configured an explicit override it is
        // used; otherwise the default is 8192 (L3), comfortable for scene-plan
        // / story-bible generation. The previous 4096 silently truncated longer
        // outputs mid-JSON-object, which surfaced as opaque
        // `text_provider_invalid_json` errors instead of clean completions. The
        // Anthropic default floor is 4096, so 8192 only ever produces longer
        // (not shorter) outputs.
        //
        // T2.2 additive multi-message support: when
        // `request.request.messages` is `Some`, system messages become the
        // top-level `system` parameter (concatenated with `\n` when multiple)
        // and user/assistant messages become the `messages` array. Anthropic
        // does not allow a `system` role inside `messages`, so system messages
        // are filtered out of the array. When `None`, `system` is omitted and
        // `messages` is `[{role:"user", content: prompt}]` (the pre-T2.2
        // shape).
        let max_tokens = request.config.max_output_tokens.unwrap_or(8192);
        let (system, messages) = anthropic_system_and_messages(request.request);
        let mut body = serde_json::json!({
            "model": request.config.model,
            "max_tokens": max_tokens,
            "messages": messages,
        });
        if let Some(system) = system {
            body["system"] = serde_json::json!(system);
        }

        let builder = self.client.post(messages_url).headers(headers).json(&body);
        let text = execute(builder)?;
        let extracted = extract_anthropic_content(&text)?;
        Ok(TextModelResponse::json(extracted.content))
    }
}

/// Pulls `content[0].text` out of an Anthropic Messages response body.
/// Inspects `stop_reason`: `"max_tokens"` surfaces as `OutputTruncated`, and
/// the content-policy indicators (`"content_filter"`, or a content block with
/// `"type":"redacted_content"`) surface as `ContentFiltered` — both before
/// JSON repair. Token usage from the `usage` object is forwarded when present.
fn extract_anthropic_content(body: &str) -> Result<ExtractedResponse, TextModelProviderError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|error| {
        TextModelProviderError::provider(
            "text_provider_http_decode",
            redact_trace_text(&format!(
                "anthropic_messages response was not JSON: {error}"
            )),
        )
    })?;
    let stop_reason = value
        .get("stop_reason")
        .and_then(|reason| reason.as_str())
        .map(str::to_string);
    // Detect truncation / content-filter from stop_reason before extracting
    // text. A `max_tokens` stop reproduces on retry; a `content_filter` stop
    // likewise reproduces. Both must surface as actionable errors, not opaque
    // invalid-JSON failures on the partial body.
    if let Some(reason) = stop_reason.as_deref() {
        match reason {
            "max_tokens" => {
                return Err(TextModelProviderError::output_truncated(
                    anthropic_output_tokens(&value),
                ));
            }
            "content_filter" => {
                return Err(TextModelProviderError::content_filtered(reason.to_string()));
            }
            _ => {}
        }
    }
    // Anthropic surfaces policy refusals as a content block whose `type` is
    // `redacted_content` (server-side redaction of a policy-triggered block).
    let has_redacted_block = value
        .get("content")
        .and_then(|content| content.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .any(|block| block.get("type").and_then(|t| t.as_str()) == Some("redacted_content"))
        })
        .unwrap_or(false);
    if has_redacted_block {
        return Err(TextModelProviderError::content_filtered("redacted_content"));
    }
    let content = value
        .get("content")
        .and_then(|content| content.get(0))
        .and_then(|part| part.get("text"))
        .and_then(|text| text.as_str())
        .ok_or_else(|| {
            TextModelProviderError::provider(
                "text_provider_http_decode",
                "anthropic_messages response missing content[0].text",
            )
        })?;
    Ok(ExtractedResponse {
        content: content.to_string(),
        finish_reason: stop_reason,
        usage: parse_anthropic_usage(value.get("usage")),
    })
}

/// Joins an endpoint base URL with a relative path, normalising slashes.
/// Accepts both `https://host/v1` and `https://host/v1/` styles.
fn join_endpoint(base_url: &str, relative: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    format!("{trimmed}/{relative}")
}

/// Extracts the OpenAI `usage.output_tokens` / legacy `completion_tokens`
/// count from a Chat Completions or Responses `usage` object. Used for the
/// `OutputTruncated` token count on a `length` / `max_output_tokens` finish.
fn output_tokens_from_usage(value: &serde_json::Value) -> Option<u64> {
    value
        .get("usage")
        .and_then(|usage| {
            usage
                .get("output_tokens")
                .and_then(|tokens| tokens.as_u64())
                .or_else(|| usage.get("completion_tokens").and_then(|tokens| tokens.as_u64()))
        })
}

/// Parses an OpenAI-style `usage` object into a `UsageInfo`. The Chat
/// Completions API uses `prompt_tokens` / `completion_tokens`; the Responses
/// API uses `input_tokens` / `output_tokens`. Both are accepted; missing
/// fields stay `None` rather than defaulting to a misleading zero.
fn parse_openai_usage(usage: Option<&serde_json::Value>) -> Option<UsageInfo> {
    let usage = usage?;
    let input_tokens = usage
        .get("input_tokens")
        .and_then(|tokens| tokens.as_u64())
        .or_else(|| usage.get("prompt_tokens").and_then(|tokens| tokens.as_u64()));
    let output_tokens = usage
        .get("output_tokens")
        .and_then(|tokens| tokens.as_u64())
        .or_else(|| usage.get("completion_tokens").and_then(|tokens| tokens.as_u64()));
    Some(UsageInfo {
        input_tokens,
        output_tokens,
    })
}

/// Parses an Anthropic `usage` object (`input_tokens` / `output_tokens`).
fn parse_anthropic_usage(usage: Option<&serde_json::Value>) -> Option<UsageInfo> {
    let usage = usage?;
    Some(UsageInfo {
        input_tokens: usage.get("input_tokens").and_then(|tokens| tokens.as_u64()),
        output_tokens: usage.get("output_tokens").and_then(|tokens| tokens.as_u64()),
    })
}

/// Extracts the Anthropic `usage.output_tokens` count for the
/// `OutputTruncated` error on a `max_tokens` stop_reason.
fn anthropic_output_tokens(value: &serde_json::Value) -> Option<u64> {
    value
        .get("usage")
        .and_then(|usage| usage.get("output_tokens"))
        .and_then(|tokens| tokens.as_u64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers_text::{TextModelProviderErrorKind, TextModelRequest, TextProviderConfig};
    use plotforge_schema::AgentRole;

    fn sample_config(provider: &str, endpoint: &str, env_var: &str) -> TextProviderConfig {
        TextProviderConfig::openai_compatible(provider, "test-model", endpoint, env_var)
    }

    fn sample_request(config: &TextProviderConfig) -> TextModelRequest {
        TextModelRequest {
            call_id: "call-1".into(),
            agent: AgentRole::ScenePlanner,
            scene_key: "scene-1".into(),
            run_seed: 1,
            prompt_version: "v1".into(),
            model_version: config.model.clone(),
            provider_config_hash: config.provider_config_hash(),
            prompt: "{\"role\":\"scene_planner\"}".into(),
            messages: None,
        }
    }

    /// Builds a `TextModelRequest` whose `messages` is `Some(vec![…])`, the
    /// T2.2 multi-message surface. Carries a system message, a user message,
    /// and an assistant prior turn so the per-provider mapping tests can assert
    /// all three roles are handled. The `prompt` field is set to the user
    /// content (it is unused when `messages` is `Some`, but kept consistent so
    /// a caller that falls back to `None` semantics would still send coherent
    /// text).
    fn sample_request_with_messages(config: &TextProviderConfig) -> TextModelRequest {
        use crate::prompts::{ChatMessage, MessageRole};
        TextModelRequest {
            call_id: "call-1".into(),
            agent: AgentRole::ScenePlanner,
            scene_key: "scene-1".into(),
            run_seed: 1,
            prompt_version: "scene_planner_v1".into(),
            model_version: config.model.clone(),
            provider_config_hash: config.provider_config_hash(),
            prompt: "user-context".into(),
            messages: Some(vec![
                ChatMessage::new(MessageRole::System, "you are the scene planner"),
                ChatMessage::new(MessageRole::User, "user-context"),
                ChatMessage::new(MessageRole::Assistant, "prior assistant turn"),
            ]),
        }
    }

    /// Variant carrying two user messages (no system) so the Responses /
    /// Anthropic concatenation-with-newline behaviour is asserted.
    fn sample_request_with_two_user_messages(config: &TextProviderConfig) -> TextModelRequest {
        use crate::prompts::{ChatMessage, MessageRole};
        TextModelRequest {
            call_id: "call-1".into(),
            agent: AgentRole::ScenePlanner,
            scene_key: "scene-1".into(),
            run_seed: 1,
            prompt_version: "scene_planner_v1".into(),
            model_version: config.model.clone(),
            provider_config_hash: config.provider_config_hash(),
            prompt: String::new(),
            messages: Some(vec![
                ChatMessage::new(MessageRole::User, "first user line"),
                ChatMessage::new(MessageRole::User, "second user line"),
            ]),
        }
    }

    #[test]
    fn join_endpoint_normalises_trailing_slash() {
        assert_eq!(
            join_endpoint("https://host/v1/", "chat/completions"),
            "https://host/v1/chat/completions"
        );
        assert_eq!(
            join_endpoint("https://host/v1", "chat/completions"),
            "https://host/v1/chat/completions"
        );
    }

    #[test]
    fn openai_chat_messages_falls_back_to_prompt_when_messages_none() {
        // When `messages` is `None`, the legacy single-prompt path wraps the
        // `prompt` as one user message — the pre-T2.2 shape. This is the
        // backward-compat contract: existing callers that never set `messages`
        // see no change in the request body.
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request(&config);
        let messages = openai_chat_messages(&request);
        let array = messages.as_array().expect("messages is an array");
        assert_eq!(array.len(), 1, "None path wraps prompt as one message");
        assert_eq!(array[0]["role"], serde_json::json!("user"));
        assert_eq!(array[0]["content"], serde_json::json!(request.prompt));
    }

    #[test]
    fn openai_chat_messages_serializes_structured_messages_in_order() {
        // When `messages` is `Some`, every message is serialized in order with
        // its lowercase wire role label. This is the T2.1/T2.2 additive path:
        // the structured chat conversation is sent verbatim.
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request_with_messages(&config);
        let messages = openai_chat_messages(&request);
        let array = messages.as_array().expect("messages is an array");
        assert_eq!(array.len(), 3, "all three roles are serialized");
        assert_eq!(array[0]["role"], serde_json::json!("system"));
        assert_eq!(
            array[0]["content"],
            serde_json::json!("you are the scene planner")
        );
        assert_eq!(array[1]["role"], serde_json::json!("user"));
        assert_eq!(array[1]["content"], serde_json::json!("user-context"));
        assert_eq!(array[2]["role"], serde_json::json!("assistant"));
        assert_eq!(
            array[2]["content"],
            serde_json::json!("prior assistant turn")
        );
    }

    #[test]
    fn openai_chat_messages_serializes_each_role_with_lowercase_label() {
        // Guard the wire-label contract: MessageRole::as_str must produce the
        // lowercase tokens the provider APIs expect, and they must round-trip
        // through the JSON serializer.
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request_with_messages(&config);
        let messages = openai_chat_messages(&request);
        let array = messages.as_array().expect("messages is an array");
        let roles: Vec<&str> = array
            .iter()
            .map(|message| message["role"].as_str().expect("role is a string"))
            .collect();
        assert_eq!(roles, vec!["system", "user", "assistant"]);
    }

    #[test]
    fn openai_responses_instructions_and_input_splits_system_and_user() {
        // The Responses API takes a single `input` string + optional
        // `instructions`. System messages become `instructions`; user messages
        // become `input`; assistant messages are dropped from `input`.
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request_with_messages(&config);
        let (instructions, input) = openai_responses_instructions_and_input(&request);
        assert_eq!(
            instructions.as_deref(),
            Some("you are the scene planner"),
            "system message becomes instructions"
        );
        assert_eq!(input, "user-context", "user content becomes input");
    }

    #[test]
    fn openai_responses_instructions_and_input_joins_multiple_user_messages() {
        // Two user messages (no system) are joined with "\n" into `input`;
        // `instructions` is `None` when there is no system message.
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request_with_two_user_messages(&config);
        let (instructions, input) = openai_responses_instructions_and_input(&request);
        assert_eq!(instructions, None, "no system message => no instructions");
        assert_eq!(
            input, "first user line\nsecond user line",
            "user messages are joined with newline"
        );
    }

    #[test]
    fn anthropic_system_and_messages_splits_system_out_of_messages_array() {
        // Anthropic's `system` is a top-level param, not inside `messages`.
        // System messages are concatenated into `system`; user+assistant stay
        // in the `messages` array in order.
        let config = sample_config("anthropic", "https://example.invalid", "TEST_KEY");
        let request = sample_request_with_messages(&config);
        let (system, messages) = anthropic_system_and_messages(&request);
        assert_eq!(
            system.as_deref(),
            Some("you are the scene planner"),
            "system message becomes the top-level system param"
        );
        let array = messages.as_array().expect("messages is an array");
        assert_eq!(
            array.len(),
            2,
            "system is removed from the messages array; user+assistant remain"
        );
        assert_eq!(array[0]["role"], serde_json::json!("user"));
        assert_eq!(array[1]["role"], serde_json::json!("assistant"));
    }

    #[test]
    fn anthropic_system_and_messages_falls_back_to_prompt_when_messages_none() {
        // The None path: system is None, messages is a single user turn
        // carrying `prompt` — the pre-T2.2 Anthropic shape.
        let config = sample_config("anthropic", "https://example.invalid", "TEST_KEY");
        let request = sample_request(&config);
        let (system, messages) = anthropic_system_and_messages(&request);
        assert_eq!(system, None, "no system message on the legacy path");
        let array = messages.as_array().expect("messages is an array");
        assert_eq!(array.len(), 1);
        assert_eq!(array[0]["role"], serde_json::json!("user"));
        assert_eq!(array[0]["content"], serde_json::json!(request.prompt));
    }

    #[test]
    fn openai_compatible_client_decodes_chat_completions_body() {
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request(&config);
        let body = r#"{"choices":[{"message":{"content":"{\"id\":\"x\"}"}}]}"#;
        let extracted = extract_openai_chat_content(body).expect("decode");
        assert_eq!(extracted.content, "{\"id\":\"x\"}");
        let response = TextModelResponse::json(extracted.content);
        assert_eq!(response.raw_json, "{\"id\":\"x\"}");
        let _ = request;
    }

    #[test]
    fn openai_compatible_client_rejects_missing_content() {
        let body = r#"{"choices":[{"message":{}}]}"#;
        let error = extract_openai_chat_content(body).expect_err("missing content");
        assert!(error.message.contains("choices[0].message.content"));
    }

    #[test]
    fn openai_responses_client_decodes_responses_body() {
        let body = r#"{"output":[{"content":[{"text":"{\"id\":\"y\"}"}]}]}"#;
        let extracted = extract_openai_responses_content(body).expect("decode");
        assert_eq!(extracted.content, "{\"id\":\"y\"}");
    }

    #[test]
    fn anthropic_client_decodes_messages_body() {
        let body = r#"{"content":[{"text":"{\"id\":\"z\"}"}]}"#;
        let extracted = extract_anthropic_content(body).expect("decode");
        assert_eq!(extracted.content, "{\"id\":\"z\"}");
    }

    #[test]
    fn openai_compatible_client_missing_endpoint_returns_explicit_error() {
        let client = OpenAiCompatibleClient::default();
        let mut config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        config.endpoint_url = None;
        let request = sample_request(&config);
        let error = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect_err("missing endpoint should error");
        assert!(error.code.contains("text_provider_missing_endpoint"));
        // No silent fallback: the error is explicit, not a 200 with empty body.
    }

    #[test]
    fn anthropic_client_missing_endpoint_returns_explicit_error() {
        let client = AnthropicMessagesClient::default();
        let mut config = sample_config("anthropic", "https://example.invalid", "TEST_KEY");
        config.endpoint_url = None;
        let request = sample_request(&config);
        let error = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect_err("missing endpoint should error");
        assert!(error.code.contains("text_provider_missing_endpoint"));
    }

    #[test]
    fn http_status_error_carries_status_code() {
        let error = http_status_error(StatusCode::UNAUTHORIZED);
        assert!(error.message.contains("401"));
    }

    // R2: `execute()` non-2xx path. A tiny in-process TCP server returns a
    // 401 response; the client must surface `text_provider_http_status`
    // (the `http_status_error` mapping) with the status code in the message.
    // No external mock dependency — `std::net::TcpListener` is enough.
    #[test]
    fn execute_surfaces_non_2xx_as_http_status_error() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let server = std::thread::spawn(move || {
            // Accept one connection; reply with a 401 and a tiny body.
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf); // read request line
            let _ = stream.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n");
            let _ = stream.flush();
        });
        let client = OpenAiCompatibleClient::default();
        let config = sample_config("openai", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request(&config);
        let error = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect_err("non-2xx must error");
        assert_eq!(error.kind, TextModelProviderErrorKind::Provider);
        assert!(error.code.contains("text_provider_http_status"));
        assert!(error.message.contains("401"));
        server.join().expect("server thread clean");
    }

    // R2: `execute()` body-read-failure path. A server that accepts the
    // connection, sends a 200 status line, then closes the connection mid
    // body forces a body-read error → `text_provider_http_body`.
    #[test]
    fn execute_surfaces_body_read_failure() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            // Claim a body exists, then close without sending it.
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n");
            let _ = stream.flush();
            // Drop the stream so the client's body read fails.
            drop(stream);
        });
        let client = OpenAiCompatibleClient::default();
        let config = sample_config("openai", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request(&config);
        let result = client.complete(TextModelClientRequest {
            config: &config,
            credential: "token",
            request: &request,
        });
        server.join().expect("server thread clean");
        // The exact error depends on reqwest's transport-layer handling of a
        // truncated body: it may surface as `text_provider_http_body` (body
        // read failure) or `text_provider_http_decode` (the empty/partial
        // body fails JSON decode). Either way it must be an explicit error,
        // never a silent success — assert it did not succeed.
        let error = result.expect_err("truncated body must error");
        assert!(matches!(
            error.kind,
            TextModelProviderErrorKind::Provider | TextModelProviderErrorKind::Timeout
        ));
        assert!(
            error.code.contains("text_provider_http_body")
                || error.code.contains("text_provider_http_decode")
                || error.code.contains("text_provider_http_status"),
            "expected an explicit http error code, got {error}"
        );
    }

    // --- Retry-After header parsing ----------------------------------------

    #[test]
    fn parse_retry_after_seconds_form() {
        let value = HeaderValue::from_str("120").unwrap();
        assert_eq!(parse_retry_after(Some(&value)), Some(120_000));
    }

    #[test]
    fn parse_retry_after_zero_seconds() {
        let value = HeaderValue::from_str("0").unwrap();
        assert_eq!(parse_retry_after(Some(&value)), Some(0));
    }

    #[test]
    fn parse_retry_after_http_date_form_is_positive_duration() {
        // A date a few hours in the future relative to the test run. We cannot
        // assert an exact ms count (it depends on wall-clock skew), only that
        // it parses to a non-negative duration. Use a date far enough out that
        // clock skew cannot make it negative.
        let value = HeaderValue::from_str("Wed, 21 Oct 2099 07:28:00 GMT").unwrap();
        let parsed = parse_retry_after(Some(&value)).expect("http-date must parse");
        // Far-future date clamps to max_delay (8_000ms) only inside the retry
        // policy; here `parse_retry_after` returns the raw delay, so just
        // assert it is a large positive number of ms.
        assert!(parsed > 1_000_000, "expected a large future delay, got {parsed}");
    }

    #[test]
    fn parse_retry_after_absent_or_unparseable_is_none() {
        assert_eq!(parse_retry_after(None), None);
        let bogus = HeaderValue::from_str("not-a-date").unwrap();
        assert_eq!(parse_retry_after(Some(&bogus)), None);
    }

    // --- Content-filter / truncation detection per provider ----------------

    #[test]
    fn openai_chat_content_filter_finish_reason_surfaces_content_filtered() {
        let body = r#"{"choices":[{"message":{"content":""},"finish_reason":"content_filter"}]}"#;
        let error = extract_openai_chat_content(body).expect_err("content_filter must error");
        assert_eq!(error.kind, TextModelProviderErrorKind::ContentFiltered { finish_reason: "content_filter".into() });
        assert_eq!(error.code, "text_provider_content_filtered");
        assert!(error.message.contains("content policy"), "actionable message, got: {error}");
    }

    #[test]
    fn openai_chat_length_finish_reason_surfaces_output_truncated() {
        let body = r#"{"choices":[{"message":{"content":"{\"id\":\"par"},"finish_reason":"length"}],"usage":{"completion_tokens":4096}}"#;
        let error = extract_openai_chat_content(body).expect_err("length must error");
        assert_eq!(error.kind, TextModelProviderErrorKind::OutputTruncated { tokens_generated: Some(4096) });
        assert_eq!(error.code, "text_provider_output_truncated");
        assert!(error.message.contains("max_tokens"), "actionable message, got: {error}");
    }

    #[test]
    fn openai_responses_refusal_surfaces_content_filtered() {
        let body = r#"{"status":"incomplete","incomplete_details":{"reason":"content_filter"},"output":[]}"#;
        let error = extract_openai_responses_content(body).expect_err("refusal must error");
        assert_eq!(error.kind, TextModelProviderErrorKind::ContentFiltered { finish_reason: "content_filter".into() });
        assert_eq!(error.code, "text_provider_content_filtered");
    }

    #[test]
    fn openai_responses_max_output_tokens_surfaces_output_truncated() {
        let body = r#"{"status":"incomplete","incomplete_details":{"reason":"max_output_tokens"},"output":[{"content":[{"text":"partial"}]}],"usage":{"output_tokens":8192}}"#;
        let error = extract_openai_responses_content(body).expect_err("truncation must error");
        assert_eq!(error.kind, TextModelProviderErrorKind::OutputTruncated { tokens_generated: Some(8192) });
        assert_eq!(error.code, "text_provider_output_truncated");
    }

    #[test]
    fn openai_responses_refusal_output_type_surfaces_content_filtered() {
        let body = r#"{"status":"completed","output":[{"type":"refusal","refusal":"policy"}]}"#;
        let error = extract_openai_responses_content(body).expect_err("refusal part must error");
        assert_eq!(error.kind, TextModelProviderErrorKind::ContentFiltered { finish_reason: "refusal".into() });
    }

    #[test]
    fn anthropic_max_tokens_stop_reason_surfaces_output_truncated() {
        let body = r#"{"stop_reason":"max_tokens","content":[{"text":"partial"}],"usage":{"output_tokens":8192}}"#;
        let error = extract_anthropic_content(body).expect_err("max_tokens must error");
        assert_eq!(error.kind, TextModelProviderErrorKind::OutputTruncated { tokens_generated: Some(8192) });
        assert_eq!(error.code, "text_provider_output_truncated");
        assert!(error.message.contains("max_tokens"), "actionable message, got: {error}");
    }

    #[test]
    fn anthropic_content_filter_stop_reason_surfaces_content_filtered() {
        let body = r#"{"stop_reason":"content_filter","content":[{"text":""}]}"#;
        let error = extract_anthropic_content(body).expect_err("content_filter must error");
        assert_eq!(error.kind, TextModelProviderErrorKind::ContentFiltered { finish_reason: "content_filter".into() });
        assert_eq!(error.code, "text_provider_content_filtered");
        assert!(error.message.contains("content policy"), "actionable message, got: {error}");
    }

    #[test]
    fn anthropic_redacted_content_block_surfaces_content_filtered() {
        let body = r#"{"stop_reason":"end_turn","content":[{"type":"redacted_content"}]}"#;
        let error = extract_anthropic_content(body).expect_err("redacted block must error");
        assert_eq!(error.kind, TextModelProviderErrorKind::ContentFiltered { finish_reason: "redacted_content".into() });
    }

    // --- 429 Retry-After end-to-end via execute() --------------------------
    //
    // A tiny in-process TCP server returns a 429 with a `Retry-After: 1`
    // header. `execute()` must surface the `RateLimit` kind with the parsed
    // retry-after (1000 ms), not the generic `Provider` http-status kind.

    #[test]
    fn execute_surfaces_429_as_rate_limit_with_retry_after() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(
                b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\nRetry-After: 1\r\n\r\n",
            );
            let _ = stream.flush();
        });
        let client = OpenAiCompatibleClient::default();
        let config = sample_config("openai", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request(&config);
        let error = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect_err("429 must error");
        assert!(
            matches!(error.kind, TextModelProviderErrorKind::RateLimit { retry_after_ms: Some(1000) }),
            "expected RateLimit{{retry_after_ms:Some(1000)}}, got {error:?}"
        );
        assert_eq!(error.code, "text_provider_rate_limit");
        server.join().expect("server thread clean");
    }

    #[test]
    fn execute_429_without_retry_after_surfaces_rate_limit_none() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\n\r\n");
            let _ = stream.flush();
        });
        let client = OpenAiCompatibleClient::default();
        let config = sample_config("openai", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request(&config);
        let error = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect_err("429 must error");
        assert!(
            matches!(error.kind, TextModelProviderErrorKind::RateLimit { retry_after_ms: None }),
            "expected RateLimit{{None}} when header absent, got {error:?}"
        );
        server.join().expect("server thread clean");
    }

    #[test]
    fn openai_chat_extracts_usage_when_present() {
        let body = r#"{"choices":[{"message":{"content":"{\"id\":\"x\"}"},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":20}}"#;
        let extracted = extract_openai_chat_content(body).expect("decode");
        let usage = extracted.usage.expect("usage present");
        assert_eq!(usage.input_tokens, Some(10));
        assert_eq!(usage.output_tokens, Some(20));
        assert_eq!(extracted.finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn anthropic_extracts_usage_when_present() {
        let body = r#"{"stop_reason":"end_turn","content":[{"text":"{\"id\":\"z\"}"}],"usage":{"input_tokens":5,"output_tokens":7}}"#;
        let extracted = extract_anthropic_content(body).expect("decode");
        let usage = extracted.usage.expect("usage present");
        assert_eq!(usage.input_tokens, Some(5));
        assert_eq!(usage.output_tokens, Some(7));
    }

    // ---------------------------------------------------------------------
    // T1.3: configurable `max_output_tokens` propagation.
    //
    // These tests verify that a configured `max_output_tokens` appears in the
    // outbound request body with the provider-specific field name, and that
    // `None` falls back to the provider default. They capture the raw request
    // body via an in-process TCP server so no real provider is contacted.
    // ---------------------------------------------------------------------

    /// Spawns a one-shot TCP server that captures the full request into the
    /// shared buffer, then replies with `reply_body` (a 200 with the JSON body
    /// the client expects to decode). Returns the bound address + the join
    /// handle + the captured request buffer.
    fn capturing_server(
        reply_body: String,
    ) -> (
        std::net::SocketAddr,
        std::thread::JoinHandle<()>,
        std::sync::Arc<std::sync::Mutex<String>>,
    ) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let captured_clone = captured.clone();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = vec![0u8; 8192];
            let read = stream.read(&mut buf).expect("read request");
            let request = String::from_utf8_lossy(&buf[..read]).to_string();
            *captured_clone.lock().expect("capture lock") = request;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                reply_body.len(),
                reply_body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        (addr, handle, captured)
    }

    #[test]
    fn openai_compatible_sends_max_tokens_when_configured() {
        let reply = r#"{"choices":[{"message":{"content":"{\"id\":\"x\"}"}}]}"#;
        let (addr, handle, captured) = capturing_server(reply.into());
        let mut config = sample_config("openai", &format!("http://{addr}"), "TEST_KEY");
        config.max_output_tokens = Some(8192);
        let request = sample_request(&config);
        let client = OpenAiCompatibleClient::default();
        let _response = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect("complete");
        handle.join().expect("server thread clean");
        let body = captured.lock().expect("capture lock").clone();
        assert!(
            body.contains("\"max_tokens\":8192"),
            "configured max_output_tokens must appear as max_tokens, got: {body}"
        );
    }

    #[test]
    fn openai_compatible_omits_max_tokens_when_none() {
        let reply = r#"{"choices":[{"message":{"content":"{\"id\":\"x\"}"}}]}"#;
        let (addr, handle, captured) = capturing_server(reply.into());
        let config = sample_config("openai", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request(&config);
        let client = OpenAiCompatibleClient::default();
        let _response = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect("complete");
        handle.join().expect("server thread clean");
        let body = captured.lock().expect("capture lock").clone();
        // When None, the field must NOT be sent (default left to the upstream).
        assert!(
            !body.contains("max_tokens"),
            "max_tokens must be omitted when None, got: {body}"
        );
    }

    #[test]
    fn openai_responses_sends_max_output_tokens_when_configured() {
        let reply = r#"{"output":[{"content":[{"text":"{\"id\":\"y\"}"}]}]}"#;
        let (addr, handle, captured) = capturing_server(reply.into());
        let mut config = sample_config("openai", &format!("http://{addr}"), "TEST_KEY");
        config.max_output_tokens = Some(4096);
        let request = sample_request(&config);
        let client = OpenAiResponsesClient::default();
        let _response = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect("complete");
        handle.join().expect("server thread clean");
        let body = captured.lock().expect("capture lock").clone();
        assert!(
            body.contains("\"max_output_tokens\":4096"),
            "configured value must appear as max_output_tokens, got: {body}"
        );
    }

    #[test]
    fn anthropic_sends_configured_max_tokens() {
        let reply = r#"{"content":[{"text":"{\"id\":\"z\"}"}]}"#;
        let (addr, handle, captured) = capturing_server(reply.into());
        let mut config = sample_config("anthropic", &format!("http://{addr}"), "TEST_KEY");
        config.max_output_tokens = Some(2048);
        let request = sample_request(&config);
        let client = AnthropicMessagesClient::default();
        let _response = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect("complete");
        handle.join().expect("server thread clean");
        let body = captured.lock().expect("capture lock").clone();
        assert!(
            body.contains("\"max_tokens\":2048"),
            "configured value must appear as max_tokens, got: {body}"
        );
    }

    #[test]
    fn anthropic_defaults_max_tokens_to_8192_when_none() {
        let reply = r#"{"content":[{"text":"{\"id\":\"z\"}"}]}"#;
        let (addr, handle, captured) = capturing_server(reply.into());
        let config = sample_config("anthropic", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request(&config);
        let client = AnthropicMessagesClient::default();
        let _response = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect("complete");
        handle.join().expect("server thread clean");
        let body = captured.lock().expect("capture lock").clone();
        // Anthropic requires max_tokens; the None default is 8192.
        assert!(
            body.contains("\"max_tokens\":8192"),
            "default max_tokens must be 8192 when None, got: {body}"
        );
    }

    // ---------------------------------------------------------------------
    // T2.2: multi-message request building.
    //
    // The HTTP clients honour `TextModelRequest.messages: Option<Vec<
    // ChatMessage>>` (the additive T2.1 surface). When `Some`, each provider
    // sends the structured conversation verbatim per its wire format; when
    // `None`, the single `prompt` is wrapped as one user message (the
    // pre-T2.2 shape). The body-building logic is extracted into
    // `openai_chat_messages`, `openai_responses_instructions_and_input`, and
    // `anthropic_system_and_messages` so these tests assert the mapping
    // directly (fast, no TCP) AND end-to-end via the `capturing_server`
    // pattern so the real `complete()` body is verified.
    // ---------------------------------------------------------------------

    // --- OpenAI-compatible Chat Completions --------------------------------

    #[test]
    fn openai_chat_messages_none_wraps_prompt_as_single_user() {
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request(&config);
        let messages = openai_chat_messages(&request);
        let array = messages.as_array().expect("messages is an array");
        assert_eq!(array.len(), 1, "None path wraps prompt as one message");
        assert_eq!(
            array[0].get("role").and_then(|r| r.as_str()),
            Some("user")
        );
        assert_eq!(
            array[0].get("content").and_then(|c| c.as_str()),
            Some("{\"role\":\"scene_planner\"}")
        );
    }

    #[test]
    fn openai_chat_messages_some_serializes_all_roles_in_order() {
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request_with_messages(&config);
        let messages = openai_chat_messages(&request);
        let array = messages.as_array().expect("messages is an array");
        assert_eq!(array.len(), 3, "all three messages preserved in order");
        let roles: Vec<&str> = array
            .iter()
            .map(|message| message.get("role").and_then(|r| r.as_str()).unwrap())
            .collect();
        assert_eq!(roles, vec!["system", "user", "assistant"]);
        assert_eq!(
            array[0].get("content").and_then(|c| c.as_str()),
            Some("you are the scene planner")
        );
        assert_eq!(
            array[2].get("content").and_then(|c| c.as_str()),
            Some("prior assistant turn")
        );
    }

    #[test]
    fn openai_compatible_sends_multi_message_body_when_some() {
        let reply = r#"{"choices":[{"message":{"content":"{\"id\":\"x\"}"}}]}"#;
        let (addr, handle, captured) = capturing_server(reply.into());
        let config = sample_config("openai", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request_with_messages(&config);
        let client = OpenAiCompatibleClient::default();
        let _response = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect("complete");
        handle.join().expect("server thread clean");
        let body = captured.lock().expect("capture lock").clone();
        // The structured conversation must be sent verbatim, in order, with
        // the lowercase role labels, and the json_object response_format must
        // still be present.
        assert!(
            body.contains("\"role\":\"system\""),
            "system role must appear, got: {body}"
        );
        assert!(
            body.contains("\"content\":\"you are the scene planner\""),
            "system content must appear, got: {body}"
        );
        assert!(
            body.contains("\"role\":\"user\""),
            "user role must appear, got: {body}"
        );
        assert!(
            body.contains("\"content\":\"user-context\""),
            "user content must appear, got: {body}"
        );
        assert!(
            body.contains("\"role\":\"assistant\""),
            "assistant role must appear, got: {body}"
        );
        assert!(
            body.contains("\"content\":\"prior assistant turn\""),
            "assistant content must appear, got: {body}"
        );
        assert!(
            body.contains("\"response_format\":{\"type\":\"json_object\"}"),
            "json_object response_format must be retained, got: {body}"
        );
    }

    #[test]
    fn openai_compatible_single_prompt_body_unchanged_when_none() {
        // Backward compat: the None path must still produce the single-user
        // message body, byte-equivalent to the pre-T2.2 shape.
        let reply = r#"{"choices":[{"message":{"content":"{\"id\":\"x\"}"}}]}"#;
        let (addr, handle, captured) = capturing_server(reply.into());
        let config = sample_config("openai", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request(&config);
        let client = OpenAiCompatibleClient::default();
        let _response = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect("complete");
        handle.join().expect("server thread clean");
        let body = captured.lock().expect("capture lock").clone();
        // Backward compat: the None path must still produce a single user
        // message wrapping the prompt. Assert role + content independently
        // (serde_json key order is not guaranteed) plus that no system role
        // leaks in.
        let payload = body
            .split("\r\n\r\n")
            .nth(1)
            .unwrap_or(&body)
            .to_string();
        let messages: serde_json::Value =
            serde_json::from_str(&payload).expect("body is JSON");
        let array = messages
            .get("messages")
            .and_then(|m| m.as_array())
            .expect("messages array present");
        assert_eq!(array.len(), 1, "None path wraps prompt as one message");
        assert_eq!(
            array[0].get("role").and_then(|r| r.as_str()),
            Some("user")
        );
        assert_eq!(
            array[0].get("content").and_then(|c| c.as_str()),
            Some("{\"role\":\"scene_planner\"}")
        );
        assert!(
            !body.contains("\"role\":\"system\""),
            "None path must not emit a system role, got: {body}"
        );
    }

    #[test]
    fn openai_responses_none_uses_prompt_as_input() {
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request(&config);
        let (instructions, input) = openai_responses_instructions_and_input(&request);
        assert_eq!(instructions, None, "None path has no instructions");
        assert_eq!(input, "{\"role\":\"scene_planner\"}");
    }

    #[test]
    fn openai_responses_some_splits_system_to_instructions_and_user_to_input() {
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request_with_messages(&config);
        let (instructions, input) = openai_responses_instructions_and_input(&request);
        assert_eq!(
            instructions.as_deref(),
            Some("you are the scene planner"),
            "system message becomes instructions"
        );
        assert_eq!(input, "user-context", "user message becomes input");
    }

    #[test]
    fn openai_responses_some_joins_multiple_user_messages_with_newline() {
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request_with_two_user_messages(&config);
        let (instructions, input) = openai_responses_instructions_and_input(&request);
        assert_eq!(instructions, None, "no system message → no instructions");
        assert_eq!(
            input, "first user line\nsecond user line",
            "multiple user messages are joined with a newline"
        );
    }

    #[test]
    fn openai_responses_sends_instructions_and_input_when_some() {
        let reply = r#"{"output":[{"content":[{"text":"{\"id\":\"y\"}"}]}]}"#;
        let (addr, handle, captured) = capturing_server(reply.into());
        let config = sample_config("openai", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request_with_messages(&config);
        let client = OpenAiResponsesClient::default();
        let _response = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect("complete");
        handle.join().expect("server thread clean");
        let body = captured.lock().expect("capture lock").clone();
        assert!(
            body.contains("\"instructions\":\"you are the scene planner\""),
            "system message must map to instructions, got: {body}"
        );
        assert!(
            body.contains("\"input\":\"user-context\""),
            "user message must map to input, got: {body}"
        );
        // Assistant messages are dropped from the single-input Responses
        // shape — they must not appear as a separate field.
        assert!(
            !body.contains("prior assistant turn"),
            "assistant content must not appear in the Responses body, got: {body}"
        );
    }

    #[test]
    fn openai_responses_single_prompt_body_unchanged_when_none() {
        let reply = r#"{"output":[{"content":[{"text":"{\"id\":\"y\"}"}]}]}"#;
        let (addr, handle, captured) = capturing_server(reply.into());
        let config = sample_config("openai", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request(&config);
        let client = OpenAiResponsesClient::default();
        let _response = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect("complete");
        handle.join().expect("server thread clean");
        let body = captured.lock().expect("capture lock").clone();
        assert!(
            body.contains("\"input\":\"{\\\"role\\\":\\\"scene_planner\\\"}\""),
            "None path must use prompt as input, got: {body}"
        );
        assert!(
            !body.contains("\"instructions\""),
            "None path must omit instructions, got: {body}"
        );
    }

    // --- Anthropic Messages API -------------------------------------------

    #[test]
    fn anthropic_none_wraps_prompt_as_single_user_no_system() {
        let config = sample_config("anthropic", "https://example.invalid", "TEST_KEY");
        let request = sample_request(&config);
        let (system, messages) = anthropic_system_and_messages(&request);
        assert_eq!(system, None, "None path has no system param");
        let array = messages.as_array().expect("messages is an array");
        assert_eq!(array.len(), 1);
        assert_eq!(
            array[0].get("role").and_then(|r| r.as_str()),
            Some("user")
        );
        assert_eq!(
            array[0].get("content").and_then(|c| c.as_str()),
            Some("{\"role\":\"scene_planner\"}")
        );
    }

    #[test]
    fn anthropic_some_lifts_system_to_top_level_and_keeps_user_assistant_in_array() {
        let config = sample_config("anthropic", "https://example.invalid", "TEST_KEY");
        let request = sample_request_with_messages(&config);
        let (system, messages) = anthropic_system_and_messages(&request);
        assert_eq!(
            system.as_deref(),
            Some("you are the scene planner"),
            "system message(s) lift to the top-level system param"
        );
        let array = messages.as_array().expect("messages is an array");
        // System is filtered out of the array; user + assistant remain.
        assert_eq!(array.len(), 2, "system must not appear inside messages");
        let roles: Vec<&str> = array
            .iter()
            .map(|message| message.get("role").and_then(|r| r.as_str()).unwrap())
            .collect();
        assert_eq!(roles, vec!["user", "assistant"]);
        assert_eq!(
            array[1].get("content").and_then(|c| c.as_str()),
            Some("prior assistant turn")
        );
    }

    #[test]
    fn anthropic_some_concatenates_multiple_system_messages_with_newline() {
        use crate::prompts::{ChatMessage, MessageRole};
        let config = sample_config("anthropic", "https://example.invalid", "TEST_KEY");
        let request = TextModelRequest {
            call_id: "call-1".into(),
            agent: AgentRole::ScenePlanner,
            scene_key: "scene-1".into(),
            run_seed: 1,
            prompt_version: "v1".into(),
            model_version: config.model.clone(),
            provider_config_hash: config.provider_config_hash(),
            prompt: String::new(),
            messages: Some(vec![
                ChatMessage::new(MessageRole::System, "rule one"),
                ChatMessage::new(MessageRole::System, "rule two"),
                ChatMessage::new(MessageRole::User, "go"),
            ]),
        };
        let (system, messages) = anthropic_system_and_messages(&request);
        assert_eq!(
            system.as_deref(),
            Some("rule one\nrule two"),
            "multiple system messages concatenate with a newline"
        );
        let array = messages.as_array().expect("messages is an array");
        assert_eq!(array.len(), 1, "only the user message remains in the array");
        assert_eq!(
            array[0].get("role").and_then(|r| r.as_str()),
            Some("user")
        );
    }

    #[test]
    fn anthropic_sends_system_param_and_messages_array_when_some() {
        let reply = r#"{"content":[{"text":"{\"id\":\"z\"}"}]}"#;
        let (addr, handle, captured) = capturing_server(reply.into());
        let config = sample_config("anthropic", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request_with_messages(&config);
        let client = AnthropicMessagesClient::default();
        let _response = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect("complete");
        handle.join().expect("server thread clean");
        let body = captured.lock().expect("capture lock").clone();
        assert!(
            body.contains("\"system\":\"you are the scene planner\""),
            "system message must lift to the top-level system param, got: {body}"
        );
        assert!(
            body.contains("\"role\":\"user\""),
            "user role must appear in the messages array, got: {body}"
        );
        assert!(
            body.contains("\"content\":\"user-context\""),
            "user content must appear, got: {body}"
        );
        assert!(
            body.contains("\"role\":\"assistant\""),
            "assistant role must appear in the messages array, got: {body}"
        );
        // max_tokens must still be present (required by Anthropic).
        assert!(
            body.contains("\"max_tokens\":8192"),
            "max_tokens must still be set, got: {body}"
        );
    }

    #[test]
    fn anthropic_single_prompt_body_unchanged_when_none() {
        let reply = r#"{"content":[{"text":"{\"id\":\"z\"}"}]}"#;
        let (addr, handle, captured) = capturing_server(reply.into());
        let config = sample_config("anthropic", &format!("http://{addr}"), "TEST_KEY");
        let request = sample_request(&config);
        let client = AnthropicMessagesClient::default();
        let _response = client
            .complete(TextModelClientRequest {
                config: &config,
                credential: "token",
                request: &request,
            })
            .expect("complete");
        handle.join().expect("server thread clean");
        let body = captured.lock().expect("capture lock").clone();
        // Backward compat: the None path must still produce a single user
        // message wrapping the prompt, and must omit the top-level system
        // param. Parse the JSON payload (key order is not guaranteed).
        let payload = body
            .split("\r\n\r\n")
            .nth(1)
            .unwrap_or(&body)
            .to_string();
        let messages: serde_json::Value =
            serde_json::from_str(&payload).expect("body is JSON");
        let array = messages
            .get("messages")
            .and_then(|m| m.as_array())
            .expect("messages array present");
        assert_eq!(array.len(), 1, "None path wraps prompt as one message");
        assert_eq!(
            array[0].get("role").and_then(|r| r.as_str()),
            Some("user")
        );
        assert_eq!(
            array[0].get("content").and_then(|c| c.as_str()),
            Some("{\"role\":\"scene_planner\"}")
        );
        assert!(
            messages.get("system").is_none(),
            "None path must omit the top-level system param, got: {body}"
        );
    }

    // --- Shared serializer + role mapping ---------------------------------

    #[test]
    fn chat_message_to_json_uses_lowercase_role_labels() {
        use crate::prompts::{ChatMessage, MessageRole};
        for (role, label) in [
            (MessageRole::System, "system"),
            (MessageRole::User, "user"),
            (MessageRole::Assistant, "assistant"),
        ] {
            let json = chat_message_to_json(&ChatMessage::new(role, "c"));
            assert_eq!(json.get("role").and_then(|r| r.as_str()), Some(label));
            assert_eq!(json.get("content").and_then(|c| c.as_str()), Some("c"));
        }
    }

    #[test]
    fn message_role_as_str_matches_provider_wire_labels() {
        use crate::prompts::MessageRole;
        assert_eq!(MessageRole::System.as_str(), "system");
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Assistant.as_str(), "assistant");
    }
}
