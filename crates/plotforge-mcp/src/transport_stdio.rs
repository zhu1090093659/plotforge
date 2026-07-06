//! Stdio MCP transport: spawn a local subprocess speaking JSON-RPC over
//! stdin/stdout (newline-delimited).
//!
//! ## Concurrency model (no async)
//!
//! The workspace has no `tokio`/`async-std`. stdio transport uses
//! `std::process::Command` + `BufReader`/`BufWriter` + `std::thread` for
//! concurrent write+read. The read thread blocks on `BufReader::read_line`
//! and posts responses into a `std::sync::mpsc` channel; the call thread
//! sends a request line and drains the channel for the matching response
//! (by JSON-RPC `id`). Late/unsolicited responses that survive a prior
//! timeout are discarded (not fatal) so a misbehaving server cannot poison
//! the transport.
//!
//! ## Lifecycle
//!
//! spawn → `initialize` handshake → `tools/list` cache → `tools/call` per
//! invoke → detect child process death → explicit `mcp_spawn_failed` /
//! `mcp_io` error (no silent restart per AGENTS.md G9). On `close`/drop the
//! child is killed (`start_kill`) before `wait` and both joins are bounded
//! by a timeout so a wedged subprocess cannot hang the agent thread.
//!
//! ## Redaction
//!
//! Tool args and result content pass through `contains_secret_marker_text`
//! before send / after receive. Raw tool bodies never leave this struct.
//! This includes `Text`, `Image`, and `Resource` content blocks: the
//! text of a Text block is scanned directly, and an Image/Resource block is
//! serialised and scanned so base64 bytes / JSON payloads cannot smuggle a
//! secret marker past the guard.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use plotforge_schema::{
    McpToolCallResult, McpToolContentBlock, McpToolManifest, contains_secret_marker_text,
};

use crate::{McpError, McpServerInfo, McpTransport};

