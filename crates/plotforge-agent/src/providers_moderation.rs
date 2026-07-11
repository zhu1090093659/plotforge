//! Moderation provider port and OpenAI-compatible blocking client.

use std::{collections::BTreeMap, io::Read};

use plotforge_schema::{
    ModerationProviderEntry, UsageInfo, contains_secret_marker_text, redact_trace_text,
};

use crate::providers_text::{
    ProviderCredentialError, ProviderCredentialResolver, TextProviderConfig,
};
use crate::shared::{join_provider_endpoint, parse_retry_after};

const MODERATION_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const MODERATION_RESPONSE_BODY_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModerationRequest {
    pub call_id: String,
    pub prompt: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModerationResponse {
    pub flagged: bool,
    pub categories: Vec<String>,
    pub usage: Option<UsageInfo>,
    pub spent_cost_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModerationProviderErrorKind {
    Provider,
    Timeout,
    ThrottlePreflight,
    RateLimit { retry_after_ms: Option<u64> },
    ContentFiltered { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModerationProviderError {
    pub kind: ModerationProviderErrorKind,
    pub code: String,
    pub message: String,
}

impl ModerationProviderError {
    pub fn provider(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: ModerationProviderErrorKind::Provider,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            kind: ModerationProviderErrorKind::Timeout,
            code: "moderation_provider_timeout".into(),
            message: message.into(),
        }
    }

    pub fn rate_limit(retry_after_ms: Option<u64>, message: impl Into<String>) -> Self {
        Self {
            kind: ModerationProviderErrorKind::RateLimit { retry_after_ms },
            code: "moderation_provider_rate_limit".into(),
            message: message.into(),
        }
    }

    fn throttle_preflight(message: impl Into<String>) -> Self {
        Self {
            kind: ModerationProviderErrorKind::ThrottlePreflight,
            code: "moderation_provider_throttle_preflight".into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ModerationProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ModerationProviderError {}

pub trait ModerationProvider {
    fn moderate(
        &self,
        request: &ModerationRequest,
    ) -> Result<ModerationResponse, ModerationProviderError>;
}

impl ModerationProvider for Box<dyn ModerationProvider> {
    fn moderate(
        &self,
        request: &ModerationRequest,
    ) -> Result<ModerationResponse, ModerationProviderError> {
        (**self).moderate(request)
    }
}

#[derive(Clone, Debug, Default)]
pub struct FakeModerationProvider {
    flagged_categories: Vec<String>,
    response_error: Option<ModerationProviderError>,
    calls: std::rc::Rc<std::cell::Cell<u32>>,
}

impl FakeModerationProvider {
    pub fn pass() -> Self {
        Self::default()
    }
    pub fn flagged(categories: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut categories = categories.into_iter().map(Into::into).collect::<Vec<_>>();
        categories.sort();
        categories.dedup();
        Self {
            flagged_categories: categories,
            ..Self::default()
        }
    }
    pub fn with_error(error: ModerationProviderError) -> Self {
        Self {
            response_error: Some(error),
            ..Self::default()
        }
    }
    pub fn call_count(&self) -> u32 {
        self.calls.get()
    }
}

impl ModerationProvider for FakeModerationProvider {
    fn moderate(
        &self,
        _request: &ModerationRequest,
    ) -> Result<ModerationResponse, ModerationProviderError> {
        self.calls.set(self.calls.get() + 1);
        if let Some(error) = self.response_error.clone() {
            return Err(error);
        }
        Ok(ModerationResponse {
            flagged: !self.flagged_categories.is_empty(),
            categories: self.flagged_categories.clone(),
            usage: None,
            spent_cost_units: 0,
        })
    }
}

#[derive(Clone)]
pub struct OpenAiModerationClient<R> {
    endpoint_url: String,
    model: String,
    credential_env_var: String,
    credential_resolver: R,
    client: reqwest::blocking::Client,
    throttle: Option<crate::throttle::ProviderThrottle>,
}

impl<R> OpenAiModerationClient<R>
where
    R: ProviderCredentialResolver,
{
    pub fn new(
        entry: &ModerationProviderEntry,
        credential_resolver: R,
    ) -> Result<Self, ModerationProviderError> {
        validate_moderation_provider_entry(entry)?;
        let endpoint_url =
            join_provider_endpoint(&entry.endpoint_url, "moderations").map_err(|message| {
                ModerationProviderError::provider("moderation_provider_invalid_endpoint", message)
            })?;
        let client = reqwest::blocking::Client::builder()
            .timeout(MODERATION_HTTP_TIMEOUT)
            .connect_timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| {
                ModerationProviderError::provider(
                    "moderation_provider_http_client",
                    redact_trace_text(&error.to_string()),
                )
            })?;
        Ok(Self {
            endpoint_url,
            model: entry.model.clone(),
            credential_env_var: entry.credential_env_var.clone(),
            credential_resolver,
            client,
            throttle: None,
        })
    }

    pub(crate) fn with_throttle(
        mut self,
        throttle: Option<crate::throttle::ProviderThrottle>,
    ) -> Self {
        self.throttle = throttle;
        self
    }
}

impl<R> ModerationProvider for OpenAiModerationClient<R>
where
    R: ProviderCredentialResolver,
{
    fn moderate(
        &self,
        request: &ModerationRequest,
    ) -> Result<ModerationResponse, ModerationProviderError> {
        if contains_secret_marker_text(&request.prompt) {
            return Err(ModerationProviderError::provider(
                "moderation_provider_prompt_secret",
                "moderation prompt contained a secret marker",
            ));
        }
        let credential = self
            .credential_resolver
            .resolve(&self.credential_env_var)
            .map_err(|_error: ProviderCredentialError| {
                ModerationProviderError::provider(
                    "moderation_provider_missing_credential",
                    format!(
                        "moderation provider credential env var `{}` is missing or empty",
                        self.credential_env_var
                    ),
                )
            })?;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        if !credential.trim().is_empty() {
            let value = reqwest::header::HeaderValue::from_str(&format!("Bearer {credential}"))
                .map_err(|_| {
                    ModerationProviderError::provider(
                        "moderation_provider_credential_header",
                        "provider credential contains bytes illegal in an HTTP header value",
                    )
                })?;
            headers.insert(reqwest::header::AUTHORIZATION, value);
        }
        let _permit = self
            .throttle
            .as_ref()
            .map(crate::throttle::ProviderThrottle::acquire)
            .transpose()
            .map_err(map_moderation_throttle_error)?;
        let response = self
            .client
            .post(&self.endpoint_url)
            .headers(headers)
            .json(&serde_json::json!({ "model": self.model, "input": request.prompt }))
            .send()
            .map_err(|error| {
                if error.is_timeout() {
                    ModerationProviderError::timeout(redact_trace_text(&error.to_string()))
                } else {
                    ModerationProviderError::provider(
                        "moderation_provider_http_send",
                        redact_trace_text(&error.to_string()),
                    )
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let retry_after_ms =
                    parse_retry_after(response.headers().get(reqwest::header::RETRY_AFTER));
                return Err(ModerationProviderError::rate_limit(
                    retry_after_ms,
                    format!("moderation provider returned HTTP {status}"),
                ));
            }
            return Err(ModerationProviderError::provider(
                "moderation_provider_http_status",
                format!("moderation provider returned HTTP {status}"),
            ));
        }
        let raw = read_moderation_response_body(response)?;
        if contains_secret_marker_text(&raw) {
            return Err(ModerationProviderError::provider(
                "moderation_provider_response_secret",
                "moderation provider response contained a secret marker",
            ));
        }
        parse_moderation_response(&raw)
    }
}

pub(crate) fn validate_moderation_provider_entry(
    entry: &ModerationProviderEntry,
) -> Result<(), ModerationProviderError> {
    TextProviderConfig {
        enabled: entry.enabled,
        provider: entry.id.clone(),
        model: entry.model.clone(),
        endpoint_url: Some(entry.endpoint_url.clone()),
        credential_env_var: entry.credential_env_var.clone(),
        max_output_tokens: None,
        supports_json_schema: false,
    }
    .validate()
    .map_err(|error| {
        ModerationProviderError::provider(
            "moderation_provider_invalid_configuration",
            error.to_string(),
        )
    })
}

fn read_moderation_response_body(
    response: reqwest::blocking::Response,
) -> Result<String, ModerationProviderError> {
    if response
        .content_length()
        .is_some_and(|length| length > MODERATION_RESPONSE_BODY_MAX_BYTES)
    {
        return Err(moderation_response_too_large());
    }
    let mut bytes = Vec::new();
    response
        .take(MODERATION_RESPONSE_BODY_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            ModerationProviderError::provider(
                "moderation_provider_http_body",
                redact_trace_text(&error.to_string()),
            )
        })?;
    if bytes.len() as u64 > MODERATION_RESPONSE_BODY_MAX_BYTES {
        return Err(moderation_response_too_large());
    }
    String::from_utf8(bytes).map_err(|_| {
        ModerationProviderError::provider(
            "moderation_provider_http_body",
            "moderation provider response was not valid UTF-8",
        )
    })
}

fn moderation_response_too_large() -> ModerationProviderError {
    ModerationProviderError::provider(
        "moderation_provider_response_too_large",
        format!(
            "moderation provider response exceeded the {} byte limit",
            MODERATION_RESPONSE_BODY_MAX_BYTES
        ),
    )
}

#[derive(serde::Deserialize)]
struct WireModerationResponse {
    results: Vec<WireModerationResult>,
}

#[derive(serde::Deserialize)]
struct WireModerationResult {
    flagged: bool,
    categories: BTreeMap<String, bool>,
}

fn parse_moderation_response(raw: &str) -> Result<ModerationResponse, ModerationProviderError> {
    let parsed: WireModerationResponse = serde_json::from_str(raw).map_err(|error| {
        ModerationProviderError::provider(
            "moderation_provider_http_decode",
            redact_trace_text(&format!("moderation response was not JSON: {error}")),
        )
    })?;
    let result = parsed.results.into_iter().next().ok_or_else(|| {
        ModerationProviderError::provider(
            "moderation_provider_http_decode",
            "moderation response missing results[0]",
        )
    })?;
    let categories = result
        .categories
        .into_iter()
        .filter_map(|(category, flagged)| flagged.then_some(category))
        .collect();
    Ok(ModerationResponse {
        flagged: result.flagged,
        categories,
        usage: None,
        spent_cost_units: 0,
    })
}

fn map_moderation_throttle_error(
    error: crate::throttle::ProviderThrottleError,
) -> ModerationProviderError {
    match error.into_failure() {
        crate::throttle::ThrottleFailure::RateLimit {
            retry_after_ms,
            message,
        } => ModerationProviderError::rate_limit(retry_after_ms, message),
        crate::throttle::ThrottleFailure::BudgetExceeded { spent, budget } => {
            ModerationProviderError::throttle_preflight(format!(
                "unsupported moderation daily token budget state: spent={spent}, budget={budget}"
            ))
        }
        crate::throttle::ThrottleFailure::Preflight { message } => {
            ModerationProviderError::throttle_preflight(redact_trace_text(&message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(endpoint_url: &str, credential_env_var: &str) -> ModerationProviderEntry {
        ModerationProviderEntry {
            id: "moderation-test".into(),
            endpoint_url: endpoint_url.into(),
            model: "omni-moderation-latest".into(),
            credential_env_var: credential_env_var.into(),
            enabled: true,
            max_concurrency: None,
            requests_per_minute: None,
            daily_token_budget: None,
        }
    }

    #[test]
    fn moderation_request_decodes_flagged_false() {
        let response = parse_moderation_response(
            r#"{"results":[{"flagged":false,"categories":{"violence":false}}]}"#,
        )
        .expect("decode");
        assert!(!response.flagged);
        assert!(response.categories.is_empty());
        assert_eq!(response.usage, None);
    }

    #[test]
    fn moderation_request_decodes_sorted_flagged_categories() {
        let response = parse_moderation_response(r#"{"results":[{"flagged":true,"categories":{"violence":true,"hate":true,"sexual":false}}]}"#).expect("decode");
        assert!(response.flagged);
        assert_eq!(response.categories, vec!["hate", "violence"]);
    }

    #[test]
    fn moderation_response_requires_first_result() {
        let error = parse_moderation_response(r#"{"results":[]}"#).expect_err("missing result");
        assert_eq!(error.code, "moderation_provider_http_decode");
    }

    #[test]
    fn fake_moderation_provider_tracks_calls() {
        let provider = FakeModerationProvider::flagged(["violence"]);
        let response = provider
            .moderate(&ModerationRequest {
                call_id: "call".into(),
                prompt: "prompt".into(),
            })
            .expect("moderate");
        assert!(response.flagged);
        assert_eq!(provider.call_count(), 1);
    }

    #[test]
    fn moderation_missing_credential_is_explicit_before_http() {
        let env_var = "PLOTFORGE_MODERATION_TEST_MISSING_CREDENTIAL";
        unsafe { std::env::remove_var(env_var) };
        let provider = OpenAiModerationClient::new(
            &entry("http://127.0.0.1:9/v1", env_var),
            crate::EnvCredentialResolver,
        )
        .expect("client builds");
        let error = provider
            .moderate(&ModerationRequest {
                call_id: "call".into(),
                prompt: "safe prompt".into(),
            })
            .expect_err("missing credential");
        assert_eq!(error.code, "moderation_provider_missing_credential");
    }

    #[test]
    fn moderation_rejects_secret_prompt_before_http() {
        let provider = OpenAiModerationClient::new(
            &entry("http://127.0.0.1:9/v1", ""),
            crate::OptionalEnvCredentialResolver,
        )
        .expect("client builds");
        let error = provider
            .moderate(&ModerationRequest {
                call_id: "call".into(),
                prompt: "OPENAI_API_KEY=sk-secret".into(),
            })
            .expect_err("secret prompt");
        assert_eq!(error.code, "moderation_provider_prompt_secret");
    }

    #[test]
    fn moderation_client_rejects_unsafe_persisted_fields_without_echo() {
        let mut unsafe_entry = entry("http://127.0.0.1:9/v1", "");
        unsafe_entry.model = "sk-secret-model".into();
        let error = match OpenAiModerationClient::new(
            &unsafe_entry,
            crate::OptionalEnvCredentialResolver,
        ) {
            Ok(_) => panic!("unsafe model must be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.code, "moderation_provider_invalid_configuration");
        assert!(!error.to_string().contains("sk-secret-model"));

        let mut unsafe_entry = entry("http://127.0.0.1:9/v1", "sk-secret-env-name");
        unsafe_entry.id = "safe-id".into();
        let error = match OpenAiModerationClient::new(&unsafe_entry, crate::EnvCredentialResolver) {
            Ok(_) => panic!("unsafe credential env-var name must be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.code, "moderation_provider_invalid_configuration");
        assert!(!error.to_string().contains("sk-secret-env-name"));
    }
}
