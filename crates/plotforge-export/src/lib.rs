use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Component, Path, PathBuf},
};

use plotforge_media::{AssetRegistry, MediaError};
use plotforge_schema::{
    AI_USAGE_MANIFEST_FILE, AiProviderSummary, AiUsageContentKind, AiUsageDisclosure,
    AiUsageManifest, AiUsageSourceKind, AssetKind, AssetRecord, AssetSourceKind, ExportManifest,
    ExportProfile, contains_secret_marker_text,
};
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
    #[error(
        "static zip archive path must not be inside the export directory: {archive} inside {output_dir}"
    )]
    ArchiveInsideOutputDir {
        archive: PathBuf,
        output_dir: PathBuf,
    },
    #[error("static zip entry path is not package-safe: {0}")]
    UnsafeZipPath(PathBuf),
    #[error("zip error at {path}: {source}")]
    Zip {
        path: PathBuf,
        #[source]
        source: zip::result::ZipError,
    },
}

#[derive(Clone, Debug)]
pub struct ExportReport {
    pub output_dir: PathBuf,
    pub files_written: Vec<PathBuf>,
    pub audit: ExportPackageAudit,
}

#[derive(Clone, Debug)]
pub struct ExportZipReport {
    pub archive_path: PathBuf,
    pub source_report: ExportReport,
    pub archived_files: Vec<PathBuf>,
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
        .iter()
        .cloned()
        .map(|record| (PathBuf::from(&record.export_path), record))
        .collect::<BTreeMap<_, _>>();
    let allowed_files = allowed_export_files(&asset_paths);
    let profile = ExportProfile::static_web();
    let ai_usage = ai_usage_manifest(&project, &profile, &asset_records);
    let manifest = ExportManifest {
        game: project.game,
        entry_scene: project.story_state.current_scene_key,
        scenes: project.scenes,
        assets: assets.clone(),
        profile,
        ai_usage_manifest_path: AI_USAGE_MANIFEST_FILE.into(),
        generated_by: "plotforge-export 0.1.0".into(),
    };
    let data = serde_json::to_string_pretty(&manifest).map_err(|source| ExportError::Json {
        path: output_dir.join("game.json"),
        source,
    })?;
    assert_no_secret_markers(&data)?;
    let ai_usage_data =
        serde_json::to_string_pretty(&ai_usage).map_err(|source| ExportError::Json {
            path: output_dir.join(AI_USAGE_MANIFEST_FILE),
            source,
        })?;
    assert_no_secret_markers(&ai_usage_data)?;

    let data_path = output_dir.join("game.json");
    let ai_usage_path = output_dir.join(AI_USAGE_MANIFEST_FILE);
    fs::write(&data_path, data + "\n").map_io(&data_path)?;
    fs::write(&ai_usage_path, ai_usage_data + "\n").map_io(&ai_usage_path)?;

