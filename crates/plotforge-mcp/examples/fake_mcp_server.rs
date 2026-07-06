//! A minimal fake MCP server for integration testing the stdio transport.
//!
//! Reads newline-delimited JSON-RPC requests from stdin, writes responses to
//! stdout. Implements just enough of the MCP protocol to exercise
//! `StdioMcpClient`'s `initialize` / `tools/list` / `tools/call` round-trip:
//!
//! - `initialize` → returns `{serverInfo: {name, version}, protocolVersion}`
//! - `notifications/initialized` → no response (notification)
//! - `tools/list` → returns one canned tool `ping`
//! - `tools/call` with `name == "ping"` → returns `{content: [{type:"text", text:"pong"}]}`
//! - `tools/call` with `name == "echo"` → returns the arguments as text
//! - any other method → returns a JSON-RPC error
//!
//! NOT a production server — kept deliberately tiny so the integration test
//! can assert exact wire shapes. Compiled as an example binary so the
//! integration test can spawn it via `env!("CARGO_BIN_EXE_fake_mcp_server")`
//! (note: examples use a different env var; the test resolves the path via
//! `std::env::var("CARGO_BIN_EXE_...")` is not available for examples —
//! instead the test uses `cargo`'s `CARGO_CRATE_MANIFEST_DIR` + relative path
//! to the example binary in `target/debug/examples/`).
//!
//! Run manually: `cargo run --example fake_mcp_server`

use std::io::{BufRead, BufReader, Write};

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut writer = std::io::BufWriter::new(stdout.lock());
    for line in BufReader::new(stdin.lock()).lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        // Notifications (no `id`) get no response.
        let id = match id {
            Some(id) => id,
            None => continue,
        };
        let response = match method {
            "initialize" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2025-06-18",
                    "serverInfo": {
                        "name": "fake-mcp-server",
                        "version": "0.1.0"
                    }
                }
            }),
            "tools/list" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "ping",
                            "description": "Returns pong.",
                            "input_schema": {"type": "object", "properties": {}}
                        },
                        {
                            "name": "echo",
                            "description": "Echoes the arguments as text.",
                            "input_schema": {"type": "object", "properties": {"text": {"type": "string"}}}
                        }
                    ]
                }
            }),
            "tools/call" => {
                let tool_name = request
                    .pointer("/params/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                match tool_name {
                    "ping" => serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{"type": "text", "text": "pong"}],
                            "isError": false
                        }
                    }),
                    "echo" => {
                        let text = request
                            .pointer("/params/arguments/text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{"type": "text", "text": text}],
                                "isError": false
                            }
                        })
                    }
                    _ => serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32601, "message": "unknown tool"}
                    }),
                }
            }
            _ => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "unknown method"}
            }),
        };
        let line = serde_json::to_string(&response).expect("serialize response");
        writer
            .write_all(line.as_bytes())
            .and_then(|_| writer.write_all(b"\n"))
            .and_then(|_| writer.flush())
            .expect("write response");
    }
}
