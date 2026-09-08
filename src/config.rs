use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    #[default]
    Minimal,
    Powerline,
    Capsule,
    Plain,
}

impl std::str::FromStr for Style {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(Style::Minimal),
            "powerline" => Ok(Style::Powerline),
            "capsule" => Ok(Style::Capsule),
            "plain" => Ok(Style::Plain),
            other => Err(format!("Unknown style: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub style: Style,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_segments")]
    pub segments: Vec<String>,
    #[serde(default)]
    pub tokens: TokensConfig,
    #[serde(default)]
    pub session_cost: CostConfig,
    #[serde(default)]
    pub month_cost: MonthCostConfig,
}

fn default_theme() -> String {
    "colorblind".to_string()
}

fn default_segments() -> Vec<String> {
    vec![
        "tokens".to_string(),
        "session_cost".to_string(),
        "month_cost".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokensConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub show_percentage: bool,
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: u64,
    #[serde(default = "default_alert_icon")]
    pub alert_icon: String,
}

impl Default for TokensConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_percentage: true,
            alert_threshold: 100_000,
            alert_icon: "⚠️ ".to_string(),
        }
    }
}

fn default_alert_threshold() -> u64 {
    100_000
}

fn default_alert_icon() -> String {
    "⚠️ ".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_session_prefix")]
    pub prefix: String,
    #[serde(default = "default_currency")]
    pub currency_symbol: String,
    #[serde(default)]
    pub show_aic: bool,
    #[serde(default = "default_decimals")]
    pub decimal_places: usize,
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: default_session_prefix(),
            currency_symbol: default_currency(),
            show_aic: false,
            decimal_places: 2,
        }
    }
}

fn default_session_prefix() -> String {
    "Session:".to_string()
}

fn default_currency() -> String {
    "$".to_string()
}

fn default_decimals() -> usize {
    2
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthCostConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_month_prefix")]
    pub prefix: String,
    #[serde(default = "default_currency")]
    pub currency_symbol: String,
    #[serde(default)]
    pub show_aic: bool,
    #[serde(default = "default_decimals")]
    pub decimal_places: usize,
    pub db_path: Option<PathBuf>,
}

impl Default for MonthCostConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: default_month_prefix(),
            currency_symbol: default_currency(),
            show_aic: false,
            decimal_places: 2,
            db_path: None,
        }
    }
}

fn default_month_prefix() -> String {
    "Month:".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            style: Style::Minimal,
            theme: default_theme(),
            segments: default_segments(),
            tokens: TokensConfig::default(),
            session_cost: CostConfig::default(),
            month_cost: MonthCostConfig::default(),
        }
    }
}

impl Config {
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn default_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".copilot").join("powerline.toml"))
    }

    pub fn load_from_file_or_default(path: Option<&Path>) -> Self {
        let config_path = path
            .map(PathBuf::from)
            .or_else(Self::default_config_path);

        if let Some(p) = config_path {
            if p.exists() {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    if let Ok(config) = Self::from_toml(&content) {
                        return config;
                    }
                }
            }
        }

        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.style, Style::Minimal);
        assert_eq!(cfg.theme, "colorblind");
        assert_eq!(cfg.segments, vec!["tokens", "session_cost", "month_cost"]);
        assert!(!cfg.session_cost.show_aic);
        assert!(!cfg.month_cost.show_aic);
    }

    #[test]
    fn test_serialize_and_deserialize_roundtrip() {
        let mut original = Config::default();
        original.style = Style::Powerline;
        original.session_cost.show_aic = true;
        original.month_cost.show_aic = true;

        let toml_str = original.to_toml_string().expect("serialize should succeed");
        let parsed: Config = Config::from_toml(&toml_str).expect("deserialize should succeed");

        assert_eq!(parsed.style, Style::Powerline);
        assert!(parsed.session_cost.show_aic);
        assert!(parsed.month_cost.show_aic);
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml_str = r#"
            style = "capsule"
            theme = "nord"

            [session_cost]
            show_aic = true
        "#;
        let parsed = Config::from_toml(toml_str).expect("partial TOML should parse");
        assert_eq!(parsed.style, Style::Capsule);
        assert_eq!(parsed.theme, "nord");
        assert!(parsed.session_cost.show_aic);
        // month_cost was omitted, should take default
        assert!(!parsed.month_cost.show_aic);
        assert_eq!(parsed.tokens.alert_threshold, 100_000);
    }
}
