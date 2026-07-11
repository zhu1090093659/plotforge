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
}