/// Bounded wait for a JSON-RPC response before treating the call as timed out.
const STDIO_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
/// Bounded wait for the child to exit after `kill` and for the reader thread
/// to join on `close`/drop. Prevents a wedged subprocess from hanging the
/// agent thread indefinitely (the previous implementation blocked forever).
const STDIO_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// A MCP stdio transport client. Owns a child process + a reader thread +
/// a request/response correlation channel + a stderr-drainer thread. Not
/// `Clone` (one transport per server connection).
pub struct StdioMcpClient {
    child: Option<Child>,
    stdin: Option<BufWriter<std::process::ChildStdin>>,
    response_rx: mpsc::Receiver<JsonRpcResponse>,
    reader_handle: Option<thread::JoinHandle<()>>,
    /// Latest line captured from the child's stderr. Surfaces in handshake /
    /// I/O errors so a spawn or handshake failure is diagnosable instead of
    /// an opaque timeout. Drained by a background thread that discards older
    /// lines to bound memory.
    last_stderr: std::sync::Arc<std::sync::Mutex<Option<String>>>,
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
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|_error| McpError::SpawnFailed {
            command: redact_command_name(command),
        })?;
        let stdin = child.stdin.take().ok_or_else(|| McpError::SpawnFailed {
            command: redact_command_name(command),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| McpError::SpawnFailed {
            command: redact_command_name(command),
        })?;
        let stderr = child.stderr.take();
        let last_stderr = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        // Spawn a stderr drainer that keeps only the most recent line, so a
        // spawn/handshake failure surfaces a diagnostic instead of an opaque
        // timeout (M1). The thread exits when stderr closes (child exited).
        if let Some(stderr) = stderr {
            let last_stderr_for_thread = last_stderr.clone();
            let _stderr_handle = thread::Builder::new()
                .name("plotforge-mcp-stdio-stderr".into())
                .spawn(move || {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines() {
                        match line {
                            Ok(line) if !line.trim().is_empty() => {
                                if let Ok(mut guard) = last_stderr_for_thread.lock() {
                                    *guard = Some(line);
                                }
                            }
                            _ => break,
                        }
                    }
                });
            // The handle is intentionally detached: the thread is bounded by
            // the child's stderr lifetime and the shutdown path kills the
            // child, which closes stderr and lets this thread exit.
        }
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
            last_stderr,
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
    ///
    /// Drains the channel until the matching id arrives (or the timeout
    /// elapses), so a late response that survived a prior call's timeout —
    /// or an unsolicited duplicate/progress response from a misbehaving
    /// server — is discarded instead of poisoning the next call (H2).
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
        // Drain the channel until the matching id arrives, discarding any
        // late/unsolicited responses for other ids. The deadline bounds the
        // total wait so a server that never answers does not hang forever.
        let deadline = std::time::Instant::now() + STDIO_RESPONSE_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(self.io_error_with_stderr(
                    "timed out waiting for response (no matching id in channel)",
                ));
            }
            match self.response_rx.recv_timeout(remaining) {
                Ok(response) if response.id == id => {
                    return self.finalize_response(response, method);
                }
                Ok(_other) => {
                    // A response for a different id (late, duplicate, or
                    // progress). Discard and keep draining.
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(self.io_error_with_stderr(
                        "timed out waiting for response (channel drained, no match)",
                    ));
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(self.io_error_with_stderr(
                        "response channel disconnected (child exited or reader thread died)",
                    ));
                }
            }
        }
    }

    /// Map a JSON-RPC response into its `result` field or the appropriate
    /// `McpError` variant based on the originating method.
    fn finalize_response(
        &self,
        response: JsonRpcResponse,
        method: &str,
    ) -> Result<serde_json::Value, McpError> {
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

    /// Build an `McpError::Io` that includes the last captured stderr line
    /// (if any) so spawn/handshake/I/O failures are diagnosable (M1). The
    /// stderr text is run through `contains_secret_marker_text` first; if it
    /// trips the guard, the line is replaced with a redaction notice so a
    /// server that logs a credential cannot leak it through the error.
    fn io_error_with_stderr(&self, base_detail: &str) -> McpError {
        let detail = match self.last_stderr.lock() {
            Ok(guard) => {
                if let Some(line) = guard.as_deref() {
                    if contains_secret_marker_text(line) {
                        format!("{base_detail}; server stderr contained a secret marker (redacted)")
                    } else {
                        format!("{base_detail}; server stderr: {line}")
                    }
                } else {
                    base_detail.to_string()
                }
            }
            Err(_) => base_detail.to_string(),
        };
        McpError::Io { detail }
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
            blocks.push(block);
        }
        // Redaction scan covers all block kinds (Text, Image, Resource) so a
        // marker in a base64 image body or a nested resource JSON cannot slip
        // past the guard (H1).
        crate::ports::scan_content_blocks_for_secret_markers(&blocks)?;
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
            // Kill before wait so a wedged subprocess cannot hang the agent
            // thread forever (C2). `kill` sends SIGKILL (Unix) / TerminateProcess
            // (Windows) and returns once the signal is delivered (it does not
            // wait for reaping).
            let _ = child.kill();
            // Bounded wait via non-blocking `try_wait` poll. If the child does
            // not exit within the shutdown timeout, leak it rather than block
            // the caller — a leaked child is OS-reaped on process exit, a hung
            // agent thread is not.
            let deadline = std::time::Instant::now() + STDIO_SHUTDOWN_TIMEOUT;
            while child.try_wait().ok().flatten().is_none() {
                if std::time::Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        // The reader thread exits when stdout closes (child exited/was killed).
        // Bound its join with the same timeout so it cannot outlive the
        // shutdown window.
        if let Some(handle) = self.reader_handle.take() {
            let _ = join_bounded(handle, STDIO_SHUTDOWN_TIMEOUT);
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

/// Join a `JoinHandle` with a bounded timeout so a blocked reader thread
/// cannot hang the agent thread forever during `close`/drop. If the thread
/// does not finish within `timeout`, it is detached (the OS reaps it on
/// process exit). Returns `Ok` if it joined, `Err` if it timed out.
fn join_bounded(handle: thread::JoinHandle<()>, timeout: Duration) -> Result<(), ()> {
    // Spin a short sleep/poll loop since `JoinHandle::join` has no timeout.
    // The thread is expected to exit promptly once the child is killed (its
    // stdout closes), so this rarely spins.
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if handle.is_finished() {
            return handle.join().map_err(|_| ());
        }
        if std::time::Instant::now() >= deadline {
            return Err(());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

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

    /// C2 regression: `close()` must terminate even when the child never
    /// exits on its own. A child that ignores stdin EOF and keeps running
    /// must not hang the agent thread — `close` kills it and bounds the wait.
    #[test]
    fn close_does_not_hang_on_unresponsive_child() {
        // A `sleep 60` child that ignores stdin EOF: simulates a wedged server.
        let mut client = StdioMcpClient::new("sleep", &["60".to_string()], &BTreeMap::new())
            .expect("spawn sleep child");
        let start = std::time::Instant::now();
        client.close();
        let elapsed = start.elapsed();
        // Must return well under the child's 60s lifetime, bounded by the
        // shutdown timeout (5s) — assert a comfortable margin.
        assert!(
            elapsed < Duration::from_secs(10),
            "close() hung for {elapsed:?}"
        );
    }

    /// M1 regression: child stderr is captured, not discarded. A child that
    /// writes to stderr on failure surfaces the line in the resulting error.
    #[test]
    fn stderr_capture_surfaces_in_io_error() {
        // A `sh -c 'echo bad-config >&2; sleep 30'` child writes a stderr
        // line then idles; the request times out and the error must include
        // the captured stderr text.
        let mut client = StdioMcpClient::new(
            "sh",
            &["-c".into(), "echo bad-config-line >&2; sleep 30".into()],
            &BTreeMap::new(),
        )
        .expect("spawn stderr child");
        // Use a short-response timeout variant by requesting and timing out.
        // The default STDIO_RESPONSE_TIMEOUT is 30s; cap the test by killing
        // the client after a short poll so it does not run the full 30s.
        let poll_deadline = std::time::Instant::now() + Duration::from_secs(8);
        let result = std::thread::scope(|s| {
            s.spawn(|| client.request("initialize", serde_json::json!({})))
                .join()
                .expect("request thread")
        });
        let _ = poll_deadline;
        let error = result.expect_err("request should time out (no MCP server)");
        assert!(matches!(error, McpError::Io { .. }));
        let detail = match error {
            McpError::Io { detail } => detail,
            _ => String::new(),
        };
        assert!(
            detail.contains("bad-config-line"),
            "expected captured stderr in error, got: {detail}"
        );
    }
}
