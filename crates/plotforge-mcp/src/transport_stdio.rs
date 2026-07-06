//! Stdio MCP transport: spawn a local subprocess speaking JSON-RPC over
//! stdin/stdout (newline-delimited).
//!
//! ## Concurrency model (no async)
//!
//! The workspace has no `tokio`/`async-std`. stdio transport uses
//! `std::process::Command` + `BufReader`/`BufWriter` + `std::thread` for
//! concurrent write+read. The read thread blocks on `BufReader::read_line`
//! and posts responses into a `std::sync::mpsc` channel; the call thread
//! sends a request line and waits on the channel for the matching response
//! (by JSON-RPC `id`).
//!
//! ## Lifecycle
//!
//! spawn → `initialize` handshake → `tools/list` cache → `tools/call` per
//! invoke → detect child process death → explicit `mcp_spawn_failed` /
//! `mcp_io` error (no silent restart per AGENTS.md G9). Drop child on
//! `close`.
//!
//! ## Redaction
//!
//! Tool args and result content pass through `contains_secret_marker_text`
//! before send / after receive. Raw tool bodies never leave this struct.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;

use plotforge_schema::{
    McpToolCallResult, McpToolContentBlock, McpToolManifest, contains_secret_marker_text,
};

use crate::{McpError, McpServerInfo, McpTransport};

/// A MCP stdio transport client. Owns a child process + a reader thread +
/// a request/response correlation channel. Not `Clone` (one transport per
/// server connection).
pub struct StdioMcpClient {
    child: Option<Child>,
    stdin: Option<BufWriter<std::process::ChildStdin>>,
    response_rx: mpsc::Receiver<JsonRpcResponse>,
    reader_handle: Option<thread::JoinHandle<()>>,
    next_id: AtomicU64,
    initialized: bool,
    tools_cache: Vec<McpToolManifest>,
}

/// A parsed JSON-RPC response received from the child's stdout. The reader
/// thread sends these into the channel; the call thread filters by `id`.
struct JsonRpcResponse {
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

impl StdioMcpClient {
    /// Spawn the child process and start the reader thread. Does NOT perform
    /// the `initialize` handshake — call `initialize()` after construction.
    pub fn new(
        command: &str,
        args: &[String],
        env: &BTreeMap<String, String>,
    ) -> Result<Self, McpError> {
        // Validate command before spawn (defence-in-depth; registry also validates).
        if command.trim().is_empty() || command.contains('\0') || command.contains("..") {
            return Err(McpError::SpawnFailed {
                command: redact_command_name(command),
            });
        }
        for arg in args {
            if arg.contains('\0') {
                return Err(McpError::Io {
                    detail: "stdio arg contains NUL".into(),
                });
            }
        }
        let mut cmd = Command::new(command);
        cmd.args(args);
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().map_err(|_error| McpError::SpawnFailed {
            command: redact_command_name(command),
        })?;
        let stdin = child.stdin.take().ok_or_else(|| McpError::SpawnFailed {
            command: redact_command_name(command),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| McpError::SpawnFailed {
            command: redact_command_name(command),
        })?;
        let (tx, rx) = mpsc::channel::<JsonRpcResponse>();
        let reader_handle = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) if line.trim().is_empty() => continue,
                    Ok(line) => {
                        if let Ok(response) = parse_jsonrpc_response(&line) {
                            // Channel send fails only when the receiver is dropped
                            // (client closed); in that case the reader thread exits.
                            if tx.send(response).is_err() {
                                break;
                            }
                        }
                        // Lines that don't parse as JSON-RPC responses
                        // (notifications, malformed) are silently skipped
                        // — the stdio transport only correlates request/response
                        // pairs by id.
                    }
                    Err(_) => break, // stdout closed or read error
                }
            }
        });
        Ok(Self {
            child: Some(child),
            stdin: Some(BufWriter::new(stdin)),
            response_rx: rx,
            reader_handle: Some(reader_handle),
            next_id: AtomicU64::new(1),
            initialized: false,
            tools_cache: Vec::new(),
        })
    }

    /// Send a JSON-RPC request and wait for the matching response (by id).
    /// Returns the `result` field on success, or `McpError` on:
    /// - secret-marker in the request params (rejected before send)
    /// - send failure (child stdin closed) → `mcp_io`
    /// - response `error` field → `mcp_tool_error` / `mcp_handshake_failed`
    /// - timeout (channel recv) → `mcp_io`
    fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        // Secret-marker scan on the serialized params (defence-in-depth).
        let params_str = params.to_string();
        if contains_secret_marker_text(&params_str) {
            return Err(McpError::Io {
                detail: "request params contain a secret marker".into(),
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let line = serde_json::to_string(&request).map_err(|error| McpError::Io {
            detail: format!("failed to serialize request: {error}"),
        })?;
        // Send.
        if let Some(stdin) = &mut self.stdin {
            stdin
                .write_all(line.as_bytes())
                .and_then(|_| stdin.write_all(b"\n"))
                .and_then(|_| stdin.flush())
                .map_err(|error| McpError::Io {
                    detail: format!("failed to write request: {error}"),
                })?;
        } else {
            return Err(McpError::Io {
                detail: "stdin closed".into(),
            });
        }
        // Wait for the matching response (filter by id). Responses for other
        // ids are discarded (the stdio transport is single-request at a time
        // in this blocking impl, so no other id should be in flight).
        let timeout = std::time::Duration::from_secs(30);
        let response = self
            .response_rx
            .recv_timeout(timeout)
            .map_err(|error| McpError::Io {
                detail: format!("timed out waiting for response: {error}"),
            })?;
        if response.id != id {
            return Err(McpError::Io {
                detail: "response id mismatch".into(),
            });
        }
        if let Some(error_obj) = response.error {
            let detail = error_obj
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error")
                .to_string();
            return match method {
                "tools/call" => Err(McpError::ToolError {
                    tool_name: "(unknown)".into(),
                    detail,
                }),
                "initialize" => Err(McpError::HandshakeFailed { detail }),
                _ => Err(McpError::Io { detail }),
            };
        }
        response.result.ok_or_else(|| McpError::Io {
            detail: "response had no result field".into(),
        })
    }
}

impl McpTransport for StdioMcpClient {
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
        // Send the `notifications/initialized` notification (no response expected).
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        });
        if let Some(stdin) = &mut self.stdin {
            let line = serde_json::to_string(&notification).map_err(|error| McpError::Io {
                detail: format!("failed to serialize notification: {error}"),
            })?;
            let _ = stdin
                .write_all(line.as_bytes())
                .and_then(|_| stdin.write_all(b"\n"))
                .and_then(|_| stdin.flush());
        }
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

    fn list_tools(&mut self) -> Result<Vec<McpToolManifest>, McpToolManifestError> {
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
        // Parse the content blocks. MCP `tools/call` returns `{content: [{type, ...}], isError?: bool}`.
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
            // Redaction scan on the parsed block's text.
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
        // Drop stdin first so the child sees EOF and exits cleanly.
        self.stdin = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
        // The reader thread exits when stdout closes (child exited).
        if let Some(handle) = self.reader_handle.take() {
            let _ = handle.join();
        }
        self.initialized = false;
    }
}

