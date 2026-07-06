//! The async runtime for `plotforge-mcp`, scoped to this crate only.
//!
//! `plotforge-agent` and all other workspace crates must not pull `tokio` as
//! a direct dependency (AGENTS.md MCP carve-out). The SSE/HTTP transports
//! (Phase 3) use `tokio` + async `reqwest` internally but expose a blocking
//! façade via [`block_on_async`] so callers never see `async`.
//!
//! The runtime is a process-wide `OnceLock<tokio::runtime::Runtime>` with a
//! multi-thread scheduler (SSE streaming needs concurrent read + heartbeat).
//!
//! ## Async/blocking bridge (panic-free)
//!
//! The transports must be callable from any thread, including one that is
//! already inside a tokio runtime context (e.g. an async Tauri command, or a
//! thread where `reqwest::blocking` has touched the runtime). Calling
//! `Runtime::block_on` from inside a runtime panics with *"Cannot start a
//! runtime from within a runtime."*
//!
//! [`block_on_async`] avoids the panic by running the future on a dedicated
//! helper thread that is *not* a tokio worker thread. The helper thread calls
//! `block_on` on the runtime normally; the caller's thread blocks on a
//! `mpsc::Receiver` until the result arrives. This works regardless of
//! whether the caller is inside or outside a runtime, and the result/error is
//! propagated (a task panic surfaces as `Err` via the channel, not as a
//! cross-thread unwind).

use std::sync::OnceLock;

use tokio::runtime::Runtime;

static MCP_RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Returns the process-wide `tokio::runtime::Runtime` owned by `plotforge-mcp`.
/// Initializes on first call. SSE/HTTP transports use [`block_on_async`] to run
/// async methods from their synchronous `McpTransport` impls.
pub fn runtime() -> &'static Runtime {
    MCP_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to build plotforge-mcp tokio runtime")
    })
}

/// Run `future` on the plotforge-mcp runtime and block the calling thread
/// until it yields a result.
///
/// Panic-free from any calling context: the future runs on the runtime from a
/// dedicated helper thread that is not a tokio worker, so `block_on` never
/// sees the "runtime within runtime" panic even when the caller is itself
/// inside an async runtime. The helper thread catches panics from the future
/// and propagates them as an `Err` on the channel, so a panicking task never
/// unwinds across threads.
pub fn block_on_async<T: Send + 'static>(
    future: impl std::future::Future<Output = T> + Send + 'static,
) -> T {
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("plotforge-mcp-block-on".into())
        .spawn(move || {
            // `catch_unwind` keeps a panicking future from tearing down the
            // helper thread; the panic message is forwarded as `Err`.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime().block_on(future)
            }));
            // `send` fails only if the caller went away (dropped `rx`); ignore.
            let _ = tx.send(result);
        })
        .expect("failed to spawn plotforge-mcp block_on helper thread");
    match rx.recv() {
        Ok(Ok(value)) => value,
        Ok(Err(payload)) => {
            std::panic::resume_unwind(payload);
        }
        Err(_) => {
            panic!("plotforge-mcp block_on helper thread exited without sending a result");
        }
    }
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

    #[test]
    fn block_on_async_runs_future_from_outside_runtime() {
        let value = block_on_async(async { 123 });
        assert_eq!(value, 123);
    }

    /// C1 regression: calling `block_on_async` from inside a tokio runtime
    /// context must NOT panic with "Cannot start a runtime from within a
    /// runtime". The helper-thread bridge keeps the calling thread's runtime
    /// context out of the `block_on` call.
    #[test]
    fn block_on_async_does_not_panic_from_inside_runtime() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build test runtime");
        rt.block_on(async {
            let value = block_on_async(async { 999 });
            assert_eq!(value, 999);
        });
    }

    /// C1 regression: a panicking future surfaces as a resumed panic on the
    /// caller's thread (not swallowed, not a cross-thread unwind).
    #[test]
    #[should_panic(expected = "boom-from-future")]
    fn block_on_async_propagates_future_panic() {
        block_on_async(async {
            panic!("boom-from-future");
        });
    }
}
