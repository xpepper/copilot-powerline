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

/// Which segment list to render: `segments` (full) or `compact_segments`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode {
    #[default]
    Full,
    Compact,
}

impl DisplayMode {
    pub fn toggled(self) -> Self {
        match self {
            DisplayMode::Full => DisplayMode::Compact,
            DisplayMode::Compact => DisplayMode::Full,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DisplayMode::Full => "full",
            DisplayMode::Compact => "compact",
        }
    }
}

impl std::str::FromStr for DisplayMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "full" => Ok(DisplayMode::Full),
            "compact" => Ok(DisplayMode::Compact),
            other => Err(format!("Unknown display mode: {other}")),
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
    #[serde(default)]
    pub mode: DisplayMode,
    #[serde(default = "default_segments")]
    pub segments: Vec<String>,
    /// Segments shown in compact mode: the always-useful essentials, leaving
    /// diagnostic segments (cache, reasoning, totals) for full mode.
    #[serde(default = "default_compact_segments")]
    pub compact_segments: Vec<String>,
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
    #[serde(default)]
    pub pr: PrConfig,
    #[serde(default)]
    pub model: ModelConfig,
}

fn default_theme() -> String {
    "colorblind".to_string()
}

fn default_compact_segments() -> Vec<String> {
    vec![
        "tokens".to_string(),
        "session_cost".to_string(),
        "month_cost".to_string(),
    ]
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
    #[serde(default = "default_tokens_alert_icon")]
    pub alert_icon: String,
}

impl Default for TokensConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: None,
            show_percentage: true,
            alert_threshold: 100_000,
            alert_icon: default_tokens_alert_icon(),
        }
    }
}

fn default_alert_threshold() -> u64 {
    100_000
}

fn default_tokens_alert_icon() -> String {
    "🔥 ".to_string()
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
    /// Flag the session cost when the latest step cost much more per token
    /// than the session average (expensive routing, cache misses). Opt-in
    /// until the thresholds are tuned on real sessions.
    #[serde(default)]
    pub spike_alert: bool,
    /// Multiple of the session's average cost per token that counts as a spike.
    #[serde(default = "default_spike_ratio")]
    pub spike_ratio: f64,
    /// Steps cheaper than this (in USD) never count as a spike.
    #[serde(default = "default_spike_min_usd")]
    pub spike_min_usd: f64,
    #[serde(default = "default_spike_icon")]
    pub spike_icon: String,
    /// Flag the session cost once it goes strictly above this many USD.
    /// Unset by default: there is no sensible budget to guess.
    #[serde(default)]
    pub alert_above_usd: Option<f64>,
    #[serde(default = "default_spend_alert_icon")]
    pub alert_icon: String,
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: None,
            currency_symbol: default_currency(),
            show_aic: false,
            decimal_places: 2,
            spike_alert: false,
            spike_ratio: default_spike_ratio(),
            spike_min_usd: default_spike_min_usd(),
            spike_icon: default_spike_icon(),
            alert_above_usd: None,
            alert_icon: default_spend_alert_icon(),
        }
    }
}

fn default_spike_icon() -> String {
    "📈 ".to_string()
}

fn default_spend_alert_icon() -> String {
    "💸 ".to_string()
}

fn default_spike_ratio() -> f64 {
    2.0
}

fn default_spike_min_usd() -> f64 {
    0.05
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
    /// Flag the month-to-date cost once it goes strictly above this many
    /// USD. Unset by default: there is no sensible budget to guess.
    #[serde(default)]
    pub alert_above_usd: Option<f64>,
    #[serde(default = "default_spend_alert_icon")]
    pub alert_icon: String,
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
            alert_above_usd: None,
            alert_icon: default_spend_alert_icon(),
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
    /// Color the hit rate and add an arrow (↑/↓) when the most recent call's
    /// cache hit rate is clearly above or below the session average.
    #[serde(default = "default_true")]
    pub show_trend: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: None,
            show_as_percentage: true,
            auto_hide_zero: true,
            show_trend: true,
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
            mode: DisplayMode::Full,
            segments: default_segments(),
            compact_segments: default_compact_segments(),
            tokens: TokensConfig::default(),
            session_cost: CostConfig::default(),
            month_cost: MonthCostConfig::default(),
            cache: CacheConfig::default(),
            reasoning: ReasoningConfig::default(),
            total_tokens: TotalTokensConfig::default(),
            pr: PrConfig::default(),
            model: ModelConfig::default(),
        }
    }
}

/// Optional active model name (e.g. `Claude Sonnet 4.5`, or
/// `Auto → Claude Opus 4.5` when `auto` routing picked a model).
///
/// Not in the default `segments` list: add `"model"` to opt in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub prefix: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: None,
        }
    }
}

/// Optional reference to the current branch's pull request (e.g. `PR #50`).
///
/// Disabled by default from the default `segments` list: add `"pr"` to
/// `segments` to opt in. Requires the `gh` CLI to be authenticated; PR
/// lookups run through the GitHub CLI in a throttled background process and
/// are cached to disk so the status line itself never blocks on a network
/// call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub prefix: Option<String>,
    #[serde(default = "default_true")]
    pub hyperlinks: bool,
    #[serde(default = "default_pr_cache_ttl")]
    pub cache_ttl_seconds: u64,
}

