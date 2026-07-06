//! MCP (Model Context Protocol) server registry schema contracts.
//!
//! These types describe the user-global, local-only MCP server registry that
//! backs MCP tool-call routing. The registry lives at `~/.plotforge/mcp.json`
//! (the third member of the providers/skills/prompts family) and never enters
//! project source, contracts, traces, or export packages. Credentials are
//! referenced indirectly by environment-variable name (`credential_env_var`);
//! the registry never stores credential values, tool-call arguments, or raw
//! tool results.
//!
//! See the MCP carve-out paragraph in `AGENTS.md` §Architecture Boundaries for
//! the boundary rules these types enforce.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The transport a MCP server speaks. Drives which transport client the
/// `plotforge-mcp` crate constructs; never carried as a credential.
#[derive(Clone, Copy, Debug, JsonSchema, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportKind {
    /// Local subprocess speaking JSON-RPC over stdin/stdout (line-delimited).
    #[default]
    Stdio,
    /// Server-Sent Events streaming transport (HTTP `text/event-stream`).
    Sse,
    /// Plain HTTP request/response JSON-RPC (no streaming).
    Http,
}

/// Transport-specific configuration for a MCP server entry. Tagged enum so
/// the on-disk JSON carries the transport kind alongside its config.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum McpTransportConfig {
    /// stdio transport: spawn a local subprocess with `command` + `args` +
    /// `env`, speaking JSON-RPC over stdin/stdout.
    #[serde(rename = "stdio")]
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
    },
    /// SSE transport: connect to an HTTP `endpoint_url` and consume
    /// `text/event-stream` frames as JSON-RPC.
    #[serde(rename = "sse")]
    Sse { endpoint_url: String },
    /// HTTP transport: POST JSON-RPC to an `endpoint_url` (request/response,
    /// no streaming).
    #[serde(rename = "http")]
    Http { endpoint_url: String },
}

/// A single registered MCP server entry. `credential_env_var` names the shell
/// environment variable that holds the credential (used for SSE/HTTP bearer
/// or header auth); the value itself is never serialized here, in traces, or
/// in any project source. For stdio servers with no network auth, this field
/// may be empty.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct McpServerEntry {
    pub id: String,
    pub kind: McpTransportKind,
    pub label: String,
    pub transport_config: McpTransportConfig,
    pub credential_env_var: String,
    pub enabled: bool,
}

/// The persisted registry file (`~/.plotforge/mcp.json`). An empty registry
/// is the default for fresh installs; MCP server wiring is opt-in.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields, default)]
pub struct McpServerRegistry {
    pub version: String,
    pub servers: Vec<McpServerEntry>,
}

/// A tool manifest discovered from a MCP server via `tools/list`. The
/// `input_schema` is a JSON Schema describing the tool's arguments; it is
/// stored as a `serde_json::Value` to avoid a hard dependency on a specific
/// JSON-Schema crate version in this schema crate.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct McpToolManifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub input_schema: serde_json::Value,
}

/// A request to invoke a MCP tool on a specific server. `arguments` is a
/// JSON value matching the tool's `input_schema`; it is validated at invoke
/// time, not at schema-deserialize time.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct McpToolCallRequest {
    pub server_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

/// A content block returned by a MCP tool call. Mirrors the MCP spec's
/// `content` array shape: text, image, or embedded resource. Tagged enum so
/// the on-wire JSON carries the block kind.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum McpToolContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(default)]
        mime_type: String,
    },
    #[serde(rename = "resource")]
    Resource { resource: serde_json::Value },
}

/// The result of a MCP tool call. `ok` is true when the server returned a
/// successful `tools/call` response; `is_error` mirrors the MCP spec's
/// `isError` flag (a tool may return content with `is_error=true` to signal
/// a tool-level error without a transport failure). The `content` blocks
/// are redaction-safe summaries only — raw tool bodies never enter traces.
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct McpToolCallResult {
    pub ok: bool,
    #[serde(default)]
    pub content: Vec<McpToolContentBlock>,
    #[serde(default)]
    pub is_error: bool,
}

