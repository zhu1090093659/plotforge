use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

use plotforge_schema::{
    AI_USAGE_MANIFEST_FILE, AiProviderSummary, AiUsageContentKind, AiUsageManifest,
    AiUsageSourceKind, ExportProfileTarget, SteamSubmissionKitDraft, SteamSubmissionKitRequest,
    WORKSHOP_ITEM_MANIFEST_FILE, WorkshopDraftVisibility, WorkshopItemPackage, WorkshopPackageFile,
    WorkshopPublishDraft,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const WORKSHOP_HASH_ALGORITHM: &str = "sha256";
pub const WORKSHOP_LIBRARY_INDEX_FILE: &str = "library-index.json";
pub const WORKSHOP_LIBRARY_ITEMS_DIR: &str = "items";
pub const WORKSHOP_LIBRARY_PACKAGE_DIR: &str = "package";
pub const WORKSHOP_PUBLISH_DRAFT_FILE: &str = "workshop-publish-draft.json";
pub const STEAM_STORE_COPY_DRAFT_FILE: &str = "steam-store-copy-draft.md";
pub const STEAM_SUBMISSION_CHECKLIST_FILE: &str = "steam-submission-checklist.md";
pub const STEAM_AI_DISCLOSURE_DRAFT_FILE: &str = "steam-ai-disclosure-draft.md";
pub const STEAM_CONTENT_WARNINGS_FILE: &str = "steam-content-warnings.md";
pub const STEAM_ASSET_REFERENCES_FILE: &str = "steam-asset-references.md";
pub const STEAM_DIRECT_CHECKLIST_FILE: &str = "steam-direct-checklist.md";
pub const STEAM_CONTENT_SAFETY_CHECKLIST_FILE: &str = "steam-content-safety-checklist.md";
pub const STEAM_PACKAGING_NOTES_FILE: &str = "steam-packaging-notes.md";

const WORKSHOP_LIBRARY_MANIFEST_VERSION: &str = "2026-06-09";
const STEAMWORKS_CONTENT_SURVEY_URL: &str =
    "https://partner.steamgames.com/doc/gettingstarted/contentsurvey";
const STEAMWORKS_APP_FEE_URL: &str = "https://partner.steamgames.com/doc/gettingstarted/appfee";
const STEAMWORKS_REVIEW_PROCESS_URL: &str =
    "https://partner.steamgames.com/doc/store/review_process";
const STEAMWORKS_WORKSHOP_URL: &str = "https://partner.steamgames.com/doc/features/workshop";

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
    #[error("invalid Steam Submission Kit request: {0}")]
    InvalidSubmissionKitRequest(String),
    #[error("workshop package contains a blocked secret marker: {0}")]
    SecretMarker(String),
    #[error("workshop library item already exists: {0}")]
    LibraryItemExists(String),
    #[error("workshop library item not found: {0}")]
    LibraryItemNotFound(String),
    #[error("workshop library item is blocked: {local_id}: {reason}")]
    LibraryItemBlocked { local_id: String, reason: String },
    #[error("invalid workshop library metadata: {0}")]
    InvalidLibraryMetadata(String),
    #[error("Steamworks upload is disabled; enable it explicitly before calling the upload port")]
    SteamworksUploadDisabled,
    #[error("Steamworks upload requires explicit credentials")]
    SteamworksCredentialsMissing,
    #[error("invalid Steamworks upload request: {0}")]
    InvalidSteamworksUpload(String),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteamSubmissionKitWriteReport {
    pub output_dir: PathBuf,
    pub draft: SteamSubmissionKitDraft,
    pub files_written: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopPublishDraftWriteReport {
    pub output_dir: PathBuf,
    pub draft: WorkshopPublishDraft,
    pub files_written: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopLibraryItem {
    pub local_id: String,
    pub package_id: String,
    pub title: String,
    pub package_dir: PathBuf,
    pub blocked: Option<WorkshopLibraryBlock>,
    pub reports: Vec<WorkshopLibraryReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopLibraryBlock {
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopLibraryReport {
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopLibraryImportReport {
    pub item: WorkshopLibraryItem,
    pub validation_report: WorkshopPackageValidationReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopLibraryLoadReport {
    pub item: WorkshopLibraryItem,
    pub validation_report: WorkshopPackageValidationReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopLibraryRemixReport {
    pub source_local_id: String,
    pub item: WorkshopLibraryItem,
    pub validation_report: WorkshopPackageValidationReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkshopLibraryDeleteReport {
    pub local_id: String,
    pub package_dir: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteamworksUploadConfig {
    pub enabled: bool,
    pub credential_label: Option<String>,
    pub app_access_confirmed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteamworksUploadRequest {
    pub package_dir: PathBuf,
    pub draft: WorkshopPublishDraft,
    pub credential_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteamworksUploadReport {
    pub package_id: String,
    pub adapter_name: String,
    pub upload_attempted: bool,
    pub steamworks_api_called: bool,
    pub message: String,
}

pub trait SteamworksUploadPort {
    fn upload_workshop_draft(
        &self,
        request: &SteamworksUploadRequest,
    ) -> Result<SteamworksUploadReport, WorkshopPackageError>;
}

pub struct LocalOnlySteamworksUploadPort;

impl SteamworksUploadPort for LocalOnlySteamworksUploadPort {
    fn upload_workshop_draft(
        &self,
        _request: &SteamworksUploadRequest,
    ) -> Result<SteamworksUploadReport, WorkshopPackageError> {
        Err(WorkshopPackageError::InvalidSteamworksUpload(
            "no Steamworks upload adapter is configured for local-only builds".into(),
        ))
    }
}

pub fn import_workshop_library_package(
    library_root: impl AsRef<Path>,
    package_dir: impl AsRef<Path>,
) -> Result<WorkshopLibraryImportReport, WorkshopPackageError> {
    let library_root = library_root.as_ref();
    let source_report = validate_workshop_package(package_dir)?;
    let local_id = source_report.manifest.package_id.clone();
    validate_library_id(&local_id)?;

    let mut items = read_workshop_library_index(library_root)?;
    if let Some(existing) = items.iter().find(|item| item.local_id == local_id) {
        if let Some(blocked) = &existing.blocked {
            return Err(WorkshopPackageError::LibraryItemBlocked {
                local_id,
                reason: blocked.reason.clone(),
            });
        }
        return Err(WorkshopPackageError::LibraryItemExists(local_id));
    }

    let target_dir = library_item_package_dir(library_root, &local_id)?;
    copy_validated_package(&source_report, &target_dir)?;
    let validation_report = validate_workshop_package(&target_dir)?;
    validate_library_item_matches_report(&local_id, &validation_report)?;

    let item = library_item_from_report(library_root, &local_id, &validation_report, None, vec![])?;
    items.push(item.clone());
    write_workshop_library_index(library_root, &items)?;

    Ok(WorkshopLibraryImportReport {
        item,
        validation_report,
    })
}

pub fn list_workshop_library(
    library_root: impl AsRef<Path>,
) -> Result<Vec<WorkshopLibraryItem>, WorkshopPackageError> {
    read_workshop_library_index(library_root.as_ref())
}

pub fn load_workshop_library_item(
    library_root: impl AsRef<Path>,
    local_id: &str,
) -> Result<WorkshopLibraryLoadReport, WorkshopPackageError> {
    let library_root = library_root.as_ref();
    validate_library_id(local_id)?;
    let item = find_library_item(library_root, local_id)?;
    reject_blocked_library_item(&item)?;

    let validation_report = validate_workshop_package(&item.package_dir)?;
    validate_library_item_matches_report(local_id, &validation_report)?;

    Ok(WorkshopLibraryLoadReport {
        item,
        validation_report,
    })
}

pub fn remix_workshop_library_item(
    library_root: impl AsRef<Path>,
    source_local_id: &str,
    new_local_id: &str,
    new_title: &str,
) -> Result<WorkshopLibraryRemixReport, WorkshopPackageError> {
    let library_root = library_root.as_ref();
    let source = load_workshop_library_item(library_root, source_local_id)?;
    validate_library_id(new_local_id)?;
    validate_library_text(new_title, "remix title")?;

    let mut items = read_workshop_library_index(library_root)?;
    if items.iter().any(|item| item.local_id == new_local_id) {
        return Err(WorkshopPackageError::LibraryItemExists(new_local_id.into()));
    }

    let target_dir = library_item_package_dir(library_root, new_local_id)?;
    copy_validated_package(&source.validation_report, &target_dir)?;
    rewrite_remix_manifest(&target_dir, &source.item, new_local_id, new_title.trim())?;

    let validation_report = validate_workshop_package(&target_dir)?;
    validate_library_item_matches_report(new_local_id, &validation_report)?;
    let item =
        library_item_from_report(library_root, new_local_id, &validation_report, None, vec![])?;
    items.push(item.clone());
    write_workshop_library_index(library_root, &items)?;

    Ok(WorkshopLibraryRemixReport {
        source_local_id: source_local_id.into(),
        item,
        validation_report,
    })
}

pub fn block_workshop_library_item(
    library_root: impl AsRef<Path>,
    local_id: &str,
    reason: &str,
) -> Result<WorkshopLibraryItem, WorkshopPackageError> {
    let library_root = library_root.as_ref();
    validate_library_id(local_id)?;
    validate_library_text(reason, "block reason")?;

    let mut items = read_workshop_library_index(library_root)?;
    let item = items
        .iter_mut()
        .find(|item| item.local_id == local_id)
        .ok_or_else(|| WorkshopPackageError::LibraryItemNotFound(local_id.into()))?;
    item.blocked = Some(WorkshopLibraryBlock {
        reason: reason.trim().into(),
    });
    let updated = item.clone();
    write_workshop_library_index(library_root, &items)?;
    Ok(updated)
}

pub fn report_workshop_library_item(
    library_root: impl AsRef<Path>,
    local_id: &str,
    reason: &str,
) -> Result<WorkshopLibraryItem, WorkshopPackageError> {
    let library_root = library_root.as_ref();
    validate_library_id(local_id)?;
    validate_library_text(reason, "report reason")?;

    let mut items = read_workshop_library_index(library_root)?;
    let item = items
        .iter_mut()
        .find(|item| item.local_id == local_id)
        .ok_or_else(|| WorkshopPackageError::LibraryItemNotFound(local_id.into()))?;
    item.reports.push(WorkshopLibraryReport {
        reason: reason.trim().into(),
    });
    let updated = item.clone();
    write_workshop_library_index(library_root, &items)?;
    Ok(updated)
}

pub fn delete_workshop_library_item(
    library_root: impl AsRef<Path>,
    local_id: &str,
) -> Result<WorkshopLibraryDeleteReport, WorkshopPackageError> {
    let library_root = library_root.as_ref();
    validate_library_id(local_id)?;

    let mut items = read_workshop_library_index(library_root)?;
    let position = items
        .iter()
        .position(|item| item.local_id == local_id)
        .ok_or_else(|| WorkshopPackageError::LibraryItemNotFound(local_id.into()))?;
    let item = items.remove(position);
    if !item.package_dir.is_dir() {
        return Err(WorkshopPackageError::MissingFile(item.package_dir));
    }
    fs::remove_dir_all(&item.package_dir).map_io(&item.package_dir)?;
    write_workshop_library_index(library_root, &items)?;

    Ok(WorkshopLibraryDeleteReport {
        local_id: local_id.into(),
        package_dir: item.package_dir,
    })
}

pub fn generate_workshop_publish_draft(
    report: &WorkshopPackageValidationReport,
) -> Result<WorkshopPublishDraft, WorkshopPackageError> {
    let draft = WorkshopPublishDraft {
        manifest_version: WORKSHOP_LIBRARY_MANIFEST_VERSION.into(),
        package_id: report.manifest.package_id.clone(),
        title: report.manifest.title.clone(),
        description: report.manifest.description.clone(),
        visibility: report.manifest.visibility.clone(),
        preview_image: report.manifest.preview_image.clone(),
        content_root: report.manifest.content_root.clone(),
        tags: report.manifest.tags.clone(),
        ai_usage_manifest_path: report.manifest.ai_usage_manifest_path.clone(),
        package_files: workshop_package_files_from_report(report),
        generated_by: "plotforge-workshop 0.1.0".into(),
        upload_enabled: false,
        requires_explicit_steamworks_credentials: true,
        steamworks_api_called: false,
        notices: vec![
            "Local Workshop publish draft only; no Steamworks API call was made.".into(),
            "A future upload adapter must require explicit credentials, app access confirmation, and user action."
                .into(),
            "This draft does not decide platform acceptance, moderation outcome, or release status."
                .into(),
        ],
    };
    validate_workshop_publish_draft(&draft)?;
    Ok(draft)
}

pub fn write_workshop_publish_draft(
    package_dir: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<WorkshopPublishDraftWriteReport, WorkshopPackageError> {
    let report = validate_workshop_package(package_dir)?;
    let draft = generate_workshop_publish_draft(&report)?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).map_io(output_dir)?;

    let data =
        serde_json::to_string_pretty(&draft).map_err(|source| WorkshopPackageError::Json {
            path: output_dir.join(WORKSHOP_PUBLISH_DRAFT_FILE),
            source,
        })?;
    assert_no_secret_markers(&data)?;
    assert_no_submission_promise_markers(&data)?;
    let output_path = output_dir.join(WORKSHOP_PUBLISH_DRAFT_FILE);
    fs::write(&output_path, data + "\n").map_io(&output_path)?;

    Ok(WorkshopPublishDraftWriteReport {
        output_dir: output_dir.to_path_buf(),
        draft,
        files_written: vec![output_path],
    })
}

pub fn upload_workshop_publish_draft<P: SteamworksUploadPort>(
    port: &P,
    config: &SteamworksUploadConfig,
    package_dir: impl AsRef<Path>,
    draft: &WorkshopPublishDraft,
) -> Result<SteamworksUploadReport, WorkshopPackageError> {
    if !config.enabled {
        return Err(WorkshopPackageError::SteamworksUploadDisabled);
    }
    if !config.app_access_confirmed {
        return Err(WorkshopPackageError::InvalidSteamworksUpload(
            "app access must be explicitly confirmed before upload".into(),
        ));
    }
    let credential_label = config
        .credential_label
        .as_deref()
        .filter(|label| !label.trim().is_empty())
        .ok_or(WorkshopPackageError::SteamworksCredentialsMissing)?;
    validate_library_text(credential_label, "credential label")?;
    validate_workshop_publish_draft(draft)?;
    let validation_report = validate_workshop_package(package_dir.as_ref())?;
    validate_publish_draft_matches_report(draft, &validation_report)?;

    let request = SteamworksUploadRequest {
        package_dir: package_dir.as_ref().to_path_buf(),
        draft: draft.clone(),
        credential_label: credential_label.into(),
    };
    port.upload_workshop_draft(&request)
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

pub fn generate_steam_submission_kit(
    report: &WorkshopPackageValidationReport,
    request: &SteamSubmissionKitRequest,
) -> Result<SteamSubmissionKitDraft, WorkshopPackageError> {
    validate_submission_kit_request(request)?;
    let draft = SteamSubmissionKitDraft {
        manifest_version: "2026-06-08".into(),
        product_name: request.product_name.clone(),
        workshop_package_id: report.manifest.package_id.clone(),
        generated_by: "plotforge-workshop 0.1.0".into(),
        source_workshop_manifest_path: WORKSHOP_ITEM_MANIFEST_FILE.into(),
        store_copy_markdown: build_store_copy_draft(report, request),
        checklist_markdown: build_submission_checklist(report, request),
        ai_disclosure_markdown: build_ai_disclosure_draft(report, request),
        content_warnings_markdown: build_content_warnings_draft(report, request),
        asset_references_markdown: build_asset_references_draft(report, request),
        steam_direct_checklist_markdown: build_steam_direct_checklist(report, request),
        content_safety_checklist_markdown: build_content_safety_checklist(report, request),
        packaging_notes_markdown: build_packaging_notes(report, request),
        official_reference_urls: official_reference_urls(),
        notices: submission_kit_notices(),
    };
    validate_submission_draft(&draft)?;
    Ok(draft)
}

pub fn write_steam_submission_kit(
    package_dir: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    request: &SteamSubmissionKitRequest,
) -> Result<SteamSubmissionKitWriteReport, WorkshopPackageError> {
    let report = validate_workshop_package(package_dir)?;
    let draft = generate_steam_submission_kit(&report, request)?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).map_io(output_dir)?;

    let files = [
        (
            STEAM_STORE_COPY_DRAFT_FILE,
            draft.store_copy_markdown.as_str(),
        ),
        (
            STEAM_SUBMISSION_CHECKLIST_FILE,
            draft.checklist_markdown.as_str(),
        ),
        (
            STEAM_AI_DISCLOSURE_DRAFT_FILE,
            draft.ai_disclosure_markdown.as_str(),
        ),
        (
            STEAM_CONTENT_WARNINGS_FILE,
            draft.content_warnings_markdown.as_str(),
        ),
        (
            STEAM_ASSET_REFERENCES_FILE,
            draft.asset_references_markdown.as_str(),
        ),
        (
            STEAM_DIRECT_CHECKLIST_FILE,
            draft.steam_direct_checklist_markdown.as_str(),
        ),
        (
            STEAM_CONTENT_SAFETY_CHECKLIST_FILE,
            draft.content_safety_checklist_markdown.as_str(),
        ),
        (
            STEAM_PACKAGING_NOTES_FILE,
            draft.packaging_notes_markdown.as_str(),
        ),
    ];
    let mut files_written = Vec::new();
    for (relative_path, contents) in files {
        assert_no_secret_markers(contents)?;
        assert_no_submission_promise_markers(contents)?;
        let output_path = output_dir.join(relative_path);
        fs::write(&output_path, contents).map_io(&output_path)?;
        files_written.push(output_path);
    }

    Ok(SteamSubmissionKitWriteReport {
        output_dir: output_dir.to_path_buf(),
        draft,
        files_written,
    })
}

fn read_workshop_library_index(
    library_root: &Path,
) -> Result<Vec<WorkshopLibraryItem>, WorkshopPackageError> {
    let index_path = library_root.join(WORKSHOP_LIBRARY_INDEX_FILE);
    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let data = read_utf8_file(&index_path)?;
    assert_no_secret_markers(&data)?;
    let value: Value = serde_json::from_str(&data).map_json(&index_path)?;
    let object = value.as_object().ok_or_else(|| {
        WorkshopPackageError::InvalidLibraryMetadata("library index must be a JSON object".into())
    })?;
    let version = required_json_string(object, "manifest_version")?;
    if version != WORKSHOP_LIBRARY_MANIFEST_VERSION {
        return Err(WorkshopPackageError::InvalidLibraryMetadata(format!(
            "unsupported library index manifest_version: {version}"
        )));
    }

    let items_value = object.get("items").ok_or_else(|| {
        WorkshopPackageError::InvalidLibraryMetadata("library index is missing items".into())
    })?;
    let items_array = items_value.as_array().ok_or_else(|| {
        WorkshopPackageError::InvalidLibraryMetadata("library index items must be an array".into())
    })?;
    let mut items = Vec::new();
    for item_value in items_array {
        items.push(parse_library_item(library_root, item_value)?);
    }
    items.sort_by(|left, right| left.local_id.cmp(&right.local_id));
    Ok(items)
}

fn write_workshop_library_index(
    library_root: &Path,
    items: &[WorkshopLibraryItem],
) -> Result<(), WorkshopPackageError> {
    fs::create_dir_all(library_root).map_io(library_root)?;

    let mut sorted = items.to_vec();
    sorted.sort_by(|left, right| left.local_id.cmp(&right.local_id));
    let value = serde_json::json!({
        "manifest_version": WORKSHOP_LIBRARY_MANIFEST_VERSION,
        "items": sorted.iter().map(library_item_to_json).collect::<Vec<_>>(),
    });
    let data =
        serde_json::to_string_pretty(&value).map_err(|source| WorkshopPackageError::Json {
            path: library_root.join(WORKSHOP_LIBRARY_INDEX_FILE),
            source,
        })?;
    assert_no_secret_markers(&data)?;
    let index_path = library_root.join(WORKSHOP_LIBRARY_INDEX_FILE);
    fs::write(&index_path, data).map_io(&index_path)
}

fn parse_library_item(
    library_root: &Path,
    value: &Value,
) -> Result<WorkshopLibraryItem, WorkshopPackageError> {
    let object = value.as_object().ok_or_else(|| {
        WorkshopPackageError::InvalidLibraryMetadata("library item must be a JSON object".into())
    })?;
    let local_id = required_json_string(object, "local_id")?.to_string();
    validate_library_id(&local_id)?;
    let package_id = required_json_string(object, "package_id")?.to_string();
    validate_library_id(&package_id)?;
    let title = required_json_string(object, "title")?.to_string();
    validate_library_text(&title, "library item title")?;

    let blocked = match object.get("blocked") {
        Some(Value::Null) | None => None,
        Some(blocked_value) => {
            let blocked_object = blocked_value.as_object().ok_or_else(|| {
                WorkshopPackageError::InvalidLibraryMetadata(
                    "library item blocked metadata must be an object".into(),
                )
            })?;
            let reason = required_json_string(blocked_object, "reason")?.to_string();
            validate_library_text(&reason, "block reason")?;
            Some(WorkshopLibraryBlock { reason })
        }
    };

    let reports_value = object.get("reports").ok_or_else(|| {
        WorkshopPackageError::InvalidLibraryMetadata("library item is missing reports".into())
    })?;
    let reports_array = reports_value.as_array().ok_or_else(|| {
        WorkshopPackageError::InvalidLibraryMetadata("library item reports must be an array".into())
    })?;
    let mut reports = Vec::new();
    for report_value in reports_array {
        let report_object = report_value.as_object().ok_or_else(|| {
            WorkshopPackageError::InvalidLibraryMetadata(
                "library report metadata must be an object".into(),
            )
        })?;
        let reason = required_json_string(report_object, "reason")?.to_string();
        validate_library_text(&reason, "report reason")?;
        reports.push(WorkshopLibraryReport { reason });
    }

    let package_dir = library_item_package_dir(library_root, &local_id)?;
    Ok(WorkshopLibraryItem {
        local_id,
        package_id,
        title,
        package_dir,
        blocked,
        reports,
    })
}

fn library_item_to_json(item: &WorkshopLibraryItem) -> Value {
    serde_json::json!({
        "local_id": item.local_id,
        "package_id": item.package_id,
        "title": item.title,
        "blocked": item.blocked.as_ref().map(|blocked| {
            serde_json::json!({ "reason": blocked.reason })
        }),
        "reports": item.reports.iter().map(|report| {
            serde_json::json!({ "reason": report.reason })
        }).collect::<Vec<_>>(),
    })
}

fn required_json_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    field: &str,
) -> Result<&'a str, WorkshopPackageError> {
    object.get(field).and_then(Value::as_str).ok_or_else(|| {
        WorkshopPackageError::InvalidLibraryMetadata(format!(
            "library metadata field must be a string: {field}"
        ))
    })
}

fn find_library_item(
    library_root: &Path,
    local_id: &str,
) -> Result<WorkshopLibraryItem, WorkshopPackageError> {
    read_workshop_library_index(library_root)?
        .into_iter()
        .find(|item| item.local_id == local_id)
        .ok_or_else(|| WorkshopPackageError::LibraryItemNotFound(local_id.into()))
}

fn reject_blocked_library_item(item: &WorkshopLibraryItem) -> Result<(), WorkshopPackageError> {
    if let Some(blocked) = &item.blocked {
        return Err(WorkshopPackageError::LibraryItemBlocked {
            local_id: item.local_id.clone(),
            reason: blocked.reason.clone(),
        });
    }
    Ok(())
}

fn copy_validated_package(
    source_report: &WorkshopPackageValidationReport,
    target_dir: &Path,
) -> Result<(), WorkshopPackageError> {
    if target_dir.exists() {
        return Err(WorkshopPackageError::LibraryItemExists(
            target_dir.display().to_string(),
        ));
    }

    let mut package_files = BTreeSet::new();
    package_files.insert(safe_relative_path(WORKSHOP_ITEM_MANIFEST_FILE)?);
    package_files.insert(safe_relative_path(
        &source_report.manifest.ai_usage_manifest_path,
    )?);
    for file in &source_report.files {
        package_files.insert(safe_relative_path(&file.path.to_string_lossy())?);
    }

    for relative_path in package_files {
        reject_private_path(&relative_path)?;
        let source_path = source_report.package_dir.join(&relative_path);
        let target_path = target_dir.join(&relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_io(parent)?;
        }
        fs::copy(&source_path, &target_path).map_io(&target_path)?;
    }
    Ok(())
}

fn rewrite_remix_manifest(
    package_dir: &Path,
    source_item: &WorkshopLibraryItem,
    new_local_id: &str,
    new_title: &str,
) -> Result<(), WorkshopPackageError> {
    let manifest_path = package_dir.join(WORKSHOP_ITEM_MANIFEST_FILE);
    let mut manifest = source_item_manifest(package_dir)?;
    manifest.package_id = new_local_id.into();
    manifest.title = new_title.into();
    manifest.visibility = WorkshopDraftVisibility::PrivateDraft;
    manifest.notices.push(format!(
        "Local remix draft copied from {} ({}); no platform moderation or publishing action was performed.",
        source_item.title, source_item.local_id
    ));
    let data =
        serde_json::to_string_pretty(&manifest).map_err(|source| WorkshopPackageError::Json {
            path: manifest_path.clone(),
            source,
        })?;
    assert_no_secret_markers(&data)?;
    fs::write(&manifest_path, data).map_io(&manifest_path)
}

fn source_item_manifest(package_dir: &Path) -> Result<WorkshopItemPackage, WorkshopPackageError> {
    let manifest_path = package_dir.join(WORKSHOP_ITEM_MANIFEST_FILE);
    let manifest_data = read_utf8_file(&manifest_path)?;
    assert_no_secret_markers(&manifest_data)?;
    serde_json::from_str(&manifest_data).map_json(&manifest_path)
}

fn library_item_from_report(
    library_root: &Path,
    local_id: &str,
    report: &WorkshopPackageValidationReport,
    blocked: Option<WorkshopLibraryBlock>,
    reports: Vec<WorkshopLibraryReport>,
) -> Result<WorkshopLibraryItem, WorkshopPackageError> {
    validate_library_item_matches_report(local_id, report)?;
    Ok(WorkshopLibraryItem {
        local_id: local_id.into(),
        package_id: report.manifest.package_id.clone(),
        title: report.manifest.title.clone(),
        package_dir: library_item_package_dir(library_root, local_id)?,
        blocked,
        reports,
    })
}

fn validate_library_item_matches_report(
    local_id: &str,
    report: &WorkshopPackageValidationReport,
) -> Result<(), WorkshopPackageError> {
    if report.manifest.package_id != local_id {
        return Err(WorkshopPackageError::InvalidLibraryMetadata(format!(
            "library item id {local_id} does not match package id {}",
            report.manifest.package_id
        )));
    }
    Ok(())
}

fn library_item_package_dir(
    library_root: &Path,
    local_id: &str,
) -> Result<PathBuf, WorkshopPackageError> {
    validate_library_id(local_id)?;
    Ok(library_root
        .join(WORKSHOP_LIBRARY_ITEMS_DIR)
        .join(local_id)
        .join(WORKSHOP_LIBRARY_PACKAGE_DIR))
}

fn validate_library_id(local_id: &str) -> Result<(), WorkshopPackageError> {
    let trimmed = local_id.trim();
    if trimmed.is_empty() || trimmed != local_id {
        return Err(WorkshopPackageError::UnsafePath(local_id.into()));
    }
    let path = safe_relative_path(local_id)?;
    if path.components().count() != 1 {
        return Err(WorkshopPackageError::UnsafePath(local_id.into()));
    }
    reject_private_path(&path)
}

fn validate_library_text(text: &str, label: &str) -> Result<(), WorkshopPackageError> {
    if text.trim().is_empty() {
        return Err(WorkshopPackageError::InvalidLibraryMetadata(format!(
            "{label} must not be empty"
        )));
    }
    assert_no_secret_markers(text)
}

fn validate_submission_kit_request(
    request: &SteamSubmissionKitRequest,
) -> Result<(), WorkshopPackageError> {
    if request.product_name.trim().is_empty() {
        return Err(WorkshopPackageError::InvalidSubmissionKitRequest(
            "product_name must not be empty".into(),
        ));
    }
    if request.store_short_description.trim().is_empty() {
        return Err(WorkshopPackageError::InvalidSubmissionKitRequest(
            "store_short_description must not be empty".into(),
        ));
    }
    if request.user_reporting_path.trim().is_empty() {
        return Err(WorkshopPackageError::InvalidSubmissionKitRequest(
            "user_reporting_path must not be empty".into(),
        ));
    }
    if request.moderation_policy.trim().is_empty() {
        return Err(WorkshopPackageError::InvalidSubmissionKitRequest(
            "moderation_policy must not be empty".into(),
        ));
    }

    for text in [
        request.product_name.as_str(),
        request.store_short_description.as_str(),
        request.user_reporting_path.as_str(),
        request.moderation_policy.as_str(),
    ] {
        validate_submission_text(text)?;
    }
    for values in [
        request.screenshot_paths.as_slice(),
        request.capsule_asset_paths.as_slice(),
        request.content_warnings.as_slice(),
        request.safety_guardrails.as_slice(),
        request.build_notes.as_slice(),
    ] {
        for value in values {
            validate_submission_text(value)?;
        }
    }
    if let Some(path) = request.desktop_build_path.as_deref() {
        validate_submission_text(path)?;
        validate_submission_path(path)?;
    }
    for path in request
        .screenshot_paths
        .iter()
        .chain(request.capsule_asset_paths.iter())
    {
        validate_submission_path(path)?;
    }
    Ok(())
}

fn validate_submission_draft(draft: &SteamSubmissionKitDraft) -> Result<(), WorkshopPackageError> {
    for text in [
        draft.manifest_version.as_str(),
        draft.product_name.as_str(),
        draft.workshop_package_id.as_str(),
        draft.generated_by.as_str(),
        draft.source_workshop_manifest_path.as_str(),
        draft.store_copy_markdown.as_str(),
        draft.checklist_markdown.as_str(),
        draft.ai_disclosure_markdown.as_str(),
        draft.content_warnings_markdown.as_str(),
        draft.asset_references_markdown.as_str(),
        draft.steam_direct_checklist_markdown.as_str(),
        draft.content_safety_checklist_markdown.as_str(),
        draft.packaging_notes_markdown.as_str(),
    ] {
        validate_submission_text(text)?;
    }
    for text in draft
        .official_reference_urls
        .iter()
        .chain(draft.notices.iter())
    {
        validate_submission_text(text)?;
    }
    Ok(())
}

fn validate_submission_text(text: &str) -> Result<(), WorkshopPackageError> {
    assert_no_secret_markers(text)?;
    assert_no_submission_promise_markers(text)
}

fn validate_submission_path(path: &str) -> Result<(), WorkshopPackageError> {
    let relative_path = safe_relative_path(path)?;
    reject_private_path(&relative_path)
}

fn validate_workshop_publish_draft(
    draft: &WorkshopPublishDraft,
) -> Result<(), WorkshopPackageError> {
    validate_library_id(&draft.package_id)?;
    for text in [
        draft.manifest_version.as_str(),
        draft.title.as_str(),
        draft.description.as_str(),
        draft.preview_image.as_str(),
        draft.content_root.as_str(),
        draft.ai_usage_manifest_path.as_str(),
        draft.generated_by.as_str(),
    ] {
        validate_submission_text(text)?;
    }
    for value in draft.tags.iter().chain(draft.notices.iter()) {
        validate_submission_text(value)?;
    }
    validate_submission_path(&draft.preview_image)?;
    validate_submission_path(&draft.content_root)?;
    if draft.ai_usage_manifest_path != AI_USAGE_MANIFEST_FILE {
        return Err(WorkshopPackageError::InvalidMetadata(format!(
            "publish draft ai_usage_manifest_path must be {AI_USAGE_MANIFEST_FILE}"
        )));
    }
    if draft.upload_enabled || draft.steamworks_api_called {
        return Err(WorkshopPackageError::InvalidSteamworksUpload(
            "local publish drafts must not claim upload state".into(),
        ));
    }
    if !draft.requires_explicit_steamworks_credentials {
        return Err(WorkshopPackageError::InvalidSteamworksUpload(
            "publish draft must require explicit Steamworks credentials".into(),
        ));
    }
    for file in &draft.package_files {
        validate_file_metadata(file)?;
    }
    Ok(())
}

fn validate_publish_draft_matches_report(
    draft: &WorkshopPublishDraft,
    report: &WorkshopPackageValidationReport,
) -> Result<(), WorkshopPackageError> {
    let manifest = &report.manifest;
    if draft.package_id != manifest.package_id {
        return Err(WorkshopPackageError::InvalidSteamworksUpload(format!(
            "publish draft package id {} does not match package {}",
            draft.package_id, manifest.package_id
        )));
    }
    if draft.title != manifest.title
        || draft.description != manifest.description
        || draft.visibility != manifest.visibility
        || draft.preview_image != manifest.preview_image
        || draft.content_root != manifest.content_root
        || draft.tags != manifest.tags
        || draft.ai_usage_manifest_path != manifest.ai_usage_manifest_path
    {
        return Err(WorkshopPackageError::InvalidSteamworksUpload(
            "publish draft metadata does not match the validated workshop package".into(),
        ));
    }

    let mut expected_files = workshop_package_files_from_report(report);
    let mut draft_files = draft.package_files.clone();
    expected_files.sort_by(|left, right| left.path.cmp(&right.path));
    draft_files.sort_by(|left, right| left.path.cmp(&right.path));
    if draft_files != expected_files {
        return Err(WorkshopPackageError::InvalidSteamworksUpload(
            "publish draft package files do not match the validated workshop package".into(),
        ));
    }

    Ok(())
}

fn workshop_package_files_from_report(
    report: &WorkshopPackageValidationReport,
) -> Vec<WorkshopPackageFile> {
    report
        .files
        .iter()
        .map(|file| WorkshopPackageFile {
            path: file.path.to_string_lossy().replace('\\', "/"),
            content_hash: file.content_hash.clone(),
            hash_algorithm: WORKSHOP_HASH_ALGORITHM.into(),
            byte_length: file.byte_length,
        })
        .collect()
}

fn validate_file_metadata(record: &WorkshopPackageFile) -> Result<(), WorkshopPackageError> {
    if record.hash_algorithm != WORKSHOP_HASH_ALGORITHM {
        return Err(WorkshopPackageError::InvalidMetadata(format!(
            "unsupported hash_algorithm for {}: {}",
            record.path, record.hash_algorithm
        )));
    }
    let relative_path = safe_relative_path(&record.path)?;
    reject_private_path(&relative_path)
}

fn build_store_copy_draft(
    report: &WorkshopPackageValidationReport,
    request: &SteamSubmissionKitRequest,
) -> String {
    let mut output = String::new();
    push_line(&mut output, "# Store Copy Draft");
    push_line(&mut output, "");
    push_line(
        &mut output,
        "Draft support material only. The creator must edit this against implemented gameplay, screenshots, store page limits, and current Steamworks guidance.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Product Name");
    push_line(&mut output, &request.product_name);
    push_line(&mut output, "");
    push_line(&mut output, "## Short Description Draft");
    push_line(&mut output, &request.store_short_description);
    push_line(&mut output, "");
    push_line(&mut output, "## Workshop Package Context");
    push_line(
        &mut output,
        &format!("- Package id: {}", report.manifest.package_id),
    );
    push_line(
        &mut output,
        &format!("- Workshop title: {}", report.manifest.title),
    );
    push_line(
        &mut output,
        &format!("- Workshop description: {}", report.manifest.description),
    );
    push_line(
        &mut output,
        &format!("- Tags: {}", report.manifest.tags.join(", ")),
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Creator Review Prompts");
    for item in [
        "Remove or rewrite any claim that is not visible in the current build.",
        "Match screenshots, capsule assets, and tags to actual gameplay.",
        "Keep Workshop/remix language separate from independent Steam store submission.",
    ] {
        push_line(&mut output, &format!("- {item}"));
    }
    output
}

fn build_submission_checklist(
    report: &WorkshopPackageValidationReport,
    request: &SteamSubmissionKitRequest,
) -> String {
    let mut output = String::new();
    push_line(&mut output, "# Steam Submission Checklist Draft");
    push_line(&mut output, "");
    push_line(
        &mut output,
        "This draft supports creator review only. It does not upload content, complete Steamworks forms, pay fees, submit review, determine compliance, or determine Valve approval.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Product");
    push_line(
        &mut output,
        &format!("- Product name: {}", request.product_name),
    );
    push_line(
        &mut output,
        &format!(
            "- Store short description draft: {}",
            request.store_short_description
        ),
    );
    push_line(
        &mut output,
        &format!("- Workshop package: {}", report.manifest.package_id),
    );
    push_line(
        &mut output,
        &format!("- Source manifest: {WORKSHOP_ITEM_MANIFEST_FILE}"),
    );
    push_line(
        &mut output,
        &format!(
            "- Package files validated locally: {} files / {} bytes",
            report.files.len(),
            package_byte_total(report)
        ),
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Checklist");
    for item in [
        "Confirm Steamworks partner access, app access, and current Steam Direct requirements.",
        "Create or update the Steam store page with only implemented product claims.",
        "Review screenshots, capsule art, tags, mature content, and supported OS selections.",
        "Upload and test the desktop build through the creator-owned Steamworks workflow.",
        "Complete the Steam Content Survey with responsible developer review.",
        "Review the AI disclosure draft against the actual shipped build and content generation policy.",
        "Review content warnings and moderation policy before submitting any store or build review.",
        "Archive the Workshop package, AI usage manifest, and this draft kit with the release evidence.",
    ] {
        push_line(&mut output, &format!("- [ ] {item}"));
    }
    push_line(&mut output, "");
    push_line(&mut output, "## Official References To Recheck");
    for url in official_reference_urls() {
        push_line(&mut output, &format!("- {url}"));
    }
    output
}

fn build_ai_disclosure_draft(
    report: &WorkshopPackageValidationReport,
    request: &SteamSubmissionKitRequest,
) -> String {
    let ai_usage = &report.ai_usage;
    let mut output = String::new();
    push_line(&mut output, "# Steam AI Disclosure Draft");
    push_line(&mut output, "");
    push_line(
        &mut output,
        "Draft support material only. The responsible developer must edit this text against the actual Steam build and current Steamworks Content Survey.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Package Evidence");
    push_line(
        &mut output,
        &format!(
            "- AI usage manifest: {}",
            report.manifest.ai_usage_manifest_path
        ),
    );
    push_line(
        &mut output,
        &format!(
            "- External model calls during export: {}",
            yes_no(ai_usage.external_model_calls_during_export)
        ),
    );
    push_line(
        &mut output,
        &format!(
            "- Provider credentials included: {}",
            yes_no(ai_usage.provider_credentials_included)
        ),
    );
    push_line(
        &mut output,
        &format!(
            "- Raw provider responses included: {}",
            yes_no(ai_usage.raw_provider_responses_included)
        ),
    );
    push_line(
        &mut output,
        &format!(
            "- Private traces included: {}",
            yes_no(ai_usage.private_traces_included)
        ),
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Pre-generated Content Draft");
    if ai_usage.disclosures.is_empty() {
        push_line(
            &mut output,
            "- TODO: Add all player-visible AI-assisted content categories before submission.",
        );
    } else {
        for disclosure in &ai_usage.disclosures {
            push_line(
                &mut output,
                &format!(
                    "- {} / {}: {}{}",
                    ai_content_kind_label(&disclosure.content_kind),
                    ai_source_kind_label(&disclosure.source_kind),
                    disclosure.summary,
                    asset_suffix(&disclosure.asset_paths)
                ),
            );
        }
    }
    push_line(&mut output, "");
    push_line(&mut output, "## Provider Summary");
    if ai_usage.provider_summaries.is_empty() {
        push_line(
            &mut output,
            "- No provider summaries are present in the package manifest.",
        );
    } else {
        for summary in &ai_usage.provider_summaries {
            push_line(&mut output, &provider_summary_line(summary));
        }
    }
    push_line(&mut output, "");
    push_line(&mut output, "## Live-generated Content Draft");
    push_line(
        &mut output,
        "- No live-generated AI content is included in this local package unless the creator adds provider-backed runtime services outside this kit.",
    );
    push_line(&mut output, "- Guardrails to review:");
    append_plain_list(
        &mut output,
        &request.safety_guardrails,
        "TODO: define live-generation guardrails before enabling provider-backed runtime services.",
    );
    push_line(
        &mut output,
        &format!("- User reporting path: {}", request.user_reporting_path),
    );
    push_line(
        &mut output,
        &format!("- Moderation policy: {}", request.moderation_policy),
    );
    output
}

fn build_content_warnings_draft(
    _report: &WorkshopPackageValidationReport,
    request: &SteamSubmissionKitRequest,
) -> String {
    let mut output = String::new();
    push_line(&mut output, "# Content Warnings Draft");
    push_line(&mut output, "");
    push_line(
        &mut output,
        "Draft support material only. The creator must review the shipped build, screenshots, store copy, and current Steam Content Survey before submission.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Creator-provided Warnings");
    append_plain_list(
        &mut output,
        &request.content_warnings,
        "TODO: document player-visible mature content, sensitive themes, and regional restrictions before submission.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Review Notes");
    push_line(
        &mut output,
        "- Check whether any Workshop, remix, or user-generated material changes the warnings.",
    );
    push_line(
        &mut output,
        "- Check AI-assisted narrative, image, audio, and localization content against the final build.",
    );
    push_line(
        &mut output,
        "- Keep warnings descriptive; this kit does not decide ratings, laws, or platform approval.",
    );
    output
}

fn build_asset_references_draft(
    report: &WorkshopPackageValidationReport,
    request: &SteamSubmissionKitRequest,
) -> String {
    let mut output = String::new();
    push_line(&mut output, "# Screenshots And Capsule References");
    push_line(&mut output, "");
    push_line(
        &mut output,
        "Draft support material only. These paths are local references for creator review and are not uploaded by PlotForge.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Preview Image");
    push_line(&mut output, &format!("- {}", report.manifest.preview_image));
    push_line(&mut output, "");
    push_line(&mut output, "## Screenshots");
    append_plain_list(
        &mut output,
        &request.screenshot_paths,
        "TODO: add screenshots that match actual gameplay.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Capsule / Cover Assets");
    append_plain_list(
        &mut output,
        &request.capsule_asset_paths,
        "TODO: add capsule/header assets that match the store page draft.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Validated Package Files");
    for file in &report.files {
        push_line(
            &mut output,
            &format!(
                "- {} ({} bytes, sha256:{})",
                file.path.display(),
                file.byte_length,
                file.content_hash
            ),
        );
    }
    output
}

fn build_steam_direct_checklist(
    _report: &WorkshopPackageValidationReport,
    request: &SteamSubmissionKitRequest,
) -> String {
    let mut output = String::new();
    push_line(&mut output, "# Steam Direct Checklist Draft");
    push_line(&mut output, "");
    push_line(
        &mut output,
        "This checklist is a local reminder for the responsible developer. It does not replace Steamworks account, fee, store, build, review, or release workflows.",
    );
    push_line(&mut output, "");
    for item in [
        "Confirm Steamworks partner account access and permissions.",
        "Review current Steam Direct fee and app setup requirements.",
        "Prepare store page copy, screenshots, capsule assets, tags, supported OS, and build notes.",
        "Upload and test builds through the creator-owned Steamworks workflow outside PlotForge.",
        "Complete Steamworks questionnaires with the final build and policy evidence.",
        "Archive this local kit with the build and store-page evidence used for review.",
    ] {
        push_line(&mut output, &format!("- [ ] {item}"));
    }
    push_line(&mut output, "");
    push_line(&mut output, "## Current Draft Inputs");
    push_line(
        &mut output,
        &format!(
            "- Desktop build draft path: {}",
            request
                .desktop_build_path
                .as_deref()
                .unwrap_or("TODO: add a creator-owned desktop build artifact path")
        ),
    );
    push_line(
        &mut output,
        &format!(
            "- Store short description: {}",
            request.store_short_description
        ),
    );
    output
}

fn build_content_safety_checklist(
    _report: &WorkshopPackageValidationReport,
    request: &SteamSubmissionKitRequest,
) -> String {
    let mut output = String::new();
    push_line(&mut output, "# Content Safety Checklist Draft");
    push_line(&mut output, "");
    push_line(
        &mut output,
        "Draft support material only. The creator must review final player-visible content, UGC policy, and current platform forms before external submission.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Content Warnings To Review");
    append_plain_list(
        &mut output,
        &request.content_warnings,
        "TODO: document mature content, sensitive themes, and regional restrictions.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Live-generation Guardrails");
    append_plain_list(
        &mut output,
        &request.safety_guardrails,
        "TODO: define guardrails before enabling live-generated content.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Reporting And Moderation");
    push_line(
        &mut output,
        &format!("- User reporting path: {}", request.user_reporting_path),
    );
    push_line(
        &mut output,
        &format!("- Moderation policy: {}", request.moderation_policy),
    );
    push_line(
        &mut output,
        "- [ ] Confirm local block/report/delete workflow for Workshop-style packages.",
    );
    push_line(
        &mut output,
        "- [ ] Recheck any remix or user-generated content before external distribution.",
    );
    output
}

fn build_packaging_notes(
    report: &WorkshopPackageValidationReport,
    request: &SteamSubmissionKitRequest,
) -> String {
    let mut output = String::new();
    push_line(&mut output, "# Packaging Notes");
    push_line(&mut output, "");
    push_line(
        &mut output,
        "These notes describe local evidence to gather before the creator uses Steamworks. No Steamworks SDK, upload client, store credential, or submission automation is included.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Build Artifacts");
    match request.desktop_build_path.as_deref() {
        Some(path) => push_line(&mut output, &format!("- Desktop build draft path: {path}")),
        None => push_line(
            &mut output,
            "- TODO: add a creator-owned desktop build artifact path.",
        ),
    }
    push_line(&mut output, "- Screenshots:");
    append_plain_list(
        &mut output,
        &request.screenshot_paths,
        "TODO: add screenshots that match shipped content.",
    );
    push_line(&mut output, "- Capsule assets:");
    append_plain_list(
        &mut output,
        &request.capsule_asset_paths,
        "TODO: add capsule/header assets that match shipped content.",
    );
    push_line(&mut output, "");
    push_line(&mut output, "## Workshop Package Evidence");
    push_line(
        &mut output,
        &format!("- Package id: {}", report.manifest.package_id),
    );
    push_line(
        &mut output,
        &format!("- Content root: {}", report.manifest.content_root),
    );
    push_line(
        &mut output,
        &format!("- Preview image: {}", report.manifest.preview_image),
    );
    push_line(&mut output, "- Validated files:");
    for file in &report.files {
        push_line(
            &mut output,
            &format!(
                "  - {} ({} bytes, {})",
                file.path.display(),
                file.byte_length,
                file.content_hash
            ),
        );
    }
    push_line(&mut output, "");
    push_line(&mut output, "## Build Notes");
    append_plain_list(
        &mut output,
        &request.build_notes,
        "TODO: document install, launch, save-data, and OS-specific checks.",
    );
    output
}

fn append_plain_list(output: &mut String, items: &[String], empty_line: &str) {
    if items.is_empty() {
        push_line(output, &format!("- {empty_line}"));
        return;
    }
    for item in items {
        push_line(output, &format!("- {item}"));
    }
}

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}

fn package_byte_total(report: &WorkshopPackageValidationReport) -> u64 {
    report.files.iter().map(|file| file.byte_length).sum()
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn asset_suffix(asset_paths: &[String]) -> String {
    if asset_paths.is_empty() {
        String::new()
    } else {
        format!(" Assets: {}.", asset_paths.join(", "))
    }
}

fn ai_content_kind_label(kind: &AiUsageContentKind) -> &'static str {
    match kind {
        AiUsageContentKind::Text => "text",
        AiUsageContentKind::Image => "image",
        AiUsageContentKind::Audio => "audio",
        AiUsageContentKind::Voice => "voice",
        AiUsageContentKind::Data => "data",
    }
}

fn ai_source_kind_label(kind: &AiUsageSourceKind) -> &'static str {
    match kind {
        AiUsageSourceKind::ProjectSource => "project source",
        AiUsageSourceKind::LocalMockProvider => "local mock provider",
        AiUsageSourceKind::ExternalProvider => "external provider",
        AiUsageSourceKind::Placeholder => "placeholder",
        AiUsageSourceKind::UserImport => "user import",
    }
}

fn provider_summary_line(summary: &AiProviderSummary) -> String {
    let model = summary.model.as_deref().unwrap_or("unspecified model");
    let prompts = if summary.prompt_hashes.is_empty() {
        "none".into()
    } else {
        summary.prompt_hashes.join(", ")
    };
    format!(
        "- {} / {}: {} generated assets, {} fallback assets, prompt hashes: {}.",
        summary.provider,
        model,
        summary.generated_asset_count,
        summary.fallback_asset_count,
        prompts
    )
}

fn official_reference_urls() -> Vec<String> {
    vec![
        STEAMWORKS_CONTENT_SURVEY_URL.into(),
        STEAMWORKS_APP_FEE_URL.into(),
        STEAMWORKS_REVIEW_PROCESS_URL.into(),
        STEAMWORKS_WORKSHOP_URL.into(),
    ]
}

fn submission_kit_notices() -> Vec<String> {
    vec![
        "Draft support material only; it does not submit content to Steam.".into(),
        "Responsible developers must review current Steamworks requirements before submission."
            .into(),
        "This kit is not legal advice and does not determine compliance, review outcome, or platform approval."
            .into(),
    ]
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

fn assert_no_submission_promise_markers(data: &str) -> Result<(), WorkshopPackageError> {
    let lower = data.to_ascii_lowercase();
    for marker in [
        "one-click steam release",
        "one click steam release",
        "automatic steam store release",
        "auto-publish",
        "guaranteed approval",
        "guarantees approval",
        "legal guarantee",
        "legal guarantees",
        "guaranteed compliance",
        "compliance guaranteed",
        "platform approval guaranteed",
        "valve-approved",
        "legally compliant",
        "release-ready",
        "ready for release",
    ] {
        if lower.contains(marker) {
            return Err(WorkshopPackageError::InvalidSubmissionKitRequest(format!(
                "submission kit text must not claim {marker}"
            )));
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
    use std::{cell::Cell, fs};

    use plotforge_schema::{
        AI_USAGE_MANIFEST_FILE, AiUsageContentKind, AiUsageDisclosure, AiUsageManifest,
        AiUsageSourceKind, ExportProfile, SteamSubmissionKitRequest, WorkshopDraftVisibility,
        WorkshopItemPackage, WorkshopPackageFile, WorkshopPublishDraft,
    };

    use super::{
        LocalOnlySteamworksUploadPort, STEAM_AI_DISCLOSURE_DRAFT_FILE, STEAM_ASSET_REFERENCES_FILE,
        STEAM_CONTENT_SAFETY_CHECKLIST_FILE, STEAM_CONTENT_WARNINGS_FILE,
        STEAM_DIRECT_CHECKLIST_FILE, STEAM_PACKAGING_NOTES_FILE, STEAM_STORE_COPY_DRAFT_FILE,
        STEAM_SUBMISSION_CHECKLIST_FILE, SteamworksUploadConfig, SteamworksUploadPort,
        SteamworksUploadReport, SteamworksUploadRequest, WORKSHOP_ITEM_MANIFEST_FILE,
        WORKSHOP_PUBLISH_DRAFT_FILE, WorkshopPackageError, block_workshop_library_item,
        delete_workshop_library_item, generate_steam_submission_kit,
        generate_workshop_publish_draft, import_workshop_library_package, list_workshop_library,
        load_workshop_library_item, remix_workshop_library_item, report_workshop_library_item,
        sha256_hex, upload_workshop_publish_draft, validate_workshop_package,
        write_steam_submission_kit, write_workshop_publish_draft,
    };

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

    #[test]
    fn steam_submission_checklist_snapshot_is_stable() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_package(temp.path());
        let report = validate_workshop_package(temp.path()).expect("valid package");
        let draft = generate_steam_submission_kit(&report, &sample_submission_kit_request())
            .expect("kit draft");

        assert_eq!(
            draft.checklist_markdown,
            r#"# Steam Submission Checklist Draft

This draft supports creator review only. It does not upload content, complete Steamworks forms, pay fees, submit review, determine compliance, or determine Valve approval.

## Product
- Product name: Dynasty Embers
- Store short description draft: A branching court drama built with PlotForge.
- Workshop package: dynasty-embers-workshop-draft
- Source manifest: workshop-item.json
- Package files validated locally: 2 files / 40 bytes

## Checklist
- [ ] Confirm Steamworks partner access, app access, and current Steam Direct requirements.
- [ ] Create or update the Steam store page with only implemented product claims.
- [ ] Review screenshots, capsule art, tags, mature content, and supported OS selections.
- [ ] Upload and test the desktop build through the creator-owned Steamworks workflow.
- [ ] Complete the Steam Content Survey with responsible developer review.
- [ ] Review the AI disclosure draft against the actual shipped build and content generation policy.
- [ ] Review content warnings and moderation policy before submitting any store or build review.
- [ ] Archive the Workshop package, AI usage manifest, and this draft kit with the release evidence.

## Official References To Recheck
- https://partner.steamgames.com/doc/gettingstarted/contentsurvey
- https://partner.steamgames.com/doc/gettingstarted/appfee
- https://partner.steamgames.com/doc/store/review_process
- https://partner.steamgames.com/doc/features/workshop
"#
        );
    }

    #[test]
    fn steam_ai_disclosure_draft_snapshot_is_stable() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_package(temp.path());
        let report = validate_workshop_package(temp.path()).expect("valid package");
        let draft = generate_steam_submission_kit(&report, &sample_submission_kit_request())
            .expect("kit draft");

        assert_eq!(
            draft.ai_disclosure_markdown,
            r#"# Steam AI Disclosure Draft

Draft support material only. The responsible developer must edit this text against the actual Steam build and current Steamworks Content Survey.

## Package Evidence
- AI usage manifest: ai-usage.json
- External model calls during export: no
- Provider credentials included: no
- Raw provider responses included: no
- Private traces included: no

## Pre-generated Content Draft
- text / project source: Story text and player choices are exported from canonical project source files.
- image / local mock provider: Preview artwork generated by a local mock provider. Assets: preview.png.

## Provider Summary
- fake-image-provider / unspecified model: 1 generated assets, 0 fallback assets, prompt hashes: sha256:preview.

## Live-generated Content Draft
- No live-generated AI content is included in this local package unless the creator adds provider-backed runtime services outside this kit.
- Guardrails to review:
- Keep provider-backed runtime services disabled for this draft package.
- Review any future live output with a blocklist and human moderation queue.
- User reporting path: support@example.invalid
- Moderation policy: Human review of player-visible text and images before submission.
"#
        );
    }

    #[test]
    fn steam_content_warnings_snapshot_is_stable() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_package(temp.path());
        let report = validate_workshop_package(temp.path()).expect("valid package");
        let draft = generate_steam_submission_kit(&report, &sample_submission_kit_request())
            .expect("kit draft");

        assert_eq!(
            draft.content_warnings_markdown,
            r#"# Content Warnings Draft

Draft support material only. The creator must review the shipped build, screenshots, store copy, and current Steam Content Survey before submission.

## Creator-provided Warnings
- Political conflict
- Textual violence

## Review Notes
- Check whether any Workshop, remix, or user-generated material changes the warnings.
- Check AI-assisted narrative, image, audio, and localization content against the final build.
- Keep warnings descriptive; this kit does not decide ratings, laws, or platform approval.
"#
        );
    }

    #[test]
    fn writes_submission_kit_files_after_package_validation() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_package(temp.path());
        let output = tempfile::tempdir().expect("output dir");

        let report = write_steam_submission_kit(
            temp.path(),
            output.path(),
            &sample_submission_kit_request(),
        )
        .expect("write kit");

        assert_eq!(report.files_written.len(), 8);
        for file in [
            STEAM_STORE_COPY_DRAFT_FILE,
            STEAM_SUBMISSION_CHECKLIST_FILE,
            STEAM_AI_DISCLOSURE_DRAFT_FILE,
            STEAM_CONTENT_WARNINGS_FILE,
            STEAM_ASSET_REFERENCES_FILE,
            STEAM_DIRECT_CHECKLIST_FILE,
            STEAM_CONTENT_SAFETY_CHECKLIST_FILE,
            STEAM_PACKAGING_NOTES_FILE,
        ] {
            assert!(output.path().join(file).is_file(), "{file} should exist");
        }
        let store_copy =
            fs::read_to_string(output.path().join(STEAM_STORE_COPY_DRAFT_FILE)).expect("copy");
        assert!(store_copy.contains("Store Copy Draft"));
        assert!(store_copy.contains("A branching court drama built with PlotForge."));
        let steam_direct =
            fs::read_to_string(output.path().join(STEAM_DIRECT_CHECKLIST_FILE)).expect("direct");
        assert!(steam_direct.contains("Steam Direct Checklist Draft"));
        let content_safety =
            fs::read_to_string(output.path().join(STEAM_CONTENT_SAFETY_CHECKLIST_FILE))
                .expect("safety");
        assert!(content_safety.contains("Content Safety Checklist Draft"));
        let packaging_notes =
            fs::read_to_string(output.path().join(STEAM_PACKAGING_NOTES_FILE)).expect("notes");
        assert!(packaging_notes.contains("No Steamworks SDK, upload client"));
        assert!(!packaging_notes.contains("one-click"));
        assert!(!packaging_notes.contains("guaranteed approval"));
        assert_eq!(
            report.draft.workshop_package_id,
            "dynasty-embers-workshop-draft"
        );
    }

    #[test]
    fn rejects_submission_kit_secret_and_release_promise_markers() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_package(temp.path());
        let report = validate_workshop_package(temp.path()).expect("valid package");

        let mut secret = sample_submission_kit_request();
        secret.store_short_description = "sk-test-secret-marker".into();
        let secret_error =
            generate_steam_submission_kit(&report, &secret).expect_err("secret marker");
        assert!(matches!(
            secret_error,
            WorkshopPackageError::SecretMarker(marker) if marker == "sk-"
        ));

        let mut promise = sample_submission_kit_request();
        promise.store_short_description = "Guaranteed approval for Steam.".into();
        let promise_error =
            generate_steam_submission_kit(&report, &promise).expect_err("promise marker");
        assert!(matches!(
            promise_error,
            WorkshopPackageError::InvalidSubmissionKitRequest(message)
                if message.contains("guaranteed approval")
        ));

        let mut legal_claim = sample_submission_kit_request();
        legal_claim.build_notes = vec!["This kit provides a legal guarantee.".into()];
        let legal_claim_error =
            generate_steam_submission_kit(&report, &legal_claim).expect_err("legal marker");
        assert!(matches!(
            legal_claim_error,
            WorkshopPackageError::InvalidSubmissionKitRequest(message)
                if message.contains("legal guarantee")
        ));
    }

    #[test]
    fn generate_workshop_publish_draft_keeps_upload_disabled_and_redaction_safe() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_valid_package(temp.path());
        let report = validate_workshop_package(temp.path()).expect("valid package");

        let draft = generate_workshop_publish_draft(&report).expect("publish draft");

        assert_eq!(draft.manifest_version, "2026-06-09");
        assert_eq!(draft.package_id, "dynasty-embers-workshop-draft");
        assert_eq!(draft.title, "Dynasty Embers");
        assert_eq!(draft.visibility, WorkshopDraftVisibility::PrivateDraft);
        assert!(!draft.upload_enabled);
        assert!(draft.requires_explicit_steamworks_credentials);
        assert!(!draft.steamworks_api_called);
        assert_eq!(draft.package_files.len(), 2);
        assert_eq!(draft.package_files[0].path, "content/game.json");
        assert_eq!(draft.package_files[0].content_hash, sha256_hex(game_json()));
        assert!(
            draft
                .notices
                .iter()
                .any(|notice| notice.contains("no Steamworks API call was made"))
        );

        let data = serde_json::to_string(&draft).expect("draft json");
        assert!(!data.contains("published_file_id"));
        assert!(!data.contains("sk-"));
        assert!(!data.contains("STEAM"));
    }

    #[test]
    fn write_workshop_publish_draft_persists_local_json_without_upload_claims() {
        let package = tempfile::tempdir().expect("package");
        let output = tempfile::tempdir().expect("output");
        write_valid_package(package.path());

        let report = write_workshop_publish_draft(package.path(), output.path())
            .expect("write publish draft");

        let output_path = output.path().join(WORKSHOP_PUBLISH_DRAFT_FILE);
        assert_eq!(report.files_written, vec![output_path.clone()]);
        assert!(output_path.is_file());

        let data = fs::read_to_string(&output_path).expect("read publish draft");
        assert!(data.ends_with('\n'));
        assert!(data.contains(r#""upload_enabled": false"#));
        assert!(data.contains(r#""steamworks_api_called": false"#));
        assert!(!data.contains("published_file_id"));
        assert!(!data.contains("guaranteed approval"));

        let written: WorkshopPublishDraft =
            serde_json::from_str(&data).expect("publish draft json");
        assert_eq!(written, report.draft);
    }

    #[test]
    fn upload_workshop_publish_draft_is_disabled_by_default_before_adapter_call() {
        let package = tempfile::tempdir().expect("package");
        write_valid_package(package.path());
        let draft = validated_publish_draft(package.path());
        let adapter = FakeSteamworksUploadPort::default();
        let disabled = SteamworksUploadConfig {
            enabled: false,
            credential_label: Some("steamworks-local-test-label".into()),
            app_access_confirmed: true,
        };

        let error = upload_workshop_publish_draft(&adapter, &disabled, package.path(), &draft)
            .expect_err("disabled upload");

        assert!(matches!(
            error,
            WorkshopPackageError::SteamworksUploadDisabled
        ));
        assert_eq!(adapter.calls.get(), 0);
    }

    #[test]
    fn upload_workshop_publish_draft_requires_app_access_and_credentials() {
        let package = tempfile::tempdir().expect("package");
        write_valid_package(package.path());
        let draft = validated_publish_draft(package.path());
        let adapter = FakeSteamworksUploadPort::default();

        let missing_access = SteamworksUploadConfig {
            enabled: true,
            credential_label: Some("steamworks-local-test-label".into()),
            app_access_confirmed: false,
        };
        let access_error =
            upload_workshop_publish_draft(&adapter, &missing_access, package.path(), &draft)
                .expect_err("missing app access confirmation");
        assert!(matches!(
            access_error,
            WorkshopPackageError::InvalidSteamworksUpload(message)
                if message.contains("app access must be explicitly confirmed")
        ));

        let missing_credentials = SteamworksUploadConfig {
            enabled: true,
            credential_label: None,
            app_access_confirmed: true,
        };
        let credentials_error =
            upload_workshop_publish_draft(&adapter, &missing_credentials, package.path(), &draft)
                .expect_err("missing credentials");
        assert!(matches!(
            credentials_error,
            WorkshopPackageError::SteamworksCredentialsMissing
        ));
        assert_eq!(adapter.calls.get(), 0);
    }

    #[test]
    fn upload_workshop_publish_draft_rejects_stale_or_tampered_draft_before_adapter_call() {
        let package = tempfile::tempdir().expect("package");
        write_valid_package(package.path());
        let mut draft = validated_publish_draft(package.path());
        draft.package_files[0].content_hash = sha256_hex(b"tampered package file");
        let adapter = FakeSteamworksUploadPort::default();
        let enabled = SteamworksUploadConfig {
            enabled: true,
            credential_label: Some("steamworks-local-test-label".into()),
            app_access_confirmed: true,
        };

        let error = upload_workshop_publish_draft(&adapter, &enabled, package.path(), &draft)
            .expect_err("tampered publish draft");

        assert!(matches!(
            error,
            WorkshopPackageError::InvalidSteamworksUpload(message)
                if message.contains("package files do not match")
        ));
        assert_eq!(adapter.calls.get(), 0);
    }

    #[test]
    fn explicit_fake_adapter_success_stays_local_and_default_port_refuses_upload() {
        let package = tempfile::tempdir().expect("package");
        write_valid_package(package.path());
        let draft = validated_publish_draft(package.path());
        let enabled = SteamworksUploadConfig {
            enabled: true,
            credential_label: Some("steamworks-local-test-label".into()),
            app_access_confirmed: true,
        };

        let local_only_error = upload_workshop_publish_draft(
            &LocalOnlySteamworksUploadPort,
            &enabled,
            package.path(),
            &draft,
        )
        .expect_err("local-only port refuses upload");
        assert!(matches!(
            local_only_error,
            WorkshopPackageError::InvalidSteamworksUpload(message)
                if message.contains("local-only builds")
        ));

        let adapter = FakeSteamworksUploadPort::default();
        let report = upload_workshop_publish_draft(&adapter, &enabled, package.path(), &draft)
            .expect("fake local upload");

        assert_eq!(adapter.calls.get(), 1);
        assert_eq!(report.package_id, "dynasty-embers-workshop-draft");
        assert_eq!(report.adapter_name, "local-fake-workshop-upload-port");
        assert!(report.upload_attempted);
        assert!(!report.steamworks_api_called);
        assert!(report.message.contains("no Steamworks API call was made"));
        assert!(!report.message.contains("steamworks-local-test-label"));
    }

    #[test]
    fn imports_loads_remixes_and_deletes_local_library_items() {
        let package = tempfile::tempdir().expect("package");
        let library = tempfile::tempdir().expect("library");
        write_valid_package(package.path());

        let import = import_workshop_library_package(library.path(), package.path())
            .expect("import package");
        assert_eq!(import.item.local_id, "dynasty-embers-workshop-draft");
        assert!(
            import
                .item
                .package_dir
                .join(WORKSHOP_ITEM_MANIFEST_FILE)
                .is_file()
        );

        let listed = list_workshop_library(library.path()).expect("list library");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "Dynasty Embers");

        let loaded = load_workshop_library_item(library.path(), "dynasty-embers-workshop-draft")
            .expect("load imported item");
        assert_eq!(loaded.validation_report.files.len(), 2);

        let remix = remix_workshop_library_item(
            library.path(),
            "dynasty-embers-workshop-draft",
            "dynasty-embers-remix",
            "Dynasty Embers Remix",
        )
        .expect("remix item");
        assert_eq!(remix.source_local_id, "dynasty-embers-workshop-draft");
        assert_eq!(remix.item.local_id, "dynasty-embers-remix");
        assert_eq!(
            remix.validation_report.manifest.title,
            "Dynasty Embers Remix"
        );
        assert_eq!(
            remix.validation_report.manifest.visibility,
            WorkshopDraftVisibility::PrivateDraft
        );
        assert!(remix.item.package_dir.join("content/game.json").is_file());

        let deleted = delete_workshop_library_item(library.path(), "dynasty-embers-workshop-draft")
            .expect("delete original");
        assert_eq!(deleted.local_id, "dynasty-embers-workshop-draft");
        assert!(!deleted.package_dir.exists());

        let remaining = list_workshop_library(library.path()).expect("list after delete");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].local_id, "dynasty-embers-remix");
        let missing = load_workshop_library_item(library.path(), "dynasty-embers-workshop-draft")
            .expect_err("deleted item is not loadable");
        assert!(matches!(
            missing,
            WorkshopPackageError::LibraryItemNotFound(local_id)
                if local_id == "dynasty-embers-workshop-draft"
        ));
    }

    #[test]
    fn local_library_rejects_unsafe_paths_and_secret_markers() {
        let unsafe_package = tempfile::tempdir().expect("unsafe package");
        let library = tempfile::tempdir().expect("library");
        write_valid_package(unsafe_package.path());
        let manifest_path = unsafe_package.path().join(WORKSHOP_ITEM_MANIFEST_FILE);
        let mut manifest = sample_package(game_json(), preview_png());
        manifest.preview_image = "../secret.png".into();
        manifest.content_files[1].path = "../secret.png".into();
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).expect("json"),
        )
        .expect("write unsafe manifest");

        let unsafe_error = import_workshop_library_package(library.path(), unsafe_package.path())
            .expect_err("unsafe content path");
        assert!(matches!(
            unsafe_error,
            WorkshopPackageError::UnsafePath(path) if path == "../secret.png"
        ));

        let secret_package = tempfile::tempdir().expect("secret package");
        write_secret_content_package(secret_package.path());
        let secret_error = import_workshop_library_package(library.path(), secret_package.path())
            .expect_err("secret package content");
        assert!(matches!(
            secret_error,
            WorkshopPackageError::SecretMarker(marker) if marker == "sk-"
        ));
    }

    #[test]
    fn block_and_report_metadata_remain_local_and_readable() {
        let package = tempfile::tempdir().expect("package");
        let library = tempfile::tempdir().expect("library");
        write_valid_package(package.path());
        import_workshop_library_package(library.path(), package.path()).expect("import package");

        let reported = report_workshop_library_item(
            library.path(),
            "dynasty-embers-workshop-draft",
            "Contains player-reported mature theme metadata.",
        )
        .expect("report item");
        assert_eq!(reported.reports.len(), 1);
        assert_eq!(
            reported.reports[0].reason,
            "Contains player-reported mature theme metadata."
        );

        let blocked = block_workshop_library_item(
            library.path(),
            "dynasty-embers-workshop-draft",
            "Local moderation block pending creator review.",
        )
        .expect("block item");
        assert_eq!(
            blocked.blocked.as_ref().map(|block| block.reason.as_str()),
            Some("Local moderation block pending creator review.")
        );

        let listed = list_workshop_library(library.path()).expect("read metadata");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].reports.len(), 1);
        assert_eq!(
            listed[0]
                .blocked
                .as_ref()
                .map(|block| block.reason.as_str()),
            Some("Local moderation block pending creator review.")
        );
    }

    #[test]
    fn blocked_items_reject_load_remix_and_duplicate_import_explicitly() {
        let package = tempfile::tempdir().expect("package");
        let library = tempfile::tempdir().expect("library");
        write_valid_package(package.path());
        import_workshop_library_package(library.path(), package.path()).expect("import package");
        block_workshop_library_item(
            library.path(),
            "dynasty-embers-workshop-draft",
            "Blocked by local moderation metadata.",
        )
        .expect("block item");

        let load_error =
            load_workshop_library_item(library.path(), "dynasty-embers-workshop-draft")
                .expect_err("blocked load");
        assert!(matches!(
            load_error,
            WorkshopPackageError::LibraryItemBlocked { local_id, reason }
                if local_id == "dynasty-embers-workshop-draft"
                    && reason == "Blocked by local moderation metadata."
        ));

        let remix_error = remix_workshop_library_item(
            library.path(),
            "dynasty-embers-workshop-draft",
            "blocked-remix",
            "Blocked Remix",
        )
        .expect_err("blocked remix");
        assert!(matches!(
            remix_error,
            WorkshopPackageError::LibraryItemBlocked { local_id, .. }
                if local_id == "dynasty-embers-workshop-draft"
        ));

        let import_error = import_workshop_library_package(library.path(), package.path())
            .expect_err("duplicate blocked import");
        assert!(matches!(
            import_error,
            WorkshopPackageError::LibraryItemBlocked { local_id, .. }
                if local_id == "dynasty-embers-workshop-draft"
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

    fn write_secret_content_package(package_dir: &std::path::Path) {
        let secret_game = b"{\"token\":\"sk-test-secret-marker\"}\n";
        fs::create_dir_all(package_dir.join("content")).expect("content dir");
        write_file(package_dir.join("content/game.json"), secret_game);
        write_file(package_dir.join("preview.png"), preview_png());
        write_file(
            package_dir.join(AI_USAGE_MANIFEST_FILE),
            serde_json::to_string_pretty(&sample_ai_usage())
                .expect("ai usage")
                .as_bytes(),
        );
        write_file(
            package_dir.join(WORKSHOP_ITEM_MANIFEST_FILE),
            serde_json::to_string_pretty(&sample_package(secret_game, preview_png()))
                .expect("manifest")
                .as_bytes(),
        );
    }

    fn validated_publish_draft(package_dir: &std::path::Path) -> WorkshopPublishDraft {
        let report = validate_workshop_package(package_dir).expect("valid package");
        generate_workshop_publish_draft(&report).expect("publish draft")
    }

    #[derive(Default)]
    struct FakeSteamworksUploadPort {
        calls: Cell<u32>,
    }

    impl SteamworksUploadPort for FakeSteamworksUploadPort {
        fn upload_workshop_draft(
            &self,
            request: &SteamworksUploadRequest,
        ) -> Result<SteamworksUploadReport, WorkshopPackageError> {
            self.calls.set(self.calls.get() + 1);
            if request.credential_label != "steamworks-local-test-label" {
                return Err(WorkshopPackageError::InvalidSteamworksUpload(
                    "fake adapter received an unexpected credential label".into(),
                ));
            }
            if !request
                .package_dir
                .join(WORKSHOP_ITEM_MANIFEST_FILE)
                .is_file()
            {
                return Err(WorkshopPackageError::InvalidSteamworksUpload(
                    "fake adapter requires a validated local package directory".into(),
                ));
            }
            Ok(SteamworksUploadReport {
                package_id: request.draft.package_id.clone(),
                adapter_name: "local-fake-workshop-upload-port".into(),
                upload_attempted: true,
                steamworks_api_called: false,
                message:
                    "Local fake adapter accepted the redaction-safe draft; no Steamworks API call was made."
                        .into(),
            })
        }
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
            disclosures: vec![
                AiUsageDisclosure {
                    content_kind: AiUsageContentKind::Text,
                    source_kind: AiUsageSourceKind::ProjectSource,
                    summary:
                        "Story text and player choices are exported from canonical project source files."
                            .into(),
                    asset_paths: Vec::new(),
                },
                AiUsageDisclosure {
                    content_kind: AiUsageContentKind::Image,
                    source_kind: AiUsageSourceKind::LocalMockProvider,
                    summary: "Preview artwork generated by a local mock provider.".into(),
                    asset_paths: vec!["preview.png".into()],
                },
            ],
            provider_summaries: vec![plotforge_schema::AiProviderSummary {
                provider: "fake-image-provider".into(),
                model: None,
                generated_asset_count: 1,
                fallback_asset_count: 0,
                prompt_hashes: vec!["sha256:preview".into()],
            }],
            ai_safety_policy: Default::default(),
            notices: vec!["No provider credentials or raw provider responses included.".into()],
        }
    }

    fn sample_submission_kit_request() -> SteamSubmissionKitRequest {
        SteamSubmissionKitRequest {
            product_name: "Dynasty Embers".into(),
            desktop_build_path: Some("builds/dynasty-embers-desktop.zip".into()),
            store_short_description: "A branching court drama built with PlotForge.".into(),
            screenshot_paths: vec!["media/screenshots/court-crisis.png".into()],
            capsule_asset_paths: vec!["media/capsules/header.png".into()],
            content_warnings: vec!["Political conflict".into(), "Textual violence".into()],
            safety_guardrails: vec![
                "Keep provider-backed runtime services disabled for this draft package.".into(),
                "Review any future live output with a blocklist and human moderation queue.".into(),
            ],
            user_reporting_path: "support@example.invalid".into(),
            moderation_policy: "Human review of player-visible text and images before submission."
                .into(),
            build_notes: vec![
                "Test launch, save-data creation, and offline play before Steamworks upload."
                    .into(),
            ],
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
