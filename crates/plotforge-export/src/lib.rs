use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Component, Path, PathBuf},
};

use plotforge_media::{AssetRegistry, MediaError};
use plotforge_schema::{
    AI_USAGE_MANIFEST_FILE, AiProviderSummary, AiUsageContentKind, AiUsageDisclosure,
    AiUsageManifest, AiUsageSourceKind, AssetKind, AssetRecord, AssetSourceKind,
    DESKTOP_RUNTIME_DRAFT_FILE, DesktopRuntimeDraft, ExportManifest, ExportProfile,
    WorkshopPackageFile, contains_secret_marker_text,
};
use plotforge_storage::{StorageError, load_project};
use sha2::{Digest, Sha256};
use thiserror::Error;

const GAME_MANIFEST_FILE: &str = "game.json";
const DESKTOP_BUILD_NOTES_FILE: &str = "desktop-build-notes.md";

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
    #[error("export package contains a blocked secret marker: {0}")]
    SecretMarker(String),
    #[error("export package asset path is not package-safe: {0}")]
    UnsafeAssetPath(String),
    #[error("export package contains a disallowed file: {0}")]
    DisallowedPackageFile(PathBuf),
    #[error("export package is missing an expected file: {0}")]
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

struct ExportPayload {
    manifest: ExportManifest,
    ai_usage: AiUsageManifest,
    asset_paths: Vec<PathBuf>,
    asset_record_by_export_path: BTreeMap<PathBuf, AssetRecord>,
}

