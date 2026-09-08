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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IconSet {
    #[default]
    Plain,
    Nerd,
    Emoji,
}

impl std::str::FromStr for IconSet {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plain" => Ok(IconSet::Plain),
            "nerd" => Ok(IconSet::Nerd),
            "emoji" => Ok(IconSet::Emoji),
            other => Err(format!("Unknown icon set: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub style: Style,
    #[serde(default)]
    pub icon_set: IconSet,
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
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub reasoning: ReasoningConfig,
    #[serde(default)]
    pub total_tokens: TotalTokensConfig,
}

fn default_theme() -> String {
    "colorblind".to_string()
}

fn default_segments() -> Vec<String> {
    vec![
        "tokens".to_string(),
        "session_cost".to_string(),
        "month_cost".to_string(),
        "cache".to_string(),
        "reasoning".to_string(),
        "total_tokens".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokensConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub prefix: Option<String>,
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
            prefix: None,
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
    pub prefix: Option<String>,
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
            prefix: None,
            currency_symbol: default_currency(),
            show_aic: false,
            decimal_places: 2,
        }
    }
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
    pub prefix: Option<String>,
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
            prefix: None,
            currency_symbol: default_currency(),
            show_aic: false,
            decimal_places: 2,
            db_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub prefix: Option<String>,
    #[serde(default = "default_true")]
    pub show_as_percentage: bool,
    #[serde(default = "default_true")]
    pub auto_hide_zero: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: None,
            show_as_percentage: true,
            auto_hide_zero: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub prefix: Option<String>,
    #[serde(default = "default_true")]
    pub auto_hide_zero: bool,
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: None,
            auto_hide_zero: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalTokensConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub prefix: Option<String>,
}

impl Default for TotalTokensConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            style: Style::Minimal,
            icon_set: IconSet::Plain,
            theme: default_theme(),
            segments: default_segments(),
            tokens: TokensConfig::default(),
            session_cost: CostConfig::default(),
            month_cost: MonthCostConfig::default(),
            cache: CacheConfig::default(),
            reasoning: ReasoningConfig::default(),
            total_tokens: TotalTokensConfig::default(),
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

        config_path
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|content| Self::from_toml(&content).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.style, Style::Minimal);
        assert_eq!(cfg.icon_set, IconSet::Plain);
        assert_eq!(cfg.theme, "colorblind");
        assert_eq!(
            cfg.segments,
            vec![
                "tokens",
                "session_cost",
                "month_cost",
                "cache",
                "reasoning",
                "total_tokens"
            ]
        );
        assert!(!cfg.session_cost.show_aic);
        assert!(!cfg.month_cost.show_aic);
    }

    #[test]
    fn test_serialize_and_deserialize_roundtrip() {
        let original = Config {
            style: Style::Powerline,
            icon_set: IconSet::Nerd,
            session_cost: CostConfig {
                show_aic: true,
                ..Default::default()
            },
            month_cost: MonthCostConfig {
                show_aic: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let toml_str = original.to_toml_string().expect("serialize should succeed");
        let parsed: Config = Config::from_toml(&toml_str).expect("deserialize should succeed");

        assert_eq!(parsed.style, Style::Powerline);
        assert_eq!(parsed.icon_set, IconSet::Nerd);
        assert!(parsed.session_cost.show_aic);
        assert!(parsed.month_cost.show_aic);
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml_str = r#"
            style = "capsule"
            theme = "nord"
            icon_set = "emoji"

            [session_cost]
            show_aic = true
        "#;
        let parsed = Config::from_toml(toml_str).expect("partial TOML should parse");
        assert_eq!(parsed.style, Style::Capsule);
        assert_eq!(parsed.theme, "nord");
        assert_eq!(parsed.icon_set, IconSet::Emoji);
        assert!(parsed.session_cost.show_aic);
        // month_cost was omitted, should take default
        assert!(!parsed.month_cost.show_aic);
        assert_eq!(parsed.tokens.alert_threshold, 100_000);
    }
}
