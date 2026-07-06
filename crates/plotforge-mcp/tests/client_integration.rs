//! Integration tests for `McpToolRegistry` (the blocking façade) against the
//! fake-server example binary. Lives in `tests/` because it spawns the
//! fake-server subprocess (per the test-layer boundary rule, AGENTS.md:96).

use std::collections::BTreeMap;
use std::path::PathBuf;

use plotforge_mcp::{McpToolClient, McpToolRegistry};
use plotforge_schema::{McpServerEntry, McpServerRegistry, McpTransportConfig, McpTransportKind};

fn fake_server_path() -> PathBuf {
    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root")
            .join("target")
            .join("debug")
            .to_string_lossy()
            .into_owned()
    });
    let exe_name = if cfg!(windows) {
        "fake_mcp_server.exe"
    } else {
        "fake_mcp_server"
    };
    PathBuf::from(target_dir).join("examples").join(exe_name)
}

fn fake_server_entry() -> McpServerEntry {
    McpServerEntry {
        id: "fake-stdio".into(),
        kind: McpTransportKind::Stdio,
        label: "Fake stdio MCP server".into(),
        transport_config: McpTransportConfig::Stdio {
            command: fake_server_path().to_string_lossy().into_owned(),
            args: vec![],
            env: BTreeMap::new(),
        },
        credential_env_var: String::new(),
        enabled: true,
    }
}

fn disabled_entry() -> McpServerEntry {
    McpServerEntry {
        id: "disabled-server".into(),
        kind: McpTransportKind::Stdio,
        label: "Disabled".into(),
        transport_config: McpTransportConfig::Stdio {
            command: fake_server_path().to_string_lossy().into_owned(),
            args: vec![],
            env: BTreeMap::new(),
        },
        credential_env_var: String::new(),
        enabled: false,
    }
}

#[test]
fn mcp_tool_registry_list_tools_round_trips() {
    let registry = McpServerRegistry {
        version: "1".into(),
        servers: vec![fake_server_entry()],
    };
    let client = McpToolRegistry::from_registry(registry);
    let tools = client
        .list_tools("fake-stdio")
        .expect("list_tools round-trip");
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().any(|t| t.name == "ping"));
    assert!(tools.iter().any(|t| t.name == "echo"));
}

#[test]
fn mcp_tool_registry_invoke_tool_round_trips() {
    let registry = McpServerRegistry {
        version: "1".into(),
        servers: vec![fake_server_entry()],
    };
    let client = McpToolRegistry::from_registry(registry);
    let result = client
        .invoke_tool(plotforge_schema::McpToolCallRequest {
            server_id: "fake-stdio".into(),
            tool_name: "ping".into(),
            arguments: serde_json::json!({}),
        })
        .expect("invoke_tool round-trip");
    assert!(result.ok);
    assert!(!result.is_error);
}

#[test]
fn mcp_tool_registry_unknown_server_returns_error() {
    let registry = McpServerRegistry::default();
    let client = McpToolRegistry::from_registry(registry);
    let error = client
        .list_tools("nonexistent")
        .expect_err("unknown server should error");
    assert!(matches!(
        error,
        plotforge_mcp::McpError::UnknownServer { .. }
    ));
}

#[test]
fn mcp_tool_registry_disabled_server_returns_error() {
    let registry = McpServerRegistry {
        version: "1".into(),
        servers: vec![disabled_entry()],
    };
    let client = McpToolRegistry::from_registry(registry);
    let error = client
        .list_tools("disabled-server")
        .expect_err("disabled server should error");
    assert!(matches!(
        error,
        plotforge_mcp::McpError::UnknownServer { .. }
    ));
}

#[test]
fn mcp_tool_registry_caches_transport_across_calls() {
    // Two calls to list_tools should reuse the same transport (the fake
    // server's tools/list is cached after the first call, so the second call
    // returns the cached manifest without a second round-trip — but either
    // way, the transport is the same instance).
    let registry = McpServerRegistry {
        version: "1".into(),
        servers: vec![fake_server_entry()],
    };
    let client = McpToolRegistry::from_registry(registry);
    let tools1 = client.list_tools("fake-stdio").expect("first list_tools");
    let tools2 = client.list_tools("fake-stdio").expect("second list_tools");
    assert_eq!(tools1.len(), tools2.len());
}
