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
