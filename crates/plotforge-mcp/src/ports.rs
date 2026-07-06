//! MCP port traits and error enum.
//!
//! These are the abstraction seams between the MCP transport layer (stdio /
//! SSE / HTTP implementations) and the agent runtime. The agent consumes
//! `McpToolClient` (the blocking façade); the transport layer implements
//! `McpTransport`.
//!
//! ## Design rationale (S.U.P.E.R P + R)
//!
//! The existing `TextModelClient` trait in `plotforge-agent` is NOT extended
//! — it returns a single `TextModelResponse { raw_json }` and has no place
//! for tool definitions or tool-call round-trips. Adding tool-use to it
//! would break 3 production impls + the fake + the test stub in one PR.
//! Instead, MCP tool-use is a parallel port: `McpToolClient` is the
//! agent-facing trait, `McpTransport` is the wire-level trait, and a
//! `complete_with_mcp_tools` wrapper orchestrator (in `plotforge-agent`,
//! Phase 4) runs the multi-turn loop outside `complete_text_agent_output`.
//!
//! `McpTransport` is swappable (stdio ↔ SSE ↔ HTTP behind one trait),
//! satisfying S.U.P.E.R R — the transport impl can be replaced without
//! breaking the agent-facing `McpToolClient`.

use plotforge_schema::{McpToolCallRequest, McpToolCallResult, McpToolManifest};

/// Information about a MCP server obtained from the `initialize` handshake.
/// Carries the server's reported name, version, and protocol version — never
/// credentials or raw transport details.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
}

/// Wire-level transport port for a single MCP server connection.
///
/// Implementations: `StdioMcpClient` (Phase 2, `std::thread` + `BufReader`),
/// `SseMcpClient` / `HttpMcpClient` (Phase 3, async `reqwest` behind a
/// blocking façade). The trait is `?Sized`-friendly via `Box<dyn>` so the
/// registry can hand back a type-erased transport.
pub trait McpTransport {
    /// Perform the JSON-RPC `initialize` handshake. Must be called before
    /// `list_tools` / `call_tool`. Returns the server's reported info.
    fn initialize(&mut self) -> Result<McpServerInfo, McpError>;

    /// Return the server's tool manifest (JSON-RPC `tools/list`). May be
    /// cached by the implementation after the first call.
    fn list_tools(&mut self) -> Result<Vec<McpToolManifest>, McpError>;

    /// Invoke a tool (JSON-RPC `tools/call`). `name` is the tool name from
    /// `list_tools`; `arguments` is a JSON value matching the tool's
    /// `input_schema`. The result content is redaction-safe (the
    /// implementation applies `contains_secret_marker_text` before return).
    fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolCallResult, McpError>;

    /// Close the transport (drop child process / close connection). After
    /// `close`, the transport must not be reused.
    fn close(&mut self);
}

/// Agent-facing port for MCP tool discovery and invocation across multiple
/// servers. Implementations (e.g. `McpToolRegistry` in Phase 2) hold a
/// `McpServerRegistry` + lazily-spawned `Box<dyn McpTransport>` per server
/// id, and expose a blocking API so `plotforge-agent` never sees `tokio`.
///
/// This is the trait `complete_with_mcp_tools` (Phase 4) consumes.
pub trait McpToolClient {
    /// List tools exposed by a specific server. Resolves the entry, builds
    /// (or reuses) the transport, calls `initialize` + `list_tools`.
    fn list_tools(&self, server_id: &str) -> Result<Vec<McpToolManifest>, McpError>;

    /// Invoke a tool on a specific server. Resolves the entry, calls
    /// `call_tool` with redaction applied to args + result.
    fn invoke_tool(&self, request: McpToolCallRequest) -> Result<McpToolCallResult, McpError>;
}

/// Redaction-safe MCP error enum. No variant carries a raw credential value,
/// raw tool result body, or raw transport bytes — messages are derived from
/// the error kind + a short redacted context string. Mirrors the discipline
/// of `PiAgentError` in `plotforge-agent` (redaction-safe `Display`).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum McpError {
    /// Failed to spawn the stdio subprocess (e.g. binary not found, permission
    /// denied). The `command` field is the redacted command name (no args).
    #[error("mcp_spawn_failed: {command}")]
    SpawnFailed { command: String },

    /// The `initialize` JSON-RPC handshake failed (e.g. server returned an
    /// error, protocol version mismatch, timeout during handshake).
    #[error("mcp_handshake_failed: {detail}")]
    HandshakeFailed { detail: String },

    /// A `tools/call` invocation returned an error or the tool name was
    /// unknown to the server. The `tool_name` is the redacted tool name.
    #[error("mcp_tool_error: {tool_name}: {detail}")]
    ToolError { tool_name: String, detail: String },

    /// The transport kind is not yet supported (e.g. SSE/HTTP before Phase 3
    /// lands). This is an explicit error, never a silent fallback to stdio.
    #[error("mcp_transport_unsupported: {kind}")]
    TransportUnsupported { kind: String },

    /// The requested server id is not in the registry, or the server is
    /// disabled in the per-project `AgentSessionConfig.enabled_mcp_servers`.
    #[error("mcp_unknown_server: {server_id}")]
    UnknownServer { server_id: String },

    /// A transport-level I/O error (e.g. child process died, SSE stream
    /// closed, HTTP timeout). The `detail` is redaction-safe.
    #[error("mcp_io: {detail}")]
    Io { detail: String },
}

