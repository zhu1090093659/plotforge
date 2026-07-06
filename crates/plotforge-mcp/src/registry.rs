//! MCP server registry IO and transport construction.
//!
//! Mirrors `plotforge_agent::registry` (provider registry IO + `build_provider_client`)
//! for MCP servers. The registry lives at `~/.plotforge/mcp.json` (the third
//! member of the providers/skills/prompts family) and never enters project
//! source, contracts, traces, or export packages.
//!
//! ## Path-traversal + secret-marker guards
//!
//! stdio server `command`/`args` are a path-injection surface (a malicious
//! `command` could be `../escape`). `validate_stdio_command` rejects `..`,
//! NUL, and shell metacharacters; `validate_server_entry` scans all string
//! fields for secret markers before write. This mirrors `is_safe_skill_name`
//! + `import_external_skill_to` in `plotforge_agent::skills`.

use std::path::{Path, PathBuf};

use plotforge_schema::{
    McpServerEntry, McpServerRegistry, McpTransportConfig, McpTransportKind,
    contains_secret_marker_text,
};

use crate::{McpError, McpTransport};

/// The user-global PlotForge config directory. Resolved from `dirs::config_dir()`
/// with a `HOME` fallback so the registry works on macOS, Linux, and a minimal
/// shell that only sets `HOME`. Mirrors `plotforge_agent::registry::user_config_dir`.
pub fn user_config_dir() -> Option<PathBuf> {
    if let Some(dir) = dirs::config_dir() {
        return Some(dir.join("plotforge"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".plotforge"))
}

/// The absolute path to `~/.plotforge/mcp.json` (or the platform equivalent
/// under the config dir). Returns `None` only when neither `dirs::config_dir()`
/// nor `HOME` is available.
pub fn mcp_registry_path() -> Option<PathBuf> {
    user_config_dir().map(|dir| dir.join("mcp.json"))
}

/// Errors raised by MCP registry IO. Redaction-safe: no variant carries a raw
/// credential value or raw file content beyond the path string.
#[derive(Debug, thiserror::Error)]
pub enum McpRegistryError {
    #[error("could not resolve a user config directory (no HOME and no platform config dir)")]
    NoConfigDir,
    #[error("failed to read MCP registry at {path}: {source}")]
    ReadFailed {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse MCP registry at {path}: {source}")]
    ParseFailed {
        path: String,
        source: serde_json::Error,
    },
    #[error("failed to write MCP registry at {path}: {source}")]
    WriteFailed {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to serialize MCP registry: {source}")]
    SerializeFailed { source: serde_json::Error },
    #[error("MCP server entry failed validation: {message}")]
    ValidationFailed { message: String },
}

/// Loads the user-global MCP server registry. A missing file returns an empty
/// registry (not an error) so a fresh install keeps working with no MCP
/// servers. A corrupt or unreadable file returns an error so the user can
/// fix it; we never silently fall back to an empty registry from a file that
/// exists.
pub fn load_mcp_registry() -> Result<McpServerRegistry, McpRegistryError> {
    let Some(path) = mcp_registry_path() else {
        return Ok(McpServerRegistry::default());
    };
    load_mcp_registry_from(&path)
}

/// Same as `load_mcp_registry` but reads from an explicit `path`. Used by
/// tests to keep the user library hermetic and to exercise the real
/// parse/IO error paths.
pub fn load_mcp_registry_from(path: &Path) -> Result<McpServerRegistry, McpRegistryError> {
    if !path.exists() {
        return Ok(McpServerRegistry::default());
    }
    let content = std::fs::read_to_string(path).map_err(|error| McpRegistryError::ReadFailed {
        path: path.display().to_string(),
        source: error,
    })?;
    let registry: McpServerRegistry =
        serde_json::from_str(&content).map_err(|error| McpRegistryError::ParseFailed {
            path: path.display().to_string(),
            source: error,
        })?;
    Ok(registry)
}

/// Writes the user-global registry to `~/.plotforge/mcp.json`, creating the
/// config directory if needed. Validates each entry (path-traversal +
/// secret-marker scan) before write so a corrupt entry never persists.
pub fn write_mcp_registry(registry: &McpServerRegistry) -> Result<(), McpRegistryError> {
    let Some(path) = mcp_registry_path() else {
        return Err(McpRegistryError::NoConfigDir);
    };
    write_mcp_registry_to(&path, registry)
}

/// Same as `write_mcp_registry` but writes to an explicit `path`. Used by
/// tests to keep the user library hermetic.
pub fn write_mcp_registry_to(
    path: &Path,
    registry: &McpServerRegistry,
) -> Result<(), McpRegistryError> {
    for server in &registry.servers {
        validate_server_entry(server).map_err(|error| McpRegistryError::ValidationFailed {
            message: error.to_string(),
        })?;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| McpRegistryError::WriteFailed {
            path: parent.display().to_string(),
            source: error,
        })?;
    }
    let content = serde_json::to_string_pretty(registry)
        .map_err(|error| McpRegistryError::SerializeFailed { source: error })?;
    std::fs::write(path, content).map_err(|error| McpRegistryError::WriteFailed {
        path: path.display().to_string(),
        source: error,
    })?;
    Ok(())
}

/// Resolve a server entry by id from the registry. Returns `None` when the id
/// is absent (the caller surfaces this as `McpError::UnknownServer`).
pub fn resolve_mcp_server<'a>(
    server_id: &str,
    registry: &'a McpServerRegistry,
) -> Option<&'a McpServerEntry> {
    registry
        .servers
        .iter()
        .find(|server| server.id == server_id)
}

