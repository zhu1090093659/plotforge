//! MCP (Model Context Protocol) client crate.
//!
//! This crate owns MCP server transport (stdio + SSE/HTTP), server lifecycle,
//! tool discovery, and tool invocation. It is the third user-global local-only
//! configuration surface alongside `plotforge-agent`'s provider registry and
//! skills library — the registry lives at `~/.plotforge/mcp.json` and never
//! enters project source, contracts, traces, or export packages.
//!
//! ## Crate boundary
//!
//! Dependency direction (preserves the workspace DAG):
//! `plotforge-schema` ← `plotforge-mcp` ← `plotforge-agent` ← `plotforge-studio`.
//!
//! This crate depends ONLY on `plotforge-schema` + transport dependencies. It
//! must NOT depend on `plotforge-agent` (that would create a cycle). The
//! agent crate consumes the blocking façade (`McpToolRegistry`) exposed here.
//!
//! The `tokio` async runtime (added in Phase 3 for SSE streaming) is scoped to
//! this crate only — `plotforge-agent` and all other workspace crates must not
//! pull `tokio` as a direct dependency.
//!
//! ## Tool-use integration
//!
//! MCP tool-use integration uses a new `McpToolClient` port trait + a
//! `complete_with_mcp_tools` wrapper orchestrator (in `plotforge-agent`); the
//! existing `TextModelClient` trait is NOT extended or modified. Transport
//! failures are explicit (`mcp_spawn_failed`, `mcp_handshake_failed`,
//! `mcp_tool_error`, `mcp_transport_unsupported`, `mcp_unknown_server`,
//! `mcp_io`); no silent fallback to a no-MCP path.
//!
//! ## Status
//!
//! Phase 1 (this commit): crate skeleton — empty module doc + schema re-export
//! placeholder. The `McpTransport` / `McpToolClient` port traits, registry IO,
//! and stdio transport land in Phase 2; SSE/HTTP transport in Phase 3.

pub mod ports;
pub use ports::{
    McpError, McpServerInfo, McpToolClient, McpTransport, scan_content_blocks_for_secret_markers,
};

pub mod client;
pub use client::McpToolRegistry;

pub mod registry;
pub use registry::{
    McpRegistryError, build_mcp_client, load_mcp_registry, load_mcp_registry_from,
    mcp_registry_path, resolve_mcp_server, user_config_dir, validate_server_entry,
    write_mcp_registry, write_mcp_registry_to,
};

pub mod runtime;
pub use runtime::runtime;

pub mod transport_stdio;
pub use transport_stdio::StdioMcpClient;

pub mod transport_sse;
pub use transport_sse::SseMcpClient;

pub mod transport_http;
pub use transport_http::HttpMcpClient;

pub use plotforge_schema::{
    McpServerEntry, McpServerRegistry, McpServerTestResult, McpToolCallRequest, McpToolCallResult,
    McpToolContentBlock, McpToolManifest, McpTransportConfig, McpTransportKind,
};
