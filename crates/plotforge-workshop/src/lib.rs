use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use plotforge_schema::{
    AI_USAGE_MANIFEST_FILE, AiUsageManifest, ExportProfileTarget, WORKSHOP_ITEM_MANIFEST_FILE,
    WorkshopItemPackage, WorkshopPackageFile,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const WORKSHOP_HASH_ALGORITHM: &str = "sha256";

#[derive(Debug, Error)]
pub enum WorkshopPackageError {
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
    #[error("workshop package path is not package-safe: {0}")]
    UnsafePath(String),
    #[error("workshop package file is missing: {0}")]
    MissingFile(PathBuf),
    #[error("workshop package file is empty: {0}")]
    EmptyFile(PathBuf),
    #[error("workshop package file hash mismatch for {path}: expected {expected}, actual {actual}")]
    HashMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error("workshop package file size mismatch for {path}: expected {expected}, actual {actual}")]
    SizeMismatch {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },
    #[error("invalid workshop package metadata: {0}")]
    InvalidMetadata(String),
    #[error("workshop package contains a blocked secret marker: {0}")]
    SecretMarker(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopPackageValidationReport {
    pub package_dir: PathBuf,
    pub manifest: WorkshopItemPackage,
    pub ai_usage: AiUsageManifest,
    pub files: Vec<WorkshopValidatedFile>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopValidatedFile {
    pub path: PathBuf,
    pub content_hash: String,
    pub byte_length: u64,
}

pub fn validate_workshop_package(
    package_dir: impl AsRef<Path>,
) -> Result<WorkshopPackageValidationReport, WorkshopPackageError> {
    let package_dir = package_dir.as_ref();
    let manifest_path = package_dir.join(WORKSHOP_ITEM_MANIFEST_FILE);
    let manifest_data = read_utf8_file(&manifest_path)?;
    assert_no_secret_markers(&manifest_data)?;
    let manifest: WorkshopItemPackage =
        serde_json::from_str(&manifest_data).map_json(&manifest_path)?;
    validate_manifest_metadata(&manifest)?;

    let ai_usage_path = safe_relative_path(&manifest.ai_usage_manifest_path)?;
    if ai_usage_path != Path::new(AI_USAGE_MANIFEST_FILE) {
        return Err(WorkshopPackageError::InvalidMetadata(format!(
            "ai_usage_manifest_path must be {AI_USAGE_MANIFEST_FILE}"
        )));
    }
    let ai_usage_data = read_utf8_file(&package_dir.join(&ai_usage_path))?;
    assert_no_secret_markers(&ai_usage_data)?;
    let ai_usage: AiUsageManifest = serde_json::from_str(&ai_usage_data)
        .map_json(package_dir.join(&ai_usage_path).as_path())?;
    validate_ai_usage_metadata(&manifest, &ai_usage)?;

    let preview_path = safe_relative_path(&manifest.preview_image)?;
    validate_existing_nonempty_file(package_dir, &preview_path)?;
    let content_root = safe_relative_path(&manifest.content_root)?;
    let content_root_path = package_dir.join(&content_root);
    if !content_root_path.is_dir() {
        return Err(WorkshopPackageError::MissingFile(content_root));
    }

    let mut files = Vec::new();
    for record in &manifest.content_files {
        files.push(validate_file_record(package_dir, record)?);
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(WorkshopPackageValidationReport {
        package_dir: package_dir.to_path_buf(),
        manifest,
        ai_usage,
        files,
    })
}

fn validate_manifest_metadata(manifest: &WorkshopItemPackage) -> Result<(), WorkshopPackageError> {
    if manifest.package_id.trim().is_empty() {
        return Err(WorkshopPackageError::InvalidMetadata(
            "package_id must not be empty".into(),
        ));
    }
    if manifest.title.trim().is_empty() {
        return Err(WorkshopPackageError::InvalidMetadata(
            "title must not be empty".into(),
        ));
    }
    if manifest.export_profile.target != ExportProfileTarget::SteamWorkshop {
        return Err(WorkshopPackageError::InvalidMetadata(
            "export_profile.target must be steam_workshop".into(),
        ));
    }
    if manifest.export_profile.platform_submission_ready {
        return Err(WorkshopPackageError::InvalidMetadata(
            "workshop package validation must not claim platform submission readiness".into(),
        ));
    }
    if manifest.export_profile.includes_provider_config
        || manifest.export_profile.includes_private_traces
    {
        return Err(WorkshopPackageError::InvalidMetadata(
            "workshop package profile must not include provider config or private traces".into(),
        ));
    }
    if manifest.content_files.is_empty() {
        return Err(WorkshopPackageError::InvalidMetadata(
            "content_files must not be empty".into(),
        ));
    }
    if !manifest
        .content_files
        .iter()
        .any(|file| file.path == manifest.preview_image)
    {
        return Err(WorkshopPackageError::InvalidMetadata(
            "preview_image must be listed in content_files".into(),
        ));
    }
    let content_root = Path::new(&manifest.content_root);
    for file in &manifest.content_files {
        let file_path = Path::new(&file.path);
        if file.path != manifest.preview_image && !file_path.starts_with(content_root) {
            return Err(WorkshopPackageError::InvalidMetadata(format!(
                "content file must be under content_root or preview_image: {}",
                file.path
            )));
        }
    }
    Ok(())
}

fn validate_ai_usage_metadata(
    manifest: &WorkshopItemPackage,
    ai_usage: &AiUsageManifest,
) -> Result<(), WorkshopPackageError> {
    if ai_usage.export_profile.target != manifest.export_profile.target {
        return Err(WorkshopPackageError::InvalidMetadata(
            "AI usage export profile must match workshop manifest profile".into(),
        ));
    }
    if ai_usage.provider_credentials_included
        || ai_usage.raw_provider_responses_included
        || ai_usage.private_traces_included
    {
        return Err(WorkshopPackageError::InvalidMetadata(
            "AI usage manifest must not include credentials, raw responses, or private traces"
                .into(),
        ));
    }
    Ok(())
}

fn validate_file_record(
    package_dir: &Path,
    record: &WorkshopPackageFile,
) -> Result<WorkshopValidatedFile, WorkshopPackageError> {
    if record.hash_algorithm != WORKSHOP_HASH_ALGORITHM {
        return Err(WorkshopPackageError::InvalidMetadata(format!(
            "unsupported hash_algorithm for {}: {}",
            record.path, record.hash_algorithm
        )));
    }

    let relative_path = safe_relative_path(&record.path)?;
    reject_private_path(&relative_path)?;
    let file_path = package_dir.join(&relative_path);
    let bytes = fs::read(&file_path).map_io(&file_path)?;
    if bytes.is_empty() {
        return Err(WorkshopPackageError::EmptyFile(relative_path));
    }
    assert_no_secret_markers(&String::from_utf8_lossy(&bytes))?;
    let actual_size = bytes.len() as u64;
    if actual_size != record.byte_length {
        return Err(WorkshopPackageError::SizeMismatch {
            path: relative_path,
            expected: record.byte_length,
            actual: actual_size,
        });
    }
    let actual_hash = sha256_hex(&bytes);
    if actual_hash != record.content_hash {
        return Err(WorkshopPackageError::HashMismatch {
            path: relative_path,
            expected: record.content_hash.clone(),
            actual: actual_hash,
        });
    }
    Ok(WorkshopValidatedFile {
        path: PathBuf::from(&record.path),
        content_hash: record.content_hash.clone(),
        byte_length: record.byte_length,
    })
}

fn validate_existing_nonempty_file(
    package_dir: &Path,
    relative_path: &Path,
) -> Result<(), WorkshopPackageError> {
    reject_private_path(relative_path)?;
    let file_path = package_dir.join(relative_path);
    let metadata = fs::metadata(&file_path).map_io(&file_path)?;
    if metadata.len() == 0 {
        return Err(WorkshopPackageError::EmptyFile(relative_path.to_path_buf()));
    }
    Ok(())
}

fn safe_relative_path(path: &str) -> Result<PathBuf, WorkshopPackageError> {
    let path_ref = Path::new(path);
    if path.is_empty() || path_ref.is_absolute() {
        return Err(WorkshopPackageError::UnsafePath(path.into()));
    }
    for component in path_ref.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(WorkshopPackageError::UnsafePath(path.into()));
        }
    }
    Ok(path_ref.to_path_buf())
}

fn reject_private_path(path: &Path) -> Result<(), WorkshopPackageError> {
    let blocked = [
        "traces",
        "providers",
        "provider_config",
        "raw_responses",
        ".plotforge",
    ];
    if path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .any(|part| blocked.contains(&part))
    {
        return Err(WorkshopPackageError::UnsafePath(
            path.to_string_lossy().into_owned(),
        ));
    }
    Ok(())
}

fn read_utf8_file(path: &Path) -> Result<String, WorkshopPackageError> {
    fs::read_to_string(path).map_io(path)
}

fn assert_no_secret_markers(data: &str) -> Result<(), WorkshopPackageError> {
    for marker in [
        "OPENAI_API_KEY",
        "api_key",
        "secret_key",
        "sk-",
        "raw_response",
        "request_id",
        "published_file_id",
        "steam_app_id",
    ] {
        if data.contains(marker) {
            return Err(WorkshopPackageError::SecretMarker(marker.into()));
        }
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

trait IoContext<T> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, WorkshopPackageError>;
}

impl<T> IoContext<T> for Result<T, std::io::Error> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, WorkshopPackageError> {
        self.map_err(|source| WorkshopPackageError::Io {
            path: path.as_ref().to_path_buf(),
            source,
        })
    }
}

trait JsonContext<T> {
    fn map_json(self, path: &Path) -> Result<T, WorkshopPackageError>;
}

impl<T> JsonContext<T> for Result<T, serde_json::Error> {
    fn map_json(self, path: &Path) -> Result<T, WorkshopPackageError> {
        self.map_err(|source| WorkshopPackageError::Json {
            path: path.to_path_buf(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use plotforge_schema::{
        AI_USAGE_MANIFEST_FILE, AiUsageManifest, ExportProfile, WorkshopDraftVisibility,
        WorkshopItemPackage, WorkshopPackageFile,
    };

    use super::{WorkshopPackageError, sha256_hex, validate_workshop_package};

    #[test]
    fn validates_local_workshop_package_without_upload_integration() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_package(temp.path());

        let report = validate_workshop_package(temp.path()).expect("valid package");

        assert_eq!(report.manifest.package_id, "dynasty-embers-workshop-draft");
        assert_eq!(
            report.manifest.export_profile,
            ExportProfile::steam_workshop()
        );
        assert_eq!(
            report.ai_usage.export_profile,
            ExportProfile::steam_workshop()
        );
        assert_eq!(
            report
                .files
                .iter()
                .map(|file| file.path.as_path())
                .collect::<Vec<_>>(),
            vec![
                std::path::Path::new("content/game.json"),
                std::path::Path::new("preview.png")
            ]
        );
    }

    #[test]
    fn workshop_metadata_snapshot_is_stable() {
        let package = sample_package(game_json(), preview_png());
        let snapshot = serde_json::to_string_pretty(&package).expect("package json");

        assert_eq!(
            snapshot,
            format!(
                r#"{{
  "manifest_version": "2026-06-08",
  "package_id": "dynasty-embers-workshop-draft",
  "title": "Dynasty Embers",
  "description": "Offline Workshop package draft for local validation.",
  "visibility": "private_draft",
  "preview_image": "preview.png",
  "content_root": "content",
  "tags": [
    "story-game",
    "strategy"
  ],
  "export_profile": {{
    "id": "steam-workshop",
    "target": "steam_workshop",
    "intent": "Describe a future Workshop metadata/package candidate.",
    "capabilities": [
      "steam_workshop_metadata",
      "static_assets"
    ],
    "requires_network_at_runtime": false,
    "includes_provider_config": false,
    "includes_private_traces": false,
    "platform_submission_ready": false,
    "notes": [
      "Workshop support is a metadata/package exploration profile only.",
      "This profile does not upload content or promise platform approval."
    ]
  }},
  "ai_usage_manifest_path": "ai-usage.json",
  "content_files": [
    {{
      "path": "content/game.json",
      "content_hash": "{}",
      "hash_algorithm": "sha256",
      "byte_length": 27
    }},
    {{
      "path": "preview.png",
      "content_hash": "{}",
      "hash_algorithm": "sha256",
      "byte_length": 13
    }}
  ],
  "notices": [
    "Local package validation only; no upload integration is included.",
    "This package does not promise platform approval or release readiness."
  ]
}}"#,
                sha256_hex(game_json()),
                sha256_hex(preview_png())
            )
        );
    }

    #[test]
    fn rejects_upload_status_fields_and_secret_markers() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_package(temp.path());
        let manifest_path = temp.path().join("workshop-item.json");
        let mut manifest =
            serde_json::to_value(sample_package(game_json(), preview_png())).expect("manifest");
        manifest["published_file_id"] = serde_json::json!("1234567890");
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).expect("json"),
        )
        .expect("write manifest");

        let upload_error = validate_workshop_package(temp.path()).expect_err("upload field");
        assert!(matches!(
            upload_error,
            WorkshopPackageError::SecretMarker(marker) if marker == "published_file_id"
        ));

        let mut secret_manifest = sample_package(game_json(), preview_png());
        secret_manifest.description = "sk-test-secret-marker".into();
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&secret_manifest).expect("json"),
        )
        .expect("write secret manifest");

        let secret_error = validate_workshop_package(temp.path()).expect_err("secret marker");
        assert!(matches!(
            secret_error,
            WorkshopPackageError::SecretMarker(marker) if marker == "sk-"
        ));
    }

    #[test]
    fn rejects_missing_or_mismatched_package_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_package(temp.path());
        fs::remove_file(temp.path().join("content/game.json")).expect("remove content");

        let missing_error = validate_workshop_package(temp.path()).expect_err("missing content");
        assert!(matches!(missing_error, WorkshopPackageError::Io { .. }));

        write_file(
            temp.path().join("content/game.json"),
            b"{\"title\":\"Changed\"}\n",
        );
        let mismatch_error = validate_workshop_package(temp.path()).expect_err("hash mismatch");
        assert!(matches!(
            mismatch_error,
            WorkshopPackageError::SizeMismatch { .. } | WorkshopPackageError::HashMismatch { .. }
        ));
    }

    #[test]
    fn rejects_unhashed_preview_or_content_outside_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_package(temp.path());
        let manifest_path = temp.path().join("workshop-item.json");

        let mut missing_preview = sample_package(game_json(), preview_png());
        missing_preview
            .content_files
            .retain(|file| file.path != missing_preview.preview_image);
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&missing_preview).expect("json"),
        )
        .expect("write missing preview manifest");
        let missing_preview_error =
            validate_workshop_package(temp.path()).expect_err("missing preview record");
        assert!(matches!(
            missing_preview_error,
            WorkshopPackageError::InvalidMetadata(message)
                if message == "preview_image must be listed in content_files"
        ));

        let mut outside_root = sample_package(game_json(), preview_png());
        outside_root.content_files[0].path = "game.json".into();
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&outside_root).expect("json"),
        )
        .expect("write outside root manifest");
        let outside_root_error = validate_workshop_package(temp.path()).expect_err("outside root");
        assert!(matches!(
            outside_root_error,
            WorkshopPackageError::InvalidMetadata(message)
                if message.contains("content file must be under content_root")
        ));
    }

    fn write_valid_package(package_dir: &std::path::Path) {
        fs::create_dir_all(package_dir.join("content")).expect("content dir");
        write_file(package_dir.join("content/game.json"), game_json());
        write_file(package_dir.join("preview.png"), preview_png());
        write_file(
            package_dir.join(AI_USAGE_MANIFEST_FILE),
            serde_json::to_string_pretty(&sample_ai_usage())
                .expect("ai usage")
                .as_bytes(),
        );
        write_file(
            package_dir.join("workshop-item.json"),
            serde_json::to_string_pretty(&sample_package(game_json(), preview_png()))
                .expect("manifest")
                .as_bytes(),
        );
    }

    fn sample_package(game: &[u8], preview: &[u8]) -> WorkshopItemPackage {
        WorkshopItemPackage {
            manifest_version: "2026-06-08".into(),
            package_id: "dynasty-embers-workshop-draft".into(),
            title: "Dynasty Embers".into(),
            description: "Offline Workshop package draft for local validation.".into(),
            visibility: WorkshopDraftVisibility::PrivateDraft,
            preview_image: "preview.png".into(),
            content_root: "content".into(),
            tags: vec!["story-game".into(), "strategy".into()],
            export_profile: ExportProfile::steam_workshop(),
            ai_usage_manifest_path: AI_USAGE_MANIFEST_FILE.into(),
            content_files: vec![
                file_record("content/game.json", game),
                file_record("preview.png", preview),
            ],
            notices: vec![
                "Local package validation only; no upload integration is included.".into(),
                "This package does not promise platform approval or release readiness.".into(),
            ],
        }
    }

    fn sample_ai_usage() -> AiUsageManifest {
        AiUsageManifest {
            manifest_version: "2026-06-08".into(),
            project_id: "dynasty-embers".into(),
            project_version: "0.1.0".into(),
            export_profile: ExportProfile::steam_workshop(),
            generated_by: "plotforge-workshop-test".into(),
            external_model_calls_during_export: false,
            provider_credentials_included: false,
            raw_provider_responses_included: false,
            private_traces_included: false,
            disclosures: Vec::new(),
            provider_summaries: Vec::new(),
            notices: vec!["No provider credentials or raw provider responses included.".into()],
        }
    }

    fn file_record(path: impl Into<String>, bytes: &[u8]) -> WorkshopPackageFile {
        WorkshopPackageFile {
            path: path.into(),
            content_hash: sha256_hex(bytes),
            hash_algorithm: "sha256".into(),
            byte_length: bytes.len() as u64,
        }
    }

    fn write_file(path: impl AsRef<std::path::Path>, bytes: &[u8]) {
        fs::write(path, bytes).expect("write file");
    }

    fn game_json() -> &'static [u8] {
        b"{\"title\":\"Dynasty Embers\"}\n"
    }

    fn preview_png() -> &'static [u8] {
        b"preview-bytes"
    }
}