/// Builds a `Box<dyn McpTransport>` for a single registered server entry.
///
/// Dispatches on `McpTransportKind`:
/// - `Stdio` → `StdioMcpClient` (Phase 2, `transport_stdio.rs`)
/// - `Sse` / `Http` → `Err(McpError::TransportUnsupported)` until Phase 3
///   lands the async transports. This is an explicit error, never a silent
///   fallback to stdio.
///
/// Mirrors `plotforge_agent::registry::build_provider_client`.
pub fn build_mcp_client(entry: &McpServerEntry) -> Result<Box<dyn McpTransport>, McpError> {
    validate_server_entry(entry)?;
    match entry.kind {
        McpTransportKind::Stdio => {
            // P2.3: stdio transport is wired. Extract command/args/env from
            // the transport config and construct a `StdioMcpClient`.
            let McpTransportConfig::Stdio { command, args, env } = &entry.transport_config else {
                return Err(McpError::SpawnFailed {
                    command: "(config kind mismatch)".into(),
                });
            };
            let client = crate::StdioMcpClient::new(command, args, env)?;
            Ok(Box::new(client))
        }
        McpTransportKind::Sse => {
            let McpTransportConfig::Sse { endpoint_url } = &entry.transport_config else {
                return Err(McpError::Io {
                    detail: "config kind mismatch for sse transport".into(),
                });
            };
            let client = crate::SseMcpClient::new(endpoint_url, &entry.credential_env_var)?;
            Ok(Box::new(client))
        }
        McpTransportKind::Http => {
            let McpTransportConfig::Http { endpoint_url } = &entry.transport_config else {
                return Err(McpError::Io {
                    detail: "config kind mismatch for http transport".into(),
                });
            };
            let client = crate::HttpMcpClient::new(endpoint_url, &entry.credential_env_var)?;
            Ok(Box::new(client))
        }
    }
}

/// Validate a server entry before persisting or constructing a transport.
///
/// Checks:
/// 1. stdio `command` is safe (no `..`, no NUL, no shell metacharacters that
///    could enable injection — mirrors `is_safe_skill_name`).
/// 2. No string field contains a secret marker (mirrors `contains_secret_marker_text`
///    applied at `upsert_provider` time in `plotforge-studio`).
///
/// Returns `Ok(())` on success, `Err(McpError)` with a redaction-safe message
/// on validation failure.
pub fn validate_server_entry(entry: &McpServerEntry) -> Result<(), McpError> {
    // stdio command path-traversal + shell-injection guard.
    if let McpTransportConfig::Stdio { command, args, env } = &entry.transport_config {
        validate_stdio_command(command)?;
        for arg in args {
            validate_stdio_arg(arg)?;
        }
        for (key, value) in env {
            if contains_secret_marker_text(key) || contains_secret_marker_text(value) {
                return Err(McpError::Io {
                    detail: "stdio env contains a secret marker".into(),
                });
            }
        }
    }
    // Secret-marker scan on all string fields.
    if contains_secret_marker_text(&entry.id)
        || contains_secret_marker_text(&entry.label)
        || contains_secret_marker_text(&entry.credential_env_var)
    {
        return Err(McpError::Io {
            detail: "server entry contains a secret marker".into(),
        });
    }
    #[allow(clippy::collapsible_if)]
    if let McpTransportConfig::Sse { endpoint_url } | McpTransportConfig::Http { endpoint_url } =
        &entry.transport_config
    {
        if contains_secret_marker_text(endpoint_url) {
            return Err(McpError::Io {
                detail: "endpoint_url contains a secret marker".into(),
            });
        }
    }
    Ok(())
}

