//! HTTP MCP transport: POST JSON-RPC requests to an HTTP `endpoint_url`,
//! receive a single JSON response (no streaming).
//!
//! For MCP servers that use plain request/response HTTP instead of SSE. Same
//! redaction/credential/validation discipline as `transport_sse`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use plotforge_schema::{
    McpToolCallResult, McpToolContentBlock, McpToolManifest, contains_secret_marker_text,
};

use crate::{McpError, McpServerInfo, McpTransport};

const HTTP_TIMEOUT: Duration = Duration::from_secs(60);

/// A MCP HTTP transport client. Owns an async `reqwest::Client` (held behind
/// the crate's `tokio` runtime). Not `Clone`.
pub struct HttpMcpClient {
    endpoint_url: String,
    credential_env_var: String,
    client: reqwest::Client,
    next_id: AtomicU64,
    initialized: bool,
    tools_cache: Vec<McpToolManifest>,
}

impl HttpMcpClient {
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
            .timeout(HTTP_TIMEOUT)
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

    fn resolve_credential(&self) -> String {
        if self.credential_env_var.trim().is_empty() {
            return String::new();
        }
        std::env::var(&self.credential_env_var).unwrap_or_default()
    }

    fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
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
        let result = crate::runtime().block_on(async move {
            let mut builder = client
                .post(&endpoint_url)
                .header("Content-Type", "application/json")
                .json(&request_body);
            if !credential.is_empty() {
                builder = builder.bearer_auth(&credential);
            }
            let response = builder.send().await.map_err(|error| McpError::Io {
                detail: format!("HTTP request failed: {error}"),
            })?;
            let status = response.status();
            if !status.is_success() {
                return Err(McpError::Io {
                    detail: format!("HTTP endpoint returned {status}"),
                });
            }
            let value: serde_json::Value = response.json().await.map_err(|error| McpError::Io {
                detail: format!("failed to parse HTTP response: {error}"),
            })?;
            let response_id = value.get("id").and_then(|v| v.as_u64());
            if response_id != Some(id) {
                return Err(McpError::Io {
                    detail: "HTTP response id mismatch".into(),
                });
            }
            if let Some(error_obj) = value.get("error") {
                let detail = error_obj
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                return Err(match method {
                    "tools/call" => McpError::ToolError {
                        tool_name: "(unknown)".into(),
                        detail,
                    },
                    "initialize" => McpError::HandshakeFailed { detail },
                    _ => McpError::Io { detail },
                });
            }
            value.get("result").cloned().ok_or_else(|| McpError::Io {
                detail: "HTTP response had no result field".into(),
            })
        })?;
        Ok(result)
    }
}

impl McpTransport for HttpMcpClient {
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
            #[allow(clippy::collapsible_if)]
            if let McpToolContentBlock::Text { text } = &block {
                if contains_secret_marker_text(text) {
                    return Err(McpError::Io {
                        detail: "tool result text contains a secret marker".into(),
                    });
                }
            }
            blocks.push(block);
        }
        Ok(McpToolCallResult {
            ok: !is_error,
            content: blocks,
            is_error,
        })
    }

    fn close(&mut self) {
        self.initialized = false;
    }
}

fn url_has_query_credential(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("?key=")
        || lower.contains("&key=")
        || lower.contains("?api_key=")
        || lower.contains("&api_key=")
        || lower.contains("?token=")
        || lower.contains("&token=")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_client_rejects_credential_in_url() {
        let error = HttpMcpClient::new("https://example.com/rpc?key=secret", "")
            .err()
            .expect("credential in URL should be rejected");
        assert!(matches!(error, McpError::Io { .. }));
    }

    #[test]
    fn http_client_constructs_for_clean_url() {
        let client = HttpMcpClient::new("https://example.com/rpc", "").expect("clean URL");
        assert_eq!(client.endpoint_url, "https://example.com/rpc");
    }
}
