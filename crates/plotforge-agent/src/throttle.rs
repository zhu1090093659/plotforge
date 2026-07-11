use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use plotforge_job::{
    SystemJobClock, ThrottleConfig, ThrottleError, ThrottleGate, ThrottlePermit, UsageLedger,
    UsageLedgerError,
};

#[derive(Clone, Debug)]
pub(crate) struct UsageLedgerLoader {
    path: Option<PathBuf>,
}

impl UsageLedgerLoader {
    pub(crate) fn user_global() -> Self {
        Self { path: None }
    }

    #[cfg(test)]
    pub(crate) fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Some(path.into()),
        }
    }

    fn load(&self) -> Result<UsageLedger<SystemJobClock>, UsageLedgerError> {
        match self.path.as_ref() {
            Some(path) => UsageLedger::load_from(path, SystemJobClock),
            None => UsageLedger::load(SystemJobClock),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProviderThrottle {
    gate: ThrottleGate<SystemJobClock>,
    usage_loader: UsageLedgerLoader,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum ProviderThrottleScope {
    Text,
    Image,
    Tts,
    Moderation,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct ProviderThrottleKey {
    scope: ProviderThrottleScope,
    provider_id: String,
    provider_config_hash: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProviderThrottleError {
    #[error("{0}")]
    Gate(#[from] ThrottleError),
    #[error("failed to load the usage ledger for daily token budget enforcement: {0}")]
    Usage(#[from] UsageLedgerError),
    #[error("provider throttle registry state is unavailable after a concurrent panic")]
    RegistryState,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ThrottleFailure {
    RateLimit {
        retry_after_ms: Option<u64>,
        message: String,
    },
    BudgetExceeded {
        spent: u64,
        budget: u64,
    },
    Preflight {
        message: String,
    },
}

impl ProviderThrottleError {
    pub(crate) fn into_failure(self) -> ThrottleFailure {
        match self {
            Self::Gate(ThrottleError::RateLimited { retry_after_ms }) => {
                ThrottleFailure::RateLimit {
                    retry_after_ms: Some(retry_after_ms),
                    message: "provider request rate is limited by local quota".into(),
                }
            }
            Self::Gate(ThrottleError::ConcurrencyCapped { max_concurrency }) => {
                ThrottleFailure::RateLimit {
                    retry_after_ms: None,
                    message: format!(
                        "provider concurrency is capped by local quota: max_concurrency={max_concurrency}"
                    ),
                }
            }
            Self::Gate(ThrottleError::BudgetExceeded { spent, budget }) => {
                ThrottleFailure::BudgetExceeded { spent, budget }
            }
            Self::Gate(error) => ThrottleFailure::Preflight {
                message: error.to_string(),
            },
            Self::Usage(error) => ThrottleFailure::Preflight {
                message: format!(
                    "failed to load the usage ledger for daily token budget enforcement: {error}"
                ),
            },
            Self::RegistryState => ThrottleFailure::Preflight {
                message: "provider throttle registry state is unavailable after a concurrent panic"
                    .into(),
            },
        }
    }
}

impl ProviderThrottle {
    /// Returns the process-shared gate for a provider kind, stable id, and
    /// redaction-safe upstream config hash. Studio rebuilds provider adapters
    /// between turns, so keeping this state outside the adapter is required
    /// for RPM and concurrency quotas to span those rebuilds. Quota edits
    /// reconfigure the existing gate in place so in-flight permits and
    /// consumed rate capacity are not reset.
    pub(crate) fn shared_from_config(
        scope: ProviderThrottleScope,
        provider_config_hash: String,
        config: ThrottleConfig,
    ) -> Result<Option<Self>, ProviderThrottleError> {
        if config.max_concurrency.is_none()
            && config.requests_per_minute.is_none()
            && config.daily_token_budget.is_none()
        {
            return Ok(None);
        }

        static REGISTRY: OnceLock<
            Mutex<HashMap<ProviderThrottleKey, ThrottleGate<SystemJobClock>>>,
        > = OnceLock::new();
        let registry = REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
        let mut registry = registry
            .lock()
            .map_err(|_| ProviderThrottleError::RegistryState)?;
        let key = ProviderThrottleKey {
            scope,
            provider_id: config.provider_id.clone(),
            provider_config_hash,
        };
        if let Some(gate) = registry.get(&key) {
            gate.reconfigure(config)?;
            return Ok(Some(Self {
                gate: gate.clone(),
                usage_loader: UsageLedgerLoader::user_global(),
            }));
        }

        let gate = ThrottleGate::new(config, SystemJobClock)?;
        registry.insert(key, gate.clone());
        Ok(Some(Self {
            gate,
            usage_loader: UsageLedgerLoader::user_global(),
        }))
    }

    #[cfg(test)]
    fn from_config_with_loader(
        config: ThrottleConfig,
        usage_loader: UsageLedgerLoader,
    ) -> Result<Option<Self>, ProviderThrottleError> {
        if config.max_concurrency.is_none()
            && config.requests_per_minute.is_none()
            && config.daily_token_budget.is_none()
        {
            return Ok(None);
        }
        Ok(Some(Self {
            gate: ThrottleGate::new(config, SystemJobClock)?,
            usage_loader,
        }))
    }

    #[cfg(test)]
    pub(crate) fn from_config_with_usage_path(
        config: ThrottleConfig,
        path: impl Into<PathBuf>,
    ) -> Result<Option<Self>, ProviderThrottleError> {
        Self::from_config_with_loader(config, UsageLedgerLoader::from_path(path))
    }

    pub(crate) fn acquire(&self) -> Result<ThrottlePermit, ProviderThrottleError> {
        let ledger = self
            .gate
            .requires_daily_usage()?
            .then(|| self.usage_loader.load())
            .transpose()?;
        self.gate
            .acquire(ledger.as_ref().map(|value| value as _))
            .map_err(ProviderThrottleError::Gate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_quota_keeps_the_legacy_path_and_does_not_read_usage() {
        let directory = tempfile::tempdir().expect("tempdir");
        let corrupt = directory.path().join("usage.json");
        std::fs::write(&corrupt, "{not-json").expect("write corrupt usage");

        let throttle = ProviderThrottle::from_config_with_usage_path(
            ThrottleConfig {
                provider_id: "provider-a".into(),
                ..ThrottleConfig::default()
            },
            corrupt,
        )
        .expect("no quota is accepted");

        assert!(throttle.is_none());
    }

    #[test]
    fn daily_budget_loads_current_ledger_and_surfaces_corruption() {
        let directory = tempfile::tempdir().expect("tempdir");
        let corrupt = directory.path().join("usage.json");
        std::fs::write(&corrupt, "{not-json").expect("write corrupt usage");
        let throttle = ProviderThrottle::from_config_with_usage_path(
            ThrottleConfig {
                provider_id: "provider-a".into(),
                daily_token_budget: Some(1),
                ..ThrottleConfig::default()
            },
            corrupt,
        )
        .expect("valid throttle")
        .expect("daily budget creates gate");

        let failure = throttle
            .acquire()
            .expect_err("corrupt ledger must fail")
            .into_failure();
        assert!(matches!(failure, ThrottleFailure::Preflight { .. }));
    }

    #[test]
    fn daily_budget_reloads_the_latest_ledger_on_each_acquire() {
        use plotforge_job::{UsageKind, UsageReport};

        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("usage.json");
        let throttle = ProviderThrottle::from_config_with_usage_path(
            ThrottleConfig {
                provider_id: "provider-a".into(),
                daily_token_budget: Some(5),
                ..ThrottleConfig::default()
            },
            &path,
        )
        .expect("valid throttle")
        .expect("daily budget creates gate");
        drop(throttle.acquire().expect("missing ledger means zero usage"));

        let mut ledger = UsageLedger::load_from(&path, SystemJobClock).expect("load ledger");
        ledger
            .report_usage(UsageReport {
                provider_id: "provider-a".into(),
                kind: UsageKind::Text,
                model: "model-a".into(),
                input_tokens: 0,
                output_tokens: 5,
                spent_cost_units: 0,
            })
            .expect("persist latest usage");

        let failure = throttle
            .acquire()
            .expect_err("new ledger usage must be observed")
            .into_failure();
        assert_eq!(
            failure,
            ThrottleFailure::BudgetExceeded {
                spent: 5,
                budget: 5
            }
        );
    }
}
