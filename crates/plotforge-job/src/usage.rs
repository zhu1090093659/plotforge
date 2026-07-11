use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use plotforge_schema::{ProviderCostReport, UsageSummary, contains_secret_marker_text};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::JobClock;

const USAGE_LEDGER_VERSION: u32 = 1;
const UTC_EPOCH_DAY_MS: u64 = 86_400_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageKind {
    Text,
    Image,
    Tts,
    Moderation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UsageLedgerEntry {
    pub provider_id: String,
    pub kind: UsageKind,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub spent_cost_units: u64,
    pub timestamp_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageReport {
    pub provider_id: String,
    pub kind: UsageKind,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub spent_cost_units: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct UsageLedgerFile {
    version: u32,
    entries: Vec<UsageLedgerEntry>,
}

#[derive(Debug, Error)]
pub enum UsageLedgerError {
    #[error("could not resolve the PlotForge user config directory")]
    NoConfigDir,
    #[error("failed to read usage ledger at {path}: {source}")]
    ReadFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse usage ledger at {path}: {source}")]
    ParseFailed {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize usage ledger: {source}")]
    SerializeFailed {
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to write usage ledger at {path}: {source}")]
    WriteFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to lock usage ledger at {path}: {source}")]
    LockFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid usage report: {0}")]
    InvalidReport(&'static str),
    #[error("unsupported usage ledger version: {0}")]
    UnsupportedVersion(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageLedger<C> {
    clock: C,
    path: Option<PathBuf>,
    entries: Vec<UsageLedgerEntry>,
}

impl<C> UsageLedger<C>
where
    C: JobClock,
{
    pub fn new(clock: C) -> Self {
        Self {
            clock,
            path: None,
            entries: Vec::new(),
        }
    }

    pub fn load(clock: C) -> Result<Self, UsageLedgerError> {
        let path = usage_ledger_path().ok_or(UsageLedgerError::NoConfigDir)?;
        Self::load_from(path, clock)
    }

    pub fn load_from(path: impl Into<PathBuf>, clock: C) -> Result<Self, UsageLedgerError> {
        let path = path.into();
        let entries = load_usage_entries_from(&path)?;

        Ok(Self {
            clock,
            path: Some(path),
            entries,
        })
    }

    pub fn entries(&self) -> &[UsageLedgerEntry] {
        &self.entries
    }

    pub fn summary(&self) -> UsageSummary {
        let mut by_provider = BTreeMap::new();
        for entry in &self.entries {
            let report = by_provider
                .entry(entry.provider_id.clone())
                .or_insert_with(|| ProviderCostReport {
                    provider_id: entry.provider_id.clone(),
                    ..ProviderCostReport::default()
                });
            add_entry(report, entry);
        }

        UsageSummary {
            total_input_tokens: self.entries.iter().map(|entry| entry.input_tokens).sum(),
            total_output_tokens: self.entries.iter().map(|entry| entry.output_tokens).sum(),
            total_spent_cost_units: self
                .entries
                .iter()
                .map(|entry| entry.spent_cost_units)
                .sum(),
            by_provider,
        }
    }

    pub fn provider_cost_report(&self, provider_id: &str) -> ProviderCostReport {
        let mut report = ProviderCostReport {
            provider_id: provider_id.to_string(),
            ..ProviderCostReport::default()
        };
        for entry in self
            .entries
            .iter()
            .filter(|entry| entry.provider_id == provider_id)
        {
            add_entry(&mut report, entry);
        }
        report
    }

    pub fn provider_output_tokens_for_utc_day(&self, provider_id: &str, at_ms: u64) -> u64 {
        let target_day = at_ms / UTC_EPOCH_DAY_MS;
        self.entries
            .iter()
            .filter(|entry| entry.provider_id == provider_id)
            .filter(|entry| entry.timestamp_ms / UTC_EPOCH_DAY_MS == target_day)
            .map(|entry| entry.output_tokens)
            .sum()
    }

    pub fn report_usage(
        &mut self,
        report: UsageReport,
    ) -> Result<UsageLedgerEntry, UsageLedgerError> {
        validate_report(&report)?;
        let entry = UsageLedgerEntry {
            provider_id: report.provider_id,
            kind: report.kind,
            model: report.model,
            input_tokens: report.input_tokens,
            output_tokens: report.output_tokens,
            spent_cost_units: report.spent_cost_units,
            timestamp_ms: self.clock.now_ms(),
        };
        let Some(path) = self.path.as_ref() else {
            self.entries.push(entry.clone());
            return Ok(entry);
        };

        // Provider calls finish before they reach this method. Keep the
        // cross-process lock scoped only to reloading and atomically replacing
        // the local ledger so no network operation can hold it.
        self.entries = append_usage_entry_at(path, entry.clone())?;
        Ok(entry)
    }

    /// Atomically replaces a path-backed ledger with this in-memory snapshot.
    /// Normal usage accounting should go through `report_usage`, which reloads
    /// under a cross-process lock before appending and therefore cannot lose a
    /// concurrent writer's entry.
    pub fn persist(&self) -> Result<(), UsageLedgerError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        write_usage_ledger_atomic(path, &self.entries, |_| Ok(()))
    }
}

fn load_usage_entries_from(path: &Path) -> Result<Vec<UsageLedgerEntry>, UsageLedgerError> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path).map_err(|source| UsageLedgerError::ReadFailed {
        path: path.display().to_string(),
        source,
    })?;
    let file: UsageLedgerFile =
        serde_json::from_str(&content).map_err(|source| UsageLedgerError::ParseFailed {
            path: path.display().to_string(),
            source,
        })?;
    if file.version != USAGE_LEDGER_VERSION {
        return Err(UsageLedgerError::UnsupportedVersion(file.version));
    }
    validate_entries(&file.entries)?;
    Ok(file.entries)
}

fn append_usage_entry_at(
    path: &Path,
    entry: UsageLedgerEntry,
) -> Result<Vec<UsageLedgerEntry>, UsageLedgerError> {
    let parent = usage_ledger_parent(path);
    fs::create_dir_all(parent).map_err(|source| UsageLedgerError::WriteFailed {
        path: parent.display().to_string(),
        source,
    })?;
    let lock_path = usage_ledger_lock_path(path);
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|source| UsageLedgerError::LockFailed {
            path: lock_path.display().to_string(),
            source,
        })?;
    lock_file
        .lock()
        .map_err(|source| UsageLedgerError::LockFailed {
            path: lock_path.display().to_string(),
            source,
        })?;

    let mut entries = load_usage_entries_from(path)?;
    entries.push(entry);
    write_usage_ledger_atomic(path, &entries, |_| Ok(()))?;
    Ok(entries)
}

fn write_usage_ledger_atomic(
    path: &Path,
    entries: &[UsageLedgerEntry],
    before_rename: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), UsageLedgerError> {
    let parent = usage_ledger_parent(path);
    fs::create_dir_all(parent).map_err(|source| UsageLedgerError::WriteFailed {
        path: parent.display().to_string(),
        source,
    })?;
    let content = serde_json::to_string_pretty(&UsageLedgerFile {
        version: USAGE_LEDGER_VERSION,
        entries: entries.to_vec(),
    })
    .map_err(|source| UsageLedgerError::SerializeFailed { source })?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|source| {
        UsageLedgerError::WriteFailed {
            path: parent.display().to_string(),
            source,
        }
    })?;
    let temp_path = temp.path().to_path_buf();
    temp.write_all(content.as_bytes())
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|source| UsageLedgerError::WriteFailed {
            path: temp_path.display().to_string(),
            source,
        })?;
    before_rename(&temp_path).map_err(|source| UsageLedgerError::WriteFailed {
        path: temp_path.display().to_string(),
        source,
    })?;
    temp.persist(path)
        .map_err(|error| UsageLedgerError::WriteFailed {
            path: path.display().to_string(),
            source: error.error,
        })?;
    sync_usage_ledger_parent(parent)
}

fn usage_ledger_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn usage_ledger_lock_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("usage.json");
    path.with_file_name(format!(".{file_name}.lock"))
}

#[cfg(unix)]
fn sync_usage_ledger_parent(parent: &Path) -> Result<(), UsageLedgerError> {
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| UsageLedgerError::WriteFailed {
            path: parent.display().to_string(),
            source,
        })
}

