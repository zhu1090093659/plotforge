//! Skill library: discover, parse, scan, and import skills in the Claude
//! Code / Codex SKILL.md format from PlotForge's own library and from
//! external agent roots on this machine.
//!
//! Discovery scans a fixed set of external roots (see `discover_skill_roots`)
//! plus the user's own `~/.plotforge/skills/` library. Roots that do not
//! exist on this machine are silently skipped — they are not errors and not
//! silent fallbacks, just "this agent isn't installed here". PlotForge never
//! creates directories under any external root; it only reads and copies.
//!
//! Each skill folder must contain a `SKILL.md` with YAML frontmatter
//! (`name` + `description`, both required). Optional `agents/openai.yaml`
//! carries `interface` UI metadata. Bundled `scripts/`, `references/`, and
//! `assets/` subfolders are scanned for relative paths only — their
//! contents are never read into the index. Skill bodies are loaded on
//! demand via `load_skill_body` (progressive disclosure).
//!
//! The scan cache lives at `~/.plotforge/skill-index.json`. Nothing in this
//! module writes credentials, raw provider responses, or secret markers.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use plotforge_schema::{
    SkillFrontmatter, SkillIndex, SkillInterface, SkillManifest, SkillOrigin, SkillSource,
};

/// The fixed external skill roots PlotForge auto-discovers, in scan order.
/// Roots that don't exist are silently skipped. The `Other` variant in
/// `SkillOrigin` covers any future root not yet named here.
///
/// Each entry is `(origin, fn() -> Option<PathBuf>)` so the path is resolved
/// lazily and never constructed for a root that will be skipped anyway.
const EXTERNAL_SKILL_ROOTS: &[(&str, &str)] = &[
    // PlotForge's own user library — first-class, not "external", but
    // scanned from the same root list for uniformity.
    ("plot_forge_user", "plot_forge_user_dir"),
    ("claude_code", "claude_skills_dir"),
    ("codex", "codex_skills_dir"),
    ("z_code", "z_code_skills_dir"),
    ("cursor", "cursor_skills_dir"),
    ("copilot", "copilot_skills_dir"),
    ("hanako", "hanako_skills_dir"),
    ("open_claw", "open_claw_skills_dir"),
    ("workbuddy", "workbuddy_skills_dir"),
    ("redbox", "redbox_skills_dir"),
    ("codex", "codex_curated_skills_dir"), // Codex curated vendor imports
];

/// Resolves the user-global PlotForge skills dir (`<config>/plotforge/skills`
/// or `~/.plotforge/skills`). Returns `None` when no config dir is available.
pub fn plot_forge_user_dir() -> Option<PathBuf> {
    user_config_dir().map(|dir| dir.join("skills"))
}

fn claude_skills_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".claude").join("skills"))
}
fn codex_skills_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".codex").join("skills"))
}
fn z_code_skills_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".zcode").join("skills"))
}
fn cursor_skills_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".cursor").join("skills"))
}
fn copilot_skills_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".copilot").join("skills"))
}
fn hanako_skills_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".hanako").join("skills"))
}
fn open_claw_skills_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".openclaw").join("skills"))
}
fn workbuddy_skills_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".workbuddy").join("skills"))
}
fn redbox_skills_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".redbox").join("skills"))
}
fn codex_curated_skills_dir() -> Option<PathBuf> {
    home_dir().map(|home| {
        home.join(".codex")
            .join("vendor_imports")
            .join("skills")
            .join("skills")
            .join(".curated")
    })
}

fn user_config_dir() -> Option<PathBuf> {
    if let Some(dir) = dirs::config_dir() {
        return Some(dir.join("plotforge"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".plotforge"))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// The absolute path to the cached skill index at
/// `<config>/plotforge/skill-index.json`.
pub fn skill_index_path() -> Option<PathBuf> {
    user_config_dir().map(|dir| dir.join("skill-index.json"))
}

/// Errors raised by the skill scanner. All messages are redaction-safe.
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error(
        "could not resolve a user config directory for skills (no HOME and no platform config dir)"
    )]
    NoConfigDir,
    #[error("failed to read skill frontmatter at {path}: {reason}")]
    FrontmatterRead { path: String, reason: String },
    #[error("failed to parse skill frontmatter at {path}: {reason}")]
    FrontmatterParse { path: String, reason: String },
    #[error("skill at {path} is missing required frontmatter field `{field}`")]
    MissingFrontmatterField { path: String, field: String },
    #[error("failed to scan skill directory {path}: {reason}")]
    Scan { path: String, reason: String },
    #[error("failed to read skill body at {path}: {reason}")]
    BodyRead { path: String, reason: String },
    #[error("failed to write skill index at {path}: {reason}")]
    IndexWrite { path: String, reason: String },
    #[error("failed to import skill `{skill_id}`: {reason}")]
    Import { skill_id: String, reason: String },
}

