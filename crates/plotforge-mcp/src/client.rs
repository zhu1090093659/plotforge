//! `McpToolRegistry` — the blocking façade that `plotforge-agent` consumes.
//!
//! Holds a `McpServerRegistry` (loaded from `~/.plotforge/mcp.json`) and
//! lazily-spawned `Box<dyn McpTransport>` per server id (cached across
//! `list_tools`/`invoke_tool` calls). Implements `McpToolClient` so the agent
//! crate can `use plotforge_mcp::McpToolClient` without seeing `tokio` (the
//! async runtime, added in Phase 3 for SSE, is scoped to `plotforge-mcp`).
//!
//! ## Lifecycle
//!
//! - `load()` reads `~/.plotforge/mcp.json` (missing → empty registry, no error)
//! - `list_tools(server_id)` resolves the entry, builds (or reuses) the
//!   transport, calls `initialize` + `list_tools`
//! - `invoke_tool(request)` resolves the entry, calls `call_tool` with
//!   redaction applied to args + result
//! - Unknown id → `McpError::UnknownServer`
//! - Disabled server → `McpError::UnknownServer` (treat disabled as
//!   unreachable; the per-project `enabled_mcp_servers` list is the gate, not
//!   the registry's `enabled` flag — but a registry-disabled server is also
//!   refused here as defence-in-depth)

use std::collections::HashMap;
use std::sync::Mutex;

use plotforge_schema::{McpServerRegistry, McpToolCallRequest, McpToolCallResult, McpToolManifest};

use crate::{
    McpError, McpToolClient, McpTransport, build_mcp_client, load_mcp_registry, resolve_mcp_server,
};

/// A blocking façade over multiple MCP servers. Thread-safe via `Mutex` (the
/// agent may call `list_tools`/`invoke_tool` from any thread; the underlying
/// `McpTransport` impls are `Send` but not `Sync`, so each call locks the
/// matching transport).
pub struct McpToolRegistry {
    registry: McpServerRegistry,
    /// Lazily-spawned transports, keyed by server id. Each transport is held
    /// behind a `Mutex` so concurrent calls to different servers don't block
    /// each other, but calls to the same server serialize (stdio transports
    /// are inherently single-request at a time).
    transports: Mutex<HashMap<String, Mutex<Box<dyn McpTransport>>>>,
}

impl McpToolRegistry {
    /// Load the registry from `~/.plotforge/mcp.json`. A missing file returns
    /// an empty registry (not an error) so a fresh install keeps working.
    pub fn load() -> Result<Self, McpError> {
        let registry = load_mcp_registry().map_err(|error| McpError::Io {
            detail: format!("failed to load MCP registry: {error}"),
        })?;
        Ok(Self {
            registry,
            transports: Mutex::new(HashMap::new()),
        })
    }

    /// Construct from an explicit registry (used by tests + the Studio layer
    /// when it already has the registry loaded).
    pub fn from_registry(registry: McpServerRegistry) -> Self {
        Self {
            registry,
            transports: Mutex::new(HashMap::new()),
        }
    }

    /// Get or lazily spawn the transport for a server id. The transport is
    /// cached in the `transports` map; subsequent calls reuse it. If the
    /// transport has died (the underlying `Mutex` is poisoned or the
    /// transport returns `Io` errors), the caller is expected to surface the
    /// error explicitly — no silent restart (AGENTS.md G9).
    fn transport_for(&self, server_id: &str) -> Result<(), McpError> {
        // Check the entry exists + is enabled before touching the transport map.
        let entry = resolve_mcp_server(server_id, &self.registry).ok_or_else(|| {
            McpError::UnknownServer {
                server_id: server_id.into(),
            }
        })?;
        if !entry.enabled {
            return Err(McpError::UnknownServer {
                server_id: server_id.into(),
            });
        }
        let mut transports = self.transports.lock().expect("transports mutex poisoned");
        if !transports.contains_key(server_id) {
            let transport = build_mcp_client(entry)?;
            transports.insert(server_id.into(), Mutex::new(transport));
        }
        Ok(())
    }

