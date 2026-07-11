//! Moderation pre-flight orchestration.
//!
//! This module owns exactly one moderation classification before the caller
//! enters its text path. It never invokes text/MCP providers and never owns a
//! retry loop; transport failures remain explicit provider errors.

use plotforge_schema::{ModerationOutcomeSummary, UsageInfo, contains_secret_marker_text};

use crate::{ModerationProvider, ModerationProviderError, ModerationRequest};

pub struct ModerationScreen<'a> {
    pub provider: &'a dyn ModerationProvider,
    pub config_hash: &'a str,
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
}

/// Runs one optional moderation pre-flight. `None` is the explicit
/// no-provider pass-through and leaves reproducibility hash state absent.
pub fn screen_with_moderation(
    screen: Option<ModerationScreen<'_>>,
    request: &ModerationRequest,
) -> Result<Option<ModerationOutcome>, ModerationLoopError> {
    let Some(screen) = screen else {
        return Ok(None);
    };
    let response = screen
        .provider
        .moderate(request)
        .map_err(ModerationLoopError::Provider)?;
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
        let outcome = screen_with_moderation(
            Some(ModerationScreen {
                provider: &provider,
                config_hash: "sha256:moderation",
            }),
            &request(),
        )
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
        let error = screen_with_moderation(
            Some(ModerationScreen {
                provider: &provider,
                config_hash: "sha256:moderation",
            }),
            &request(),
        )
        .expect_err("flagged");
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
        let error = screen_with_moderation(
            Some(ModerationScreen {
                provider: &provider,
                config_hash: "sha256:moderation",
            }),
            &request(),
        )
        .expect_err("secret marker");
        assert_eq!(error, ModerationLoopError::ResponseSecretMarker);
    }

    #[test]
    fn moderation_loop_propagates_provider_error_without_retry() {
        let provider = FakeModerationProvider::with_error(ModerationProviderError::rate_limit(
            Some(1_000),
            "rate limited",
        ));
        let error = screen_with_moderation(
            Some(ModerationScreen {
                provider: &provider,
                config_hash: "sha256:moderation",
            }),
            &request(),
        )
        .expect_err("provider error");
        assert_eq!(provider.call_count(), 1, "moderation loop never retries");
        assert!(matches!(error, ModerationLoopError::Provider(_)));
    }
}