fn default_pr_cache_ttl() -> u64 {
    60
}

impl Default for PrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: None,
            hyperlinks: true,
            cache_ttl_seconds: default_pr_cache_ttl(),
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

    pub fn segments_for(&self, mode: DisplayMode) -> &[String] {
        match mode {
            DisplayMode::Full => &self.segments,
            DisplayMode::Compact => &self.compact_segments,
        }
    }

    pub fn default_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".copilot").join("powerline.toml"))
    }

    pub fn load_from_file_or_default(path: Option<&Path>) -> Self {
        let config_path = path.map(PathBuf::from).or_else(Self::default_config_path);

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
        // Opt-in until the thresholds are tuned on real sessions.
        assert!(!cfg.session_cost.spike_alert);
        assert_eq!(cfg.session_cost.spike_ratio, 2.0);
        assert_eq!(cfg.session_cost.spike_min_usd, 0.05);
        // No sensible default budget: off until the user sets a limit.
        assert_eq!(cfg.session_cost.alert_above_usd, None);
        assert_eq!(cfg.session_cost.alert_icon, "💸 ");
        assert_eq!(cfg.month_cost.alert_above_usd, None);
        assert_eq!(cfg.month_cost.alert_icon, "💸 ");
        // One icon per kind of alert, so the icon alone says which one fired.
        assert_eq!(cfg.tokens.alert_icon, "🔥 ");
        assert_eq!(cfg.session_cost.spike_icon, "📈 ");
        assert!(cfg.pr.enabled);
        assert!(cfg.pr.hyperlinks);
        assert_eq!(cfg.pr.cache_ttl_seconds, 60);
        // "pr" is opt-in: it must not appear in the default segment list.
        assert!(!cfg.segments.iter().any(|s| s == "pr"));
        assert!(!cfg.segments.iter().any(|s| s == "model"));
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

            [tokens]
            alert_threshold = 100000
        "#;
        let parsed = Config::from_toml(toml_str).expect("partial TOML should parse");
        assert_eq!(parsed.style, Style::Capsule);
        assert_eq!(parsed.theme, "nord");
        assert_eq!(parsed.icon_set, IconSet::Emoji);
        assert!(parsed.session_cost.show_aic);
        // spike_alert omitted from a present [session_cost] section: opt-in
        assert!(!parsed.session_cost.spike_alert);
        assert_eq!(parsed.session_cost.spike_icon, "📈 ");
        // month_cost was omitted, should take default
        assert!(!parsed.month_cost.show_aic);
        assert_eq!(parsed.tokens.alert_threshold, 100_000);
        assert_eq!(parsed.tokens.alert_icon, "🔥 ");
    }

    #[test]
    fn test_parse_session_cost_alert_limit() {
        let toml_str = r#"
            [session_cost]
            alert_above_usd = 5.0
        "#;
        let parsed = Config::from_toml(toml_str).expect("TOML should parse");
        assert_eq!(parsed.session_cost.alert_above_usd, Some(5.0));
        assert_eq!(parsed.session_cost.alert_icon, "💸 ");
    }

    #[test]
    fn test_parse_month_cost_alert_limit() {
        let toml_str = r#"
            [month_cost]
            alert_above_usd = 300.0
        "#;
        let parsed = Config::from_toml(toml_str).expect("TOML should parse");
        assert_eq!(parsed.month_cost.alert_above_usd, Some(300.0));
        assert_eq!(parsed.month_cost.alert_icon, "💸 ");
        // Independent of the session limit.
        assert_eq!(parsed.session_cost.alert_above_usd, None);
    }

    #[test]
    fn test_display_mode_defaults_to_full_with_essential_compact_list() {
        let cfg = Config::default();
        assert_eq!(cfg.mode, DisplayMode::Full);
        assert_eq!(cfg.segments_for(DisplayMode::Full), cfg.segments.as_slice());
        assert_eq!(
            cfg.segments_for(DisplayMode::Compact),
            ["tokens", "session_cost", "month_cost"]
        );
    }

    #[test]
    fn test_parse_compact_mode_and_custom_list() {
        let parsed = Config::from_toml(
            r#"
            mode = "compact"
            compact_segments = ["tokens", "model"]
        "#,
        )
        .expect("compact config should parse");
        assert_eq!(parsed.mode, DisplayMode::Compact);
        assert_eq!(parsed.segments_for(parsed.mode), ["tokens", "model"]);
    }

    #[test]
    fn test_display_mode_toggle_and_parse() {
        assert_eq!(DisplayMode::Full.toggled(), DisplayMode::Compact);
        assert_eq!(DisplayMode::Compact.toggled(), DisplayMode::Full);
        for mode in [DisplayMode::Full, DisplayMode::Compact] {
            assert_eq!(mode.as_str().parse::<DisplayMode>(), Ok(mode));
        }
        assert_eq!(
            " Compact\n".parse::<DisplayMode>(),
            Ok(DisplayMode::Compact)
        );
        assert!("sideways".parse::<DisplayMode>().is_err());
    }
}