    /// Borrow the transport for a server id and run a closure on it. The
    /// closure receives `&mut dyn McpTransport` (locked for the duration).
    fn with_transport<R>(
        &self,
        server_id: &str,
        f: impl FnOnce(&mut dyn McpTransport) -> Result<R, McpError>,
    ) -> Result<R, McpError> {
        self.transport_for(server_id)?;
        // Re-fetch the entry to validate again (defence-in-depth; the entry
        // could theoretically be removed between the transport_for call and
        // here, though this struct is immutable post-construction).
        let entry = resolve_mcp_server(server_id, &self.registry).ok_or_else(|| {
            McpError::UnknownServer {
                server_id: server_id.into(),
            }
        })?;
        if !entry.enabled {
            return Err(McpError::UnknownServer {
                server_id: server_id.into(),
            });
        }
        let transports = self.transports.lock().expect("transports mutex poisoned");
        let transport_mutex = transports
            .get(server_id)
            .ok_or_else(|| McpError::UnknownServer {
                server_id: server_id.into(),
            })?;
        let mut transport = transport_mutex.lock().expect("transport mutex poisoned");
        f(transport.as_mut())
    }

    /// Return the redaction-safe, non-secret hash-input string per enabled
    /// server, in the order given. Each entry is
    /// `{server_id}|{transport_kind}|{endpoint_or_command}|{credential_env_var_name}`
    /// — never the credential value, never raw transport bytes. Servers in
    /// `enabled` that are absent from the registry are skipped (they surface
    /// as `mcp_unknown_server` at invoke time; the hash input only covers
    /// servers that actually exist + are enabled in the registry).
    ///
    /// This is the input the `complete_with_mcp_tools` wrapper hashes to
    /// derive the combined `mcp_tool_call_hash` stamped onto reproducibility
    /// (mirrors `provider_config_hash`). Derived only from non-secret config
    /// fields per the MCP carve-out.
    pub fn hash_inputs_for(&self, enabled: &[String]) -> Vec<String> {
        enabled
            .iter()
            .filter_map(|server_id| {
                let entry = resolve_mcp_server(server_id, &self.registry)?;
                if !entry.enabled {
                    return None;
                }
                Some(hash_input_for_entry(entry))
            })
            .collect()
    }
}

/// Build the redaction-safe hash-input string for a single server entry.
/// Mirrors the `provider_config_hash` discipline: only non-secret config
/// fields participate; the credential value is never included (only the
/// `credential_env_var` *name*). The transport kind is the snake_case tag.
fn hash_input_for_entry(entry: &plotforge_schema::McpServerEntry) -> String {
    let transport_kind = match entry.transport_config {
        plotforge_schema::McpTransportConfig::Stdio { .. } => "stdio",
        plotforge_schema::McpTransportConfig::Sse { .. } => "sse",
        plotforge_schema::McpTransportConfig::Http { .. } => "http",
    };
    // For stdio, the `command` basename is the redaction-safe identity (the
    // full args/env may carry secrets or user paths). For SSE/HTTP, the
    // `endpoint_url` is the identity (already secret-marker-scanned at
    // upsert time). Either way, only non-secret fields enter the hash input.
    let endpoint_or_command = match &entry.transport_config {
        plotforge_schema::McpTransportConfig::Stdio { command, .. } => {
            // Keep only the basename to avoid leaking absolute user paths
            // into the reproducibility hash (a path like `/home/user/.…/mcp`
            // is not a secret but is not reproducibility-relevant either).
            std::path::Path::new(command)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(command)
                .to_string()
        }
        plotforge_schema::McpTransportConfig::Sse { endpoint_url } => endpoint_url.clone(),
        plotforge_schema::McpTransportConfig::Http { endpoint_url } => endpoint_url.clone(),
    };
    format!(
        "{}|{}|{}|{}",
        entry.id, transport_kind, endpoint_or_command, entry.credential_env_var,
    )
}

impl McpToolClient for McpToolRegistry {
    fn list_tools(&self, server_id: &str) -> Result<Vec<McpToolManifest>, McpError> {
        self.with_transport(server_id, |transport| {
            // initialize is idempotent (returns cached info if already initialized)
            transport.initialize()?;
            transport.list_tools()
        })
    }

