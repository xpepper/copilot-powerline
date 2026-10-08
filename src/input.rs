use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CopilotInput {
    pub session_id: Option<String>,
    #[serde(default)]
    pub context_window: ContextWindow,
    #[serde(default)]
    pub ai_used: AiUsed,
    #[serde(default)]
    pub model: Option<ModelInfo>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ContextWindow {
    #[serde(default)]
    pub current_context_tokens: Option<u64>,
    #[serde(default)]
    pub displayed_context_limit: Option<u64>,
    pub current_context_used_percentage: Option<f64>,
    #[serde(default)]
    pub total_input_tokens: Option<u64>,
    #[serde(default)]
    pub total_output_tokens: Option<u64>,
    #[serde(default)]
    pub total_cache_read_tokens: Option<u64>,
    #[serde(default)]
    pub total_cache_write_tokens: Option<u64>,
    #[serde(default)]
    pub total_reasoning_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    /// Token usage of the most recent API call. `input_tokens` includes
    /// cache reads and writes, matching how `total_input_tokens` is counted.
    #[serde(default)]
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CurrentUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AiUsed {
    #[serde(default)]
    pub total_nano_aiu: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelInfo {
    #[serde(default)]
    pub id: Option<String>,
    /// Human-readable model name. When the `auto` model is selected and has
    /// routed a request, Copilot CLI renders it as `Auto → <resolved model>`.
    #[serde(default)]
    pub display_name: Option<String>,
}

impl CopilotInput {
    pub fn from_json(json_str: &str) -> Self {
        serde_json::from_str(json_str).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_payload() {
        let json = r#"{
            "session_id": "sess-123",
            "context_window": {
                "current_context_tokens": 1250,
                "displayed_context_limit": 200000,
                "current_context_used_percentage": 0.6,
                "total_input_tokens": 50000,
                "total_output_tokens": 12000,
                "total_cache_read_tokens": 35000,
                "total_cache_write_tokens": 8000,
                "total_reasoning_tokens": 3200,
                "total_tokens": 62000,
                "current_usage": {
                    "input_tokens": 4000,
                    "output_tokens": 300,
                    "cache_creation_input_tokens": 100,
                    "cache_read_input_tokens": 3500
                }
            },
            "ai_used": {
                "total_nano_aiu": 500000000000
            },
            "model": {
                "id": "claude-3-7-sonnet",
                "display_name": "Claude 3.7 Sonnet"
            }
        }"#;

        let input = CopilotInput::from_json(json);
        assert_eq!(input.session_id.as_deref(), Some("sess-123"));
        assert_eq!(input.context_window.current_context_tokens, Some(1250));
        assert_eq!(input.context_window.displayed_context_limit, Some(200000));
        assert_eq!(
            input.context_window.current_context_used_percentage,
            Some(0.6)
        );
        assert_eq!(input.context_window.total_cache_read_tokens, Some(35000));
        assert_eq!(input.context_window.total_cache_write_tokens, Some(8000));
        assert_eq!(input.context_window.total_reasoning_tokens, Some(3200));
        assert_eq!(input.context_window.total_tokens, Some(62000));
        let usage = input.context_window.current_usage.as_ref().unwrap();
        assert_eq!(usage.input_tokens, Some(4000));
        assert_eq!(usage.cache_read_input_tokens, Some(3500));
        assert_eq!(input.ai_used.total_nano_aiu, 500000000000);
        assert_eq!(
            input.model.as_ref().and_then(|m| m.id.as_deref()),
            Some("claude-3-7-sonnet")
        );
        assert_eq!(
            input.model.as_ref().and_then(|m| m.display_name.as_deref()),
            Some("Claude 3.7 Sonnet")
        );
    }

    #[test]
    fn test_parse_empty_payload() {
        let input = CopilotInput::from_json("{}");
        assert_eq!(input.session_id, None);
        assert_eq!(input.context_window.current_context_tokens, None);
        assert_eq!(input.ai_used.total_nano_aiu, 0);
        assert!(input.model.is_none());
    }

    #[test]
    fn test_parse_invalid_json_falls_back_to_default() {
        let input = CopilotInput::from_json("invalid json");
        assert_eq!(input.session_id, None);
        assert_eq!(input.ai_used.total_nano_aiu, 0);
    }
}
