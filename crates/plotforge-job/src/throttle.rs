use std::sync::{Arc, Mutex};

use thiserror::Error;

use crate::{JobClock, UsageLedger};

const TOKEN_SCALE: u128 = 60_000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThrottleConfig {
    pub provider_id: String,
    pub max_concurrency: Option<u32>,
    pub requests_per_minute: Option<u32>,
    pub daily_token_budget: Option<u64>,
}

pub trait DailyTokenUsage {
    fn provider_output_tokens_for_utc_day(&self, provider_id: &str, at_ms: u64) -> u64;
}

impl<C> DailyTokenUsage for UsageLedger<C>
where
    C: JobClock,
{
    fn provider_output_tokens_for_utc_day(&self, provider_id: &str, at_ms: u64) -> u64 {
        self.provider_output_tokens_for_utc_day(provider_id, at_ms)
    }
}

#[derive(Clone, Debug)]
pub struct ThrottleGate<C> {
    clock: C,
    state: Arc<Mutex<ThrottleState>>,
}

#[derive(Debug)]
struct ThrottleState {
    config: ThrottleConfig,
    available_token_units: u128,
    last_refill_ms: u64,
    in_flight: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ThrottleError {
    #[error("invalid throttle configuration: {0}")]
    InvalidConfig(&'static str),
    #[error("provider request rate is limited; retry after {retry_after_ms} ms")]
    RateLimited { retry_after_ms: u64 },
    #[error("provider concurrency cap reached: max_concurrency={max_concurrency}")]
    ConcurrencyCapped { max_concurrency: u32 },
    #[error("throttle gate state is unavailable after a concurrent panic")]
    StatePoisoned,
    #[error("usage ledger is required to enforce daily_token_budget")]
    BudgetLedgerRequired,
    #[error("provider daily token budget exceeded: spent={spent}, budget={budget}")]
    BudgetExceeded { spent: u64, budget: u64 },
}

impl ThrottleError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. } | Self::ConcurrencyCapped { .. }
        )
    }
}

#[derive(Debug)]
#[must_use = "the throttle permit must be held for the full provider call"]
pub struct ThrottlePermit {
    concurrency_state: Option<Arc<Mutex<ThrottleState>>>,
}

impl<C> ThrottleGate<C>
where
    C: JobClock,
{
    pub fn new(config: ThrottleConfig, clock: C) -> Result<Self, ThrottleError> {
        validate_config(&config)?;
        let now_ms = clock.now_ms();
        let available_token_units = config
            .requests_per_minute
            .map(token_capacity_units)
            .unwrap_or_default();

        Ok(Self {
            clock,
            state: Arc::new(Mutex::new(ThrottleState {
                config,
                available_token_units,
                last_refill_ms: now_ms,
                in_flight: 0,
            })),
        })
    }

    pub fn acquire(
        &self,
        usage_ledger: Option<&dyn DailyTokenUsage>,
    ) -> Result<ThrottlePermit, ThrottleError> {
        let now_ms = self.clock.now_ms();
        let mut state = self
            .state
            .lock()
            .map_err(|_| ThrottleError::StatePoisoned)?;
        enforce_daily_budget(&state.config, usage_ledger, now_ms)?;
        enforce_concurrency(&state.config, &state)?;
        enforce_request_rate(&mut state, now_ms)?;

        let concurrency_state = if state.config.max_concurrency.is_some() {
            state.in_flight += 1;
            Some(Arc::clone(&self.state))
        } else {
            None
        };
        drop(state);

        Ok(ThrottlePermit { concurrency_state })
    }

    pub fn reconfigure(&self, config: ThrottleConfig) -> Result<(), ThrottleError> {
        validate_config(&config)?;
        let now_ms = self.clock.now_ms();
        let mut state = self
            .state
            .lock()
            .map_err(|_| ThrottleError::StatePoisoned)?;
        if config.provider_id != state.config.provider_id {
            return Err(ThrottleError::InvalidConfig(
                "provider_id cannot change when reconfiguring a throttle gate",
            ));
        }
        let previous_rpm = state.config.requests_per_minute;
        if let Some(requests_per_minute) = previous_rpm {
            refill_tokens(&mut state, requests_per_minute, now_ms);
        }
        state.available_token_units = match (previous_rpm, config.requests_per_minute) {
            (_, None) => 0,
            (None, Some(requests_per_minute)) => token_capacity_units(requests_per_minute),
            (Some(previous_rpm), Some(requests_per_minute)) => {
                let consumed_units =
                    token_capacity_units(previous_rpm).saturating_sub(state.available_token_units);
                token_capacity_units(requests_per_minute).saturating_sub(consumed_units)
            }
        };
        state.last_refill_ms = now_ms;
        state.config = config;
        Ok(())
    }

    pub fn requires_daily_usage(&self) -> Result<bool, ThrottleError> {
        let state = self
            .state
            .lock()
            .map_err(|_| ThrottleError::StatePoisoned)?;
        Ok(state.config.daily_token_budget.is_some())
    }
}

