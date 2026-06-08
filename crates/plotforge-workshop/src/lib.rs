use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use plotforge_schema::{
    AI_USAGE_MANIFEST_FILE, AiProviderSummary, AiUsageContentKind, AiUsageManifest,
    AiUsageSourceKind, ExportProfileTarget, SteamSubmissionKitDraft, SteamSubmissionKitRequest,
    WORKSHOP_ITEM_MANIFEST_FILE, WorkshopItemPackage, WorkshopPackageFile,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const WORKSHOP_HASH_ALGORITHM: &str = "sha256";
pub const STEAM_SUBMISSION_CHECKLIST_FILE: &str = "steam-submission-checklist.md";
pub const STEAM_AI_DISCLOSURE_DRAFT_FILE: &str = "steam-ai-disclosure-draft.md";
pub const STEAM_CONTENT_WARNINGS_FILE: &str = "steam-content-warnings.md";
pub const STEAM_PACKAGING_NOTES_FILE: &str = "steam-packaging-notes.md";

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
        checklist_markdown: build_submission_checklist(report, request),
        ai_disclosure_markdown: build_ai_disclosure_draft(report, request),
        content_warnings_markdown: build_content_warnings_draft(report, request),
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
        draft.checklist_markdown.as_str(),
        draft.ai_disclosure_markdown.as_str(),
        draft.content_warnings_markdown.as_str(),
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
    use std::fs;

    use plotforge_schema::{
        AI_USAGE_MANIFEST_FILE, AiUsageContentKind, AiUsageDisclosure, AiUsageManifest,
        AiUsageSourceKind, ExportProfile, SteamSubmissionKitRequest, WorkshopDraftVisibility,
        WorkshopItemPackage, WorkshopPackageFile,
    };

    use super::{
        STEAM_AI_DISCLOSURE_DRAFT_FILE, STEAM_CONTENT_WARNINGS_FILE, STEAM_PACKAGING_NOTES_FILE,
        STEAM_SUBMISSION_CHECKLIST_FILE, WorkshopPackageError, generate_steam_submission_kit,
        sha256_hex, validate_workshop_package, write_steam_submission_kit,
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

        assert_eq!(report.files_written.len(), 4);
        for file in [
            STEAM_SUBMISSION_CHECKLIST_FILE,
            STEAM_AI_DISCLOSURE_DRAFT_FILE,
            STEAM_CONTENT_WARNINGS_FILE,
            STEAM_PACKAGING_NOTES_FILE,
        ] {
            assert!(output.path().join(file).is_file(), "{file} should exist");
        }
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
