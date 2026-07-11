//! Integration tests for the production provider pipeline.
//!
//! These tests exercise the full provider HTTP pipeline (text, image, TTS)
//! against in-process mock TCP servers, with no real provider contacted. They
//! follow the `capturing_server` pattern from `providers_http::tests`: a
//! one-shot `std::net::TcpListener` bound to `127.0.0.1:0` accepts a single
//! connection, captures the request into a shared buffer, and replies with a
//! canned HTTP response.
//!
//! # Gating
//!
//! Every test here is marked `#[ignore]`. The normal `cargo test` (and CI) run
//! skips them; they only run when invoked explicitly:
//!
//! ```sh
//! cargo test -p plotforge-agent --test provider_integration -- --ignored
//! ```
//!
//! This keeps the unit-test layer (`src/tests.rs`, `src/providers_*.rs`) as the
//! fast, always-on business-logic layer, and reserves these tests for the
//! slower, network-shaped composition concerns: real request body wire shape
//! through the public client API, full response parse, and the error mappings
//! that span transport + decode. They re-use the crate's public types
//! (`ConfiguredTextModelProvider`, `OpenAiCompatibleClient`,
//! `OpenAiImageClient`, `OpenAiTtsClient`) so a regression in the public
//! provider surface surfaces here.
//!
//! # Live tests
//!
//! A `PLOTFORGE_LIVE_TEST=1`-gated test (`live_text_provider_smoke`) is
//! included as a stub: it only runs when the env var is set AND a real
//! `PLOTFORGE_LIVE_TEXT_ENDPOINT` + credential are configured. It is a
//! manual, opt-in smoke against a real provider and is skipped by default
//! (and always skipped in CI).

#![cfg(test)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use plotforge_agent::{
    ConfiguredTextModelProvider, EnvCredentialResolver, ImageGenerationRequest, ImageProvider,
    OpenAiCompatibleClient, OpenAiImageClient, OpenAiTtsClient, OptionalEnvCredentialResolver,
    TextModelClient, TextModelClientRequest, TextModelProvider, TextModelProviderErrorKind,
    TextModelRequest, TextProviderConfig, TtsProvider, TtsRequest, TtsTarget, build_image_provider,
    build_text_provider, build_tts_provider,
};
use plotforge_media::{AssetRecordInput, AssetRegistry};
use plotforge_schema::{
    AgentOutputEnvelope, AgentOutputProposal, AgentProposalPayload, AgentRole, AssetKind,
    AssetProviderMetadata, AssetReference, AssetReferenceKind, AssetSourceKind,
    CONTRACT_SCHEMA_VERSION, CONTRACT_VERSION, Choice, ImageProviderEntry, NarrativeFunction,
    ProviderEntry, ProviderKind, ReproducibilityMetadata, ScenePlanProposal, TtsProviderEntry,
};

// ---------------------------------------------------------------------------
// Mock HTTP server helpers (mirrors the `capturing_server` unit-test pattern).
// ---------------------------------------------------------------------------

/// A captured raw HTTP request plus the canned reply the server was asked to
/// send back. `captured` is filled by the server thread after the request is
/// read; `handle` lets the test join the server thread so it does not leak.
struct CapturingServer {
    addr: std::net::SocketAddr,
    handle: thread::JoinHandle<()>,
    captured: Arc<Mutex<String>>,
}

/// Spawns a one-shot TCP server that:
///   1. accepts a single connection,
///   2. reads the full request into a shared buffer,
///   3. writes `reply` (a complete HTTP response — status line, headers,
///      blank line, body) back to the client,
///   4. closes the connection and exits.
///
/// The buffer is sized to 64 KiB so the T2.4 JSON-Schema `response_format`
/// body (≈13 KiB) is captured in a single read, matching the unit-test
/// helper. Returns the bound address + join handle + captured-request buffer.
fn capturing_server(reply: Vec<u8>) -> CapturingServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().expect("local addr");
    let captured = Arc::new(Mutex::new(String::new()));
    let captured_clone = captured.clone();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept mock connection");
        // Read the full request. The JSON-Schema response_format body is
        // ~13 KiB, so a single 8 KiB read would truncate it and miss the keys
        // the tests assert on. Use a 64 KiB buffer to match the unit-test
        // helper and capture the whole request in one read.
        let mut buf = vec![0u8; 65_536];
        let read = stream.read(&mut buf).expect("read mock request");
        let request = String::from_utf8_lossy(&buf[..read]).to_string();
        *captured_clone.lock().expect("capture lock") = request;
        let _ = stream.write_all(&reply);
        let _ = stream.flush();
        // Drop the stream so the client sees EOF after the body.
        drop(stream);
    });
    CapturingServer {
        addr,
        handle,
        captured,
    }
}

fn capturing_server_replies(replies: Vec<Vec<u8>>) -> CapturingServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().expect("local addr");
    let captured = Arc::new(Mutex::new(String::new()));
    let captured_clone = captured.clone();
    let handle = thread::spawn(move || {
        for reply in replies {
            let (mut stream, _) = listener.accept().expect("accept mock connection");
            let mut buf = vec![0u8; 65_536];
            let read = stream.read(&mut buf).expect("read mock request");
            captured_clone
                .lock()
                .expect("capture lock")
                .push_str(&String::from_utf8_lossy(&buf[..read]));
            let _ = stream.write_all(&reply);
            let _ = stream.flush();
        }
    });
    CapturingServer {
        addr,
        handle,
        captured,
    }
}

