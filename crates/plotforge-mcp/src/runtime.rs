//! The async runtime for `plotforge-mcp`, scoped to this crate only.
//!
//! `plotforge-agent` and all other workspace crates must not pull `tokio` as
//! a direct dependency (AGENTS.md MCP carve-out). The SSE/HTTP transports
//! (Phase 3) use `tokio` + async `reqwest` internally but expose a blocking
//! façade via `runtime().block_on(...)` so callers never see `async`.
//!
//! The runtime is a process-wide `OnceLock<tokio::runtime::Runtime>` with a
//! multi-thread scheduler (SSE streaming needs concurrent read + heartbeat).

use std::sync::OnceLock;

use tokio::runtime::Runtime;

static MCP_RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Returns the process-wide `tokio::runtime::Runtime` owned by `plotforge-mcp`.
/// Initializes on first call. SSE/HTTP transports call `runtime().block_on(...)`
/// to run async methods from their synchronous `McpTransport` impls.
pub fn runtime() -> &'static Runtime {
    MCP_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build plotforge-mcp tokio runtime")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_initializes_once_and_runs_async() {
        let rt1 = runtime();
        let rt2 = runtime();
        // Same instance (OnceLock).
        assert!(std::ptr::eq(rt1, rt2));
        // Can run an async block.
        let value = rt1.block_on(async { 42 });
        assert_eq!(value, 42);
    }
}