fn enforce_daily_budget(
    config: &ThrottleConfig,
    usage_ledger: Option<&dyn DailyTokenUsage>,
    now_ms: u64,
) -> Result<(), ThrottleError> {
    let Some(budget) = config.daily_token_budget else {
        return Ok(());
    };
    let ledger = usage_ledger.ok_or(ThrottleError::BudgetLedgerRequired)?;
    let spent = ledger.provider_output_tokens_for_utc_day(&config.provider_id, now_ms);
    if spent >= budget {
        return Err(ThrottleError::BudgetExceeded { spent, budget });
    }
    Ok(())
}

impl Drop for ThrottlePermit {
    fn drop(&mut self) {
        let Some(state) = self.concurrency_state.take() else {
            return;
        };
        let mut state = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        debug_assert!(state.in_flight > 0, "throttle permit count underflow");
        state.in_flight = state.in_flight.saturating_sub(1);
    }
}

fn validate_config(config: &ThrottleConfig) -> Result<(), ThrottleError> {
    if config.max_concurrency == Some(0) {
        return Err(ThrottleError::InvalidConfig(
            "max_concurrency must be greater than 0",
        ));
    }
    if config.requests_per_minute == Some(0) {
        return Err(ThrottleError::InvalidConfig(
            "requests_per_minute must be greater than 0",
        ));
    }
    if config.daily_token_budget.is_some() && config.provider_id.trim().is_empty() {
        return Err(ThrottleError::InvalidConfig(
            "provider_id must not be empty when daily_token_budget is configured",
        ));
    }
    Ok(())
}

fn enforce_concurrency(
    config: &ThrottleConfig,
    state: &ThrottleState,
) -> Result<(), ThrottleError> {
    if let Some(max_concurrency) = config.max_concurrency
        && state.in_flight >= max_concurrency
    {
        return Err(ThrottleError::ConcurrencyCapped { max_concurrency });
    }
    Ok(())
}

fn enforce_request_rate(state: &mut ThrottleState, now_ms: u64) -> Result<(), ThrottleError> {
    let Some(requests_per_minute) = state.config.requests_per_minute else {
        return Ok(());
    };

    refill_tokens(state, requests_per_minute, now_ms);
    if state.available_token_units < TOKEN_SCALE {
        let missing_units = TOKEN_SCALE - state.available_token_units;
        let retry_after_ms = div_ceil(missing_units, u128::from(requests_per_minute));
        return Err(ThrottleError::RateLimited {
            retry_after_ms: retry_after_ms as u64,
        });
    }

    state.available_token_units -= TOKEN_SCALE;
    Ok(())
}

fn refill_tokens(state: &mut ThrottleState, requests_per_minute: u32, now_ms: u64) {
    if now_ms < state.last_refill_ms {
        state.last_refill_ms = now_ms;
        return;
    }
    if now_ms == state.last_refill_ms {
        return;
    }
    let elapsed_ms = now_ms - state.last_refill_ms;
    let refill_units = u128::from(elapsed_ms) * u128::from(requests_per_minute);
    state.available_token_units = state
        .available_token_units
        .saturating_add(refill_units)
        .min(token_capacity_units(requests_per_minute));
    state.last_refill_ms = now_ms;
}

fn token_capacity_units(requests_per_minute: u32) -> u128 {
    u128::from(requests_per_minute) * TOKEN_SCALE
}