impl McpError {
    /// Returns true if the error message contains any obvious secret marker.
    /// This is a defence-in-depth check — the construction sites above
    /// already avoid embedding raw credentials, but a caller that passes
    /// through an unredacted context string would be caught here.
    ///
    /// Mirrors `contains_secret_marker_text` from `plotforge-schema`.
    pub fn message_has_secret_marker(&self) -> bool {
        plotforge_schema::contains_secret_marker_text(&self.to_string())
    }
}

/// Scan the content blocks of a `tools/call` result for secret markers,
/// returning `Err(McpError::Io)` if any block carries a marker.
///
/// All block kinds are covered (H1):
/// - `Text { text }`: scanned directly.
/// - `Image { data, mime_type }`: `data` is base64-encoded bytes and could
///   embed a marker (some servers log `data:application/json;base64,...` with
///   a credential); the `data` + `mime_type` are scanned.
/// - `Resource { resource }`: an arbitrary JSON `Value`; serialised and
///   scanned so a nested credential cannot slip past.
///
/// AGENTS.md mandates that MCP tool-call results pass through
/// `contains_secret_marker_text` before entering traces; this helper is the
/// single place that contract is enforced for the `tools/call` result body,
/// so all three transports share one redaction gate.
pub fn scan_content_blocks_for_secret_markers(
    blocks: &[plotforge_schema::McpToolContentBlock],
) -> Result<(), McpError> {
    use plotforge_schema::McpToolContentBlock;
    for block in blocks {
        match block {
            McpToolContentBlock::Text { text } => {
                if plotforge_schema::contains_secret_marker_text(text) {
                    return Err(McpError::Io {
                        detail: "tool result text contains a secret marker".into(),
                    });
                }
            }
            McpToolContentBlock::Image { data, mime_type } => {
                if plotforge_schema::contains_secret_marker_text(data)
                    || plotforge_schema::contains_secret_marker_text(mime_type)
                {
                    return Err(McpError::Io {
                        detail: "tool result image block contains a secret marker".into(),
                    });
                }
            }
            McpToolContentBlock::Resource { resource } => {
                let serialised = resource.to_string();
                if plotforge_schema::contains_secret_marker_text(&serialised) {
                    return Err(McpError::Io {
                        detail: "tool result resource block contains a secret marker".into(),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Assert (at compile time of the test) that `McpError` Display does not
/// leak obvious secret markers for representative inputs. This pins the
/// redaction-safe `Display` contract.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_error_display_is_redaction_safe() {
        let errors = [
            McpError::SpawnFailed {
                command: "mcp-server-fs".into(),
            },
            McpError::HandshakeFailed {
                detail: "protocol version mismatch".into(),
            },
            McpError::ToolError {
                tool_name: "read_file".into(),
                detail: "file not found".into(),
            },
            McpError::TransportUnsupported { kind: "sse".into() },
            McpError::UnknownServer {
                server_id: "local-fs".into(),
            },
            McpError::Io {
                detail: "child process exited".into(),
            },
        ];
        for error in &errors {
            let display = error.to_string();
            assert!(
                !plotforge_schema::contains_secret_marker_text(&display),
                "McpError Display leaked a secret marker: {display}"
            );
            assert!(!error.message_has_secret_marker());
        }
    }

    /// A `MockMcpTransport` for unit-testing the trait shape without spawning
    /// any process or opening any network connection. Returns canned values.
    #[derive(Default)]
    struct MockMcpTransport {
        initialized: bool,
        closed: bool,
        tools: Vec<McpToolManifest>,
    }

    impl McpTransport for MockMcpTransport {
        fn initialize(&mut self) -> Result<McpServerInfo, McpError> {
            self.initialized = true;
            Ok(McpServerInfo {
                name: "mock-mcp".into(),
                version: "0.1.0".into(),
                protocol_version: "2025-06-18".into(),
            })
        }

        fn list_tools(&mut self) -> Result<Vec<McpToolManifest>, McpError> {
            if !self.initialized {
                return Err(McpError::HandshakeFailed {
                    detail: "not initialized".into(),
                });
            }
            Ok(self.tools.clone())
        }

        fn call_tool(
            &mut self,
            _name: &str,
            _arguments: serde_json::Value,
        ) -> Result<McpToolCallResult, McpError> {
            if !self.initialized {
                return Err(McpError::HandshakeFailed {
                    detail: "not initialized".into(),
                });
            }
            Ok(McpToolCallResult {
                ok: true,
                content: vec![],
                is_error: false,
            })
        }

        fn close(&mut self) {
            self.closed = true;
        }
    }

    #[test]
    fn mock_mcp_transport_round_trip() {
        let mut transport = MockMcpTransport::default();
        let info = transport.initialize().expect("initialize");
        assert_eq!(info.name, "mock-mcp");
        let _tools = transport.list_tools().expect("list_tools");
        let result = transport
            .call_tool("ping", serde_json::json!({}))
            .expect("call_tool");
        assert!(result.ok);
        transport.close();
        assert!(transport.closed);
    }

    #[test]
    fn mock_mcp_transport_rejects_calls_before_initialize() {
        let mut transport = MockMcpTransport::default();
        let err = transport
            .list_tools()
            .expect_err("list_tools before initialize");
        assert!(matches!(err, McpError::HandshakeFailed { .. }));
    }

    /// A `MockMcpToolClient` for unit-testing the agent-facing port shape.
    struct MockMcpToolClient {
        tools: Vec<McpToolManifest>,
    }

    impl McpToolClient for MockMcpToolClient {
        fn list_tools(&self, _server_id: &str) -> Result<Vec<McpToolManifest>, McpError> {
            Ok(self.tools.clone())
        }

        fn invoke_tool(&self, _request: McpToolCallRequest) -> Result<McpToolCallResult, McpError> {
            Ok(McpToolCallResult {
                ok: true,
                content: vec![],
                is_error: false,
            })
        }
    }

    #[test]
    fn mock_mcp_tool_client_round_trip() {
        let client = MockMcpToolClient {
            tools: vec![McpToolManifest {
                name: "ping".into(),
                description: "ping tool".into(),
                input_schema: serde_json::Value::Null,
            }],
        };
        let tools = client.list_tools("any-server").expect("list_tools");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "ping");
        let result = client
            .invoke_tool(McpToolCallRequest {
                server_id: "any-server".into(),
                tool_name: "ping".into(),
                arguments: serde_json::json!({}),
            })
            .expect("invoke_tool");
        assert!(result.ok);
    }

    /// H1 regression: a secret marker in an Image or Resource content block
    /// must trip the scan, not just a Text block. Before the fix, only Text
    /// blocks were scanned so base64 image bytes / nested resource JSON could
    /// smuggle a credential past the redaction guard.
    #[test]
    fn content_block_scan_rejects_secret_in_image_and_resource() {
        use plotforge_schema::McpToolContentBlock;
        let text_ok = vec![McpToolContentBlock::Text {
            text: "harmless output".into(),
        }];
        scan_content_blocks_for_secret_markers(&text_ok).expect("clean text passes");

        let image_bad = vec![McpToolContentBlock::Image {
            data: "c2tfcmVhbGtleQ==".into(), // base64; not a marker itself
            mime_type: "image/png".into(),
        }];
        scan_content_blocks_for_secret_markers(&image_bad).expect("clean image passes");

        let image_with_marker = vec![McpToolContentBlock::Image {
            data: "sk-test-secret-marker".into(),
            mime_type: String::new(),
        }];
        let error = scan_content_blocks_for_secret_markers(&image_with_marker)
            .expect_err("image marker must be rejected");
        assert!(matches!(error, McpError::Io { .. }));

        let resource_with_marker = vec![McpToolContentBlock::Resource {
            resource: serde_json::json!({"note": "leaked api_key sk-realkey-here"}),
        }];
        let error = scan_content_blocks_for_secret_markers(&resource_with_marker)
            .expect_err("nested resource marker must be rejected");
        assert!(matches!(error, McpError::Io { .. }));

        // Text is still rejected (regression guard).
        let text_bad = vec![McpToolContentBlock::Text {
            text: "sk-test-secret-marker".into(),
        }];
        let error =
            scan_content_blocks_for_secret_markers(&text_bad).expect_err("text marker rejected");
        assert!(matches!(error, McpError::Io { .. }));
    }
}