/// Rust's standard library has no portable directory-sync primitive on
/// non-Unix targets. The file itself is still flushed before atomic replace.
#[cfg(not(unix))]
fn sync_usage_ledger_parent(_parent: &Path) -> Result<(), UsageLedgerError> {
    Ok(())
}

pub fn usage_ledger_path() -> Option<PathBuf> {
    if let Some(dir) = dirs::config_dir() {
        return Some(dir.join("plotforge").join("usage.json"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".plotforge/usage.json"))
}

fn validate_entries(entries: &[UsageLedgerEntry]) -> Result<(), UsageLedgerError> {
    for entry in entries {
        validate_identity(&entry.provider_id, "provider_id must not be empty")?;
        validate_identity(&entry.model, "model must not be empty")?;
    }
    Ok(())
}

fn validate_report(report: &UsageReport) -> Result<(), UsageLedgerError> {
    validate_identity(&report.provider_id, "provider_id must not be empty")?;
    validate_identity(&report.model, "model must not be empty")
}

fn validate_identity(value: &str, empty_message: &'static str) -> Result<(), UsageLedgerError> {
    if value.trim().is_empty() {
        return Err(UsageLedgerError::InvalidReport(empty_message));
    }
    if contains_secret_marker_text(value) {
        return Err(UsageLedgerError::InvalidReport(
            "provider_id and model must not contain secret markers",
        ));
    }
    Ok(())
}

fn add_entry(report: &mut ProviderCostReport, entry: &UsageLedgerEntry) {
    match entry.kind {
        UsageKind::Text => report.text_calls += 1,
        UsageKind::Image => report.image_calls += 1,
        UsageKind::Tts => report.tts_calls += 1,
        UsageKind::Moderation => report.moderation_calls += 1,
    }
    report.input_tokens += entry.input_tokens;
    report.output_tokens += entry.output_tokens;
    report.spent_cost_units += entry.spent_cost_units;
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        fs, io,
        rc::Rc,
        sync::{Arc, Barrier},
        thread,
    };

    use tempfile::tempdir;

    use super::*;

    #[derive(Clone, Debug)]
    struct FakeClock(Rc<Cell<u64>>);

    impl FakeClock {
        fn new(now_ms: u64) -> Self {
            Self(Rc::new(Cell::new(now_ms)))
        }

        fn set(&self, now_ms: u64) {
            self.0.set(now_ms);
        }
    }

    impl JobClock for FakeClock {
        fn now_ms(&self) -> u64 {
            self.0.get()
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct StaticClock(u64);

    impl JobClock for StaticClock {
        fn now_ms(&self) -> u64 {
            self.0
        }
    }

    #[test]
    fn usage_ledger_reports_usage() {
        let clock = FakeClock::new(1_000);
        let mut ledger = UsageLedger::new(clock);

        let entry = ledger
            .report_usage(report("provider-a", UsageKind::Text, "model-a"))
            .expect("report usage");

        assert_eq!(entry.timestamp_ms, 1_000);
        assert_eq!(ledger.entries(), &[entry]);
        assert_eq!(ledger.summary().total_input_tokens, 100);
    }

    #[test]
    fn moderation_usage_kind_serializes_snake_case() {
        let encoded = serde_json::to_string(&UsageKind::Moderation).expect("serialize kind");
        assert_eq!(encoded, "\"moderation\"");
        let decoded: UsageKind = serde_json::from_str(&encoded).expect("deserialize kind");
        assert_eq!(decoded, UsageKind::Moderation);
    }

    #[test]
    fn usage_summary_aggregates_by_provider() {
        let mut ledger = UsageLedger::new(FakeClock::new(1_000));
        ledger
            .report_usage(report("provider-a", UsageKind::Text, "model-a"))
            .expect("text report");
        ledger
            .report_usage(report("provider-a", UsageKind::Image, "image-a"))
            .expect("image report");
        ledger
            .report_usage(report("provider-b", UsageKind::Tts, "voice-b"))
            .expect("tts report");
        ledger
            .report_usage(report("provider-a", UsageKind::Moderation, "moderation-a"))
            .expect("moderation report");

        let summary = ledger.summary();

        assert_eq!(summary.total_input_tokens, 400);
        assert_eq!(summary.total_output_tokens, 100);
        assert_eq!(summary.total_spent_cost_units, 12);
        assert_eq!(summary.by_provider.len(), 2);
        assert_eq!(summary.by_provider["provider-a"].text_calls, 1);
        assert_eq!(summary.by_provider["provider-a"].image_calls, 1);
        assert_eq!(summary.by_provider["provider-a"].moderation_calls, 1);
        assert_eq!(summary.by_provider["provider-b"].tts_calls, 1);
    }

    #[test]
    fn usage_ledger_provider_cost_report_filters_by_id() {
        let mut ledger = UsageLedger::new(FakeClock::new(1_000));
        ledger
            .report_usage(report("provider-a", UsageKind::Text, "model-a"))
            .expect("provider a report");
        ledger
            .report_usage(report("provider-b", UsageKind::Image, "image-b"))
            .expect("provider b report");

        let report = ledger.provider_cost_report("provider-a");

        assert_eq!(report.provider_id, "provider-a");
        assert_eq!(report.text_calls, 1);
        assert_eq!(report.image_calls, 0);
        assert_eq!(report.input_tokens, 100);
        assert_eq!(report.output_tokens, 25);
        assert_eq!(report.spent_cost_units, 3);
    }

    #[test]
    fn usage_ledger_provider_output_tokens_filters_by_utc_epoch_day() {
        const DAY_MS: u64 = 86_400_000;
        let clock = FakeClock::new(DAY_MS - 1);
        let mut ledger = UsageLedger::new(clock.clone());
        ledger
            .report_usage(report("provider-a", UsageKind::Text, "model-a"))
            .expect("previous day report");
        clock.set(DAY_MS);
        ledger
            .report_usage(report("provider-a", UsageKind::Text, "model-a"))
            .expect("current day report");
        ledger
            .report_usage(report("provider-b", UsageKind::Text, "model-b"))
            .expect("other provider report");

        assert_eq!(
            ledger.provider_output_tokens_for_utc_day("provider-a", DAY_MS + 1),
            25
        );
        assert_eq!(
            ledger.provider_output_tokens_for_utc_day("provider-a", DAY_MS - 1),
            25
        );
    }

    #[test]
    fn usage_ledger_missing_file_is_empty_and_persists_on_report() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("config").join("usage.json");
        let mut ledger =
            UsageLedger::load_from(&path, FakeClock::new(2_000)).expect("missing ledger is empty");

        assert!(ledger.entries().is_empty());
        ledger
            .report_usage(report("provider-a", UsageKind::Image, "image-model"))
            .expect("persist report");

        assert!(path.exists());
        let reloaded = UsageLedger::load_from(path, FakeClock::new(3_000)).expect("reload ledger");
        assert_eq!(reloaded.entries().len(), 1);
        assert_eq!(reloaded.entries()[0].timestamp_ms, 2_000);
    }

    #[test]
    fn stale_path_backed_handles_reload_before_each_append() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("usage.json");
        let mut first =
            UsageLedger::load_from(&path, StaticClock(1_000)).expect("load first handle");
        let mut second =
            UsageLedger::load_from(&path, StaticClock(2_000)).expect("load second handle");

        first
            .report_usage(report("provider-a", UsageKind::Text, "model-a"))
            .expect("first report");
        second
            .report_usage(report("provider-b", UsageKind::Image, "model-b"))
            .expect("second report");

        assert_eq!(
            second.entries().len(),
            2,
            "stale handle refreshes in memory"
        );
        let reloaded = UsageLedger::load_from(path, StaticClock(3_000)).expect("reload ledger");
        assert_eq!(reloaded.entries().len(), 2);
        assert_eq!(reloaded.entries()[0].provider_id, "provider-a");
        assert_eq!(reloaded.entries()[1].provider_id, "provider-b");
    }

    #[test]
    fn concurrent_path_backed_reports_preserve_every_entry() {
        const WRITERS: usize = 8;
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("usage.json");
        let barrier = Arc::new(Barrier::new(WRITERS));
        let mut writers = Vec::new();

        for index in 0..WRITERS {
            let mut ledger = UsageLedger::load_from(&path, StaticClock(index as u64))
                .expect("load independent handle");
            let barrier = Arc::clone(&barrier);
            writers.push(thread::spawn(move || {
                barrier.wait();
                ledger
                    .report_usage(report(
                        &format!("provider-{index}"),
                        UsageKind::Moderation,
                        "moderation-model",
                    ))
                    .expect("concurrent report");
            }));
        }
        for writer in writers {
            writer.join().expect("writer thread");
        }

        let reloaded = UsageLedger::load_from(path, StaticClock(10_000)).expect("reload ledger");
        assert_eq!(reloaded.entries().len(), WRITERS);
        let mut provider_ids = reloaded
            .entries()
            .iter()
            .map(|entry| entry.provider_id.as_str())
            .collect::<Vec<_>>();
        provider_ids.sort_unstable();
        assert_eq!(
            provider_ids,
            (0..WRITERS)
                .map(|index| format!("provider-{index}"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn report_usage_rejects_corruption_created_after_handle_load() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("usage.json");
        let mut ledger =
            UsageLedger::load_from(&path, StaticClock(1_000)).expect("load clean handle");
        fs::write(&path, "not-json").expect("corrupt current ledger");

        let error = ledger
            .report_usage(report("provider-a", UsageKind::Text, "model-a"))
            .expect_err("corrupt current ledger must fail explicitly");

        assert!(matches!(error, UsageLedgerError::ParseFailed { .. }));
        assert!(ledger.entries().is_empty());
        assert_eq!(
            fs::read_to_string(path).expect("read corruption"),
            "not-json"
        );
    }

    #[test]
    fn failed_atomic_replace_preserves_previous_ledger() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("usage.json");
        let previous = UsageLedgerEntry {
            provider_id: "provider-before".into(),
            kind: UsageKind::Text,
            model: "model-before".into(),
            input_tokens: 1,
            output_tokens: 2,
            spent_cost_units: 3,
            timestamp_ms: 4,
        };
        write_usage_ledger_atomic(&path, std::slice::from_ref(&previous), |_| Ok(()))
            .expect("write baseline");
        let baseline = fs::read(&path).expect("read baseline");
        let files_before = directory_file_names(dir.path());
        let replacement = UsageLedgerEntry {
            provider_id: "provider-after".into(),
            ..previous
        };

        let error = write_usage_ledger_atomic(&path, &[replacement], |_| {
            Err(io::Error::other("simulated pre-rename failure"))
        })
        .expect_err("replace must fail");

        assert!(matches!(error, UsageLedgerError::WriteFailed { .. }));
        assert_eq!(fs::read(&path).expect("old ledger survives"), baseline);
        assert_eq!(directory_file_names(dir.path()), files_before);
    }

    #[test]
    fn usage_ledger_corrupt_file_errors() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("usage.json");
        fs::write(&path, "not-json").expect("write corrupt ledger");

        let error =
            UsageLedger::load_from(path, FakeClock::new(0)).expect_err("corrupt ledger must error");

        assert!(matches!(error, UsageLedgerError::ParseFailed { .. }));
    }

    #[test]
    fn usage_ledger_injected_clock_is_deterministic() {
        let clock = FakeClock::new(10);
        let mut ledger = UsageLedger::new(clock.clone());
        let first = ledger
            .report_usage(report("provider-a", UsageKind::Text, "model-a"))
            .expect("first report");
        clock.set(20);
        let second = ledger
            .report_usage(report("provider-a", UsageKind::Tts, "voice-a"))
            .expect("second report");

        assert_eq!(first.timestamp_ms, 10);
        assert_eq!(second.timestamp_ms, 20);
    }

    #[test]
    fn usage_ledger_rejects_secret_markers() {
        let mut ledger = UsageLedger::new(FakeClock::new(0));
        let error = ledger
            .report_usage(report("provider-a", UsageKind::Text, "sk-secret"))
            .expect_err("secret marker must be rejected");

        assert!(matches!(error, UsageLedgerError::InvalidReport(_)));
        assert!(ledger.entries().is_empty());
    }

    fn report(provider_id: &str, kind: UsageKind, model: &str) -> UsageReport {
        UsageReport {
            provider_id: provider_id.into(),
            kind,
            model: model.into(),
            input_tokens: 100,
            output_tokens: 25,
            spent_cost_units: 3,
        }
    }

    fn directory_file_names(path: &Path) -> Vec<String> {
        let mut names = fs::read_dir(path)
            .expect("read directory")
            .map(|entry| {
                entry
                    .expect("directory entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        names.sort();
        names
    }
}