    let mut files_written = write_player_package(output_dir)?;
    files_written.push(data_path);
    files_written.push(ai_usage_path);
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

pub fn export_static_web_zip(
    project_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    archive_path: impl AsRef<Path>,
) -> Result<ExportZipReport, ExportError> {
    let output_dir = output_dir.as_ref();
    let archive_path = archive_path.as_ref();
    ensure_archive_outside_output_dir(output_dir, archive_path)?;

    let source_report = export_static_web(project_path, output_dir)?;
    let archived_files =
        write_static_zip_archive(output_dir, archive_path, &source_report.audit.files_found)?;

    Ok(ExportZipReport {
        archive_path: archive_path.to_path_buf(),
        source_report,
        archived_files,
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

fn ensure_archive_outside_output_dir(
    output_dir: &Path,
    archive_path: &Path,
) -> Result<(), ExportError> {
    let output_dir = normalized_absolute_path(output_dir)?;
    let archive_path = normalized_absolute_path(archive_path)?;
    if archive_path.starts_with(&output_dir) {
        return Err(ExportError::ArchiveInsideOutputDir {
            archive: archive_path,
            output_dir,
        });
    }
    Ok(())
}

fn write_static_zip_archive(
    output_dir: &Path,
    archive_path: &Path,
    files_found: &[PathBuf],
) -> Result<Vec<PathBuf>, ExportError> {
    if let Some(parent) = archive_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_io(parent)?;
    }

    let file = fs::File::create(archive_path).map_io(archive_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for relative_path in files_found {
        let entry_name = zip_entry_name(relative_path)?;
        zip.start_file(&entry_name, options).map_zip(archive_path)?;
        let mut input = fs::File::open(output_dir.join(relative_path))
            .map_io(output_dir.join(relative_path))?;
        io::copy(&mut input, &mut zip).map_io(archive_path)?;
    }

    zip.finish().map_zip(archive_path)?;
    Ok(files_found.to_vec())
}

fn zip_entry_name(path: &Path) -> Result<String, ExportError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(ExportError::UnsafeZipPath(path.to_path_buf()));
    }

    let mut parts = Vec::new();
    for component in path.components() {
        let Component::Normal(value) = component else {
            return Err(ExportError::UnsafeZipPath(path.to_path_buf()));
        };
        let Some(value) = value.to_str() else {
            return Err(ExportError::UnsafeZipPath(path.to_path_buf()));
        };
        parts.push(value);
    }
    Ok(parts.join("/"))
}

fn ai_usage_manifest(
    project: &plotforge_schema::ProjectData,
    profile: &ExportProfile,
    asset_records: &[AssetRecord],
) -> AiUsageManifest {
    AiUsageManifest {
        manifest_version: "2026-06-08".into(),
        project_id: project.game.id.clone(),
        project_version: project.game.version.clone(),
        export_profile: profile.clone(),
        generated_by: "plotforge-export 0.1.0".into(),
        external_model_calls_during_export: false,
        provider_credentials_included: false,
        raw_provider_responses_included: false,
        private_traces_included: false,
        disclosures: ai_usage_disclosures(asset_records),
        provider_summaries: ai_provider_summaries(asset_records),
        ai_safety_policy: project.ai_safety_policy.clone(),
        notices: vec![
            "Static export packages project content and reachable assets only.".into(),
            "No provider credentials, raw provider responses, or private traces are included."
                .into(),
            "This manifest is an engineering disclosure surface, not a legal compliance guarantee."
                .into(),
        ],
    }
}

fn ai_usage_disclosures(asset_records: &[AssetRecord]) -> Vec<AiUsageDisclosure> {
    let mut disclosures = vec![AiUsageDisclosure {
        content_kind: AiUsageContentKind::Text,
        source_kind: AiUsageSourceKind::ProjectSource,
        summary: "Story text, scene data, and choice text are exported from canonical project source files."
            .into(),
        asset_paths: Vec::new(),
    }];
    for record in asset_records {
        disclosures.push(AiUsageDisclosure {
            content_kind: ai_content_kind(&record.kind),
            source_kind: ai_source_kind(record),
            summary: format!(
                "Reachable {:?} asset included in the static package.",
                record.kind
            ),
            asset_paths: vec![record.export_path.clone()],
        });
    }
    disclosures
}

fn ai_content_kind(kind: &AssetKind) -> AiUsageContentKind {
    match kind {
        AssetKind::Image => AiUsageContentKind::Image,
        AssetKind::Audio => AiUsageContentKind::Audio,
        AssetKind::Voice => AiUsageContentKind::Voice,
        AssetKind::Data => AiUsageContentKind::Data,
    }
}

fn ai_source_kind(record: &AssetRecord) -> AiUsageSourceKind {
    match record.source {
        AssetSourceKind::UserImport => AiUsageSourceKind::UserImport,
        AssetSourceKind::Generated => {
            if record.provider_metadata.as_ref().is_some_and(|metadata| {
                !matches!(
                    metadata.provider.as_str(),
                    "fake-image-provider" | "fake-image" | "plotforge-placeholder"
                )
            }) {
                AiUsageSourceKind::ExternalProvider
            } else {
                AiUsageSourceKind::LocalMockProvider
            }
        }
        AssetSourceKind::Placeholder => AiUsageSourceKind::Placeholder,
        AssetSourceKind::External => AiUsageSourceKind::ExternalProvider,
    }
}

fn ai_provider_summaries(asset_records: &[AssetRecord]) -> Vec<AiProviderSummary> {
    let mut summaries: BTreeMap<(String, Option<String>), AiProviderSummary> = BTreeMap::new();
    for record in asset_records {
        let Some(metadata) = record.provider_metadata.as_ref() else {
            continue;
        };
        let key = (metadata.provider.clone(), metadata.model.clone());
        let summary = summaries.entry(key).or_insert_with(|| AiProviderSummary {
            provider: metadata.provider.clone(),
            model: metadata.model.clone(),
            generated_asset_count: 0,
            fallback_asset_count: 0,
            prompt_hashes: Vec::new(),
        });
        if metadata.fallback_used {
            summary.fallback_asset_count += 1;
        } else {
            summary.generated_asset_count += 1;
        }
        if let Some(prompt_hash) = metadata.prompt_hash.as_ref()
            && !summary.prompt_hashes.contains(prompt_hash)
        {
            summary.prompt_hashes.push(prompt_hash.clone());
        }
    }
    summaries.into_values().collect()
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
    if contains_secret_marker_text(data) {
        return Err(ExportError::SecretMarker("secret marker".into()));
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
    let mut allowed_files = BTreeSet::from([
        PathBuf::from("game.json"),
        PathBuf::from(AI_USAGE_MANIFEST_FILE),
    ]);
    allowed_files.extend(
        PLAYER_PACKAGE_FILES
            .iter()
            .map(|(relative_path, _)| PathBuf::from(relative_path)),
    );
    allowed_files.extend(asset_paths.iter().cloned());
    allowed_files
}

fn normalized_absolute_path(path: &Path) -> Result<PathBuf, ExportError> {
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_io(".")?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute_path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
        }
    }
    Ok(normalized)
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

trait ZipContext<T> {
    fn map_zip(self, path: impl AsRef<Path>) -> Result<T, ExportError>;
}

impl<T> ZipContext<T> for zip::result::ZipResult<T> {
    fn map_zip(self, path: impl AsRef<Path>) -> Result<T, ExportError> {
        self.map_err(|source| ExportError::Zip {
            path: path.as_ref().to_path_buf(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use plotforge_storage::create_demo_project;

    use super::{ExportError, export_static_web, export_static_web_zip};

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
                .join(plotforge_schema::AI_USAGE_MANIFEST_FILE)
                .exists()
        );
        assert!(
            output_dir
                .join("assets/generated/court-crisis-001.png")
                .exists()
        );
        assert!(report.files_written.len() >= 3);
        assert_eq!(report.audit.allowed_files, report.audit.files_found);
    }

    #[test]
    fn exports_static_player_zip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project_path = temp.path().join("project");
        let output_dir = temp.path().join("export");
        let archive_path = temp.path().join("static.zip");
        create_demo_project(&project_path, false).expect("create demo");

        let report =
            export_static_web_zip(&project_path, &output_dir, &archive_path).expect("export zip");

        assert_eq!(report.archive_path, archive_path);
        assert!(archive_path.is_file());
        assert_eq!(
            report.archived_files,
            report.source_report.audit.allowed_files
        );
    }

    #[test]
    fn rejects_zip_archive_inside_export_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project_path = temp.path().join("project");
        let output_dir = temp.path().join("export");
        let archive_path = output_dir.join("static.zip");
        create_demo_project(&project_path, false).expect("create demo");

        let error = export_static_web_zip(&project_path, &output_dir, &archive_path)
            .expect_err("archive inside output dir");

        assert!(matches!(error, ExportError::ArchiveInsideOutputDir { .. }));
        assert!(!archive_path.exists());
    }
}
