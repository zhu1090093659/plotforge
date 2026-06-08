use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use plotforge_media::{AssetRegistry, MediaError};
use plotforge_schema::{AssetRecord, ExportManifest};
use plotforge_storage::{StorageError, load_project};
use thiserror::Error;

const PLAYER_PACKAGE_FILES: &[(&str, &str)] = &[
    (
        "index.html",
        include_str!("../../../apps/player-web/static/index.html"),
    ),
    (
        "styles.css",
        include_str!("../../../apps/player-web/static/styles.css"),
    ),
    (
        "player-core.js",
        include_str!("../../../apps/player-web/static/player-core.js"),
    ),
    (
        "player.js",
        include_str!("../../../apps/player-web/static/player.js"),
    ),
];

#[derive(Debug, Error)]
pub enum ExportError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Media(#[from] MediaError),
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("json error at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("static export contains a blocked secret marker: {0}")]
    SecretMarker(String),
    #[error("static export asset path is not package-safe: {0}")]
    UnsafeAssetPath(String),
    #[error("static export package contains a disallowed file: {0}")]
    DisallowedPackageFile(PathBuf),
    #[error("static export package is missing an expected file: {0}")]
    MissingPackageFile(PathBuf),
}

#[derive(Clone, Debug)]
pub struct ExportReport {
    pub output_dir: PathBuf,
    pub files_written: Vec<PathBuf>,
    pub audit: ExportPackageAudit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportPackageAudit {
    pub allowed_files: Vec<PathBuf>,
    pub files_found: Vec<PathBuf>,
}

pub fn export_static_web(
    project_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<ExportReport, ExportError> {
    let project_path = project_path.as_ref();
    let project = load_project(project_path)?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).map_io(output_dir)?;
    fs::create_dir_all(output_dir.join("assets/generated"))
        .map_io(output_dir.join("assets/generated"))?;

    let mut asset_registry = AssetRegistry::new();
    asset_registry.register_scene_background_assets(project_path, &project)?;
    let asset_records = asset_registry
        .reachable_records()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    let assets = asset_records
        .iter()
        .map(|record| record.export_path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let asset_paths = assets
        .iter()
        .map(|asset| validate_export_asset_path(asset))
        .collect::<Result<Vec<_>, _>>()?;
    let asset_record_by_export_path = asset_records
        .into_iter()
        .map(|record| (PathBuf::from(&record.export_path), record))
        .collect::<BTreeMap<_, _>>();
    let allowed_files = allowed_export_files(&asset_paths);
    let manifest = ExportManifest {
        game: project.game,
        entry_scene: project.story_state.current_scene_key,
        scenes: project.scenes,
        assets: assets.clone(),
        generated_by: "plotforge-export 0.1.0".into(),
    };
    let data = serde_json::to_string_pretty(&manifest).map_err(|source| ExportError::Json {
        path: output_dir.join("game.json"),
        source,
    })?;
    assert_no_secret_markers(&data)?;

    let data_path = output_dir.join("game.json");
    fs::write(&data_path, data + "\n").map_io(&data_path)?;

    let mut files_written = write_player_package(output_dir)?;
    files_written.push(data_path);
    for asset in asset_paths {
        let record = asset_record_by_export_path
            .get(&asset)
            .ok_or_else(|| ExportError::MissingPackageFile(asset.clone()))?;
        let asset_path = copy_referenced_asset(project_path, output_dir, record)?;
        files_written.push(asset_path);
    }
    let audit = audit_export_package(output_dir, &allowed_files)?;

    Ok(ExportReport {
        output_dir: output_dir.to_path_buf(),
        files_written,
        audit,
    })
}

fn write_player_package(output_dir: &Path) -> Result<Vec<PathBuf>, ExportError> {
    PLAYER_PACKAGE_FILES
        .iter()
        .map(|(relative_path, contents)| {
            let output_path = output_dir.join(relative_path);
            fs::write(&output_path, contents).map_io(&output_path)?;
            Ok(output_path)
        })
        .collect()
}

fn copy_referenced_asset(
    project_path: &Path,
    output_dir: &Path,
    record: &AssetRecord,
) -> Result<PathBuf, ExportError> {
    let source_path = project_path.join(&record.project_path);
    let output_path = output_dir.join(&record.export_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_io(parent)?;
    }
    fs::metadata(&source_path).map_io(&source_path)?;
    fs::copy(&source_path, &output_path).map_io(&output_path)?;
    Ok(output_path)
}

fn assert_no_secret_markers(data: &str) -> Result<(), ExportError> {
    for marker in ["OPENAI_API_KEY", "api_key", "secret_key", "sk-"] {
        if data.contains(marker) {
            return Err(ExportError::SecretMarker(marker.into()));
        }
    }
    Ok(())
}

fn validate_export_asset_path(asset: &str) -> Result<PathBuf, ExportError> {
    let path = Path::new(asset);
    if asset.is_empty() || path.is_absolute() || !path.starts_with("assets") {
        return Err(ExportError::UnsafeAssetPath(asset.into()));
    }

    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(ExportError::UnsafeAssetPath(asset.into()));
        }
    }

    Ok(path.to_path_buf())
}

fn allowed_export_files(asset_paths: &[PathBuf]) -> BTreeSet<PathBuf> {
    let mut allowed_files = BTreeSet::from([PathBuf::from("game.json")]);
    allowed_files.extend(
        PLAYER_PACKAGE_FILES
            .iter()
            .map(|(relative_path, _)| PathBuf::from(relative_path)),
    );
    allowed_files.extend(asset_paths.iter().cloned());
    allowed_files
}

fn audit_export_package(
    output_dir: &Path,
    allowed_files: &BTreeSet<PathBuf>,
) -> Result<ExportPackageAudit, ExportError> {
    let files_found = export_file_manifest(output_dir)?;
    for file in &files_found {
        if !allowed_files.contains(file) {
            return Err(ExportError::DisallowedPackageFile(file.clone()));
        }
    }
    for file in allowed_files {
        if !files_found.contains(file) {
            return Err(ExportError::MissingPackageFile(file.clone()));
        }
    }

    Ok(ExportPackageAudit {
        allowed_files: allowed_files.iter().cloned().collect(),
        files_found: files_found.into_iter().collect(),
    })
}

fn export_file_manifest(output_dir: &Path) -> Result<BTreeSet<PathBuf>, ExportError> {
    let mut manifest = BTreeSet::new();
    collect_export_files(output_dir, output_dir, &mut manifest)?;
    Ok(manifest)
}

fn collect_export_files(
    output_dir: &Path,
    dir: &Path,
    manifest: &mut BTreeSet<PathBuf>,
) -> Result<(), ExportError> {
    let mut entries = fs::read_dir(dir)
        .map_io(dir)?
        .map(|entry| entry.map(|entry| entry.path()).map_io(dir))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();

    for path in entries {
        if path.is_dir() {
            collect_export_files(output_dir, &path, manifest)?;
            continue;
        }

        let relative_path = path
            .strip_prefix(output_dir)
            .map_err(|source| ExportError::Io {
                path: path.clone(),
                source: std::io::Error::other(source),
            })?;
        manifest.insert(relative_path.to_path_buf());
    }
    Ok(())
}

trait IoContext<T> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, ExportError>;
}

impl<T> IoContext<T> for Result<T, std::io::Error> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, ExportError> {
        self.map_err(|source| ExportError::Io {
            path: path.as_ref().to_path_buf(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use plotforge_storage::create_demo_project;

    use super::export_static_web;

    #[test]
    fn exports_static_player_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project_path = temp.path().join("project");
        let output_dir = temp.path().join("export");
        create_demo_project(&project_path, false).expect("create demo");

        let report = export_static_web(&project_path, &output_dir).expect("export");

        assert!(output_dir.join("index.html").exists());
        assert!(output_dir.join("game.json").exists());
        assert!(
            output_dir
                .join("assets/generated/court-crisis-001.png")
                .exists()
        );
        assert!(report.files_written.len() >= 3);
        assert_eq!(report.audit.allowed_files, report.audit.files_found);
    }
}