pub fn export_static_web(
    project_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<ExportReport, ExportError> {
    let project_path = project_path.as_ref();
    let output_dir = output_dir.as_ref();

    let payload = build_export_payload(project_path, ExportProfile::static_web())?;
    let allowed_files = allowed_export_files(&payload.asset_paths);
    let files_written = write_static_runtime_files(project_path, output_dir, &payload)?;
    let audit = audit_export_package(output_dir, &allowed_files)?;

    Ok(ExportReport {
        output_dir: output_dir.to_path_buf(),
        files_written,
        audit,
    })
}

pub fn export_desktop_runtime_draft(
    project_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<ExportReport, ExportError> {
    let project_path = project_path.as_ref();
    let output_dir = output_dir.as_ref();

    let payload = build_export_payload(project_path, ExportProfile::desktop_bundle())?;
    let mut allowed_files = allowed_export_files(&payload.asset_paths);
    allowed_files.insert(PathBuf::from(DESKTOP_BUILD_NOTES_FILE));
    allowed_files.insert(PathBuf::from(DESKTOP_RUNTIME_DRAFT_FILE));

    let mut files_written = write_static_runtime_files(project_path, output_dir, &payload)?;
    let build_notes = desktop_build_notes(&payload.manifest);
    assert_no_secret_markers(&build_notes)?;
    let build_notes_path = output_dir.join(DESKTOP_BUILD_NOTES_FILE);
    fs::write(&build_notes_path, build_notes.clone()).map_io(&build_notes_path)?;
    files_written.push(build_notes_path);

    let mut package_file_paths = allowed_export_files(&payload.asset_paths);
    package_file_paths.insert(PathBuf::from(DESKTOP_BUILD_NOTES_FILE));
    let package_files = package_file_records(output_dir, &package_file_paths)?;
    let draft = desktop_runtime_draft(&payload, build_notes, package_files);
    let draft_data = serde_json::to_string_pretty(&draft).map_err(|source| ExportError::Json {
        path: output_dir.join(DESKTOP_RUNTIME_DRAFT_FILE),
        source,
    })?;
    assert_no_secret_markers(&draft_data)?;
    let draft_path = output_dir.join(DESKTOP_RUNTIME_DRAFT_FILE);
    fs::write(&draft_path, draft_data + "\n").map_io(&draft_path)?;
    files_written.push(draft_path);

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

fn build_export_payload(
    project_path: &Path,
    profile: ExportProfile,
) -> Result<ExportPayload, ExportError> {
    let project = load_project_for_export(project_path)?;
    let mut asset_registry = AssetRegistry::new();
    asset_registry.register_project_assets(project_path, &project)?;
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
    let ai_usage = ai_usage_manifest(&project, &profile, &asset_records);
    let manifest = ExportManifest {
        game: project.game,
        entry_scene: project.story_state.current_scene_key,
        scenes: project.scenes,
        assets,
        asset_records,
        profile,
        ai_usage_manifest_path: AI_USAGE_MANIFEST_FILE.into(),
        generated_by: "plotforge-export 0.1.0".into(),
    };

    Ok(ExportPayload {
        manifest,
        ai_usage,
        asset_paths,
        asset_record_by_export_path,
    })
}

fn load_project_for_export(
    project_path: &Path,
) -> Result<plotforge_schema::ProjectData, ExportError> {
    load_project(project_path).map_err(|source| match source {
        StorageError::Media { source, .. } => ExportError::Media(source),
        source => ExportError::Storage(source),
    })
}

fn write_static_runtime_files(
    project_path: &Path,
    output_dir: &Path,
    payload: &ExportPayload,
) -> Result<Vec<PathBuf>, ExportError> {
    fs::create_dir_all(output_dir).map_io(output_dir)?;
    fs::create_dir_all(output_dir.join("assets/generated"))
        .map_io(output_dir.join("assets/generated"))?;

    let manifest_data =
        serde_json::to_string_pretty(&payload.manifest).map_err(|source| ExportError::Json {
            path: output_dir.join(GAME_MANIFEST_FILE),
            source,
        })?;
    assert_no_secret_markers(&manifest_data)?;
    let ai_usage_data =
        serde_json::to_string_pretty(&payload.ai_usage).map_err(|source| ExportError::Json {
            path: output_dir.join(AI_USAGE_MANIFEST_FILE),
            source,
        })?;
    assert_no_secret_markers(&ai_usage_data)?;

    let manifest_path = output_dir.join(GAME_MANIFEST_FILE);
    let ai_usage_path = output_dir.join(AI_USAGE_MANIFEST_FILE);
    fs::write(&manifest_path, manifest_data + "\n").map_io(&manifest_path)?;
    fs::write(&ai_usage_path, ai_usage_data + "\n").map_io(&ai_usage_path)?;

    let mut files_written = write_player_package(output_dir)?;
    files_written.push(manifest_path);
    files_written.push(ai_usage_path);
    for asset in &payload.asset_paths {
        let record = payload
            .asset_record_by_export_path
            .get(asset)
            .ok_or_else(|| ExportError::MissingPackageFile(asset.clone()))?;
        let asset_path = copy_referenced_asset(project_path, output_dir, record)?;
        files_written.push(asset_path);
    }
    Ok(files_written)
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

fn desktop_runtime_draft(
    payload: &ExportPayload,
    build_notes_markdown: String,
    package_files: Vec<WorkshopPackageFile>,
) -> DesktopRuntimeDraft {
    DesktopRuntimeDraft {
        manifest_version: "2026-06-08".into(),
        project_id: payload.manifest.game.id.clone(),
        project_version: payload.manifest.game.version.clone(),
        export_profile: payload.manifest.profile.clone(),
        static_manifest_path: GAME_MANIFEST_FILE.into(),
        ai_usage_manifest_path: AI_USAGE_MANIFEST_FILE.into(),
        build_notes_markdown,
        package_files,
        runtime_entrypoint: "index.html".into(),
        requires_network_at_runtime: payload.manifest.profile.requires_network_at_runtime,
        provider_credentials_included: false,
        private_traces_included: false,
        raw_provider_responses_included: false,
        notices: vec![
            "Local desktop runtime draft only; no installer or platform submission is generated."
                .into(),
            "No provider credentials, raw provider responses, or private traces are included."
                .into(),
            "Creators must review and build any desktop shell themselves before distribution."
                .into(),
        ],
    }
}

fn desktop_build_notes(manifest: &ExportManifest) -> String {
    format!(
        "\
# Desktop Runtime Draft

Project: {} ({})
Profile: {}
Runtime entrypoint: index.html
Static manifest: game.json
AI usage manifest: {}

This package is a local draft around the no-network static player. It does not include a Tauri build, installer, provider credentials, private traces, raw provider responses, Steamworks upload state, or platform approval evidence.

Next manual steps:
- Review the generated static player package over local HTTP.
- Build and test a desktop shell outside this export package if distribution is desired.
- Recheck platform requirements and store copy before any external submission.
",
        manifest.game.title,
        manifest.game.version,
        manifest.profile.id,
        AI_USAGE_MANIFEST_FILE
    )
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
        notices: ai_usage_notices(profile),
    }
}

fn ai_usage_notices(profile: &ExportProfile) -> Vec<String> {
    let package_notice = if profile.id == "desktop-runtime" {
        "Desktop runtime draft packages project content, the static player, build notes, and reachable assets only."
    } else {
        "Static export packages project content and reachable assets only."
    };
    vec![
        package_notice.into(),
        "No provider credentials, raw provider responses, or private traces are included.".into(),
        "This manifest is an engineering disclosure surface, not a legal compliance guarantee."
            .into(),
    ]
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
                "Reachable {:?} asset included in the export package.",
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
        PathBuf::from(GAME_MANIFEST_FILE),
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

fn package_file_records(
    output_dir: &Path,
    files: &BTreeSet<PathBuf>,
) -> Result<Vec<WorkshopPackageFile>, ExportError> {
    files
        .iter()
        .map(|relative_path| {
            let file_path = output_dir.join(relative_path);
            let bytes = fs::read(&file_path).map_io(&file_path)?;
            Ok(WorkshopPackageFile {
                path: relative_path.to_string_lossy().replace('\\', "/"),
                content_hash: sha256_hex(&bytes),
                hash_algorithm: "sha256".into(),
                byte_length: bytes.len() as u64,
            })
        })
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
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
