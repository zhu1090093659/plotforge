//! Moderation pre-flight orchestration.
//!
//! This module owns exactly one moderation classification before the caller
//! enters its text path. It never invokes text/MCP providers and never owns a
//! retry loop; transport failures remain explicit provider errors.

use plotforge_job::{UsageKind, UsageReport};
use plotforge_schema::{ModerationOutcomeSummary, UsageInfo, contains_secret_marker_text};

use crate::{
    ModerationProvider, ModerationProviderError, ModerationRequest, ProviderUsageIdentity,
    UsageReporter,
};

pub struct ModerationScreen<'a> {
    pub provider: &'a dyn ModerationProvider,
    pub config_hash: &'a str,
    pub usage_identity: ProviderUsageIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModerationOutcome {
    pub summary: ModerationOutcomeSummary,
    pub moderation_config_hash: String,
    pub usage: Option<UsageInfo>,
    pub spent_cost_units: u64,
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum ModerationLoopError {
    #[error("moderation provider failure: {0}")]
    Provider(ModerationProviderError),
    #[error("moderation provider flagged content in categories: {categories:?}")]
    ContentFlagged { categories: Vec<String> },
    #[error("moderation provider response contained a secret marker")]
    ResponseSecretMarker,
    #[error("moderation usage report failed: {message}")]
    UsageReport { message: String },
}

/// Runs one optional moderation pre-flight. `None` is the explicit
/// no-provider pass-through and leaves reproducibility hash state absent.
pub fn screen_with_moderation(
    screen: Option<ModerationScreen<'_>>,
    request: &ModerationRequest,
) -> Result<Option<ModerationOutcome>, ModerationLoopError> {
    screen_with_moderation_with_usage_reporter(screen, request, None)
}

/// Runs one optional moderation pre-flight and records every successful
/// provider response exactly once. Accounting happens before secret/category
/// validation and before the flagged/pass branch so blocked calls are not
/// silently omitted from the user-global usage ledger.
pub fn screen_with_moderation_with_usage_reporter(
    screen: Option<ModerationScreen<'_>>,
    request: &ModerationRequest,
    reporter: Option<&mut dyn UsageReporter>,
) -> Result<Option<ModerationOutcome>, ModerationLoopError> {
    let Some(screen) = screen else {
        return Ok(None);
    };
    let response = screen
        .provider
        .moderate(request)
        .map_err(ModerationLoopError::Provider)?;
    if let Some(reporter) = reporter {
        reporter
            .report_usage(UsageReport {
                provider_id: screen.usage_identity.provider_id,
                kind: UsageKind::Moderation,
                model: screen.usage_identity.model,
                input_tokens: response
                    .usage
                    .as_ref()
                    .and_then(|usage| usage.input_tokens)
                    .unwrap_or(0),
                output_tokens: response
                    .usage
                    .as_ref()
                    .and_then(|usage| usage.output_tokens)
                    .unwrap_or(0),
                spent_cost_units: response.spent_cost_units,
            })
            .map_err(|error| ModerationLoopError::UsageReport {
                message: error.to_string(),
            })?;
    }
    if contains_secret_marker_text(&response.categories.join(" ")) {
        return Err(ModerationLoopError::ResponseSecretMarker);
    }
    let mut categories = response.categories;
    categories.sort();
    categories.dedup();
    if response.flagged {
        return Err(ModerationLoopError::ContentFlagged { categories });
    }
    Ok(Some(ModerationOutcome {
        summary: ModerationOutcomeSummary {
            flagged: false,
            categories,
        },
        moderation_config_hash: screen.config_hash.to_string(),
        usage: response.usage,
        spent_cost_units: response.spent_cost_units,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FakeModerationProvider, ModerationProviderError, ModerationResponse};
    use plotforge_job::{JobClock, UsageLedger};

    #[derive(Clone, Copy, Debug)]
    struct TestClock;

    impl JobClock for TestClock {
        fn now_ms(&self) -> u64 {
            1_000
        }
    }

    fn screen<'a>(provider: &'a dyn ModerationProvider) -> ModerationScreen<'a> {
        ModerationScreen {
            provider,
            config_hash: "sha256:moderation",
            usage_identity: ProviderUsageIdentity::new("stable-moderation-id", "moderation-model"),
        }
    }

    fn request() -> ModerationRequest {
        ModerationRequest {
            call_id: "moderation-call".into(),
            prompt: "a safe player input".into(),
        }
    }

    #[test]
    fn moderation_loop_none_is_passthrough() {
        assert_eq!(
            screen_with_moderation(None, &request()).expect("pass"),
            None
        );
    }

    #[test]
    fn moderation_loop_not_flagged_returns_summary_and_hash() {
        let provider = FakeModerationProvider::pass();
        let outcome = screen_with_moderation(Some(screen(&provider)), &request())
            .expect("screen")
            .expect("outcome");
        assert_eq!(provider.call_count(), 1);
        assert_eq!(outcome.moderation_config_hash, "sha256:moderation");
        assert_eq!(
            outcome.summary,
            ModerationOutcomeSummary {
                flagged: false,
                categories: Vec::new(),
            }
        );
    }

    #[test]
    fn moderation_loop_flagged_calls_once_and_blocks() {
        let provider = FakeModerationProvider::flagged(["violence", "hate"]);
        let error =
            screen_with_moderation(Some(screen(&provider)), &request()).expect_err("flagged");
        assert_eq!(provider.call_count(), 1, "moderation loop never retries");
        assert_eq!(
            error,
            ModerationLoopError::ContentFlagged {
                categories: vec!["hate".into(), "violence".into()],
            }
        );
    }

    struct StaticProvider(Result<ModerationResponse, ModerationProviderError>);

    impl ModerationProvider for StaticProvider {
        fn moderate(
            &self,
            _request: &ModerationRequest,
        ) -> Result<ModerationResponse, ModerationProviderError> {
            self.0.clone()
        }
    }

    #[test]
    fn moderation_loop_rejects_structured_secret_marker() {
        let provider = StaticProvider(Ok(ModerationResponse {
            flagged: false,
            categories: vec!["OPENAI_API_KEY=sk-secret".into()],
            usage: None,
            spent_cost_units: 0,
        }));
        let error =
            screen_with_moderation(Some(screen(&provider)), &request()).expect_err("secret marker");
        assert_eq!(error, ModerationLoopError::ResponseSecretMarker);
    }

    #[test]
    fn moderation_loop_propagates_provider_error_without_retry() {
        let provider = FakeModerationProvider::with_error(ModerationProviderError::rate_limit(
            Some(1_000),
            "rate limited",
        ));
        let error = screen_with_moderation(Some(screen(&provider)), &request())
            .expect_err("provider error");
        assert_eq!(provider.call_count(), 1, "moderation loop never retries");
        assert!(matches!(error, ModerationLoopError::Provider(_)));
    }

    struct UsageProvider {
        flagged: bool,
    }

    impl ModerationProvider for UsageProvider {
        fn moderate(
            &self,
            _request: &ModerationRequest,
        ) -> Result<ModerationResponse, ModerationProviderError> {
            Ok(ModerationResponse {
                flagged: self.flagged,
                categories: self
                    .flagged
                    .then(|| "violence".into())
                    .into_iter()
                    .collect(),
                usage: Some(UsageInfo {
                    input_tokens: Some(17),
                    output_tokens: Some(3),
                }),
                spent_cost_units: 5,
            })
        }
    }

    #[test]
    fn moderation_loop_accounts_nonzero_pass_response_once() {
        let provider = UsageProvider { flagged: false };
        let mut ledger = UsageLedger::new(TestClock);

        screen_with_moderation_with_usage_reporter(
            Some(screen(&provider)),
            &request(),
            Some(&mut ledger),
        )
        .expect("screen pass");

        let report = ledger.provider_cost_report("stable-moderation-id");
        assert_eq!(report.moderation_calls, 1);
        assert_eq!(report.text_calls, 0);
        assert_eq!(report.input_tokens, 17);
        assert_eq!(report.output_tokens, 3);
        assert_eq!(report.spent_cost_units, 5);
    }

    #[test]
    fn moderation_loop_accounts_missing_usage_as_zero_tokens() {
        let provider = FakeModerationProvider::pass();
        let mut ledger = UsageLedger::new(TestClock);

        screen_with_moderation_with_usage_reporter(
            Some(screen(&provider)),
            &request(),
            Some(&mut ledger),
        )
        .expect("screen pass");

        let report = ledger.provider_cost_report("stable-moderation-id");
        assert_eq!(report.moderation_calls, 1);
        assert_eq!(report.input_tokens, 0);
        assert_eq!(report.output_tokens, 0);
        assert_eq!(report.spent_cost_units, 0);
    }

    #[test]
    fn moderation_loop_accounts_nonzero_flagged_response_once() {
        let provider = UsageProvider { flagged: true };
        let mut ledger = UsageLedger::new(TestClock);

        let error = screen_with_moderation_with_usage_reporter(
            Some(screen(&provider)),
            &request(),
            Some(&mut ledger),
        )
        .expect_err("flagged");

        assert!(matches!(error, ModerationLoopError::ContentFlagged { .. }));
        let report = ledger.provider_cost_report("stable-moderation-id");
        assert_eq!(report.moderation_calls, 1);
        assert_eq!(report.input_tokens, 17);
        assert_eq!(report.output_tokens, 3);
        assert_eq!(report.spent_cost_units, 5);
    }
}
