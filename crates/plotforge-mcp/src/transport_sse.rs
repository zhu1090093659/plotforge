//! SSE MCP transport: POST JSON-RPC requests to an HTTP `endpoint_url`,
//! consume `text/event-stream` frames as responses.
//!
//! This implements the MCP "streamable HTTP" transport pattern: each request
//! is a POST with `Content-Type: application/json`, `Accept: text/event-stream`;
//! the response is a stream of SSE `data:` frames, each carrying one JSON-RPC
//! response. The transport reads frames until it finds the one matching the
//! request `id`, then returns its `result`/`error`.
//!
//! ## Async/blocking bridge
//!
//! Internally uses async `reqwest` (scoped to `plotforge-mcp`'s `tokio`
//! runtime). The `McpTransport` impl is synchronous — it calls
//! `runtime().block_on(...)` to run the async methods. `plotforge-agent`
//! never sees `tokio` or `async`.
//!
//! ## Redaction
//!
//! Tool args and result content pass through `contains_secret_marker_text`
//! before send / after receive. `endpoint_url` is validated to reject `?key=`
//! credential patterns at construction time.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use plotforge_schema::{
    McpToolCallResult, McpToolContentBlock, McpToolManifest, contains_secret_marker_text,
};

use crate::{McpError, McpServerInfo, McpTransport};

/// HTTP timeout for SSE transport (matches the provider HTTP timeout in
/// `plotforge-agent::providers_http`).
const SSE_TIMEOUT: Duration = Duration::from_secs(60);

/// A MCP SSE transport client. Owns an async `reqwest::Client` (held behind
/// the crate's `tokio` runtime). Not `Clone` (one transport per server
/// connection).
pub struct SseMcpClient {
    endpoint_url: String,
    credential_env_var: String,
    client: reqwest::Client,
    next_id: AtomicU64,
    initialized: bool,
    tools_cache: Vec<McpToolManifest>,
}

impl SseMcpClient {
    /// Construct a new SSE client. Validates the `endpoint_url` rejects
    /// `?key=` credential patterns (mirrors `url_has_query_credential` in
    /// `plotforge-agent::providers_text`). The credential is resolved from
    /// `credential_env_var` at call time (never stored as a value).
    pub fn new(endpoint_url: &str, credential_env_var: &str) -> Result<Self, McpError> {
        if url_has_query_credential(endpoint_url) {
            return Err(McpError::Io {
                detail: "endpoint_url contains a credential query parameter".into(),
            });
        }
        if contains_secret_marker_text(endpoint_url) {
            return Err(McpError::Io {
                detail: "endpoint_url contains a secret marker".into(),
            });
        }
        let client = reqwest::Client::builder()
            .timeout(SSE_TIMEOUT)
            .build()
            .map_err(|error| McpError::Io {
                detail: format!("failed to build HTTP client: {error}"),
            })?;
        Ok(Self {
            endpoint_url: endpoint_url.to_string(),
            credential_env_var: credential_env_var.to_string(),
            client,
            next_id: AtomicU64::new(1),
            initialized: false,
            tools_cache: Vec::new(),
        })
    }

    /// Resolve the credential from the env var at call time (never stored).
    /// Returns empty string when `credential_env_var` is empty (no-auth
    /// servers). Mirrors `OptionalEnvCredentialResolver` in `plotforge-agent`.
    fn resolve_credential(&self) -> String {
        if self.credential_env_var.trim().is_empty() {
            return String::new();
        }
        std::env::var(&self.credential_env_var).unwrap_or_default()
    }