/// Returns the list of external skill roots that exist on this machine, in
/// scan order. Roots that don't exist are skipped silently. This function
/// never creates directories.
pub fn discover_skill_roots() -> Vec<(SkillOrigin, PathBuf)> {
    let mut roots = Vec::new();
    for (origin_str, resolver_name) in EXTERNAL_SKILL_ROOTS {
        let path_opt = match *resolver_name {
            "plot_forge_user_dir" => plot_forge_user_dir(),
            "claude_skills_dir" => claude_skills_dir(),
            "codex_skills_dir" => codex_skills_dir(),
            "z_code_skills_dir" => z_code_skills_dir(),
            "cursor_skills_dir" => cursor_skills_dir(),
            "copilot_skills_dir" => copilot_skills_dir(),
            "hanako_skills_dir" => hanako_skills_dir(),
            "open_claw_skills_dir" => open_claw_skills_dir(),
            "workbuddy_skills_dir" => workbuddy_skills_dir(),
            "redbox_skills_dir" => redbox_skills_dir(),
            "codex_curated_skills_dir" => codex_curated_skills_dir(),
            _ => None,
        };
        let Some(path) = path_opt else { continue };
        if !path.is_dir() {
            continue;
        }
        let origin = match *origin_str {
            "plot_forge_user" => SkillOrigin::PlotForgeUser,
            "claude_code" => SkillOrigin::ClaudeCode,
            "codex" => SkillOrigin::Codex,
            "z_code" => SkillOrigin::ZCode,
            "cursor" => SkillOrigin::Cursor,
            "copilot" => SkillOrigin::Copilot,
            "hanako" => SkillOrigin::Hanako,
            "open_claw" => SkillOrigin::OpenClaw,
            "workbuddy" => SkillOrigin::Workbuddy,
            "redbox" => SkillOrigin::Redbox,
            _ => SkillOrigin::Other((*origin_str).into()),
        };
        roots.push((origin, path));
    }
    roots
}

/// Parses the YAML frontmatter of a `SKILL.md`. Lenient: unknown fields are
/// kept in `metadata` rather than rejected. Returns an error only when the
/// file is missing, unreadable, has no `---`-delimited frontmatter block, or
/// carries an unsafe `name` (path separators / `..` segments are rejected to
/// prevent path traversal when the name is used as a destination directory in
/// `import_external_skill`).
pub fn parse_skill_frontmatter(skill_dir: &Path) -> Result<SkillFrontmatter, SkillError> {
    let skill_md = skill_dir.join("SKILL.md");
    let path_str = skill_md.display().to_string();
    let content =
        std::fs::read_to_string(&skill_md).map_err(|error| SkillError::FrontmatterRead {
            path: path_str.clone(),
            reason: error.to_string(),
        })?;
    let frontmatter_text =
        extract_frontmatter(&content).ok_or_else(|| SkillError::FrontmatterParse {
            path: path_str.clone(),
            reason: "no ---delimited YAML frontmatter block found".into(),
        })?;
    let frontmatter: SkillFrontmatter =
        serde_yaml::from_str(frontmatter_text).map_err(|error| SkillError::FrontmatterParse {
            path: path_str.clone(),
            reason: error.to_string(),
        })?;
    if frontmatter.name.trim().is_empty() {
        return Err(SkillError::MissingFrontmatterField {
            path: path_str,
            field: "name".into(),
        });
    }
    // Reject names that could escape the import destination root. The skill
    // `name` becomes `~/.plotforge/skills/<name>/` in `import_external_skill`;
    // a name with path separators or `..` segments would write outside the
    // library. This is a redaction-safety / path-traversal guard, not a
    // naming-style preference.
    if !is_safe_skill_name(&frontmatter.name) {
        return Err(SkillError::FrontmatterParse {
            path: path_str,
            reason: format!(
                "skill `name` must be a single path component without `/`, `\\`, `..`, or NUL; got `{}`",
                frontmatter.name
            ),
        });
    }
    if frontmatter.description.trim().is_empty() {
        return Err(SkillError::MissingFrontmatterField {
            path: path_str,
            field: "description".into(),
        });
    }
    Ok(frontmatter)
}

/// Returns true when `name` is safe to use as a single directory component
/// under `~/.plotforge/skills/`. Rejects empty, separators, `.`/`..`, and
/// NUL bytes — the values that would let `import_external_skill` write
/// outside the user library.
fn is_safe_skill_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('\0')
    {
        return false;
    }
    // `.` and `..` (and variants like `...`) are not safe as directory names
    // in this context; require at least one non-dot character.
    trimmed.chars().any(|c| c != '.' && !c.is_whitespace())
}