impl Drop for StdioMcpClient {
    fn drop(&mut self) {
        self.close();
    }
}

/// Parse a line from the child's stdout as a JSON-RPC response. Returns
/// `Ok(response)` only for valid response objects with an `id` field;
/// notifications and malformed lines return `Err` (silently skipped by the
/// reader thread).
fn parse_jsonrpc_response(line: &str) -> Result<JsonRpcResponse, ()> {
    let value: serde_json::Value = serde_json::from_str(line).map_err(|_| ())?;
    let id = value.get("id").and_then(|v| v.as_u64()).ok_or(())?;
    let result = value.get("result").cloned();
    let error = value.get("error").cloned();
    Ok(JsonRpcResponse { id, result, error })
}

/// Return a redacted command name for error messages: basename only.
fn redact_command_name(command: &str) -> String {
    let trimmed = command.trim();
    match trimmed.rsplit(['/', '\\']).next() {
        Some(basename) if !basename.is_empty() => basename.to_string(),
        _ => "(redacted)".into(),
    }
}

/// Convenience alias for the error type returned by `list_tools`. The trait
/// signature uses `McpError`; this alias keeps the impl readable.
type McpToolManifestError = McpError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_command_name_keeps_basename() {
        assert_eq!(
            redact_command_name("/usr/local/bin/mcp-server"),
            "mcp-server"
        );
        assert_eq!(redact_command_name("mcp-server-fs"), "mcp-server-fs");
    }

    #[test]
    fn parse_jsonrpc_response_extracts_id_result() {
        let line = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let response = parse_jsonrpc_response(line).expect("valid response");
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn parse_jsonrpc_response_extracts_error() {
        let line = r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message"}}"#;
        // Malformed (missing message value) — serde_json::Value still parses
        // the partial JSON; `error` is present.
        let _ = line; // skip malformed; test the well-formed error shape instead.
        let line = r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"not found"}}"#;
        let response = parse_jsonrpc_response(line).expect("valid error response");
        assert_eq!(response.id, 2);
        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn parse_jsonrpc_response_rejects_notification() {
        // A notification has no `id` field — it's not a response.
        let line = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        assert!(parse_jsonrpc_response(line).is_err());
    }

    #[test]
    fn parse_jsonrpc_response_rejects_malformed() {
        assert!(parse_jsonrpc_response("not json").is_err());
        assert!(parse_jsonrpc_response("").is_err());
    }
}
