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
    pub cycle_cost: CycleCostConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub reasoning: ReasoningConfig,
    #[serde(default)]
    pub total_tokens: TotalTokensConfig,
    #[serde(default)]
    pub cache_expiry: CacheExpiryConfig,
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
        "cache_expiry".to_string(),
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
        "cache_expiry".to_string(),
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

/// Optional count of AI credits GitHub says you used this billing cycle,
/// across every Copilot surface (IDE, github.com, CLI), unlike `month_cost`,
/// which estimates Copilot CLI spend from the local database.
///
/// Not in the default `segments` list: add `"cycle_cost"` to opt in.
/// Requires the `gh` CLI to be authenticated. The value comes from GitHub's
/// internal, undocumented `/copilot_internal/user` API, fetched in a
/// throttled background process and cached to disk, like the `pr` segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleCostConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub prefix: Option<String>,
    #[serde(default = "default_currency")]
    pub currency_symbol: String,
    #[serde(default)]
    pub show_aic: bool,
    #[serde(default = "default_decimals")]
    pub decimal_places: usize,
    /// Flag the cycle cost once it goes strictly above this many USD.
    /// Unset by default: there is no sensible budget to guess.
    #[serde(default)]
    pub alert_above_usd: Option<f64>,
    #[serde(default = "default_spend_alert_icon")]
    pub alert_icon: String,
    /// How long a fetched value is considered fresh. GitHub caches the
    /// response for 60 s, and each fetch costs about a second of `gh` time.
    #[serde(default = "default_cycle_cache_ttl")]
    pub cache_ttl_seconds: u64,
}

fn default_cycle_cache_ttl() -> u64 {
    300
}

impl Default for CycleCostConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: None,
            currency_symbol: default_currency(),
            show_aic: false,
            decimal_places: 2,
            alert_above_usd: None,
            alert_icon: default_spend_alert_icon(),
            cache_ttl_seconds: default_cycle_cache_ttl(),
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

/// Warning shown once the session has been idle longer than the prompt
/// cache TTL, so the next turn rewrites the whole context (e.g.
/// `~92k uncached · /clear to start fresh`). Renders nothing otherwise.
///
/// Needs `statusLine.refreshInterval` in Copilot CLI's settings: without it
/// the status line is not refreshed while idle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheExpiryConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub prefix: Option<String>,
    /// Prompt cache lifetime: idle time after which the cache is assumed gone.
    #[serde(default = "default_cache_ttl_seconds")]
    pub ttl_seconds: u64,
    /// Stay quiet below this context size, where starting fresh saves little.
    #[serde(default = "default_cache_expiry_min_tokens")]
    pub min_tokens: u64,
    /// Shown after the uncached size; empty hides it.
    #[serde(default = "default_cache_expiry_hint")]
    pub hint: String,
}

fn default_cache_ttl_seconds() -> u64 {
    300
}

fn default_cache_expiry_min_tokens() -> u64 {
    50_000
}

fn default_cache_expiry_hint() -> String {
    "/clear to start fresh".to_string()
}

impl Default for CacheExpiryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: None,
            ttl_seconds: default_cache_ttl_seconds(),
            min_tokens: default_cache_expiry_min_tokens(),
            hint: default_cache_expiry_hint(),
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
            cycle_cost: CycleCostConfig::default(),
            cache: CacheConfig::default(),
            reasoning: ReasoningConfig::default(),
            total_tokens: TotalTokensConfig::default(),
            cache_expiry: CacheExpiryConfig::default(),
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
                "total_tokens",
                "cache_expiry"
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
        // "cycle_cost" calls an internal GitHub API in the background: opt-in.
        assert!(!cfg.segments.iter().any(|s| s == "cycle_cost"));
        assert!(!cfg.compact_segments.iter().any(|s| s == "cycle_cost"));
        assert!(cfg.cycle_cost.enabled);
        assert_eq!(cfg.cycle_cost.cache_ttl_seconds, 300);
        assert_eq!(cfg.cycle_cost.alert_above_usd, None);
        assert!(cfg.cache_expiry.enabled);
        assert_eq!(cfg.cache_expiry.ttl_seconds, 300);
        assert_eq!(cfg.cache_expiry.min_tokens, 50_000);
        assert_eq!(cfg.cache_expiry.hint, "/clear to start fresh");
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
            ["tokens", "session_cost", "month_cost", "cache_expiry"]
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

    /// Paths of keys in `example` that do not exist in `known`.
    fn unknown_keys(example: &toml::Table, known: &toml::Table, path: &str) -> Vec<String> {
        let mut unknown = Vec::new();
        for (key, value) in example {
            let key_path = format!("{path}{key}");
            match (value, known.get(key)) {
                (_, None) => unknown.push(key_path),
                (toml::Value::Table(sub), Some(toml::Value::Table(known_sub))) => {
                    unknown.extend(unknown_keys(sub, known_sub, &format!("{key_path}.")));
                }
                _ => {}
            }
        }
        unknown
    }

    /// Unknown keys are silently ignored when parsing, so a typo or a
    /// renamed option in the shipped example would go unnoticed.
    #[test]
    fn test_example_config_uses_only_known_keys() {
        let example = include_str!("../examples/powerline.toml");
        Config::from_toml(example).expect("example config should parse");

        // Optional keys are left out of the serialized form while unset.
        let mut all = Config::default();
        let prefix = Some("x".to_string());
        all.tokens.prefix = prefix.clone();
        all.session_cost.prefix = prefix.clone();
        all.session_cost.alert_above_usd = Some(1.0);
        all.month_cost.prefix = prefix.clone();
        all.month_cost.alert_above_usd = Some(1.0);
        all.month_cost.db_path = Some(PathBuf::from("x.db"));
        all.cycle_cost.prefix = prefix.clone();
        all.cycle_cost.alert_above_usd = Some(1.0);
        all.cache.prefix = prefix.clone();
        all.reasoning.prefix = prefix.clone();
        all.total_tokens.prefix = prefix.clone();
        all.cache_expiry.prefix = prefix.clone();
        all.model.prefix = prefix.clone();
        all.pr.prefix = prefix;
        let known: toml::Table = toml::from_str(&all.to_toml_string().unwrap()).unwrap();

        let example: toml::Table = toml::from_str(example).unwrap();
        assert_eq!(unknown_keys(&example, &known, ""), Vec::<String>::new());
    }
}