/// The result of a `test_mcp_server` Studio command. Mirrors
/// `ProviderTestResult { ok, message }` plus a `tools_count` so the UI can
/// show how many tools the server exposed during the probe. The `message`
/// field is redaction-safe (`redact_trace_text` applied by the Studio layer).
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct McpServerTestResult {
    pub ok: bool,
    pub message: String,
    pub tools_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_stdio_entry() -> McpServerEntry {
        McpServerEntry {
            id: "local-fs".into(),
            kind: McpTransportKind::Stdio,
            label: "Local filesystem MCP".into(),
            transport_config: McpTransportConfig::Stdio {
                command: "mcp-server-fs".into(),
                args: vec!["--root".into(), "/tmp".into()],
                env: BTreeMap::new(),
            },
            credential_env_var: String::new(),
            enabled: true,
        }
    }

    fn sample_sse_entry() -> McpServerEntry {
        McpServerEntry {
            id: "remote-sse".into(),
            kind: McpTransportKind::Sse,
            label: "Remote SSE MCP".into(),
            transport_config: McpTransportConfig::Sse {
                endpoint_url: "https://mcp.example.com/sse".into(),
            },
            credential_env_var: "MCP_BEARER_TOKEN".into(),
            enabled: true,
        }
    }

    #[test]
    fn mcp_transport_kind_renames_snake_case() {
        let kinds = [
            (McpTransportKind::Stdio, "stdio"),
            (McpTransportKind::Sse, "sse"),
            (McpTransportKind::Http, "http"),
        ];
        for (kind, expected) in kinds {
            let encoded = serde_json::to_string(&kind).expect("serialize kind");
            assert_eq!(encoded, format!("\"{expected}\""));
            let decoded: McpTransportKind =
                serde_json::from_str(&encoded).expect("deserialize kind");
            assert_eq!(decoded, kind);
        }
    }

    #[test]
    fn mcp_transport_config_tags_roundtrip() {
        let configs = [
            (
                McpTransportConfig::Stdio {
                    command: "mcp-server".into(),
                    args: vec!["--port".into(), "8080".into()],
                    env: BTreeMap::from([("LOG_LEVEL".into(), "debug".into())]),
                },
                "stdio",
            ),
            (
                McpTransportConfig::Sse {
                    endpoint_url: "https://example.com/sse".into(),
                },
                "sse",
            ),
            (
                McpTransportConfig::Http {
                    endpoint_url: "https://example.com/rpc".into(),
                },
                "http",
            ),
        ];
        for (config, expected_tag) in configs {
            let encoded = serde_json::to_string(&config).expect("serialize config");
            assert!(
                encoded.contains(&format!("\"kind\":\"{expected_tag}\"")),
                "encoded config should carry kind tag: {encoded}"
            );
            let decoded: McpTransportConfig =
                serde_json::from_str(&encoded).expect("deserialize config");
            assert_eq!(decoded, config);
        }
    }

    #[test]
    fn mcp_server_entry_roundtrips_json() {
        for entry in [sample_stdio_entry(), sample_sse_entry()] {
            let encoded = serde_json::to_string_pretty(&entry).expect("serialize entry");
            let decoded: McpServerEntry =
                serde_json::from_str(&encoded).expect("deserialize entry");
            assert_eq!(decoded, entry);
        }
    }

    #[test]
    fn mcp_server_entry_rejects_secret_fields() {
        let entry = sample_stdio_entry();
        let mut value = serde_json::to_value(&entry).expect("entry value");
        value["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<McpServerEntry>(value)
            .expect_err("api_key field should be rejected");
        assert!(error.to_string().contains("unknown field"));

        let mut value = serde_json::to_value(sample_sse_entry()).expect("entry value");
        value["secret_token"] = serde_json::json!("tok-secret-marker");
        let error = serde_json::from_value::<McpServerEntry>(value)
            .expect_err("secret_token field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn mcp_server_registry_roundtrips_json() {
        let registry = McpServerRegistry {
            version: "1".into(),
            servers: vec![sample_stdio_entry(), sample_sse_entry()],
        };
        let encoded = serde_json::to_string_pretty(&registry).expect("serialize registry");
        let decoded: McpServerRegistry =
            serde_json::from_str(&encoded).expect("deserialize registry");
        assert_eq!(decoded, registry);
        assert_eq!(decoded.servers.len(), 2);
    }

    #[test]
    fn mcp_server_registry_default_is_empty() {
        let registry = McpServerRegistry::default();
        assert!(registry.servers.is_empty());
        assert!(registry.version.is_empty());
    }

    #[test]
    fn mcp_server_registry_rejects_unknown_fields() {
        let mut value = serde_json::to_value(McpServerRegistry::default()).expect("registry value");
        value["api_keys"] = serde_json::json!([{"secret": "sk-test-secret-marker"}]);
        let error = serde_json::from_value::<McpServerRegistry>(value)
            .expect_err("unknown registry field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn mcp_tool_manifest_roundtrips_json() {
        let manifest = McpToolManifest {
            name: "read_file".into(),
            description: "Read a file from the filesystem.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        };
        let encoded = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
        let decoded: McpToolManifest =
            serde_json::from_str(&encoded).expect("deserialize manifest");
        assert_eq!(decoded, manifest);
    }

    #[test]
    fn mcp_tool_manifest_rejects_secret_fields() {
        let manifest = McpToolManifest {
            name: "tool".into(),
            description: String::new(),
            input_schema: serde_json::Value::Null,
        };
        let mut value = serde_json::to_value(&manifest).expect("manifest value");
        value["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<McpToolManifest>(value)
            .expect_err("api_key field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn mcp_tool_call_request_roundtrips_json() {
        let request = McpToolCallRequest {
            server_id: "local-fs".into(),
            tool_name: "read_file".into(),
            arguments: serde_json::json!({"path": "/tmp/test.txt"}),
        };
        let encoded = serde_json::to_string_pretty(&request).expect("serialize request");
        let decoded: McpToolCallRequest =
            serde_json::from_str(&encoded).expect("deserialize request");
        assert_eq!(decoded, request);
    }

    #[test]
    fn mcp_tool_call_result_roundtrips_json() {
        let result = McpToolCallResult {
            ok: true,
            content: vec![
                McpToolContentBlock::Text {
                    text: "file contents here".into(),
                },
                McpToolContentBlock::Image {
                    data: "base64data".into(),
                    mime_type: "image/png".into(),
                },
            ],
            is_error: false,
        };
        let encoded = serde_json::to_string_pretty(&result).expect("serialize result");
        let decoded: McpToolCallResult =
            serde_json::from_str(&encoded).expect("deserialize result");
        assert_eq!(decoded, result);
    }

    #[test]
    fn mcp_tool_content_block_tags_roundtrip() {
        let blocks = [
            (
                McpToolContentBlock::Text {
                    text: "hello".into(),
                },
                "text",
            ),
            (
                McpToolContentBlock::Image {
                    data: "data".into(),
                    mime_type: "image/png".into(),
                },
                "image",
            ),
        ];
        for (block, expected_tag) in blocks {
            let encoded = serde_json::to_string(&block).expect("serialize block");
            assert!(encoded.contains(&format!("\"type\":\"{expected_tag}\"")));
            let decoded: McpToolContentBlock =
                serde_json::from_str(&encoded).expect("deserialize block");
            assert_eq!(decoded, block);
        }
    }

    #[test]
    fn mcp_server_test_result_roundtrips_json() {
        let result = McpServerTestResult {
            ok: true,
            message: "Server responded; 5 tools available.".into(),
            tools_count: 5,
        };
        let encoded = serde_json::to_string_pretty(&result).expect("serialize test result");
        let decoded: McpServerTestResult =
            serde_json::from_str(&encoded).expect("deserialize test result");
        assert_eq!(decoded, result);
    }

    #[test]
    fn mcp_server_test_result_rejects_secret_fields() {
        let result = McpServerTestResult {
            ok: true,
            message: "ok".into(),
            tools_count: 0,
        };
        let mut value = serde_json::to_value(&result).expect("test result value");
        value["api_key"] = serde_json::json!("sk-test-secret-marker");
        let error = serde_json::from_value::<McpServerTestResult>(value)
            .expect_err("api_key field should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    /// Secret-marker rejection for endpoint URLs: the schema types
    /// themselves don't validate URL content (URL `?key=` credential
    /// detection is the transport layer's job, mirroring
    /// `url_has_query_credential` in `providers_text.rs`). But the schema
    /// crate's `contains_secret_marker_text` helper is the redaction
    /// primitive the transport layer relies on — this test pins that it
    /// catches the obvious secret markers a malicious endpoint URL might
    /// embed as a path segment.
    #[test]
    fn mcp_endpoint_url_with_secret_marker_is_flagged() {
        use crate::contains_secret_marker_text;
        // A bearer-token marker in a URL fragment is flagged.
        let url_with_bearer = "https://example.com/sse?authorization=Bearer";
        assert!(
            contains_secret_marker_text(url_with_bearer),
            "endpoint_url with 'Bearer' marker must be flagged by contains_secret_marker_text"
        );
        // A token= marker is flagged.
        let url_with_token = "https://example.com/sse?token=abc123";
        assert!(
            contains_secret_marker_text(url_with_token),
            "endpoint_url with 'token=' marker must be flagged"
        );
        let clean_url = "https://example.com/sse";
        assert!(
            !contains_secret_marker_text(clean_url),
            "clean endpoint_url must not be flagged"
        );
    }
}
