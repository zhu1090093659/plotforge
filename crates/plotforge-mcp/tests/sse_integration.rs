//! Integration tests for the SSE MCP transport against an in-process
//! `TcpListener` mock server.
//!
//! The mock server speaks a minimal SSE response sequence: receives a POST
//! with a JSON-RPC body, responds with `text/event-stream` containing a
//! single `data:` frame carrying the matching JSON-RPC response. No real
//! network — binds to `127.0.0.1:0` (per AGENTS.md:149 "no hidden network
//! calls in tests").

use std::io::{Read, Write};
use std::net::TcpListener;

use plotforge_mcp::{McpTransport, SseMcpClient};

/// Spawn an in-process TCP mock server that handles one MCP SSE request per
/// connection. Returns the bound `127.0.0.1:{port}` URL. The server runs on a
/// background thread; it exits when the listener is dropped (after the test).
fn spawn_sse_mock(port_holder: &mut Option<u16>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let port = listener.local_addr().expect("local addr").port();
    *port_holder = Some(port);
    std::thread::spawn(move || {
        // Handle up to 16 sequential connections (each test makes a few requests,
        // each request is a separate connection).
        for stream in listener.incoming().take(16) {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            // Read until we see the body (after \r\n\r\n) — give the client time
            // to send the full request. Read in a loop with a small buffer.
            let mut buf = Vec::with_capacity(8192);
            let mut chunk = [0u8; 1024];
            loop {
                match stream.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf.extend_from_slice(&chunk[..n]);
                        if buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() > 4 {
                            // Have headers; check if body is present (Content-Length or just read more).
                            // For simplicity, read once more to catch the body if not yet in buf.
                            let _ =
                                stream.set_read_timeout(Some(std::time::Duration::from_millis(50)));
                            match stream.read(&mut chunk) {
                                Ok(n) if n > 0 => buf.extend_from_slice(&chunk[..n]),
                                _ => {}
                            }
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let request_text = String::from_utf8_lossy(&buf);
            // Extract the JSON-RPC body (after the blank line).
            let body = request_text.split("\r\n\r\n").nth(1).unwrap_or("");
            let request: serde_json::Value = match serde_json::from_str(body.trim()) {
                Ok(v) => v,
                Err(_) => {
                    // Malformed request — respond with an error so the client
                    // surfaces it explicitly rather than hanging.
                    let err = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 0,
                        "error": {"code": -32700, "message": "parse error"}
                    });
                    let response_line = format!("data: {err}\n\n");
                    let http_response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{}",
                        response_line.len(),
                        response_line
                    );
                    let _ = stream.write_all(http_response.as_bytes());
                    let _ = stream.flush();
                    continue;
                }
            };
            let id = request
                .get("id")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let response_json = match method {
                "initialize" => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2025-06-18",
                        "serverInfo": {"name": "mock-sse", "version": "0.1.0"}
                    }
                }),
                "tools/list" => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [{
                            "name": "ping",
                            "description": "Returns pong.",
                            "input_schema": {"type": "object", "properties": {}}
                        }]
                    }
                }),
                "tools/call" => {
                    let tool_name = request
                        .pointer("/params/name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if tool_name == "ping" {
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{"type": "text", "text": "pong"}],
                                "isError": false
                            }
                        })
                    } else {
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {"code": -32601, "message": "unknown tool"}
                        })
                    }
                }
                _ => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {"code": -32601, "message": "unknown method"}
                }),
            };
            let response_line = format!("data: {}\n\n", response_json);
            let http_response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_line.len(),
                response_line
            );
            let _ = stream.write_all(http_response.as_bytes());
            let _ = stream.flush();
        }
    });
    // Give the mock server thread a moment to enter the `incoming()` loop.
    std::thread::sleep(std::time::Duration::from_millis(50));
}

fn sse_client_for_mock(port: u16) -> SseMcpClient {
    let url = format!("http://127.0.0.1:{port}/sse");
    SseMcpClient::new(&url, "").expect("construct SSE client")
}

#[test]
fn sse_initialize_handshake_round_trips() {
    let mut port = None;
    spawn_sse_mock(&mut port);
    let port = port.expect("mock server port");
    let mut client = sse_client_for_mock(port);
    let info = client.initialize().expect("initialize");
    assert_eq!(info.name, "mock-sse");
    assert_eq!(info.protocol_version, "2025-06-18");
    client.close();
}

#[test]
fn sse_list_tools_returns_canned_manifest() {
    let mut port = None;
    spawn_sse_mock(&mut port);
    let port = port.expect("mock server port");
    let mut client = sse_client_for_mock(port);
    client.initialize().expect("initialize");
    let tools = client.list_tools().expect("list_tools");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "ping");
    client.close();
}

#[test]
fn sse_call_tool_ping_returns_pong() {
    let mut port = None;
    spawn_sse_mock(&mut port);
    let port = port.expect("mock server port");
    let mut client = sse_client_for_mock(port);
    client.initialize().expect("initialize");
    let result = client
        .call_tool("ping", serde_json::json!({}))
        .expect("call ping");
    assert!(result.ok);
    assert!(!result.is_error);
    match &result.content[0] {
        plotforge_schema::McpToolContentBlock::Text { text } => assert_eq!(text, "pong"),
        other => panic!("expected text block, got {other:?}"),
    }
    client.close();
}

#[test]
fn sse_call_tool_unknown_returns_tool_error() {
    let mut port = None;
    spawn_sse_mock(&mut port);
    let port = port.expect("mock server port");
    let mut client = sse_client_for_mock(port);
    client.initialize().expect("initialize");
    let error = client
        .call_tool("nonexistent", serde_json::json!({}))
        .expect_err("unknown tool should error");
    assert!(matches!(error, plotforge_mcp::McpError::ToolError { .. }));
    client.close();
}

#[test]
fn sse_call_before_initialize_returns_handshake_error() {
    let mut port = None;
    spawn_sse_mock(&mut port);
    let port = port.expect("mock server port");
    let mut client = sse_client_for_mock(port);
    let error = client
        .list_tools()
        .expect_err("list_tools before initialize should error");
    assert!(matches!(
        error,
        plotforge_mcp::McpError::HandshakeFailed { .. }
    ));
    client.close();
}