/// Builds a complete HTTP/1.1 response: status line + `Content-Type` +
/// `Content-Length` headers + blank line + body. Used by `capturing_server`.
fn http_response(status_line: &str, content_type: &str, body: &str) -> Vec<u8> {
    format!(
        "{status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

/// Builds a complete HTTP/1.1 response that carries extra raw headers (e.g.
/// `Retry-After`) before the blank line. Used for the 429-retry test.
fn http_response_with_headers(status_line: &str, extra_headers: &str, body: &str) -> Vec<u8> {
    format!(
        "{status_line}\r\nContent-Length: {}\r\n{extra_headers}\r\n{body}",
        body.len()
    )
    .into_bytes()
}

/// Builds a 200 OK response with a raw binary body (e.g. PNG bytes for the
/// image test, audio bytes for the TTS test). `Content-Type` is `binary`.
fn http_binary_response(content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

// ---------------------------------------------------------------------------
// Fixture builders (mirror the unit-test helpers in providers_http::tests).
// ---------------------------------------------------------------------------

/// A `TextProviderConfig` for an OpenAI-compatible provider pointing at the
/// mock server. `credential_env_var` is a unique per-test name so concurrent
/// tests do not race on the same env var; the credential value is passed
/// directly to the client via `TextModelClientRequest` (or resolved through a
/// resolver in the full-pipeline tests).
fn text_config(endpoint: &str, credential_env_var: &str) -> TextProviderConfig {
    TextProviderConfig::openai_compatible("openai", "test-model", endpoint, credential_env_var)
}

fn text_entry(endpoint: &str) -> ProviderEntry {
    ProviderEntry {
        id: "openai-test".into(),
        kind: ProviderKind::OpenAiCompatible,
        label: "openai".into(),
        endpoint_url: endpoint.into(),
        model: "test-model".into(),
        credential_env_var: String::new(),
        enabled: true,
        max_output_tokens: None,
        max_concurrency: None,
        requests_per_minute: None,
        daily_token_budget: None,
    }
}

/// A minimal `TextModelRequest` carrying a single `prompt` and `messages:
/// None` (the pre-T2.2 shape). Used for the request-body / response-parse
/// tests that do not exercise multi-message routing.
fn single_prompt_request(config: &TextProviderConfig) -> TextModelRequest {
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

/// A `TextModelRequest` carrying a structured `messages` conversation (a
/// system message, a user message, and an assistant prior turn) so the
/// multi-message body-shape test can assert all three roles are wired.
fn multi_message_request(config: &TextProviderConfig) -> TextModelRequest {
    use plotforge_agent::{ChatMessage, MessageRole};
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

/// Builds a valid `AgentOutputEnvelope` JSON string (a `ScenePlan` proposal)
/// for the success-path mock response. The envelope is the wire shape the
/// `complete_text_agent_output` pipeline expects to JSON-repair + validate;
/// here it is returned verbatim by the mock provider so the parse path sees a
/// structurally valid envelope. The integration test re-parses it as
/// `AgentOutputEnvelope` to confirm the model-emitted body is a valid envelope
/// (the validation pipeline itself is `pub(crate)` and not reachable from an
/// integration test, so we assert the envelope parses and round-trips).
fn valid_envelope_json() -> String {
    let proposal = AgentOutputProposal {
        id: "scene-plan-proposal-001".into(),
        agent: AgentRole::ScenePlanner,
        output: AgentProposalPayload::ScenePlan(Box::new(ScenePlanProposal {
            scene_key: "scene-1".into(),
            title: "Test scene".into(),
            location: "Test location".into(),
            scene_summary: "A scene used by the integration test.".into(),
            dramatic_purpose: "Verify the provider pipeline.".into(),
            hook: "The mock server replies with a valid envelope.".into(),
            emotional_goal: Some("confidence".into()),
            cast: vec!["protagonist".into()],
            entry_beat_id: "scene-1-beat-001".into(),
            background_asset: Some("assets/generated/scene-1.png".into()),
        })),
    };
    let envelope = AgentOutputEnvelope {
        id: "scene-1-envelope".into(),
        contract_version: CONTRACT_VERSION.into(),
        schema_version: CONTRACT_SCHEMA_VERSION,
        agent: proposal.agent.clone(),
        reproducibility: ReproducibilityMetadata {
            run_seed: 1,
            prompt_version: "v1".into(),
            model_version: "test-model".into(),
            provider_config_hash: "sha256:integration".into(),
            mcp_tool_call_hash: None,
            moderation_config_hash: None,
            trace_id: None,
            snapshot_id: None,
        },
        proposal,
    };
    serde_json::to_string(&envelope).expect("serialize envelope")
}

/// The Chat Completions 200-OK body shape the OpenAI-compatible client decodes:
/// `choices[0].message.content` carries the envelope JSON, with a `stop`
/// finish reason and a small token-usage block.
fn openai_chat_body(content: &str) -> String {
    let escaped = serde_json::to_string(content).expect("escape content");
    format!(
        r#"{{"choices":[{{"message":{{"content":{escaped}}},"finish_reason":"stop"}}],"usage":{{"prompt_tokens":10,"completion_tokens":20}}}}"#
    )
}

/// An `ImageProviderEntry` pointing at the mock image server, with a unique
/// `credential_env_var` per test so env-var reads do not race across tests.
fn image_entry(endpoint: &str, credential_env_var: &str) -> ImageProviderEntry {
    ImageProviderEntry {
        id: "openai-image-test".into(),
        endpoint_url: endpoint.into(),
        model: "gpt-image-test".into(),
        credential_env_var: credential_env_var.into(),
        enabled: true,
        default_size: "1024x1024".into(),
        default_quality: "medium".into(),
        max_concurrency: None,
        requests_per_minute: None,
        daily_token_budget: None,
    }
}

/// A `TtsProviderEntry` pointing at the mock TTS server, with a unique
/// `credential_env_var` per test.
fn tts_entry(endpoint: &str, credential_env_var: &str) -> TtsProviderEntry {
    TtsProviderEntry {
        id: "openai-tts-test".into(),
        endpoint_url: endpoint.into(),
        model: "gpt-tts-test".into(),
        credential_env_var: credential_env_var.into(),
        enabled: true,
        voice: "coral".into(),
        format: "mp3".into(),
        max_concurrency: None,
        requests_per_minute: None,
        daily_token_budget: None,
    }
}

/// A minimal 8-byte PNG-ish sentinel so the image test can assert the decoded
/// bytes round-trip without depending on a real PNG encoder. The image client
/// base64-decodes the `b64_json` field verbatim, so any non-empty byte
/// sequence is sufficient to prove "mock image API → bytes returned".
fn image_bytes() -> Vec<u8> {
    // A tiny sentinel that is clearly not a real PNG header, so the test
    // documents "the bytes the mock returned are the bytes recorded".
    vec![
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, b'i', b'n', b't', b'e', b'g',
    ]
}

/// Base64-encodes `bytes` for the OpenAI Images `data[0].b64_json` field.
fn base64_image_field(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Joins the server thread and returns the captured request body (everything
/// after the blank line that ends the HTTP headers).
fn join_and_body(server: CapturingServer) -> String {
    server.handle.join().expect("mock server thread clean");
    let request = server.captured.lock().expect("capture lock").clone();
    request.split("\r\n\r\n").nth(1).unwrap_or("").to_string()
}

/// Like `join_and_body` but returns the full captured HTTP request (headers
/// + body), so tests can assert on request headers (e.g. Authorization).
fn join_and_full_request(server: CapturingServer) -> String {
    server.handle.join().expect("mock server thread clean");
    server.captured.lock().expect("capture lock").clone()
}

// ===========================================================================
// Test scenario 1: successful text generation (full provider pipeline).
//
// Prompt → mock HTTP server → response parse → envelope validate. The mock
// returns a Chat Completions body whose `choices[0].message.content` is a
// valid `AgentOutputEnvelope` JSON. The OpenAI-compatible client parses it
// into `TextModelResponse::raw_json`; the test re-parses `raw_json` as an
// `AgentOutputEnvelope` to confirm the model-emitted body is a structurally
// valid envelope (the redaction/validation pipeline is `pub(crate)`).
// ===========================================================================
#[test]
#[ignore]
fn text_full_pipeline_success_parses_and_validates_envelope() {
    let envelope_json = valid_envelope_json();
    let reply = http_response(
        "HTTP/1.1 200 OK",
        "application/json",
        &openai_chat_body(&envelope_json),
    );
    let server = capturing_server(reply);
    let config = text_config(&format!("http://{}", server.addr), "PFIT_SUCCESS_TOKEN");
    let request = single_prompt_request(&config);
    let client = OpenAiCompatibleClient::default();

    let response = client
        .complete(TextModelClientRequest {
            config: &config,
            credential: "token",
            request: &request,
        })
        .expect("successful completion");

    // The client surfaces the model text verbatim as `raw_json`.
    assert_eq!(response.raw_json, envelope_json, "content round-trips");

    // The model-emitted body must be a structurally valid envelope: it parses
    // back into `AgentOutputEnvelope` with the right agent + payload kind.
    let envelope: AgentOutputEnvelope =
        serde_json::from_str(&response.raw_json).expect("raw_json is valid envelope JSON");
    assert_eq!(envelope.agent, AgentRole::ScenePlanner);
    assert!(
        matches!(envelope.proposal.output, AgentProposalPayload::ScenePlan(_)),
        "envelope carries a ScenePlan payload"
    );
    assert_eq!(envelope.contract_version, CONTRACT_VERSION);
    assert_eq!(envelope.schema_version, CONTRACT_SCHEMA_VERSION);
}

// ===========================================================================
// Test scenario 2: rate limit + retry.
//
// The mock server returns 429 with `Retry-After: 1` on the first call. The
// `execute()` transport layer must surface `RateLimit { retry_after_ms:
// Some(1000) }` (the retry-policy boundary documents this as a retryable,
// transient kind). This test exercises the client directly, so it surfaces
// the rate-limit error on the first attempt; a separate retry-loop test is
// not possible from the integration layer (the retry loop is `pub(crate)`),
// but the rate-limit *kind* is the contract the retry loop branches on.
// ===========================================================================
#[test]
#[ignore]
fn throttle_rate_limit_surfaces_before_a_second_http_request() {
    let body = openai_chat_body(&valid_envelope_json());
    let server = capturing_server(http_response("HTTP/1.1 200 OK", "application/json", &body));
    let endpoint = format!("http://{}", server.addr);
    let mut entry = text_entry(&endpoint);
    entry.requests_per_minute = Some(1);
    let provider = build_text_provider(&entry).expect("build throttled provider");
    let config = text_config(&endpoint, "");
    let request = single_prompt_request(&config);

    provider
        .complete(&request)
        .expect("first request reaches HTTP");
    let error = provider
        .complete(&request)
        .expect_err("second request throttled");

    let TextModelProviderErrorKind::RateLimit {
        retry_after_ms: Some(retry_after_ms),
    } = &error.kind
    else {
        panic!("expected local RateLimit with exact retry delay, got {error:?}");
    };
    assert!((1..=60_000).contains(retry_after_ms));
    server.handle.join().expect("server thread");
    assert_eq!(
        server
            .captured
            .lock()
            .expect("capture lock")
            .matches("POST ")
            .count(),
        1,
        "the throttled call must not reach the HTTP server"
    );
}

#[test]
#[ignore]
fn throttle_concurrency_permit_releases_after_http_error() {
    let success_body = openai_chat_body(&valid_envelope_json());
    let server = capturing_server_replies(vec![
        http_response(
            "HTTP/1.1 500 Internal Server Error",
            "application/json",
            "{}",
        ),
        http_response("HTTP/1.1 200 OK", "application/json", &success_body),
    ]);
    let endpoint = format!("http://{}", server.addr);
    let mut entry = text_entry(&endpoint);
    entry.max_concurrency = Some(1);
    let provider = build_text_provider(&entry).expect("build throttled provider");
    let config = text_config(&endpoint, "");
    let request = single_prompt_request(&config);

    provider
        .complete(&request)
        .expect_err("first HTTP call fails");
    provider
        .complete(&request)
        .expect("permit released for second HTTP call");

    server.handle.join().expect("server thread");
    assert_eq!(
        server
            .captured
            .lock()
            .expect("capture lock")
            .matches("POST ")
            .count(),
        2
    );
}

#[test]
#[ignore]
fn text_rate_limit_surfaces_retryable_kind_with_retry_after() {
    let reply =
        http_response_with_headers("HTTP/1.1 429 Too Many Requests", "Retry-After: 1\r\n", "");
    let server = capturing_server(reply);
    let config = text_config(&format!("http://{}", server.addr), "PFIT_RATELIMIT_TOKEN");
    let request = single_prompt_request(&config);
    let client = OpenAiCompatibleClient::default();

    let error = client
        .complete(TextModelClientRequest {
            config: &config,
            credential: "token",
            request: &request,
        })
        .expect_err("429 must surface an error");

    assert!(
        matches!(
            error.kind,
            TextModelProviderErrorKind::RateLimit {
                retry_after_ms: Some(1000)
            }
        ),
        "expected RateLimit{{retry_after_ms:Some(1000)}}, got {error:?}"
    );
    assert_eq!(error.code, "text_provider_rate_limit");
}

/// A retry-loop-flavoured variant: a two-shot mock server that returns 429 on
/// the first connection and 200 on the second, proving the rate-limit kind is
/// the retryable contract and the loop can recover. Because the retry loop is
/// `pub(crate)`, this test drives the client twice manually to demonstrate the
/// "429 → retry → success" scenario end-to-end through the public client API:
/// the first call surfaces `RateLimit` (retryable), the second call (the
/// retried attempt) succeeds and parses the envelope.
#[test]
#[ignore]
fn text_rate_limit_then_retry_succeeds_on_second_attempt() {
    // Two independent one-shot servers: the first replies 429, the second
    // replies 200 with a valid envelope. A real retry loop would point both
    // attempts at the same endpoint; here we swap the endpoint URL between
    // attempts to simulate "the retry hit a healthy upstream".
    let first = capturing_server(http_response_with_headers(
        "HTTP/1.1 429 Too Many Requests",
        "Retry-After: 0\r\n",
        "",
    ));
    let second_reply = openai_chat_body(&valid_envelope_json());
    let second = capturing_server(http_response(
        "HTTP/1.1 200 OK",
        "application/json",
        &second_reply,
    ));

    let first_config = text_config(&format!("http://{}", first.addr), "PFIT_RETRY_TOKEN");
    let request = single_prompt_request(&first_config);
    let client = OpenAiCompatibleClient::default();

    // First attempt: 429 → retryable RateLimit kind (the retry loop's branch).
    let first_error = client
        .complete(TextModelClientRequest {
            config: &first_config,
            credential: "token",
            request: &request,
        })
        .expect_err("first attempt must surface RateLimit");
    assert!(
        matches!(
            first_error.kind,
            TextModelProviderErrorKind::RateLimit { .. }
        ),
        "first attempt must be a retryable RateLimit, got {first_error:?}"
    );

    // Retried attempt against the healthy upstream: success + valid envelope.
    let second_config = text_config(&format!("http://{}", second.addr), "PFIT_RETRY_TOKEN");
    let response = client
        .complete(TextModelClientRequest {
            config: &second_config,
            credential: "token",
            request: &request,
        })
        .expect("retried attempt must succeed");
    let envelope: AgentOutputEnvelope =
        serde_json::from_str(&response.raw_json).expect("retried body is a valid envelope");
    assert_eq!(envelope.agent, AgentRole::ScenePlanner);
}

// ===========================================================================
// Test scenario 3: content filter.
//
// The mock returns a Chat Completions body with
// `finish_reason: "content_filter"`. The client must surface
// `ContentFiltered` *before* attempting JSON repair on the (possibly empty)
// partial content — otherwise a filtered response would masquerade as opaque
// `text_provider_invalid_json`. Non-retryable: retrying reproduces the filter.
// ===========================================================================
#[test]
#[ignore]
fn text_content_filter_surfaces_explicit_non_retryable_error() {
    let body = r#"{"choices":[{"message":{"content":""},"finish_reason":"content_filter"}]}"#;
    let reply = http_response("HTTP/1.1 200 OK", "application/json", body);
    let server = capturing_server(reply);
    let config = text_config(
        &format!("http://{}", server.addr),
        "PFIT_CONTENTFILTER_TOKEN",
    );
    let request = single_prompt_request(&config);
    let client = OpenAiCompatibleClient::default();

    let error = client
        .complete(TextModelClientRequest {
            config: &config,
            credential: "token",
            request: &request,
        })
        .expect_err("content_filter must error");

    assert!(
        matches!(
            error.kind,
            TextModelProviderErrorKind::ContentFiltered { ref finish_reason }
                if finish_reason == "content_filter"
        ),
        "expected ContentFiltered{{finish_reason:\"content_filter\"}}, got {error:?}"
    );
    assert_eq!(error.code, "text_provider_content_filtered");
    // Non-retryable: the kind must not be in the retryable set.
    assert!(
        !error.kind.is_retryable_public(),
        "content-filter must be non-retryable"
    );
}

// ===========================================================================
// Test scenario 4: truncation.
//
// The mock returns `finish_reason: "length"` with a partial content body and
// a token-usage block. The client must surface `OutputTruncated` (carrying
// the provider-reported output token count) before JSON repair. Non-retryable:
// retrying with the same prompt/limit reproduces the truncation.
// ===========================================================================
#[test]
#[ignore]
fn text_truncation_surfaces_explicit_output_truncated_error() {
    let body = r#"{"choices":[{"message":{"content":"{\"id\":\"par"},"finish_reason":"length"}],"usage":{"completion_tokens":4096}}"#;
    let reply = http_response("HTTP/1.1 200 OK", "application/json", body);
    let server = capturing_server(reply);
    let config = text_config(&format!("http://{}", server.addr), "PFIT_TRUNC_TOKEN");
    let request = single_prompt_request(&config);
    let client = OpenAiCompatibleClient::default();

    let error = client
        .complete(TextModelClientRequest {
            config: &config,
            credential: "token",
            request: &request,
        })
        .expect_err("length finish_reason must error");

    assert!(
        matches!(
            error.kind,
            TextModelProviderErrorKind::OutputTruncated {
                tokens_generated: Some(4096)
            }
        ),
        "expected OutputTruncated{{tokens_generated:Some(4096)}}, got {error:?}"
    );
    assert_eq!(error.code, "text_provider_output_truncated");
    assert!(
        !error.kind.is_retryable_public(),
        "truncation must be non-retryable"
    );
}

// ===========================================================================
// Test scenario 5: multi-message request body shape.
//
// `TextModelRequest.messages: Some(vec![system, user, assistant])` must be
// serialized verbatim into the Chat Completions `messages` array, in order,
// with the lowercase role labels — and the `json_object` response_format
// must still be present. Captured via the mock server so the *real* `complete()`
// body is verified (not just the extracted helper).
// ===========================================================================
#[test]
#[ignore]
fn text_multi_message_body_has_system_user_assistant_in_order() {
    let reply = http_response(
        "HTTP/1.1 200 OK",
        "application/json",
        &openai_chat_body("{\"id\":\"x\"}"),
    );
    let server = capturing_server(reply);
    let config = text_config(&format!("http://{}", server.addr), "PFIT_MULTIMSG_TOKEN");
    let request = multi_message_request(&config);
    let client = OpenAiCompatibleClient::default();

    let _response = client
        .complete(TextModelClientRequest {
            config: &config,
            credential: "token",
            request: &request,
        })
        .expect("complete");

    let body = join_and_body(server);
    let payload: serde_json::Value = serde_json::from_str(&body).expect("captured body is JSON");

    let messages = payload
        .get("messages")
        .and_then(|m| m.as_array())
        .expect("messages array present");
    assert_eq!(messages.len(), 3, "all three roles are serialized in order");

    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m.get("role").and_then(|r| r.as_str()).unwrap())
        .collect();
    assert_eq!(roles, vec!["system", "user", "assistant"]);

    assert_eq!(
        messages[0].get("content").and_then(|c| c.as_str()),
        Some("you are the scene planner"),
        "system content round-trips"
    );
    assert_eq!(
        messages[2].get("content").and_then(|c| c.as_str()),
        Some("prior assistant turn"),
        "assistant content round-trips"
    );

    // The json_object response_format must be retained on the multi-message
    // path (supports_json_schema=false here).
    assert!(
        body.contains("\"response_format\":{\"type\":\"json_object\"}"),
        "json_object response_format must be present, got: {body}"
    );
    // The single-prompt fallback must NOT fire on the Some(messages) path.
    assert!(
        !body.contains("\"role\":\"system\",\"content\":\"{\\\"role\\\":\\\"scene_planner\\\"}\""),
        "prompt must not be re-wrapped as a user message on the Some path"
    );
}

/// A variant of scenario 5 that asserts `supports_json_schema=true` switches
/// the OpenAI-compatible body to the `json_schema` response_format carrying
/// the `AgentOutputEnvelope` schema. This guards the T2.4 wire contract
/// through the public client (the unit tests cover it in isolation; this
/// test confirms it survives the full `complete()` path).
#[test]
#[ignore]
fn text_supports_json_schema_sends_envelope_schema_response_format() {
    let reply = http_response(
        "HTTP/1.1 200 OK",
        "application/json",
        &openai_chat_body("{\"id\":\"x\"}"),
    );
    let server = capturing_server(reply);
    let config = text_config(&format!("http://{}", server.addr), "PFIT_JSONSCHEMA_TOKEN")
        .with_json_schema_support(true);
    let request = single_prompt_request(&config);
    let client = OpenAiCompatibleClient::default();

    let _response = client
        .complete(TextModelClientRequest {
            config: &config,
            credential: "token",
            request: &request,
        })
        .expect("complete");

    let body = join_and_body(server);
    // serde_json key order is not guaranteed, so assert on the keys
    // independently rather than a fixed-order substring.
    assert!(
        body.contains("\"type\":\"json_schema\""),
        "supports_json_schema=true must send a json_schema response_format type, got: {body}"
    );
    assert!(
        body.contains("\"name\":\"agent_output_envelope\""),
        "json_schema must name the envelope schema, got: {body}"
    );
    assert!(
        !body.contains("\"type\":\"json_object\""),
        "json_object must not appear when json_schema is enabled, got: {body}"
    );
}

// ===========================================================================
// Test scenario 5b: Anthropic Messages JSON Schema (tool_use) path.
//
// When `supports_json_schema` is true, the Anthropic client sends a forced
// `emit_envelope` tool with the envelope schema as `input_schema`, and
// `extract_anthropic_content` must decode the `tool_use` content block's
// `input` field as JSON. This test exercises the full Anthropic tool_use
// wire shape + response decode at the integration layer (the unit tests
// cover the extract logic, but the integration test verifies the public
// client end-to-end). No real Anthropic API is contacted.
// ===========================================================================
#[test]
#[ignore]
fn anthropic_tool_use_json_schema_extracts_envelope_from_tool_input() {
    use plotforge_agent::AnthropicMessagesClient;

    // The response carries a tool_use content block whose `input` is a
    // minimal valid envelope-shaped object. The extract path must serialize
    // `input` back to a JSON string.
    let tool_input = serde_json::json!({
        "id": "anthropic-tool-use-test",
        "contract_version": "1",
        "schema_version": 1,
        "agent": "scene_planner"
    });
    let body = format!(
        r#"{{"content":[{{"type":"tool_use","name":"emit_envelope","input":{}}}]}}"#,
        tool_input
    );
    let reply = http_response("HTTP/1.1 200 OK", "application/json", &body);
    let server = capturing_server(reply);

    let mut config = TextProviderConfig::openai_compatible(
        "anthropic-test",
        "claude-sonnet-5",
        format!("http://{}", server.addr),
        "PFIT_ANTHROPIC_TOKEN",
    );
    config.supports_json_schema = true;
    let request = single_prompt_request(&config);
    let client = AnthropicMessagesClient::default();

    let response = client
        .complete(TextModelClientRequest {
            config: &config,
            credential: "anthropic-key",
            request: &request,
        })
        .expect("anthropic tool_use complete");

    // The raw_json must contain the serialized tool_use input.
    assert!(
        response
            .raw_json
            .contains("\"id\":\"anthropic-tool-use-test\""),
        "tool_use input must be serialized to raw_json, got: {}",
        response.raw_json
    );

    let raw_request = join_and_body(server);
    // The request must carry the forced tool definition.
    assert!(
        raw_request.contains("\"emit_envelope\""),
        "Anthropic request must define the emit_envelope tool, got: {raw_request}"
    );
    assert!(
        raw_request.contains("\"tool_choice\""),
        "Anthropic request must force tool_choice, got: {raw_request}"
    );
}

// ===========================================================================
// Test scenario 6: image generation → bytes returned → asset recorded.
//
// The mock image API returns a 200 with `data[0].b64_json` carrying a small
// sentinel. The `OpenAiImageClient` must decode the base64 into bytes; the
// test then records those bytes into an `AssetRegistry` (mirroring the scene
// image pipeline) and asserts the asset is recorded with `Generated` source
// + image kind + provider metadata. No real image API is contacted.
// ===========================================================================
#[test]
#[ignore]
fn image_generation_returns_bytes_and_records_asset() {
    let bytes = image_bytes();
    let b64 = base64_image_field(&bytes);
    let body = format!(r#"{{"data":[{{"b64_json":"{b64}"}}]}}"#);
    let reply = http_response("HTTP/1.1 200 OK", "application/json", &body);
    let server = capturing_server(reply);

    let entry = image_entry(&format!("http://{}", server.addr), "PFIT_IMAGE_TOKEN");
    // OptionalEnvCredentialResolver yields an empty credential (env var unset)
    // → the client omits the Authorization header. A real auth-required
    // provider would 401; the mock accepts the request regardless.
    let client =
        OpenAiImageClient::new(&entry, OptionalEnvCredentialResolver).expect("build image client");

    let request = ImageGenerationRequest {
        scene_key: "scene-1".into(),
        prompt: "a test background image".into(),
        output_path: "assets/generated/scene-1.png".into(),
    };
    let response = client
        .generate(&request)
        .expect("image generation succeeds");

    // The decoded bytes must match the mock's payload exactly.
    assert_eq!(
        response.bytes, bytes,
        "decoded bytes round-trip the mock body"
    );
    assert_eq!(response.provider, "openai-image");
    assert_eq!(response.model.as_deref(), Some("gpt-image-test"));
    assert_eq!(response.spent_cost_units, 1);

    // Record the bytes into an AssetRegistry the way the scene image pipeline
    // does, and assert the asset is recorded (Generated source, Image kind).
    let mut registry = AssetRegistry::new();
    let asset_id = registry
        .insert_bytes(
            AssetRecordInput {
                kind: AssetKind::Image,
                source: AssetSourceKind::Generated,
                project_path: request.output_path.clone(),
                export_path: Some(request.output_path.clone()),
                provider_metadata: Some(AssetProviderMetadata {
                    provider: response.provider.clone(),
                    model: response.model.clone(),
                    request_id: response.request_id.clone(),
                    prompt_hash: Some("sha256:test".into()),
                    fallback_used: false,
                }),
                references: vec![AssetReference {
                    reference_kind: AssetReferenceKind::Scene,
                    reference_id: request.scene_key.clone(),
                    slot: "background_asset".into(),
                }],
            },
            &response.bytes,
        )
        .expect("asset recorded");
    let record = registry.get(&asset_id).expect("asset is retrievable");
    assert_eq!(record.kind, AssetKind::Image);
    assert_eq!(record.source, AssetSourceKind::Generated);
    assert_eq!(record.byte_length, bytes.len() as u64);
    assert_eq!(
        record.provider_metadata.as_ref().unwrap().provider,
        "openai-image"
    );
    assert!(!record.provider_metadata.as_ref().unwrap().fallback_used);

    // Also assert the captured request body has the OpenAI Images shape:
    // model, prompt, size, quality, output_format=png, n=1.
    let body = join_and_body(server);
    let payload: serde_json::Value =
        serde_json::from_str(&body).expect("captured image body is JSON");
    assert_eq!(
        payload.get("model").and_then(|v| v.as_str()),
        Some("gpt-image-test")
    );
    assert_eq!(
        payload.get("prompt").and_then(|v| v.as_str()),
        Some("a test background image")
    );
    assert_eq!(
        payload.get("size").and_then(|v| v.as_str()),
        Some("1024x1024")
    );
    assert_eq!(
        payload.get("quality").and_then(|v| v.as_str()),
        Some("medium")
    );
    assert_eq!(
        payload.get("output_format").and_then(|v| v.as_str()),
        Some("png")
    );
    assert_eq!(payload.get("n").and_then(|v| v.as_u64()), Some(1));
}

#[test]
#[ignore]
fn throttle_image_rate_limit_surfaces_before_a_second_http_request() {
    let bytes = image_bytes();
    let b64 = base64_image_field(&bytes);
    let body = format!(r#"{{"data":[{{"b64_json":"{b64}"}}]}}"#);
    let server = capturing_server(http_response("HTTP/1.1 200 OK", "application/json", &body));
    let env_var = "PFIT_THROTTLE_IMAGE_TOKEN";
    unsafe { std::env::set_var(env_var, "test-token") };
    let mut entry = image_entry(&format!("http://{}", server.addr), env_var);
    entry.requests_per_minute = Some(1);
    let provider = build_image_provider(&entry).expect("build throttled image provider");
    let request = ImageGenerationRequest {
        scene_key: "scene-1".into(),
        prompt: "quota composition".into(),
        output_path: "assets/generated/quota.png".into(),
    };

    provider.generate(&request).expect("first image request");
    let error = provider
        .generate(&request)
        .expect_err("second image throttled");

    assert!(matches!(
        error.kind,
        plotforge_agent::ImageProviderErrorKind::RateLimit {
            retry_after_ms: Some(_)
        }
    ));
    server.handle.join().expect("server thread");
    assert_eq!(
        server
            .captured
            .lock()
            .expect("capture")
            .matches("POST ")
            .count(),
        1
    );
    unsafe { std::env::remove_var(env_var) };
}

// ===========================================================================
// Test scenario 7: TTS generation → audio bytes returned.
//
// The mock TTS API returns 200 with a raw binary body (audio bytes). The
// `OpenAiTtsClient` must surface those bytes as `TtsProviderOutput::audio`.
// The captured request body is asserted to carry model/input/voice/format.
// ===========================================================================
#[test]
#[ignore]
fn tts_generation_returns_audio_bytes() {
    let audio = vec![0x49, 0x44, 0x33, 0x04, b't', b't', b's', b'!']; // "ID3\x04 tts!"
    let reply = http_binary_response("audio/mpeg", &audio);
    let server = capturing_server(reply);

    let entry = tts_entry(&format!("http://{}", server.addr), "PFIT_TTS_TOKEN");
    // The TTS client resolves the credential via the injected
    // `EnvCredentialResolver` (strict: missing/empty surfaces
    // `tts_provider_missing_credential`). Set the env var so the client
    // omits neither the call nor the auth header; clean up at the end so
    // the global state does not leak.
    let tts_credential = "test-tts-credential";
    unsafe { std::env::set_var("PFIT_TTS_TOKEN", tts_credential) };
    let client = OpenAiTtsClient::from_entry(&entry, plotforge_agent::EnvCredentialResolver)
        .expect("build tts client");

    let request = TtsRequest {
        target: TtsTarget::Scene {
            scene_key: "scene-1".into(),
        },
        text: "A short narration line for the TTS integration test.".into(),
        voice: String::new(), // fall back to the entry default voice (coral)
        output_path: "assets/generated/audio/scene-1-scene.wav".into(),
        asset_kind: AssetKind::Audio,
    };
    let output = client.synthesize(&request).expect("tts synthesis succeeds");

    assert_eq!(output.bytes, audio, "audio bytes round-trip the mock body");
    assert_eq!(output.provider, "openai_tts");
    assert_eq!(output.model.as_deref(), Some("gpt-tts-test"));
    assert_eq!(output.spent_cost_units, 1);

    // The captured request must carry model/input/voice/format, with the
    // entry default voice (coral) since the request voice was empty. Use
    // `join_and_full_request` so we can also assert on the Authorization
    // header (F8).
    let full_request = join_and_full_request(server);
    let body = full_request
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .to_string();
    let payload: serde_json::Value =
        serde_json::from_str(&body).expect("captured tts body is JSON");
    assert_eq!(
        payload.get("model").and_then(|v| v.as_str()),
        Some("gpt-tts-test")
    );
    assert_eq!(
        payload.get("input").and_then(|v| v.as_str()),
        Some("A short narration line for the TTS integration test.")
    );
    assert_eq!(payload.get("voice").and_then(|v| v.as_str()), Some("coral"));
    assert_eq!(
        payload.get("response_format").and_then(|v| v.as_str()),
        Some("mp3")
    );

    // F8: assert the credential actually reached the wire as a Bearer header.
    // The text integration test checks this for the text provider; the TTS
    // client must send the same header pattern.
    assert!(
        full_request
            .to_lowercase()
            .contains("authorization: bearer test-tts-credential"),
        "TTS request must carry the Authorization: Bearer header, got: {full_request}"
    );

    unsafe { std::env::remove_var("PFIT_TTS_TOKEN") };
}

#[test]
#[ignore]
fn throttle_tts_rate_limit_surfaces_before_a_second_http_request() {
    let audio = b"ID3-throttle-audio";
    let server = capturing_server(http_binary_response("audio/mpeg", audio));
    let env_var = "PFIT_THROTTLE_TTS_TOKEN";
    unsafe { std::env::set_var(env_var, "test-token") };
    let mut entry = tts_entry(&format!("http://{}", server.addr), env_var);
    entry.requests_per_minute = Some(1);
    let provider = build_tts_provider(&entry).expect("build throttled TTS provider");
    let request = TtsRequest {
        target: TtsTarget::Scene {
            scene_key: "scene-1".into(),
        },
        text: "quota composition".into(),
        voice: String::new(),
        output_path: "assets/generated/quota.mp3".into(),
        asset_kind: AssetKind::Audio,
    };

    provider.synthesize(&request).expect("first TTS request");
    let error = provider
        .synthesize(&request)
        .expect_err("second TTS throttled");

    assert!(matches!(
        error.kind,
        plotforge_agent::TtsProviderErrorKind::RateLimit {
            retry_after_ms: Some(_)
        }
    ));
    server.handle.join().expect("server thread");
    assert_eq!(
        server
            .captured
            .lock()
            .expect("capture")
            .matches("POST ")
            .count(),
        1
    );
    unsafe { std::env::remove_var(env_var) };
}

// ===========================================================================
// Test scenario 8: missing credential → explicit error, no silent fallback.
//
// `ConfiguredTextModelProvider` (the public full-pipeline adapter: validate →
// secret-marker scan → credential resolve → HTTP client → redact) must surface
// an explicit `text_provider_missing_credential` error when the
// `credential_env_var` is unset/empty, with NO silent fallback to a no-auth or
// mock run. The strict `EnvCredentialResolver` is the production resolver; an
// unset env var yields `ProviderCredentialError::Missing`, which the adapter
// maps to `text_provider_missing_credential`. No HTTP call is made (the mock
// server is never contacted — if it were, the test would hang on `accept`).
// ===========================================================================
#[test]
#[ignore]
fn text_missing_credential_is_explicit_error_no_silent_fallback() {
    // A deliberately-unset, unique env var. We do NOT set it; the strict
    // resolver must surface Missing. Use a fresh name so a parallel test or
    // the user shell cannot leak a value in.
    let env_var = "PFIT_MISSING_CREDENTIAL_TOKEN";
    // Defensive: ensure it is unset for this process regardless of the shell.
    // `set_var`/`remove_var` are unsafe in Rust 2024 (they mutate global state).
    unsafe { std::env::remove_var(env_var) };

    let config = text_config("http://127.0.0.1:1", env_var);
    let request = single_prompt_request(&config);
    let provider = ConfiguredTextModelProvider::new(
        config,
        OpenAiCompatibleClient::default(),
        EnvCredentialResolver,
    );

    let error = provider
        .complete(&request)
        .expect_err("missing credential must error, no silent fallback");

    assert_eq!(error.code, "text_provider_missing_credential");
    assert!(
        matches!(error.kind, TextModelProviderErrorKind::Provider),
        "missing credential must be a Provider-kind error, got {error:?}"
    );
    // The error message must name the env var so the user knows what to set,
    // and must NOT contain a credential value (none exists).
    assert!(
        error.message.contains(env_var),
        "error must name the missing env var, got: {error}"
    );
}

/// A complementary guard: when the credential env var IS set, the same
/// `ConfiguredTextModelProvider` must succeed end-to-end against the mock
/// server. This proves the missing-credential error is a true gate, not a
/// blanket failure — and that the credential value flows from the resolver to
/// the client without being logged.
#[test]
#[ignore]
fn text_present_credential_succeeds_through_full_pipeline_adapter() {
    let env_var = "PFIT_PRESENT_CREDENTIAL_TOKEN";
    let credential = "test-credential-value";
    unsafe { std::env::set_var(env_var, credential) };

    let envelope_json = valid_envelope_json();
    let reply = http_response(
        "HTTP/1.1 200 OK",
        "application/json",
        &openai_chat_body(&envelope_json),
    );
    let server = capturing_server(reply);

    let config = text_config(&format!("http://{}", server.addr), env_var);
    let request = single_prompt_request(&config);
    let provider = ConfiguredTextModelProvider::new(
        config,
        OpenAiCompatibleClient::default(),
        EnvCredentialResolver,
    );

    let response = provider.complete(&request).expect("full pipeline succeeds");
    let envelope: AgentOutputEnvelope =
        serde_json::from_str(&response.raw_json).expect("envelope parses");
    assert_eq!(envelope.agent, AgentRole::ScenePlanner);

    // The captured request must carry the Bearer auth header (the credential
    // flowed from the resolver to the client). Header names are
    // case-insensitive on the wire; reqwest may emit either `Authorization`
    // or `authorization`, so compare case-insensitively. The credential value
    // must NOT appear in any error/trace path (there is no error here; this
    // just confirms the header was added and the value reached the wire).
    let request_text = server.captured.lock().expect("capture lock").clone();
    let request_lower = request_text.to_lowercase();
    let expected_header = format!("authorization: bearer {credential}");
    assert!(
        request_lower.contains(&expected_header),
        "credential must flow into the Authorization header; captured request headers:\n{request_text}"
    );

    unsafe { std::env::remove_var(env_var) };
}

// ===========================================================================
// Optional live test (PLOTFORGE_LIVE_TEST=1).
//
// A stub for a real-provider smoke. It only runs when BOTH
// `PLOTFORGE_LIVE_TEST=1` is set AND a `PLOTFORGE_LIVE_TEXT_ENDPOINT` is
// configured with a credential. Otherwise it is skipped (not failed). This
// keeps CI green and reserves real-API validation for manual, opt-in runs.
// ===========================================================================
#[test]
#[ignore]
fn live_text_provider_smoke() {
    // Skip entirely unless the operator has explicitly opted into a live run.
    if std::env::var("PLOTFORGE_LIVE_TEST").ok().as_deref() != Some("1") {
        eprintln!(
            "live_text_provider_smoke: skipped (set PLOTFORGE_LIVE_TEST=1, \
             PLOTFORGE_LIVE_TEXT_ENDPOINT, and PLOTFORGE_LIVE_TEXT_TOKEN to run)"
        );
        return;
    }
    let endpoint = match std::env::var("PLOTFORGE_LIVE_TEXT_ENDPOINT") {
        Ok(value) if !value.is_empty() => value,
        _ => {
            eprintln!("live_text_provider_smoke: skipped (PLOTFORGE_LIVE_TEXT_ENDPOINT unset)");
            return;
        }
    };
    let token_env = "PLOTFORGE_LIVE_TEXT_TOKEN";
    let credential = std::env::var(token_env).unwrap_or_default();

    let config =
        TextProviderConfig::openai_compatible("openai", "live-model", &endpoint, token_env);
    let request = TextModelRequest {
        call_id: "live-smoke".into(),
        agent: AgentRole::ScenePlanner,
        scene_key: "live-scene".into(),
        run_seed: 1,
        prompt_version: "v1".into(),
        model_version: config.model.clone(),
        provider_config_hash: config.provider_config_hash(),
        prompt: "Respond with a single JSON object: {\"id\":\"live-smoke\"}".into(),
        messages: None,
    };
    let client = OpenAiCompatibleClient::default();
    let response = client
        .complete(TextModelClientRequest {
            config: &config,
            credential: &credential,
            request: &request,
        })
        .expect("live provider call succeeded");
    assert!(
        !response.raw_json.trim().is_empty(),
        "live provider returned an empty body"
    );
}

// ---------------------------------------------------------------------------
// Local trait extension so the integration test can ask "is this kind
// retryable?" without reaching the `pub(crate)` `is_retryable` method. The
// retry policy documents retryable kinds as RateLimit / Provider / Timeout;
// content-filter and truncation are non-retryable. Mirroring that contract
// here lets the content-filter / truncation tests assert non-retryability
// through the public type.
// ---------------------------------------------------------------------------

trait TextModelErrorKindExt {
    fn is_retryable_public(&self) -> bool;
}

impl TextModelErrorKindExt for TextModelProviderErrorKind {
    fn is_retryable_public(&self) -> bool {
        matches!(
            self,
            TextModelProviderErrorKind::RateLimit { .. }
                | TextModelProviderErrorKind::Provider
                | TextModelProviderErrorKind::Timeout
        )
    }
}

// Keep a reference to the BeatDrafts types so the integration test module
// compiles even when only the ScenePlan path is exercised — the envelope
// builder is intentionally narrow, but the imports document the available
// payload kinds for future scenario additions.
#[allow(dead_code)]
fn _payload_kind_documentation() -> AgentProposalPayload {
    AgentProposalPayload::BeatDrafts(Box::new(plotforge_schema::BeatDraftsProposal {
        scene_key: String::new(),
        beats: vec![plotforge_schema::BeatDraftProposal {
            id: String::new(),
            scene_key: String::new(),
            text: String::new(),
            choices: vec![Choice {
                id: String::new(),
                label: String::new(),
                action_type: String::new(),
                input_terms: Vec::new(),
                dramatic_purpose: String::new(),
                change_scene: false,
            }],
            narrative_function: NarrativeFunction::Hook,
        }],
    }))
}

// A short sleep helper kept for any future retry-timing scenario; unused now
// but documents the 500ms-base / 8s-cap backoff the retry policy uses.
#[allow(dead_code)]
fn _backoff_documentation() -> Duration {
    Duration::from_millis(500)
}