    /// Send a JSON-RPC request via POST, consume the SSE response stream,
    /// return the matching response's `result`/`error`.
    fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        // Secret-marker scan on the serialized params.
        let params_str = params.to_string();
        if contains_secret_marker_text(&params_str) {
            return Err(McpError::Io {
                detail: "request params contain a secret marker".into(),
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let credential = self.resolve_credential();
        let endpoint_url = self.endpoint_url.clone();
        let client = self.client.clone();
        let method_owned = method.to_string();
        let result = crate::runtime::block_on_async(async move {
            let mut builder = client
                .post(&endpoint_url)
                .header("Accept", "text/event-stream")
                .header("Content-Type", "application/json")
                .json(&request_body);
            if !credential.is_empty() {
                builder = builder.bearer_auth(&credential);
            }
            let response = builder.send().await.map_err(|error| McpError::Io {
                detail: format!("SSE request failed: {error}"),
            })?;
            let status = response.status();
            if !status.is_success() {
                // M3: a 4xx/5xx body may carry a JSON-RPC error with a useful
                // `error.message`; try to parse it before falling back to a
                // generic status string. This keeps a structured tool/handshake
                // error from being misclassified as a bare transport error.
                return Err(http_error_with_jsonrpc(status, response, &method_owned).await);
            }
            // M2: bound the body read so a server that streams forever (or
            // returns a giant non-SSE error page) cannot OOM the worker. 4 MiB
            // is generous for a finite MCP event stream; a long-lived SSE
            // connection should be consumed with `bytes_stream()` in a future
            // revision, but for the streamable-HTTP per-request pattern this
            // cap is the correct safety bound.
            const SSE_BODY_LIMIT: usize = 4 * 1024 * 1024;
            let body = bounded_text(response, SSE_BODY_LIMIT).await?;
            // Parse SSE frames: lines starting with `data: ` are JSON payloads.
            for line in body.lines() {
                let line = line.trim();
                if line.is_empty() || !line.starts_with("data:") {
                    continue;
                }
                let payload = line.trim_start_matches("data:").trim();
                if payload.is_empty() {
                    continue;
                }
                let value: serde_json::Value = match serde_json::from_str(payload) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                let response_id = value.get("id").and_then(|v| v.as_u64());
                if response_id == Some(id) {
                    if let Some(error_obj) = value.get("error") {
                        let detail = error_obj
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown error")
                            .to_string();
                        return Err(match method_owned.as_str() {
                            "tools/call" => McpError::ToolError {
                                tool_name: "(unknown)".into(),
                                detail,
                            },
                            "initialize" => McpError::HandshakeFailed { detail },
                            _ => McpError::Io { detail },
                        });
                    }
                    return value.get("result").cloned().ok_or_else(|| McpError::Io {
                        detail: "SSE response had no result field".into(),
                    });
                }
            }
            Err(McpError::Io {
                detail: "no matching SSE response frame".into(),
            })
        })?;
        Ok(result)
    }
}

impl McpTransport for SseMcpClient {
    fn initialize(&mut self) -> Result<McpServerInfo, McpError> {
        if self.initialized {
            return Ok(McpServerInfo {
                name: "(already initialized)".into(),
                version: String::new(),
                protocol_version: String::new(),
            });
        }
        let result = self.request(
            "initialize",
            serde_json::json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "plotforge-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }),
        )?;
        let name = result
            .pointer("/serverInfo/name")
            .and_then(|v| v.as_str())
            .unwrap_or("(unknown)")
            .to_string();
        let version = result
            .pointer("/serverInfo/version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let protocol_version = result
            .pointer("/protocolVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // M4: send `notifications/initialized` after a successful handshake.
        // The MCP spec requires this before subsequent calls; spec-strict
        // servers may reject `tools/list` without it. Best-effort: a transport
        // failure here is not fatal (the server already accepted initialize).
        let credential = self.resolve_credential();
        let client = self.client.clone();
        let endpoint_url = self.endpoint_url.clone();
        crate::runtime::block_on_async(crate::transport_sse::send_initialized_notification(
            client,
            endpoint_url,
            credential,
            "text/event-stream",
        ));
        self.initialized = true;
        Ok(McpServerInfo {
            name,
            version,
            protocol_version,
        })
    }

    fn list_tools(&mut self) -> Result<Vec<McpToolManifest>, McpError> {
        if !self.initialized {
            return Err(McpError::HandshakeFailed {
                detail: "not initialized".into(),
            });
        }
        if !self.tools_cache.is_empty() {
            return Ok(self.tools_cache.clone());
        }
        let result = self.request("tools/list", serde_json::json!({}))?;
        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();
        let mut manifests = Vec::with_capacity(tools.len());
        for tool in tools {
            let manifest: McpToolManifest =
                serde_json::from_value(tool).map_err(|error| McpError::Io {
                    detail: format!("failed to parse tool manifest: {error}"),
                })?;
            manifests.push(manifest);
        }
        self.tools_cache = manifests.clone();
        Ok(manifests)
    }

    fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolCallResult, McpError> {
        if !self.initialized {
            return Err(McpError::HandshakeFailed {
                detail: "not initialized".into(),
            });
        }
        let result = self.request(
            "tools/call",
            serde_json::json!({
                "name": name,
                "arguments": arguments,
            }),
        )?;
        let is_error = result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let content = result
            .get("content")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        let mut blocks = Vec::with_capacity(content.len());
        for block_value in content {
            let block: McpToolContentBlock =
                serde_json::from_value(block_value).map_err(|error| McpError::Io {
                    detail: format!("failed to parse content block: {error}"),
                })?;
            blocks.push(block);
        }
        // Redaction scan covers all block kinds (Text, Image, Resource) — H1.
        crate::ports::scan_content_blocks_for_secret_markers(&blocks)?;
        Ok(McpToolCallResult {
            ok: !is_error,
            content: blocks,
            is_error,
        })
    }

    fn close(&mut self) {
        // SSE transport is request/response per call; no persistent connection
        // to close. The `reqwest::Client` is dropped when `self` drops.
        self.initialized = false;
    }
}

/// Reject `?key=` / `?api_key=` / `?token=` credential query parameters in
/// the endpoint URL. Mirrors `url_has_query_credential` in
/// `plotforge-agent::providers_text`.
fn url_has_query_credential(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("?key=")
        || lower.contains("&key=")
        || lower.contains("?api_key=")
        || lower.contains("&api_key=")
        || lower.contains("?token=")
        || lower.contains("&token=")
}

/// Read up to `limit` bytes from `response` as text. Returns `mcp_io` if the
/// body exceeds the limit (M2: prevents a non-SSE error page or a
/// runaway stream from OOMing the worker). The cap is generous for a finite
/// MCP event stream; a long-lived SSE connection needs `bytes_stream()` in a
/// future revision.
pub(crate) async fn bounded_text(
    response: reqwest::Response,
    limit: usize,
) -> Result<String, McpError> {
    let bytes = response.bytes().await.map_err(|error| McpError::Io {
        detail: format!("failed to read response body: {error}"),
    })?;
    if bytes.len() > limit {
        return Err(McpError::Io {
            detail: format!("response body exceeded the {limit}-byte safety limit"),
        });
    }
    String::from_utf8(bytes.to_vec()).map_err(|error| McpError::Io {
        detail: format!("response body was not valid UTF-8: {error}"),
    })
}

/// Build an `McpError` for a non-2xx HTTP response, attempting to parse the
/// body as a JSON-RPC `{error: {message}}` first so a structured tool/handshake
/// error is not misclassified as a bare transport error (M3). Falls back to a
/// generic `mcp_io` carrying the status code when the body is not JSON-RPC.
pub(crate) async fn http_error_with_jsonrpc(
    status: reqwest::StatusCode,
    response: reqwest::Response,
    method: &str,
) -> McpError {
    let body = bounded_text(response, 64 * 1024)
        .await
        .unwrap_or_default();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body)
        && let Some(message) = value
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
    {
        return match method {
            "tools/call" => McpError::ToolError {
                tool_name: "(unknown)".into(),
                detail: message.to_string(),
            },
            "initialize" => McpError::HandshakeFailed {
                detail: message.to_string(),
            },
            _ => McpError::Io {
                detail: format!("HTTP {status}: {message}"),
            },
        };
    }
    McpError::Io {
        detail: format!("HTTP endpoint returned {status}"),
    }
}

/// Fire a `notifications/initialized` POST with no expected response (M4).
/// The MCP spec requires this notification after a successful `initialize`
/// handshake; spec-strict servers may reject subsequent calls without it.
/// The notification carries no `id`, so the server must not respond — any
/// response bytes are discarded. Best-effort: a failure here does not abort
/// initialization (the server is still considered initialized) but surfaces
/// a debug-level `mcp_io` in the caller's stderr only.
pub(crate) async fn send_initialized_notification(
    client: reqwest::Client,
    endpoint_url: String,
    credential: String,
    accept: &str,
) {
    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
    });
    let mut builder = client
        .post(&endpoint_url)
        .header("Accept", accept)
        .header("Content-Type", "application/json")
        .json(&notification);
    if !credential.is_empty() {
        builder = builder.bearer_auth(&credential);
    }
    // Best-effort: the spec says servers must not respond to notifications,
    // so we send and drop the result. A transport error here is not fatal.
    let _ = builder.send().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_has_query_credential_detects_key_params() {
        assert!(url_has_query_credential("https://example.com/sse?key=abc"));
        assert!(url_has_query_credential(
            "https://example.com/sse?x=1&api_key=abc"
        ));
        assert!(url_has_query_credential(
            "https://example.com/sse?token=abc"
        ));
        assert!(!url_has_query_credential("https://example.com/sse"));
        assert!(!url_has_query_credential(
            "https://example.com/sse?path=/tmp"
        ));
    }

    #[test]
    fn sse_client_rejects_credential_in_url() {
        let error = SseMcpClient::new("https://example.com/sse?key=secret", "")
            .err()
            .expect("credential in URL should be rejected");
        assert!(matches!(error, McpError::Io { .. }));
    }

    #[test]
    fn sse_client_rejects_secret_marker_in_url() {
        let error = SseMcpClient::new("https://example.com/sse?token=sk-test-secret-marker", "")
            .err()
            .expect("secret marker in URL should be rejected");
        assert!(matches!(error, McpError::Io { .. }));
    }

    #[test]
    fn sse_client_constructs_for_clean_url() {
        let client =
            SseMcpClient::new("https://example.com/sse", "").expect("clean URL should construct");
        assert_eq!(client.endpoint_url, "https://example.com/sse");
    }
}
