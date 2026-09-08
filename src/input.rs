use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CopilotInput {
    pub session_id: Option<String>,
    #[serde(default)]
    pub context_window: ContextWindow,
    #[serde(default)]
    pub ai_used: AiUsed,
    #[allow(dead_code)]
    pub model: Option<ModelInfo>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ContextWindow {
    #[serde(default)]
    pub current_context_tokens: Option<u64>,
    #[serde(default)]
    pub displayed_context_limit: Option<u64>,
    pub current_context_used_percentage: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AiUsed {
    #[serde(default)]
    pub total_nano_aiu: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelInfo {
    #[allow(dead_code)]
    pub id: Option<String>,
    #[allow(dead_code)]
    pub name: Option<String>,
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
                "current_context_used_percentage": 0.6
            },
            "ai_used": {
                "total_nano_aiu": 500000000000
            },
            "model": {
                "id": "claude-3-7-sonnet",
                "name": "Claude 3.7 Sonnet"
            }
        }"#;

        let input = CopilotInput::from_json(json);
        assert_eq!(input.session_id.as_deref(), Some("sess-123"));
        assert_eq!(input.context_window.current_context_tokens, Some(1250));
        assert_eq!(input.context_window.displayed_context_limit, Some(200000));
        assert_eq!(input.context_window.current_context_used_percentage, Some(0.6));
        assert_eq!(input.ai_used.total_nano_aiu, 500000000000);
        assert_eq!(input.model.as_ref().and_then(|m| m.id.as_deref()), Some("claude-3-7-sonnet"));
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
