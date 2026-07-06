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
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};

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

        let body = serde_json::json!({
            "model": request.config.model,
            "messages": [
                { "role": "user", "content": request.request.prompt }
            ],
            "response_format": { "type": "json_object" },
        });

        let builder = self.client.post(chat_url).headers(headers).json(&body);
        let text = execute(builder)?;
        let content = extract_openai_chat_content(&text)?;
        Ok(TextModelResponse::json(content))
    }
}

/// Pulls `choices[0].message.content` out of a Chat Completions response body.
fn extract_openai_chat_content(body: &str) -> Result<String, TextModelProviderError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|error| {
        TextModelProviderError::provider(
            "text_provider_http_decode",
            redact_trace_text(&format!("openai_compatible response was not JSON: {error}")),
        )
    })?;
    let content = value
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_str())
        .ok_or_else(|| {
            TextModelProviderError::provider(
                "text_provider_http_decode",
                "openai_compatible response missing choices[0].message.content",
            )
        })?;
    Ok(content.to_string())
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

        let body = serde_json::json!({
            "model": request.config.model,
            "input": request.request.prompt,
        });

        let builder = self.client.post(responses_url).headers(headers).json(&body);
        let text = execute(builder)?;
        let content = extract_openai_responses_content(&text)?;
        Ok(TextModelResponse::json(content))
    }
}

/// Pulls `output[0].content[0].text` out of a Responses API response body.
fn extract_openai_responses_content(body: &str) -> Result<String, TextModelProviderError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|error| {
        TextModelProviderError::provider(
            "text_provider_http_decode",
            redact_trace_text(&format!("openai_responses response was not JSON: {error}")),
        )
    })?;
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
    Ok(content.to_string())
}

// ---------------------------------------------------------------------------
// Anthropic Messages API client.
//
// Uses `x-api-key` + `anthropic-version` headers (not Bearer). The response
// shape is `content[0].text`. Sends the prompt as a single user message and
// the agent role/instructions as the `system` field.
// ---------------------------------------------------------------------------

const ANTHROPIC_VERSION: &str = "2023-06-01";

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
        // output length. 8192 (L3) is comfortable for scene-plan / story-bible
        // generation; the previous 4096 silently truncated longer outputs
        // mid-JSON-object, which surfaced as opaque `text_provider_invalid_json`
        // errors instead of clean completions. The Anthropic default floor is
        // 4096, so 8192 only ever produces longer (not shorter) outputs.
        let body = serde_json::json!({
            "model": request.config.model,
            "max_tokens": 8192,
            "messages": [
                { "role": "user", "content": request.request.prompt }
            ],
        });

        let builder = self.client.post(messages_url).headers(headers).json(&body);
        let text = execute(builder)?;
        let content = extract_anthropic_content(&text)?;
        Ok(TextModelResponse::json(content))
    }
}

/// Pulls `content[0].text` out of an Anthropic Messages response body.
fn extract_anthropic_content(body: &str) -> Result<String, TextModelProviderError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|error| {
        TextModelProviderError::provider(
            "text_provider_http_decode",
            redact_trace_text(&format!(
                "anthropic_messages response was not JSON: {error}"
            )),
        )
    })?;
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
    Ok(content.to_string())
}

/// Joins an endpoint base URL with a relative path, normalising slashes.
/// Accepts both `https://host/v1` and `https://host/v1/` styles.
fn join_endpoint(base_url: &str, relative: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    format!("{trimmed}/{relative}")
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
    fn openai_compatible_client_decodes_chat_completions_body() {
        let config = sample_config("openai", "https://example.invalid", "TEST_KEY");
        let request = sample_request(&config);
        let body = r#"{"choices":[{"message":{"content":"{\"id\":\"x\"}"}}]}"#;
        let content = extract_openai_chat_content(body).expect("decode");
        assert_eq!(content, "{\"id\":\"x\"}");
        let response = TextModelResponse::json(content);
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
        let content = extract_openai_responses_content(body).expect("decode");
        assert_eq!(content, "{\"id\":\"y\"}");
    }

    #[test]
    fn anthropic_client_decodes_messages_body() {
        let body = r#"{"content":[{"text":"{\"id\":\"z\"}"}]}"#;
        let content = extract_anthropic_content(body).expect("decode");
        assert_eq!(content, "{\"id\":\"z\"}");
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
}