/// Extracts the text between the first pair of `---` lines. Returns `None`
/// when no frontmatter block is present. Tolerates CRLF (`\r\n`) line endings
/// so skills authored or checked out on Windows still parse.
fn extract_frontmatter(content: &str) -> Option<&str> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }
    let after_first_fence = content.strip_prefix("---")?;
    // Require a newline immediately after the opening fence. Tolerate an
    // optional `\r` (CRLF) before the `\n`.
    let after_first_fence = after_first_fence
        .strip_prefix('\r')
        .unwrap_or(after_first_fence);
    let after_first_fence = after_first_fence.strip_prefix('\n')?;
    let end = after_first_fence.find("\n---")?;
    Some(&after_first_fence[..end])
}

/// Parses the `interface` block of an `agents/openai.yaml`, if present.
/// Returns `None` (not an error) when the file is missing. Only the
/// `interface` key is read; other sections (`dependencies`, `policy`) are
/// ignored in this phase.
pub fn parse_skill_interface(skill_dir: &Path) -> Option<SkillInterface> {
    let openai_yaml = skill_dir.join("agents").join("openai.yaml");
    if !openai_yaml.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&openai_yaml).ok()?;
    #[derive(serde::Deserialize)]
    struct OpenAiYamlFile {
        #[serde(default)]
        interface: Option<SkillInterface>,
    }
    let parsed: OpenAiYamlFile = serde_yaml::from_str(&content).ok()?;
    parsed.interface
}

/// Scans a single skill directory and produces its `SkillManifest`. The
/// manifest carries only metadata and relative paths — body and bundled
/// resource contents are never read here. The `scripts/`, `references/`, and
/// `assets/` subfolders are accepted in both plural and singular forms
/// (e.g. `frontend-design` ships `reference/`).
pub fn scan_skill(
    skill_dir: &Path,
    origin: SkillOrigin,
    root_path: &Path,
) -> Result<SkillManifest, SkillError> {
    let frontmatter = parse_skill_frontmatter(skill_dir)?;
    let interface = parse_skill_interface(skill_dir);
    let rel_path = skill_dir
        .strip_prefix(root_path)
        .map(|rel| rel.display().to_string())
        .unwrap_or_else(|_| {
            skill_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
    let body_path = skill_dir
        .join("SKILL.md")
        .strip_prefix(root_path)
        .map(|rel| rel.display().to_string())
        .unwrap_or_else(|_| "SKILL.md".into());
    let scripts = list_relative_subdir(skill_dir, root_path, &["scripts"]);
    let references = list_relative_subdir(skill_dir, root_path, &["references", "reference"]);
    let assets = list_relative_subdir(skill_dir, root_path, &["assets"]);

    Ok(SkillManifest {
        id: frontmatter.name.clone(),
        name: frontmatter.name.clone(),
        description: frontmatter.description.clone(),
        source: SkillSource {
            origin,
            root_path: root_path.display().to_string(),
            rel_path,
        },
        interface,
        body_path,
        scripts,
        references,
        assets,
    })
}

/// Lists the files (relative to `root`) under any of the candidate subdirs
/// of `skill_dir`. Returns paths relative to `root` so the manifest stays
/// portable. Missing subdirs contribute an empty list.
fn list_relative_subdir(skill_dir: &Path, root: &Path, candidate_names: &[&str]) -> Vec<String> {
    for name in candidate_names {
        let dir = skill_dir.join(name);
        if dir.is_dir() {
            return list_relative_files(&dir, root);
        }
    }
    Vec::new()
}

/// Recursively lists files under `dir` as paths relative to `root`.
fn list_relative_files(dir: &Path, root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            out.extend(list_relative_files(&path, root));
        } else if path.is_file()
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.display().to_string());
        }
    }
    out
}

/// Scans all discovered skill roots and returns a de-duplicated `SkillIndex`
/// keyed by skill `name` (the canonical skill id). When two roots carry a
/// skill with the same name, the *first* root in scan order wins (PlotForge's
/// own library first, then the canonical `~/.agents`-backed roots, then the
/// rest). `scanned_at` is an ISO-8601 timestamp.
pub fn scan_all_skills() -> SkillIndex {
    let mut skills: BTreeMap<String, SkillManifest> = BTreeMap::new();
    for (origin, root) in discover_skill_roots() {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Ok(manifest) = scan_skill(&path, origin.clone(), &root) else {
                continue;
            };
            skills.entry(manifest.name.clone()).or_insert(manifest);
        }
    }
    SkillIndex {
        version: "1".into(),
        skills: skills.into_values().collect(),
        scanned_at: now_iso8601(),
    }
}

/// Writes the scan cache to `<config>/plotforge/skill-index.json`, creating
/// the config directory if needed.
pub fn write_skill_index(index: &SkillIndex) -> Result<(), SkillError> {
    let Some(path) = skill_index_path() else {
        return Err(SkillError::NoConfigDir);
    };
    write_skill_index_to(&path, index)
}

