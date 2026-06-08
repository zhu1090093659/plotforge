use std::{
    fs,
    path::{Path, PathBuf},
};

use plotforge_media::AssetRegistry;
use plotforge_schema::{AssetRecord, ProjectData, RuntimeTrace};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{IoContext, StorageError, load_project, read_json};

pub const SQLITE_CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqliteCacheSummary {
    pub schema_version: u32,
    pub project_id: String,
    pub title: String,
    pub version: String,
    pub entry_scene: String,
    pub scene_count: u32,
    pub character_count: u32,
    pub rule_count: u32,
    pub source_file_count: u32,
    pub asset_count: u32,
    pub trace_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceFileRecord {
    path: String,
    byte_length: u64,
    content_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TraceCacheRecord {
    id: String,
    path: String,
    timestamp_ms: u64,
    fallback_used: bool,
    turn: u32,
}

pub fn sqlite_cache_path(project_path: impl AsRef<Path>) -> PathBuf {
    project_path.as_ref().join(".plotforge/cache.sqlite")
}

pub fn rebuild_sqlite_cache(
    project_path: impl AsRef<Path>,
) -> Result<SqliteCacheSummary, StorageError> {
    let project_path = project_path.as_ref();
    let project = load_project(project_path)?;
    let source_files = source_file_records(project_path)?;
    let asset_records = asset_records(project_path, &project)?;
    let trace_records = trace_records(project_path)?;
    let cache_path = sqlite_cache_path(project_path);
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_io(parent)?;
    }

    let mut connection = Connection::open(&cache_path).map_sqlite(&cache_path)?;
    migrate_sqlite_cache(&connection, &cache_path)?;
    let transaction = connection.transaction().map_sqlite(&cache_path)?;
    replace_cache_rows(
        &transaction,
        &cache_path,
        &project,
        &source_files,
        &asset_records,
        &trace_records,
    )?;
    transaction.commit().map_sqlite(&cache_path)?;

    Ok(summary_from_project(
        &project,
        source_files.len(),
        asset_records.len(),
        trace_records.len(),
    ))
}

pub fn read_sqlite_cache_summary(
    project_path: impl AsRef<Path>,
) -> Result<Option<SqliteCacheSummary>, StorageError> {
    let cache_path = sqlite_cache_path(project_path);
    if !cache_path.exists() {
        return Ok(None);
    }

    let connection = Connection::open(&cache_path).map_sqlite(&cache_path)?;
    verify_sqlite_cache_schema(&connection, &cache_path)?;
    connection
        .query_row(
            "SELECT project_id, title, version, entry_scene, scene_count, character_count, rule_count, source_file_count, asset_count, trace_count FROM project_index LIMIT 1",
            [],
            |row| {
                Ok(SqliteCacheSummary {
                    schema_version: SQLITE_CACHE_SCHEMA_VERSION,
                    project_id: row.get(0)?,
                    title: row.get(1)?,
                    version: row.get(2)?,
                    entry_scene: row.get(3)?,
                    scene_count: row.get::<_, u32>(4)?,
                    character_count: row.get::<_, u32>(5)?,
                    rule_count: row.get::<_, u32>(6)?,
                    source_file_count: row.get::<_, u32>(7)?,
                    asset_count: row.get::<_, u32>(8)?,
                    trace_count: row.get::<_, u32>(9)?,
                })
            },
        )
        .optional()
        .map_sqlite(&cache_path)
}

fn migrate_sqlite_cache(connection: &Connection, cache_path: &Path) -> Result<(), StorageError> {
    let current_version = sqlite_user_version(connection, cache_path)?;
    if current_version > SQLITE_CACHE_SCHEMA_VERSION {
        return Err(StorageError::UnsupportedSqliteCacheSchemaVersion {
            expected: SQLITE_CACHE_SCHEMA_VERSION,
            actual: current_version,
        });
    }

    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS project_index (
                project_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                version TEXT NOT NULL,
                entry_scene TEXT NOT NULL,
                scene_count INTEGER NOT NULL,
                character_count INTEGER NOT NULL,
                rule_count INTEGER NOT NULL,
                source_file_count INTEGER NOT NULL,
                asset_count INTEGER NOT NULL,
                trace_count INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS source_files (
                path TEXT PRIMARY KEY,
                byte_length INTEGER NOT NULL,
                content_hash TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS traces (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                timestamp_ms INTEGER NOT NULL,
                fallback_used INTEGER NOT NULL,
                turn INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS assets (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                source TEXT NOT NULL,
                project_path TEXT NOT NULL,
                export_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                byte_length INTEGER NOT NULL
            );

            INSERT OR IGNORE INTO schema_migrations (version, name)
            VALUES (1, 'initial-cache-index');

            PRAGMA user_version = 1;
            "#,
        )
        .map_sqlite(cache_path)
}

fn verify_sqlite_cache_schema(
    connection: &Connection,
    cache_path: &Path,
) -> Result<(), StorageError> {
    let current_version = sqlite_user_version(connection, cache_path)?;
    if current_version == SQLITE_CACHE_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(StorageError::UnsupportedSqliteCacheSchemaVersion {
            expected: SQLITE_CACHE_SCHEMA_VERSION,
            actual: current_version,
        })
    }
}

fn sqlite_user_version(connection: &Connection, cache_path: &Path) -> Result<u32, StorageError> {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_sqlite(cache_path)
}

fn replace_cache_rows(
    transaction: &Transaction<'_>,
    cache_path: &Path,
    project: &ProjectData,
    source_files: &[SourceFileRecord],
    asset_records: &[AssetRecord],
    trace_records: &[TraceCacheRecord],
) -> Result<(), StorageError> {
    for table in ["project_index", "source_files", "traces", "assets"] {
        transaction
            .execute(&format!("DELETE FROM {table}"), [])
            .map_sqlite(cache_path)?;
    }

    transaction
        .execute(
            "INSERT INTO project_index (project_id, title, version, entry_scene, scene_count, character_count, rule_count, source_file_count, asset_count, trace_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                &project.game.id,
                &project.game.title,
                &project.game.version,
                &project.game.entry_scene,
                project.scenes.len() as u32,
                project.characters.len() as u32,
                project.rules.len() as u32,
                source_files.len() as u32,
                asset_records.len() as u32,
                trace_records.len() as u32,
            ],
        )
        .map_sqlite(cache_path)?;

    for source_file in source_files {
        transaction
            .execute(
                "INSERT INTO source_files (path, byte_length, content_hash) VALUES (?1, ?2, ?3)",
                params![
                    &source_file.path,
                    source_file.byte_length,
                    &source_file.content_hash,
                ],
            )
            .map_sqlite(cache_path)?;
    }

    for trace in trace_records {
        transaction
            .execute(
                "INSERT INTO traces (id, path, timestamp_ms, fallback_used, turn) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    &trace.id,
                    &trace.path,
                    trace.timestamp_ms,
                    trace.fallback_used,
                    trace.turn,
                ],
            )
            .map_sqlite(cache_path)?;
    }

    for asset in asset_records {
        transaction
            .execute(
                "INSERT INTO assets (id, kind, source, project_path, export_path, content_hash, byte_length)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &asset.id,
                    serde_string(cache_path, &asset.kind)?,
                    serde_string(cache_path, &asset.source)?,
                    &asset.project_path,
                    &asset.export_path,
                    &asset.content_hash,
                    asset.byte_length,
                ],
            )
            .map_sqlite(cache_path)?;
    }

    Ok(())
}

