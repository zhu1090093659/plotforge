//! Integration tests for the stdio MCP transport against an in-process
//! fake-server binary (committed as `examples/fake_mcp_server.rs`).
//!
//! These tests spawn the fake server as a subprocess, so they live in the
//! `tests/` directory (not `#[cfg(test)] mod tests` in `src/`), per the test-
//! layer boundary rule "no process spawns in Rust unit tests" (AGENTS.md:96).
//! This is the first `tests/` directory for `plotforge-mcp`.

use std::collections::BTreeMap;

use plotforge_mcp::{McpTransport, StdioMcpClient};

/// Resolve the path to the fake-server example binary built by `cargo test`.
/// Examples are placed under `<workspace-target>/debug/examples/fake_mcp_server`.
/// `CARGO_TARGET_DIR` is set by `cargo test` when invoked at workspace level;
/// when running `cargo test -p plotforge-mcp` from a subdir, fall back to
/// `<manifest>/../../target/debug` (workspace root target).
fn fake_server_path() -> std::path::PathBuf {
    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // crates/plotforge-mcp -> workspace root -> target/debug
        manifest
            .parent() // crates/
            .and_then(|p| p.parent()) // workspace root
            .expect("workspace root")
            .join("target")
            .join("debug")
            .to_string_lossy()
            .into_owned()
    });
    let target_dir = std::path::PathBuf::from(target_dir);
    let exe_name = if cfg!(windows) {
        "fake_mcp_server.exe"
    } else {
        "fake_mcp_server"
    };
    target_dir.join("examples").join(exe_name)
}

fn spawn_fake_server() -> StdioMcpClient {
    let path = fake_server_path();
    StdioMcpClient::new(
        path.to_str().expect("fake server path is utf-8"),
        &[],
        &BTreeMap::new(),
    )
    .expect("spawn fake MCP server")
}

#[test]
fn stdio_initialize_handshake_round_trips() {
    let mut client = spawn_fake_server();
    let info = client.initialize().expect("initialize handshake");
    assert_eq!(info.name, "fake-mcp-server");
    assert_eq!(info.version, "0.1.0");
    assert_eq!(info.protocol_version, "2025-06-18");
    client.close();
}

#[test]
fn stdio_list_tools_returns_canned_manifests() {
    let mut client = spawn_fake_server();
    client.initialize().expect("initialize");
    let tools = client.list_tools().expect("list_tools");
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().any(|t| t.name == "ping"));
    assert!(tools.iter().any(|t| t.name == "echo"));
    // Second call is cached (no second round-trip to the server).
    let tools_again = client.list_tools().expect("list_tools cached");
    assert_eq!(tools_again.len(), 2);
    client.close();
}

#[test]
fn stdio_call_tool_ping_returns_pong() {
    let mut client = spawn_fake_server();
    client.initialize().expect("initialize");
    let result = client
        .call_tool("ping", serde_json::json!({}))
        .expect("call ping");
    assert!(result.ok);
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
    match &result.content[0] {
        plotforge_schema::McpToolContentBlock::Text { text } => {
            assert_eq!(text, "pong");
        }
        other => panic!("expected text block, got {other:?}"),
    }
    client.close();
}

#[test]
fn stdio_call_tool_echo_returns_argument_text() {
    let mut client = spawn_fake_server();
    client.initialize().expect("initialize");
    let result = client
        .call_tool("echo", serde_json::json!({"text": "hello world"}))
        .expect("call echo");
    assert!(result.ok);
    match &result.content[0] {
        plotforge_schema::McpToolContentBlock::Text { text } => {
            assert_eq!(text, "hello world");
        }
        other => panic!("expected text block, got {other:?}"),
    }
    client.close();
}

#[test]
fn stdio_call_tool_unknown_returns_tool_error() {
    let mut client = spawn_fake_server();
    client.initialize().expect("initialize");
    let error = client
        .call_tool("nonexistent", serde_json::json!({}))
        .expect_err("unknown tool should error");
    assert!(matches!(error, plotforge_mcp::McpError::ToolError { .. }));
    client.close();
}

#[test]
fn stdio_call_before_initialize_returns_handshake_error() {
    let mut client = spawn_fake_server();
    let error = client
        .list_tools()
        .expect_err("list_tools before initialize should error");
    assert!(matches!(
        error,
        plotforge_mcp::McpError::HandshakeFailed { .. }
    ));
    client.close();
}

#[test]
#[allow(clippy::err_expect)] // StdioMcpClient does not derive Debug (owns Child + channels)
fn stdio_spawn_failed_for_nonexistent_binary() {
    let error = StdioMcpClient::new("this-binary-does-not-exist-12345", &[], &BTreeMap::new())
        .err()
        .expect("nonexistent binary should fail to spawn");
    assert!(matches!(error, plotforge_mcp::McpError::SpawnFailed { .. }));
}
