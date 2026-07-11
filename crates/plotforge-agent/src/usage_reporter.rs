//! Optional usage-accounting side channel for provider pipelines.
//!
//! The agent crate reports successful real-provider calls through this small
//! object-safe port. Persistence and aggregation remain owned by
//! `plotforge-job`; no ledger state enters agent envelopes, results, or traces.

use plotforge_job::{JobClock, UsageLedger, UsageLedgerError, UsageReport};

pub trait UsageReporter {
    fn report_usage(&mut self, report: UsageReport) -> Result<(), UsageLedgerError>;
}

impl<C> UsageReporter for UsageLedger<C>
where
    C: JobClock,
{
    fn report_usage(&mut self, report: UsageReport) -> Result<(), UsageLedgerError> {
        UsageLedger::report_usage(self, report).map(|_| ())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderUsageIdentity {
    pub provider_id: String,
    pub model: String,
}

impl ProviderUsageIdentity {
    pub fn new(provider_id: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            model: model.into(),
        }
    }
}
