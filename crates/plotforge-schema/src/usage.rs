use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Redaction-safe token usage reported by a model provider.
///
/// Both counts are optional because providers may omit one or both values.
#[derive(Clone, Debug, Default, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UsageInfo {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Clone, Debug, Default, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProviderCostReport {
    pub provider_id: String,
    pub text_calls: u64,
    pub image_calls: u64,
    pub tts_calls: u64,
    #[serde(default)]
    pub moderation_calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub spent_cost_units: u64,
}

#[derive(Clone, Debug, Default, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UsageSummary {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_spent_cost_units: u64,
    pub by_provider: BTreeMap<String, ProviderCostReport>,
}

/// Redaction-safe usage totals for one pi-Agent apply turn.
///
/// This deliberately carries only numeric totals. Provider ids, prompts,
/// response bodies, credentials, and global ledger state stay outside the
/// per-turn evidence contract.
#[derive(Clone, Debug, Default, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TurnUsageSummary {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_spent_cost_units: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_info_roundtrips_json() {
        let usage = UsageInfo {
            input_tokens: Some(1_024),
            output_tokens: Some(256),
        };

        let encoded = serde_json::to_string(&usage).expect("serialize usage info");
        let decoded: UsageInfo = serde_json::from_str(&encoded).expect("deserialize usage info");

        assert_eq!(decoded, usage);
    }

    #[test]
    fn usage_info_rejects_unknown_fields() {
        for field in ["api_key", "secret"] {
            let mut value = serde_json::json!({
                "input_tokens": 1_024,
                "output_tokens": 256,
            });
            value[field] = serde_json::json!("must-not-be-accepted");

            let error = serde_json::from_value::<UsageInfo>(value)
                .expect_err("credential-shaped unknown field should be rejected");

            assert!(
                error.to_string().contains("unknown field"),
                "unexpected error for {field}: {error}"
            );
        }
    }

    #[test]
    fn usage_summary_roundtrips_json() {
        let report = ProviderCostReport {
            provider_id: "provider-a".into(),
            text_calls: 2,
            image_calls: 1,
            tts_calls: 1,
            moderation_calls: 1,
            input_tokens: 1_024,
            output_tokens: 256,
            spent_cost_units: 12,
        };
        let summary = UsageSummary {
            total_input_tokens: report.input_tokens,
            total_output_tokens: report.output_tokens,
            total_spent_cost_units: report.spent_cost_units,
            by_provider: BTreeMap::from([(report.provider_id.clone(), report)]),
        };

        let encoded = serde_json::to_string(&summary).expect("serialize usage summary");
        let decoded: UsageSummary =
            serde_json::from_str(&encoded).expect("deserialize usage summary");

        assert_eq!(decoded, summary);
    }

    #[test]
    fn turn_usage_summary_roundtrips_json() {
        let summary = TurnUsageSummary {
            total_input_tokens: 29,
            total_output_tokens: 7,
            total_spent_cost_units: 11,
        };

        let encoded = serde_json::to_string(&summary).expect("serialize turn usage");
        let decoded: TurnUsageSummary =
            serde_json::from_str(&encoded).expect("deserialize turn usage");

        assert_eq!(decoded, summary);
        assert!(!encoded.contains("prompt"));
        assert!(!encoded.contains("response"));
        assert!(!encoded.contains("credential"));
    }

    #[test]
    fn usage_summary_and_provider_report_reject_unknown_fields() {
        let summary_error = serde_json::from_value::<UsageSummary>(serde_json::json!({
            "total_input_tokens": 0,
            "total_output_tokens": 0,
            "total_spent_cost_units": 0,
            "by_provider": {},
            "api_key": "must-not-be-accepted"
        }))
        .expect_err("usage summary must reject unknown fields");
        assert!(summary_error.to_string().contains("unknown field"));

        let report_error = serde_json::from_value::<ProviderCostReport>(serde_json::json!({
            "provider_id": "provider-a",
            "text_calls": 0,
            "image_calls": 0,
            "tts_calls": 0,
            "moderation_calls": 0,
            "input_tokens": 0,
            "output_tokens": 0,
            "spent_cost_units": 0,
            "secret": "must-not-be-accepted"
        }))
        .expect_err("provider report must reject unknown fields");
        assert!(report_error.to_string().contains("unknown field"));
    }

    #[test]
    fn provider_report_defaults_moderation_calls_for_legacy_json() {
        let decoded: ProviderCostReport = serde_json::from_value(serde_json::json!({
            "provider_id": "provider-a",
            "text_calls": 1,
            "image_calls": 0,
            "tts_calls": 0,
            "input_tokens": 10,
            "output_tokens": 2,
            "spent_cost_units": 3
        }))
        .expect("legacy provider report");

        assert_eq!(decoded.moderation_calls, 0);
    }
}