fn summary_from_project(
    project: &ProjectData,
    source_file_count: usize,
    asset_count: usize,
    trace_count: usize,
) -> SqliteCacheSummary {
    SqliteCacheSummary {
        schema_version: SQLITE_CACHE_SCHEMA_VERSION,
        project_id: project.game.id.clone(),
        title: project.game.title.clone(),
        version: project.game.version.clone(),
        entry_scene: project.game.entry_scene.clone(),
        scene_count: project.scenes.len() as u32,
        character_count: project.characters.len() as u32,
        rule_count: project.rules.len() as u32,
        source_file_count: source_file_count as u32,
        asset_count: asset_count as u32,
        trace_count: trace_count as u32,
    }
}

fn source_file_records(project_path: &Path) -> Result<Vec<SourceFileRecord>, StorageError> {
    let mut records = Vec::new();
    collect_source_files(project_path, project_path, &mut records)?;
    records.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(records)
}

fn collect_source_files(
    project_path: &Path,
    dir: &Path,
    records: &mut Vec<SourceFileRecord>,
) -> Result<(), StorageError> {
    let mut entries = fs::read_dir(dir)
        .map_io(dir)?
        .map(|entry| entry.map(|entry| entry.path()).map_io(dir))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();

    for path in entries {
        if path.is_dir() {
            if should_skip_cache_index_dir(project_path, &path)? {
                continue;
            }
            collect_source_files(project_path, &path, records)?;
            continue;
        }

        if !is_indexable_source_file(&path) {
            continue;
        }

        let bytes = fs::read(&path).map_io(&path)?;
        let relative_path = relative_project_path(project_path, &path)?;
        records.push(SourceFileRecord {
            path: relative_path,
            byte_length: bytes.len() as u64,
            content_hash: sha256_hex(&bytes),
        });
    }

    Ok(())
}

