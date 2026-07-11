use std::{collections::BTreeMap, fs, path::PathBuf};

use plotforge_schema::{ProviderCostReport, UsageSummary, contains_secret_marker_text};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::JobClock;

const USAGE_LEDGER_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageKind {
    Text,
    Image,
    Tts,
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
        if !path.exists() {
            return Ok(Self {
                clock,
                path: Some(path),
                entries: Vec::new(),
            });
        }

        let content = fs::read_to_string(&path).map_err(|source| UsageLedgerError::ReadFailed {
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

        Ok(Self {
            clock,
            path: Some(path),
            entries: file.entries,
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
        self.entries.push(entry.clone());
        if let Err(error) = self.persist() {
            self.entries.pop();
            return Err(error);
        }
        Ok(entry)
    }

    pub fn persist(&self) -> Result<(), UsageLedgerError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| UsageLedgerError::WriteFailed {
                path: parent.display().to_string(),
                source,
            })?;
        }
        let content = serde_json::to_string_pretty(&UsageLedgerFile {
            version: USAGE_LEDGER_VERSION,
            entries: self.entries.clone(),
        })
        .map_err(|source| UsageLedgerError::SerializeFailed { source })?;
        fs::write(path, content).map_err(|source| UsageLedgerError::WriteFailed {
            path: path.display().to_string(),
            source,
        })
    }
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
    }
    report.input_tokens += entry.input_tokens;
    report.output_tokens += entry.output_tokens;
    report.spent_cost_units += entry.spent_cost_units;
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, fs, rc::Rc};

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

        let summary = ledger.summary();

        assert_eq!(summary.total_input_tokens, 300);
        assert_eq!(summary.total_output_tokens, 75);
        assert_eq!(summary.total_spent_cost_units, 9);
        assert_eq!(summary.by_provider.len(), 2);
        assert_eq!(summary.by_provider["provider-a"].text_calls, 1);
        assert_eq!(summary.by_provider["provider-a"].image_calls, 1);
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
}