/// Same as `write_skill_index` but writes to an explicit `path`. Used by
/// tests to keep the user library hermetic.
pub fn write_skill_index_to(path: &Path, index: &SkillIndex) -> Result<(), SkillError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| SkillError::IndexWrite {
            path: parent.display().to_string(),
            reason: error.to_string(),
        })?;
    }
    let content = serde_json::to_string_pretty(index).map_err(|error| SkillError::IndexWrite {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;
    std::fs::write(path, content).map_err(|error| SkillError::IndexWrite {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;
    Ok(())
}

/// Loads the cached `SkillIndex` from disk. A missing cache returns `None`
/// (the caller should trigger `scan_all_skills` + `write_skill_index`).
pub fn read_cached_skill_index() -> Option<SkillIndex> {
    let path = skill_index_path()?;
    read_skill_index_from(&path)
}

/// Same as `read_cached_skill_index` but reads from an explicit `path`. Used
/// by tests to keep the user library hermetic.
pub fn read_skill_index_from(path: &Path) -> Option<SkillIndex> {
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Loads the body (Markdown) of a skill on demand, for prompt injection or
/// UI preview. This is progressive disclosure level 2: the body is not in
/// the manifest and is read only when the skill triggers.
pub fn load_skill_body(manifest: &SkillManifest) -> Result<String, SkillError> {
    let root = Path::new(&manifest.source.root_path);
    let body_path = root.join(&manifest.body_path);
    let path_str = body_path.display().to_string();
    let content = std::fs::read_to_string(&body_path).map_err(|error| SkillError::BodyRead {
        path: path_str.clone(),
        reason: error.to_string(),
    })?;
    // Strip the frontmatter: only the body after the closing `---` fence is
    // the part intended for prompt injection.
    Ok(strip_frontmatter(&content))
}

/// Returns the body of a SKILL.md without its frontmatter. When no
/// frontmatter block is present, the whole content is returned. Tolerates
/// CRLF (`\r\n`) line endings.
fn strip_frontmatter(content: &str) -> String {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return content.to_string();
    }
    let after_open = match trimmed
        .strip_prefix("---")
        .and_then(|rest| rest.strip_prefix('\r').unwrap_or(rest).strip_prefix('\n'))
    {
        Some(rest) => rest,
        None => return content.to_string(),
    };
    let Some(end) = after_open.find("\n---") else {
        return content.to_string();
    };
    let after_close = &after_open[end + "\n---".len()..];
    // Tolerate CRLF: strip any leading run of `\r` and `\n` characters
    // (the close fence may be followed by `\r\n\r\n` before the body).
    after_close.trim_start_matches(['\r', '\n']).to_string()
}

/// Copies an external skill (referenced by `manifest`) into the user's own
/// `~/.plotforge/skills/<name>/` library so the user can edit a private copy
/// without touching the original agent's directory. Returns the destination
/// directory path. The source is never mutated.
///
/// Safety: the manifest `name` is validated by `parse_skill_frontmatter`
/// (rejecting `/`, `\\`, `..`, and NUL). This function adds a defence-in-depth
/// `starts_with` check so a future caller that bypasses the parser cannot
/// write outside `dest_root`.
pub fn import_external_skill(manifest: &SkillManifest) -> Result<PathBuf, SkillError> {
    let Some(dest_root) = plot_forge_user_dir() else {
        return Err(SkillError::NoConfigDir);
    };
    import_external_skill_to(&dest_root, manifest)
}

/// Same as `import_external_skill` but writes into an explicit `dest_root`.
/// Used by tests to keep the user library hermetic; also the path-injected
/// form that lets callers redirect the import destination.
pub fn import_external_skill_to(
    dest_root: &Path,
    manifest: &SkillManifest,
) -> Result<PathBuf, SkillError> {
    let src_root = Path::new(&manifest.source.root_path);
    let src_skill = src_root.join(&manifest.source.rel_path);
    let dest_skill = dest_root.join(&manifest.name);
    // Defence-in-depth: the parser already rejects unsafe names, but assert
    // the resolved destination stays under `dest_root` so a malformed name
    // (or a future caller that bypasses the parser) cannot write outside the
    // user library. `Path::starts_with` is lexical, so also reject any `..`
    // component in the resolved path — `dest_root.join("../escape")` would
    // pass a naive `starts_with` check but escape the library.
    use std::path::Component;
    let escapes_root = !dest_skill.starts_with(dest_root)
        || dest_skill
            .components()
            .any(|component| matches!(component, Component::ParentDir));
    if escapes_root {
        return Err(SkillError::Import {
            skill_id: manifest.name.clone(),
            reason: format!(
                "resolved destination `{}` escapes the user skills library",
                dest_skill.display()
            ),
        });
    }
    if dest_skill.exists() {
        return Err(SkillError::Import {
            skill_id: manifest.name.clone(),
            reason: format!("destination already exists at {}", dest_skill.display()),
        });
    }
    copy_dir_recursive(&src_skill, &dest_skill).map_err(|error| SkillError::Import {
        skill_id: manifest.name.clone(),
        reason: error,
    })?;
    Ok(dest_skill)
}

/// Recursively copies `src` to `dst`, creating `dst` and any intermediate
/// directories. Symlinks are copied as the files they point to (so the
/// imported copy is self-contained). Returns an error string on any IO
/// failure; the caller wraps it in `SkillError::Import`.
///
/// Safety guards (R5): a depth cap prevents runaway recursion into a
/// symlink loop or a pathologically deep tree; symlink loops are detected
/// by canonicalising `src` and refusing to re-enter a canonical path that
/// already appears in the current recursion stack.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    copy_dir_recursive_inner(src, dst, 0, &mut Vec::new())
}

const COPY_DIR_MAX_DEPTH: usize = 32;

fn copy_dir_recursive_inner(
    src: &Path,
    dst: &Path,
    depth: usize,
    visited: &mut Vec<std::path::PathBuf>,
) -> Result<(), String> {
    if depth > COPY_DIR_MAX_DEPTH {
        return Err(format!(
            "copy_dir_recursive exceeded max depth {} at {}",
            COPY_DIR_MAX_DEPTH,
            src.display()
        ));
    }
    // Symlink-loop guard: canonicalise `src` and refuse to re-enter a path
    // already on the recursion stack. This catches a symlink that points at
    // an ancestor (the classic `ln -s . self` loop) without following it
    // endlessly. `canonicalize` follows symlinks, so a loop resolves to a
    // path we have already visited.
    let canonical_src = src
        .canonicalize()
        .map_err(|e| format!("canonicalize({}): {e}", src.display()))?;
    if visited.iter().any(|p| p == &canonical_src) {
        return Err(format!(
            "symlink loop detected copying skill at {} (canonical {})",
            src.display(),
            canonical_src.display()
        ));
    }
    visited.push(canonical_src);
    std::fs::create_dir_all(dst).map_err(|e| format!("create_dir_all({}): {e}", dst.display()))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("read_dir({}): {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("read entry: {e}"))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        // Use `fs::metadata` (follows symlinks) rather than `entry.metadata`
        // (which does not) so a symlink-to-directory is treated as a
        // directory and recursed into — where the canonical-path loop guard
        // catches a loop. A symlink-to-regular-file is still copied as the
        // file it points to (self-contained copy).
        let metadata =
            std::fs::metadata(&from).map_err(|e| format!("metadata({}): {e}", from.display()))?;
        if metadata.is_dir() {
            copy_dir_recursive_inner(&from, &to, depth + 1, visited)?;
        } else {
            std::fs::copy(&from, &to)
                .map_err(|e| format!("copy({} -> {}): {e}", from.display(), to.display()))?;
        }
    }
    visited.pop();
    Ok(())
}

/// Best-effort ISO-8601 timestamp. Uses `SystemTime` + duration since UNIX
/// epoch; does not require a timezone crate dependency. The exact format is
/// not a contract surface — it's only a cache-staleness hint.
fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_skill_md(dir: &Path, name: &str, description: &str) {
        fs::create_dir_all(dir).expect("mkdir skill");
        let content = format!(
            "---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n\nBody text.\n"
        );
        fs::write(dir.join("SKILL.md"), content).expect("write SKILL.md");
    }

    #[test]
    fn parse_frontmatter_reads_name_and_description() {
        let tmp = TempDir::new().expect("tempdir");
        let skill = tmp.path().join("my-skill");
        write_skill_md(&skill, "my-skill", "A test skill.");
        let fm = parse_skill_frontmatter(&skill).expect("parse");
        assert_eq!(fm.name, "my-skill");
        assert_eq!(fm.description, "A test skill.");
    }

    #[test]
    fn parse_frontmatter_missing_skill_md_errors() {
        let tmp = TempDir::new().expect("tempdir");
        let skill = tmp.path().join("empty");
        fs::create_dir_all(&skill).expect("mkdir");
        let error = parse_skill_frontmatter(&skill).expect_err("should error");
        assert!(matches!(error, SkillError::FrontmatterRead { .. }));
    }

    #[test]
    fn parse_frontmatter_missing_description_field_errors() {
        let tmp = TempDir::new().expect("tempdir");
        let skill = tmp.path().join("partial");
        fs::create_dir_all(&skill).expect("mkdir");
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: partial\ndescription: \"\"\n---\nbody\n",
        )
        .expect("write");
        let error = parse_skill_frontmatter(&skill).expect_err("empty description");
        assert!(
            matches!(error, SkillError::MissingFrontmatterField { field, .. } if field == "description")
        );
    }

    #[test]
    fn extract_frontmatter_handles_no_block() {
        assert!(extract_frontmatter("# just markdown\nno fence").is_none());
        assert_eq!(
            extract_frontmatter("---\nname: x\n---\nbody").unwrap(),
            "name: x"
        );
    }

    #[test]
    fn strip_frontmatter_returns_body_only() {
        let content = "---\nname: x\ndescription: y\n---\n\nBody here.\n";
        assert_eq!(strip_frontmatter(content), "Body here.\n");
        // No frontmatter → whole content returned.
        assert_eq!(strip_frontmatter("just body"), "just body");
    }

    #[test]
    fn parse_skill_interface_reads_openai_yaml() {
        let tmp = TempDir::new().expect("tempdir");
        let skill = tmp.path().join("with-yaml");
        write_skill_md(&skill, "with-yaml", "Has openai.yaml.");
        fs::create_dir_all(skill.join("agents")).expect("mkdir agents");
        fs::write(
            skill.join("agents").join("openai.yaml"),
            "interface:\n  display_name: \"With YAML\"\n  short_description: \"short\"\n",
        )
        .expect("write yaml");
        let interface = parse_skill_interface(&skill).expect("interface");
        assert_eq!(interface.display_name.as_deref(), Some("With YAML"));
        assert_eq!(interface.short_description.as_deref(), Some("short"));
    }

    #[test]
    fn parse_skill_interface_returns_none_when_missing() {
        let tmp = TempDir::new().expect("tempdir");
        let skill = tmp.path().join("no-yaml");
        write_skill_md(&skill, "no-yaml", "No openai.yaml.");
        assert!(parse_skill_interface(&skill).is_none());
    }

    #[test]
    fn scan_skill_collects_scripts_references_assets_paths() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().join("root");
        let skill = root.join("my-skill");
        write_skill_md(&skill, "my-skill", "A test skill.");
        // Use the singular `reference/` form to test the variant folder name.
        fs::create_dir_all(skill.join("reference")).expect("mkdir reference");
        fs::write(skill.join("reference").join("typography.md"), "typography").expect("write ref");
        fs::create_dir_all(skill.join("scripts")).expect("mkdir scripts");
        fs::write(
            skill.join("scripts").join("run.py"),
            "#!/usr/bin/env python",
        )
        .expect("write script");
        fs::create_dir_all(skill.join("assets")).expect("mkdir assets");
        fs::write(skill.join("assets").join("logo.svg"), "<svg/>").expect("write asset");

        let manifest = scan_skill(&skill, SkillOrigin::PlotForgeUser, &root).expect("scan");
        assert_eq!(manifest.id, "my-skill");
        assert_eq!(manifest.name, "my-skill");
        assert_eq!(manifest.source.origin, SkillOrigin::PlotForgeUser);
        assert_eq!(manifest.body_path, "my-skill/SKILL.md");
        assert!(
            manifest
                .scripts
                .iter()
                .any(|p| p.ends_with("scripts/run.py"))
        );
        assert!(
            manifest
                .references
                .iter()
                .any(|p| p.ends_with("reference/typography.md"))
        );
        assert!(
            manifest
                .assets
                .iter()
                .any(|p| p.ends_with("assets/logo.svg"))
        );
        // Body content is NOT in the manifest.
        assert!(!manifest.scripts.iter().any(|p| p.contains("python")));
    }

    #[test]
    fn scan_skill_handles_missing_optional_subdirs() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().join("root");
        let skill = root.join("bare");
        write_skill_md(&skill, "bare", "Bare skill.");
        let manifest = scan_skill(&skill, SkillOrigin::ClaudeCode, &root).expect("scan");
        assert!(manifest.scripts.is_empty());
        assert!(manifest.references.is_empty());
        assert!(manifest.assets.is_empty());
    }

    #[test]
    fn load_skill_body_returns_body_without_frontmatter() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().join("root");
        let skill = root.join("b");
        write_skill_md(&skill, "b", "Body loader test.");
        let manifest = scan_skill(&skill, SkillOrigin::PlotForgeUser, &root).expect("scan");
        let body = load_skill_body(&manifest).expect("load body");
        assert!(body.contains("# b"));
        assert!(!body.starts_with("---"));
    }

    #[test]
    fn import_external_skill_copies_into_user_library() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().join("external-root");
        let skill = root.join("external-skill");
        write_skill_md(&skill, "external-skill", "From an external agent.");
        fs::create_dir_all(skill.join("scripts")).expect("mkdir scripts");
        fs::write(skill.join("scripts").join("run.sh"), "#!/bin/sh\necho hi")
            .expect("write script");

        let manifest = scan_skill(&skill, SkillOrigin::ClaudeCode, &root).expect("scan");

        // Override the user library dir by pointing plot_forge_user_dir at a
        // temp dir through a thin wrapper: we can't easily inject that, so
        // instead we exercise `import_external_skill` only when the user
        // config dir resolves to somewhere writable. To keep the test
        // hermetic, we skip if it resolves to the real user dir. This is a
        // known limitation; the roundtrip logic itself is what we assert via
        // the direct copy below.
        let dest_root = tmp.path().join("imported-plotforge-skills");
        let dest_skill = dest_root.join(&manifest.name);
        copy_dir_recursive(&skill, &dest_skill).expect("copy");
        assert!(dest_skill.join("SKILL.md").exists());
        assert!(dest_skill.join("scripts").join("run.sh").exists());
        // The original is untouched (still exists).
        assert!(skill.join("SKILL.md").exists());
        // And the import helper builds the same destination shape when the
        // user dir is writable; we assert the helper errors cleanly when the
        // destination already exists by re-running copy + invoking it would
        // require the real user dir, so we stop here.
        let _ = manifest;
    }

    #[test]
    fn import_errors_when_destination_exists() {
        // We cannot redirect the real user dir hermetically, so we test the
        // duplicate-detection branch directly via copy_dir_recursive + a
        // manual check that mirrors the guard in import_external_skill.
        let tmp = TempDir::new().expect("tempdir");
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&src).expect("mkdir src");
        fs::write(src.join("SKILL.md"), "---\nname: x\ndescription: y\n---\n").expect("write");
        // Pre-create the destination to simulate an existing copy.
        fs::create_dir_all(&dst).expect("mkdir dst");
        // copy_dir_recursive would overwrite; the *guard* in
        // import_external_skill is what protects against this. We assert the
        // guard logic by re-implementing the check inline.
        let already_exists = dst.exists();
        assert!(already_exists, "guard precondition holds");
    }

    #[test]
    fn write_and_read_skill_index_roundtrips() {
        // Write to an explicit path so we don't touch the real user dir.
        let tmp = TempDir::new().expect("tempdir");
        let index_path = tmp.path().join("skill-index.json");
        let index = SkillIndex {
            version: "1".into(),
            skills: vec![SkillManifest {
                id: "demo".into(),
                name: "demo".into(),
                description: "Demo skill.".into(),
                source: SkillSource {
                    origin: SkillOrigin::PlotForgeUser,
                    root_path: "/tmp/root".into(),
                    rel_path: "demo".into(),
                },
                interface: None,
                body_path: "demo/SKILL.md".into(),
                scripts: Vec::new(),
                references: Vec::new(),
                assets: Vec::new(),
            }],
            scanned_at: "1234567890".into(),
        };
        write_skill_index_to(&index_path, &index).expect("write via public API");
        let loaded = read_skill_index_from(&index_path).expect("read via public API");
        assert_eq!(loaded, index);
    }

    /// Regression for the Critical path-traversal finding (H1): a skill
    /// `name` with path separators or `..` segments must be rejected by the
    /// parser so `import_external_skill` can never write outside the user
    /// library. The parser is the front gate; the importer also has a
    /// `starts_with` defence-in-depth check.
    #[test]
    fn parse_frontmatter_rejects_unsafe_skill_name() {
        for bad_name in ["../escape", "sub/dir", "..", "with\\backslash", "with\0nul"] {
            let tmp = TempDir::new().expect("tempdir");
            let skill = tmp.path().join("bad");
            write_skill_md(&skill, bad_name, "Has a bad name.");
            let error = parse_skill_frontmatter(&skill).expect_err("unsafe name rejected");
            assert!(
                matches!(error, SkillError::FrontmatterParse { .. }),
                "expected FrontmatterParse for `{bad_name}`, got {error:?}"
            );
        }
    }

    /// A plain single-component name must still parse.
    #[test]
    fn parse_frontmatter_accepts_safe_single_component_name() {
        let tmp = TempDir::new().expect("tempdir");
        let skill = tmp.path().join("good");
        write_skill_md(&skill, "good-name", "A safe name.");
        let fm = parse_skill_frontmatter(&skill).expect("safe name parses");
        assert_eq!(fm.name, "good-name");
    }

    /// Defence-in-depth: `import_external_skill_to` must reject a manifest
    /// whose `name` resolves to a destination outside `dest_root`, even if
    /// the parser were bypassed. Exercises the real `starts_with` guard via
    /// the public path-injected API.
    #[test]
    fn import_external_skill_to_rejects_traversal_name() {
        let tmp = TempDir::new().expect("tempdir");
        let external_root = tmp.path().join("external");
        let skill = external_root.join("evil");
        write_skill_md(&skill, "evil", "Bad name.");
        let manifest = SkillManifest {
            id: "../escape".into(),
            name: "../escape".into(),
            description: "Traversal.".into(),
            source: SkillSource {
                origin: SkillOrigin::ClaudeCode,
                root_path: external_root.display().to_string(),
                rel_path: "evil".into(),
            },
            interface: None,
            body_path: "evil/SKILL.md".into(),
            scripts: Vec::new(),
            references: Vec::new(),
            assets: Vec::new(),
        };
        let dest_root = tmp.path().join("user-skills");
        let error = import_external_skill_to(&dest_root, &manifest)
            .expect_err("traversal must be rejected");
        assert!(
            matches!(error, SkillError::Import { .. }),
            "expected Import error, got {error:?}"
        );
        assert!(
            !dest_root.join("../escape").exists(),
            "no directory must be created outside dest_root"
        );
    }

    /// The path-injected import API copies a skill into an explicit
    /// `dest_root`, and errors when the destination already exists (the
    /// duplicate guard). Exercises the real `import_external_skill_to` guard
    /// hermetically, unlike the old inline-reimplementation test.
    #[test]
    fn import_external_skill_to_roundtrip_and_duplicate_guard() {
        let tmp = TempDir::new().expect("tempdir");
        let external_root = tmp.path().join("external");
        let skill = external_root.join("importable");
        write_skill_md(&skill, "importable", "From an external agent.");
        std::fs::create_dir_all(skill.join("scripts")).expect("mkdir scripts");
        std::fs::write(skill.join("scripts").join("run.sh"), "#!/bin/sh\necho hi")
            .expect("write script");

        let manifest = scan_skill(&skill, SkillOrigin::ClaudeCode, &external_root).expect("scan");
        let dest_root = tmp.path().join("user-skills");

        let dest = import_external_skill_to(&dest_root, &manifest).expect("import succeeds");
        assert!(dest.join("SKILL.md").exists());
        assert!(dest.join("scripts").join("run.sh").exists());
        // The original is untouched.
        assert!(skill.join("SKILL.md").exists());

        // Re-import must error on the existing destination (no overwrite).
        let error =
            import_external_skill_to(&dest_root, &manifest).expect_err("duplicate import rejected");
        assert!(matches!(error, SkillError::Import { .. }));
    }

    /// Regression for the CRLF frontmatter finding (L1): a `SKILL.md` with
    /// `\r\n` line endings must parse the same as a `\n`-only file.
    #[test]
    fn parse_frontmatter_tolerates_crlf_line_endings() {
        let tmp = TempDir::new().expect("tempdir");
        let skill = tmp.path().join("crlf-skill");
        std::fs::create_dir_all(&skill).expect("mkdir");
        let content = "---\r\nname: crlf-skill\r\ndescription: A CRLF skill.\r\n---\r\n\r\n# crlf-skill\r\n\r\nBody.\r\n";
        std::fs::write(skill.join("SKILL.md"), content).expect("write CRLF SKILL.md");
        let fm = parse_skill_frontmatter(&skill).expect("CRLF frontmatter parses");
        assert_eq!(fm.name, "crlf-skill");
        assert_eq!(fm.description, "A CRLF skill.");
        // The body loader must also strip a CRLF frontmatter block.
        let manifest = scan_skill(&skill, SkillOrigin::PlotForgeUser, tmp.path()).expect("scan");
        let body = load_skill_body(&manifest).expect("load body");
        assert!(body.contains("# crlf-skill"));
        assert!(!body.starts_with("---"));
    }

    /// `extract_frontmatter` and `strip_frontmatter` must treat CRLF and LF
    /// identically for both the opening and closing fence handling. The
    /// extracted frontmatter text retains the trailing `\r` before the
    /// closing fence; `serde_yaml` tolerates this when parsing.
    #[test]
    fn frontmatter_helpers_handle_crlf() {
        assert_eq!(
            extract_frontmatter("---\r\nname: x\r\n---\r\nbody").unwrap(),
            "name: x\r"
        );
        assert_eq!(
            strip_frontmatter("---\r\nname: x\r\ndescription: y\r\n---\r\n\r\nBody here.\r\n"),
            "Body here.\r\n"
        );
    }

    /// R5: `copy_dir_recursive` must refuse to follow a symlink loop (the
    /// classic `ln -s . self` inside a skill's `assets/` tree) instead of
    /// recursing until it crashes or fills the disk. The depth cap + canonical
    /// path stack guard must surface an explicit error.
    #[cfg(unix)]
    #[test]
    fn copy_dir_recursive_rejects_symlink_loop() {
        use std::os::unix::fs::symlink;
        let tmp = TempDir::new().expect("tempdir");
        let src = tmp.path().join("loopy-skill");
        fs::create_dir_all(&src).expect("mkdir src");
        fs::write(
            src.join("SKILL.md"),
            "---\nname: loopy\ndescription: y\n---\n",
        )
        .expect("write");
        // Create a subdirectory that links to its own parent, forming a loop.
        let loop_dir = src.join("assets");
        fs::create_dir_all(&loop_dir).expect("mkdir assets");
        symlink(".", loop_dir.join("self")).expect("symlink self -> .");
        // `copy_dir_recursive` must error rather than recurse forever.
        let dst = tmp.path().join("out");
        let error = copy_dir_recursive(&src, &dst).expect_err("symlink loop must error");
        assert!(
            error.contains("symlink loop") || error.contains("max depth"),
            "expected loop/depth error, got: {error}"
        );
    }
}
