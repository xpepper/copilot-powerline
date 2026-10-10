use super::spend_format::nano_to_aic;
use super::spend_total::{self, SpendTotal};
use crate::config::MonthCostConfig;
use crate::theme::Palette;

/// Whether the segment will render, so callers can skip the month-to-date
/// query (a full scan of the usage table) when nothing would show it.
pub fn is_visible(segments: &[String], config: &MonthCostConfig) -> bool {
    config.enabled && segments.iter().any(|s| s == "month_cost")
}

pub fn render_month_cost_segment(
    total_month_nano: u64,
    config: &MonthCostConfig,
    icon_set: crate::config::IconSet,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }

    let aic = nano_to_aic(total_month_nano);
    let icon = crate::icons::month_icon(icon_set, config.prefix.as_deref());
    let total = SpendTotal {
        currency_symbol: &config.currency_symbol,
        decimal_places: config.decimal_places,
        show_aic: config.show_aic,
        alert_above_usd: config.alert_above_usd,
        alert_icon: &config.alert_icon,
    };
    Some(spend_total::render(icon, aic, &total, palette))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IconSet;

    #[test]
    fn test_month_cost_no_aic() {
        let cfg = MonthCostConfig::default(); // show_aic: false
        let p = Palette::for_theme("plain");

        // 25938000000000 nano aiu = 25938 AIC = $259.38
        let rendered =
            render_month_cost_segment(25_938_000_000_000, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Month: $259.38");
    }

    #[test]
    fn test_month_cost_emoji() {
        let cfg = MonthCostConfig::default();
        let p = Palette::for_theme("plain");

        let rendered =
            render_month_cost_segment(25_938_000_000_000, &cfg, IconSet::Emoji, &p).unwrap();
        assert_eq!(rendered, "📅 $259.38");
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn test_visible_when_listed_and_enabled() {
        let cfg = MonthCostConfig::default();
        assert!(is_visible(&names(&["tokens", "month_cost"]), &cfg));
    }

    #[test]
    fn test_hidden_when_not_in_active_segments() {
        let cfg = MonthCostConfig::default();
        assert!(!is_visible(&names(&["tokens", "session_cost"]), &cfg));
    }

    #[test]
    fn test_hidden_when_disabled() {
        let cfg = MonthCostConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!is_visible(&names(&["month_cost"]), &cfg));
    }

    #[test]
    fn test_month_cost_with_aic() {
        let cfg = MonthCostConfig {
            show_aic: true,
            ..Default::default()
        };
        let p = Palette::for_theme("plain");

        let rendered =
            render_month_cost_segment(25_938_000_000_000, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Month: $259.38 (25938 AIC)");
    }

    fn with_limit(usd: f64) -> MonthCostConfig {
        MonthCostConfig {
            alert_above_usd: Some(usd),
            ..Default::default()
        }
    }

    #[test]
    fn test_month_cost_at_limit_is_not_flagged() {
        let p = Palette::for_theme("plain");

        let rendered =
            render_month_cost_segment(25_938_000_000_000, &with_limit(259.38), IconSet::Plain, &p)
                .unwrap();
        assert_eq!(rendered, "Month: $259.38");
    }

    #[test]
    fn test_month_cost_over_limit_is_flagged() {
        let p = Palette::for_theme("plain");

        let rendered =
            render_month_cost_segment(25_938_000_000_000, &with_limit(250.0), IconSet::Plain, &p)
                .unwrap();
        assert_eq!(rendered, "Month: 💸 $259.38");
    }

    #[test]
    fn test_month_cost_over_limit_uses_alert_color() {
        let p = Palette::for_theme("github");

        let rendered =
            render_month_cost_segment(25_938_000_000_000, &with_limit(250.0), IconSet::Plain, &p)
                .unwrap();
        assert!(rendered.contains(&format!("{}$259.38{}", p.tokens_alert, p.reset)));
    }

    #[test]
    fn test_month_cost_without_limit_is_never_flagged() {
        let cfg = MonthCostConfig::default(); // alert_above_usd: None
        let p = Palette::for_theme("github");

        let rendered =
            render_month_cost_segment(25_938_000_000_000, &cfg, IconSet::Plain, &p).unwrap();
        assert!(rendered.contains(&format!("{}$259.38{}", p.spend, p.reset)));
        assert!(!rendered.contains("💸"));
    }
}