fn should_skip_cache_index_dir(project_path: &Path, dir: &Path) -> Result<bool, StorageError> {
    let relative_path = dir
        .strip_prefix(project_path)
        .map_err(|_| StorageError::InvalidProjectPath(dir.to_path_buf()))?;
    Ok(relative_path
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .is_some_and(|name| matches!(name, ".plotforge" | "exports" | "traces")))
}

fn is_indexable_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "json" | "markdown" | "md" | "png" | "toml"))
}

fn asset_records(
    project_path: &Path,
    project: &ProjectData,
) -> Result<Vec<AssetRecord>, StorageError> {
    let mut registry = AssetRegistry::new();
    registry
        .register_scene_background_assets(project_path, project)
        .map_err(|source| StorageError::Media {
            path: project_path.to_path_buf(),
            source,
        })?;
    Ok(registry.records().cloned().collect())
}

fn trace_records(project_path: &Path) -> Result<Vec<TraceCacheRecord>, StorageError> {
    let traces_dir = project_path.join("traces");
    if !traces_dir.exists() {
        return Ok(Vec::new());
    }

    let mut paths = fs::read_dir(&traces_dir)
        .map_io(&traces_dir)?
        .map(|entry| entry.map(|entry| entry.path()).map_io(&traces_dir))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();

    let mut records = Vec::new();
    for path in paths {
        if path.file_name().and_then(|name| name.to_str()) == Some("latest.json") {
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }

        let trace = read_json::<RuntimeTrace>(&path)?;
        records.push(TraceCacheRecord {
            id: trace.id,
            path: relative_project_path(project_path, &path)?,
            timestamp_ms: trace.timestamp_ms,
            fallback_used: trace.fallback_used,
            turn: trace.story_state_after.turn,
        });
    }

    Ok(records)
}

fn relative_project_path(project_path: &Path, path: &Path) -> Result<String, StorageError> {
    let relative_path = path
        .strip_prefix(project_path)
        .map_err(|_| StorageError::InvalidProjectPath(path.to_path_buf()))?;
    relative_path
        .to_str()
        .map(|path| path.replace('\\', "/"))
        .ok_or_else(|| StorageError::InvalidProjectPath(relative_path.to_path_buf()))
}

fn serde_string<T: Serialize>(cache_path: &Path, value: &T) -> Result<String, StorageError> {
    match serde_json::to_value(value).map_err(|source| StorageError::Json {
        path: cache_path.to_path_buf(),
        source,
    })? {
        serde_json::Value::String(value) => Ok(value),
        other => Err(StorageError::InvalidSqliteCacheValue(format!(
            "expected string enum, got {other}"
        ))),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

trait SqliteContext<T> {
    fn map_sqlite(self, path: &Path) -> Result<T, StorageError>;
}

impl<T> SqliteContext<T> for rusqlite::Result<T> {
    fn map_sqlite(self, path: &Path) -> Result<T, StorageError> {
        self.map_err(|source| StorageError::Sqlite {
            path: path.to_path_buf(),
            source,
        })
    }
}