    fn invoke_tool(&self, request: McpToolCallRequest) -> Result<McpToolCallResult, McpError> {
        let server_id = request.server_id.clone();
        let tool_name = request.tool_name.clone();
        self.with_transport(&server_id, |transport| {
            transport.initialize()?;
            transport.call_tool(&tool_name, request.arguments)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use plotforge_schema::{
        McpServerEntry, McpServerRegistry, McpTransportConfig, McpTransportKind,
    };

    use super::*;

    fn stdio_entry(id: &str, command: &str, enabled: bool) -> McpServerEntry {
        McpServerEntry {
            id: id.into(),
            kind: McpTransportKind::Stdio,
            label: format!("{id} label"),
            transport_config: McpTransportConfig::Stdio {
                command: command.into(),
                args: Vec::new(),
                env: BTreeMap::new(),
            },
            credential_env_var: "MCP_TOKEN".into(),
            enabled,
        }
    }

    fn sse_entry(id: &str, url: &str) -> McpServerEntry {
        McpServerEntry {
            id: id.into(),
            kind: McpTransportKind::Sse,
            label: format!("{id} label"),
            transport_config: McpTransportConfig::Sse {
                endpoint_url: url.into(),
            },
            credential_env_var: "MCP_BEARER".into(),
            enabled: true,
        }
    }

    /// `hash_inputs_for` returns one redaction-safe input per enabled server
    /// present in the registry, in the order of the `enabled` list. The input
    /// carries the server id, transport kind, endpoint/command, and the
    /// credential env-var *name* — never the credential value.
    #[test]
    fn hash_inputs_for_returns_redaction_safe_inputs() {
        let registry = McpServerRegistry {
            version: "1".into(),
            servers: vec![
                stdio_entry("local-fs", "/usr/local/bin/mcp-server-fs", true),
                sse_entry("remote-sse", "https://mcp.example.com/sse"),
            ],
        };
        let client = McpToolRegistry::from_registry(registry);
        let inputs = client.hash_inputs_for(&[
            "local-fs".to_string(),
            "remote-sse".to_string(),
            "absent-server".to_string(),
        ]);
        // The absent server is skipped (it would surface as
        // `mcp_unknown_server` at invoke time; the hash only covers servers
        // that exist + are enabled in the registry).
        assert_eq!(inputs.len(), 2, "absent server must be skipped");
        // stdio input: basename only (no absolute user path leak), env-var name.
        assert_eq!(
            inputs[0], "local-fs|stdio|mcp-server-fs|MCP_TOKEN",
            "stdio input must use the command basename + env-var name"
        );
        // SSE input: endpoint_url + env-var name.
        assert_eq!(
            inputs[1], "remote-sse|sse|https://mcp.example.com/sse|MCP_BEARER",
            "sse input must use the endpoint_url + env-var name"
        );
    }

    /// A disabled server in the registry is skipped from the hash inputs
    /// (it cannot be invoked, so it carries no reproducibility identity).
    #[test]
    fn hash_inputs_for_skips_disabled_servers() {
        let registry = McpServerRegistry {
            version: "1".into(),
            servers: vec![
                stdio_entry("enabled-srv", "mcp-enabled", true),
                stdio_entry("disabled-srv", "mcp-disabled", false),
            ],
        };
        let client = McpToolRegistry::from_registry(registry);
        let inputs =
            client.hash_inputs_for(&["enabled-srv".to_string(), "disabled-srv".to_string()]);
        assert_eq!(inputs.len(), 1, "disabled server must be skipped");
        assert!(inputs[0].starts_with("enabled-srv|"));
    }

    /// The hash input must never contain a credential value, only the env-var
    /// *name*. A secret marker in the credential value would be a config
    /// error, but the hash input only carries the name regardless.
    #[test]
    fn hash_inputs_for_never_carries_credential_value() {
        let entry = McpServerEntry {
            id: "leak-test".into(),
            kind: McpTransportKind::Http,
            label: "leak test".into(),
            transport_config: McpTransportConfig::Http {
                endpoint_url: "https://mcp.example.com/http".into(),
            },
            // The *name* is `MCP_KEY`; the *value* (which a real caller
            // would inject via the env var) is never serialized into the
            // registry or the hash input.
            credential_env_var: "MCP_KEY".into(),
            enabled: true,
        };
        let input = hash_input_for_entry(&entry);
        assert!(
            input.contains("MCP_KEY"),
            "hash input must carry the env-var name"
        );
        // A hypothetical credential value must not appear — the hash input
        // is constructed only from the name field, never the value.
        assert!(
            !input.contains("sk-secret-value"),
            "hash input must not carry the credential value"
        );
        assert!(
            !plotforge_schema::contains_secret_marker_text(&input),
            "hash input must be free of secret markers"
        );
    }
}