/// Reject `..`, NUL, and obvious shell metacharacters in a stdio command.
/// This is a defence-in-depth guard, not a full shell-safety analysis — the
/// spawned process uses `Command::new(command)` (no shell), so metacharacters
/// are not interpreted, but `..` could still escape a relative-path resolution.
fn validate_stdio_command(command: &str) -> Result<(), McpError> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(McpError::SpawnFailed {
            command: "(empty)".into(),
        });
    }
    if trimmed.contains('\0') || trimmed.contains("..") {
        return Err(McpError::SpawnFailed {
            command: redact_command_name(trimmed),
        });
    }
    Ok(())
}

/// Reject NUL in stdio args. Args are passed verbatim to the subprocess (no
/// shell), so metacharacters are safe, but NUL would truncate the arg.
fn validate_stdio_arg(arg: &str) -> Result<(), McpError> {
    if arg.contains('\0') {
        return Err(McpError::Io {
            detail: "stdio arg contains NUL".into(),
        });
    }
    Ok(())
}

/// Return a redacted command name for error messages: keep only the basename
/// (last path segment) so a full path is not leaked, and never include args.
fn redact_command_name(command: &str) -> String {
    let trimmed = command.trim();
    match trimmed.rsplit(['/', '\\']).next() {
        Some(basename) if !basename.is_empty() => basename.to_string(),
        _ => "(redacted)".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

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
            credential_env_var: "MCP_AUTH_KEY".into(),
            enabled: true,
        }
    }

    #[test]
    fn mcp_registry_roundtrips_through_tempdir() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("mcp.json");
        let registry = McpServerRegistry {
            version: "1".into(),
            servers: vec![sample_stdio_entry(), sample_sse_entry()],
        };
        write_mcp_registry_to(&path, &registry).expect("write registry");
        assert!(path.exists(), "registry file should exist after write");
        let loaded = load_mcp_registry_from(&path).expect("load registry");
        assert_eq!(loaded, registry);
        assert_eq!(loaded.servers.len(), 2);
    }

    #[test]
    fn mcp_registry_missing_file_returns_empty() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("nonexistent-mcp.json");
        let loaded =
            load_mcp_registry_from(&path).expect("missing file should be empty, not error");
        assert!(loaded.servers.is_empty());
        assert!(loaded.version.is_empty());
    }

    #[test]
    fn mcp_registry_corrupt_file_returns_parse_error() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("corrupt-mcp.json");
        std::fs::write(&path, "{ not valid json").expect("write corrupt file");
        let error = load_mcp_registry_from(&path).expect_err("corrupt file should error");
        assert!(matches!(error, McpRegistryError::ParseFailed { .. }));
    }

    #[test]
    fn mcp_registry_create_parent_dir_on_write() {
        let dir = tempdir().expect("temp dir");
        let nested = dir.path().join("nested").join("sub").join("mcp.json");
        let registry = McpServerRegistry::default();
        write_mcp_registry_to(&nested, &registry).expect("write creates parent dirs");
        assert!(nested.exists());
    }

    #[test]
    fn resolve_mcp_server_finds_entry_by_id() {
        let registry = McpServerRegistry {
            version: "1".into(),
            servers: vec![sample_stdio_entry(), sample_sse_entry()],
        };
        let found = resolve_mcp_server("remote-sse", &registry).expect("server should be found");
        assert_eq!(found.kind, McpTransportKind::Sse);
        assert!(resolve_mcp_server("nonexistent", &registry).is_none());
    }

    #[test]
    #[allow(clippy::err_expect)] // Box<dyn McpTransport> is not Debug
    fn build_mcp_client_stdio_returns_spawn_failed_for_missing_command() {
        // P2.3: stdio transport is wired — a non-existent command surfaces an
        // explicit `mcp_spawn_failed` (no silent fallback).
        let entry = McpServerEntry {
            id: "missing-binary".into(),
            kind: McpTransportKind::Stdio,
            label: "Missing binary".into(),
            transport_config: McpTransportConfig::Stdio {
                command: "this-binary-does-not-exist-12345".into(),
                args: vec![],
                env: BTreeMap::new(),
            },
            credential_env_var: String::new(),
            enabled: true,
        };
        let error = build_mcp_client(&entry)
            .err()
            .expect("missing binary should fail to spawn");
        assert!(matches!(error, McpError::SpawnFailed { .. }));
    }

    #[test]
    fn build_mcp_client_sse_http_construct_for_clean_urls() {
        // P3.4: SSE/HTTP transports are wired — clean URLs construct successfully.
        // (The transport is created but NOT connected; no network call until
        // initialize/list_tools/call_tool is invoked.)
        let sse_entry = sample_sse_entry();
        let _transport = build_mcp_client(&sse_entry).expect("sse client constructs for clean URL");

        let http_entry = McpServerEntry {
            id: "remote-http".into(),
            kind: McpTransportKind::Http,
            label: "Remote HTTP MCP".into(),
            transport_config: McpTransportConfig::Http {
                endpoint_url: "https://mcp.example.com/rpc".into(),
            },
            credential_env_var: String::new(),
            enabled: true,
        };
        let _transport =
            build_mcp_client(&http_entry).expect("http client constructs for clean URL");
    }

    #[test]
    fn validate_server_entry_rejects_path_traversal_command() {
        let entry = McpServerEntry {
            id: "evil".into(),
            kind: McpTransportKind::Stdio,
            label: "Evil".into(),
            transport_config: McpTransportConfig::Stdio {
                command: "../../../bin/escape".into(),
                args: vec![],
                env: BTreeMap::new(),
            },
            credential_env_var: String::new(),
            enabled: true,
        };
        let error = validate_server_entry(&entry).expect_err("path traversal should be rejected");
        assert!(matches!(error, McpError::SpawnFailed { .. }));
    }

    #[test]
    fn validate_server_entry_rejects_empty_command() {
        let entry = McpServerEntry {
            id: "empty".into(),
            kind: McpTransportKind::Stdio,
            label: "Empty".into(),
            transport_config: McpTransportConfig::Stdio {
                command: "   ".into(),
                args: vec![],
                env: BTreeMap::new(),
            },
            credential_env_var: String::new(),
            enabled: true,
        };
        let error = validate_server_entry(&entry).expect_err("empty command should be rejected");
        assert!(matches!(error, McpError::SpawnFailed { .. }));
    }

    #[test]
    fn validate_server_entry_rejects_nul_in_command() {
        let entry = McpServerEntry {
            id: "nul".into(),
            kind: McpTransportKind::Stdio,
            label: "NUL".into(),
            transport_config: McpTransportConfig::Stdio {
                command: "mcp-server\0evil".into(),
                args: vec![],
                env: BTreeMap::new(),
            },
            credential_env_var: String::new(),
            enabled: true,
        };
        let error = validate_server_entry(&entry).expect_err("NUL in command should be rejected");
        assert!(matches!(error, McpError::SpawnFailed { .. }));
    }

    #[test]
    fn validate_server_entry_rejects_secret_marker_in_id() {
        // `contains_secret_marker_text` catches `sk-` at the start of a
        // whitespace-delimited token; a standalone `sk-...` id is flagged.
        let entry = McpServerEntry {
            id: "sk-test-secret-marker".into(),
            kind: McpTransportKind::Stdio,
            label: "Evil".into(),
            transport_config: McpTransportConfig::Stdio {
                command: "mcp-server".into(),
                args: vec![],
                env: BTreeMap::new(),
            },
            credential_env_var: String::new(),
            enabled: true,
        };
        let error =
            validate_server_entry(&entry).expect_err("secret marker in id should be rejected");
        assert!(matches!(error, McpError::Io { .. }));
    }

    #[test]
    fn validate_server_entry_rejects_secret_marker_in_endpoint_url() {
        let entry = McpServerEntry {
            id: "remote".into(),
            kind: McpTransportKind::Sse,
            label: "Remote".into(),
            transport_config: McpTransportConfig::Sse {
                endpoint_url: "https://example.com/sse?token=sk-test-secret-marker".into(),
            },
            credential_env_var: String::new(),
            enabled: true,
        };
        let error =
            validate_server_entry(&entry).expect_err("secret marker in URL should be rejected");
        assert!(matches!(error, McpError::Io { .. }));
    }

    #[test]
    fn validate_server_entry_rejects_secret_marker_in_env_value() {
        let mut env = BTreeMap::new();
        env.insert("API_TOKEN".into(), "Bearer sk-test-secret-marker".into());
        let entry = McpServerEntry {
            id: "env-evil".into(),
            kind: McpTransportKind::Stdio,
            label: "Env Evil".into(),
            transport_config: McpTransportConfig::Stdio {
                command: "mcp-server".into(),
                args: vec![],
                env,
            },
            credential_env_var: String::new(),
            enabled: true,
        };
        let error = validate_server_entry(&entry)
            .expect_err("secret marker in env value should be rejected");
        assert!(matches!(error, McpError::Io { .. }));
    }

    #[test]
    fn validate_server_entry_accepts_clean_entries() {
        validate_server_entry(&sample_stdio_entry()).expect("clean stdio entry should validate");
        validate_server_entry(&sample_sse_entry()).expect("clean sse entry should validate");
    }

    #[test]
    fn redact_command_name_keeps_basename_only() {
        assert_eq!(
            redact_command_name("/usr/local/bin/mcp-server"),
            "mcp-server"
        );
        assert_eq!(redact_command_name("mcp-server-fs"), "mcp-server-fs");
        assert_eq!(redact_command_name("  /path/to/binary  "), "binary");
    }
}