fn div_ceil(numerator: u128, denominator: u128) -> u128 {
    numerator.div_ceil(denominator)
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use crate::{UsageKind, UsageReport};

    use super::*;

    const UTC_DAY_MS: u64 = 86_400_000;

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
    fn throttle_gate_token_bucket_blocks_excess_with_exact_retry_delay() {
        let clock = FakeClock::new(0);
        let gate = gate(
            ThrottleConfig {
                requests_per_minute: Some(2),
                ..config()
            },
            clock.clone(),
        );

        drop(gate.acquire(None).expect("first token"));
        drop(gate.acquire(None).expect("second token"));
        assert_eq!(
            gate.acquire(None).expect_err("bucket empty"),
            ThrottleError::RateLimited {
                retry_after_ms: 30_000
            }
        );

        clock.set(29_999);
        assert_eq!(
            gate.acquire(None).expect_err("one unit short"),
            ThrottleError::RateLimited { retry_after_ms: 1 }
        );
        clock.set(30_000);
        drop(gate.acquire(None).expect("one token refilled"));
    }

    #[test]
    fn throttle_gate_clock_rollback_resets_refill_baseline() {
        let clock = FakeClock::new(10_000);
        let gate = gate(
            ThrottleConfig {
                requests_per_minute: Some(1),
                ..config()
            },
            clock.clone(),
        );
        drop(gate.acquire(None).expect("initial token"));

        clock.set(5_000);
        assert_eq!(
            gate.acquire(None)
                .expect_err("rollback does not mint a token"),
            ThrottleError::RateLimited {
                retry_after_ms: 60_000
            }
        );
        clock.set(64_999);
        assert_eq!(
            gate.acquire(None).expect_err("one millisecond remains"),
            ThrottleError::RateLimited { retry_after_ms: 1 }
        );
        clock.set(65_000);
        drop(
            gate.acquire(None)
                .expect("token refills from rollback baseline"),
        );
    }

    #[test]
    fn throttle_gate_clones_share_rate_state() {
        let gate = gate(
            ThrottleConfig {
                requests_per_minute: Some(1),
                ..config()
            },
            FakeClock::new(0),
        );
        let clone = gate.clone();

        drop(gate.acquire(None).expect("initial token"));
        assert_eq!(
            clone.acquire(None).expect_err("shared bucket empty"),
            ThrottleError::RateLimited {
                retry_after_ms: 60_000
            }
        );
    }

    #[test]
    fn throttle_gate_reconfigure_preserves_in_flight_and_rate_state() {
        let clock = FakeClock::new(0);
        let gate = gate(
            ThrottleConfig {
                max_concurrency: Some(1),
                requests_per_minute: Some(2),
                ..config()
            },
            clock,
        );
        let permit = gate.acquire(None).expect("initial permit and token");

        gate.reconfigure(ThrottleConfig {
            max_concurrency: Some(1),
            requests_per_minute: Some(1),
            ..config()
        })
        .expect("quota hot update");

        assert_eq!(
            gate.acquire(None)
                .expect_err("in-flight request survives update"),
            ThrottleError::ConcurrencyCapped { max_concurrency: 1 }
        );
        drop(permit);
        assert!(matches!(
            gate.acquire(None),
            Err(ThrottleError::RateLimited { .. })
        ));
    }

    #[test]
    fn throttle_gate_reconfigure_rejects_identity_changes() {
        let gate = gate(config(), FakeClock::new(0));

        assert_eq!(
            gate.reconfigure(ThrottleConfig {
                provider_id: "provider-b".into(),
                requests_per_minute: Some(1),
                ..ThrottleConfig::default()
            })
            .expect_err("gate identity is stable"),
            ThrottleError::InvalidConfig(
                "provider_id cannot change when reconfiguring a throttle gate"
            )
        );
    }

    #[test]
    fn throttle_gate_concurrency_cap_blocks_until_permit_drop() {
        let gate = gate(
            ThrottleConfig {
                max_concurrency: Some(1),
                ..config()
            },
            FakeClock::new(0),
        );
        let clone = gate.clone();

        let permit = gate.acquire(None).expect("first permit");
        assert_eq!(
            clone.acquire(None).expect_err("concurrency capped"),
            ThrottleError::ConcurrencyCapped { max_concurrency: 1 }
        );
        drop(permit);
        drop(clone.acquire(None).expect("permit released"));
    }

    #[test]
    fn throttle_gate_concurrency_rejection_does_not_consume_rate_token() {
        let gate = gate(
            ThrottleConfig {
                max_concurrency: Some(1),
                requests_per_minute: Some(2),
                ..config()
            },
            FakeClock::new(0),
        );

        let permit = gate.acquire(None).expect("first permit and token");
        assert_eq!(
            gate.acquire(None).expect_err("concurrency capped"),
            ThrottleError::ConcurrencyCapped { max_concurrency: 1 }
        );
        drop(permit);
        drop(gate.acquire(None).expect("second token remains"));
        assert!(matches!(
            gate.acquire(None),
            Err(ThrottleError::RateLimited { .. })
        ));
    }

    #[test]
    fn throttle_gate_passthrough_when_no_quota() {
        let gate = gate(config(), FakeClock::new(0));

        for _ in 0..100 {
            drop(gate.acquire(None).expect("no-op permit"));
        }
    }

    #[test]
    fn throttle_gate_budget_uses_only_current_utc_epoch_day() {
        let now_ms = UTC_DAY_MS + 1_000;
        let clock = FakeClock::new(UTC_DAY_MS - 1);
        let mut ledger = UsageLedger::new(clock.clone());
        ledger
            .report_usage(usage_report(90))
            .expect("previous-day usage");
        clock.set(now_ms);
        ledger
            .report_usage(usage_report(40))
            .expect("current-day usage");
        let gate = gate(
            ThrottleConfig {
                daily_token_budget: Some(40),
                ..config()
            },
            clock,
        );

        let error = gate.acquire(Some(&ledger)).expect_err("budget reached");

        assert_eq!(
            error,
            ThrottleError::BudgetExceeded {
                spent: 40,
                budget: 40
            }
        );
        assert!(!error.is_retryable());
    }

    #[test]
    fn throttle_gate_budget_below_limit_and_none_skip_check() {
        let clock = FakeClock::new(UTC_DAY_MS);
        let mut ledger = UsageLedger::new(clock.clone());
        ledger.report_usage(usage_report(39)).expect("usage");
        let budgeted = gate(
            ThrottleConfig {
                daily_token_budget: Some(40),
                ..config()
            },
            clock.clone(),
        );
        let unbudgeted = gate(config(), clock);

        drop(budgeted.acquire(Some(&ledger)).expect("below budget"));
        drop(unbudgeted.acquire(None).expect("no ledger needed"));
    }

    #[test]
    fn throttle_gate_budget_requires_ledger() {
        let gate = gate(
            ThrottleConfig {
                daily_token_budget: Some(1),
                ..config()
            },
            FakeClock::new(0),
        );

        assert_eq!(
            gate.acquire(None).expect_err("ledger is required"),
            ThrottleError::BudgetLedgerRequired
        );
    }

    #[test]
    fn throttle_gate_transient_capacity_errors_are_retryable() {
        assert!(ThrottleError::RateLimited { retry_after_ms: 1 }.is_retryable());
        assert!(ThrottleError::ConcurrencyCapped { max_concurrency: 1 }.is_retryable());
        assert!(!ThrottleError::BudgetLedgerRequired.is_retryable());
    }

    #[test]
    fn throttle_gate_rejects_zero_limits() {
        assert_eq!(
            ThrottleGate::new(
                ThrottleConfig {
                    requests_per_minute: Some(0),
                    ..config()
                },
                FakeClock::new(0),
            )
            .expect_err("zero rpm rejected"),
            ThrottleError::InvalidConfig("requests_per_minute must be greater than 0")
        );
        assert_eq!(
            ThrottleGate::new(
                ThrottleConfig {
                    max_concurrency: Some(0),
                    ..config()
                },
                FakeClock::new(0),
            )
            .expect_err("zero concurrency rejected"),
            ThrottleError::InvalidConfig("max_concurrency must be greater than 0")
        );
    }

    fn config() -> ThrottleConfig {
        ThrottleConfig {
            provider_id: "provider-a".into(),
            ..ThrottleConfig::default()
        }
    }

    fn gate(config: ThrottleConfig, clock: FakeClock) -> ThrottleGate<FakeClock> {
        ThrottleGate::new(config, clock).expect("valid gate")
    }

    fn usage_report(output_tokens: u64) -> UsageReport {
        UsageReport {
            provider_id: "provider-a".into(),
            kind: UsageKind::Text,
            model: "model-a".into(),
            input_tokens: 0,
            output_tokens,
            spent_cost_units: 0,
        }
    }
}
