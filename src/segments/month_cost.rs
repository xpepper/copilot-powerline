use crate::config::MonthCostConfig;
use crate::theme::Palette;

pub fn calculate_month_spend(total_month_nano: u64) -> (f64, f64) {
    let aic = total_month_nano as f64 / 1e9;
    let usd = aic * 0.01;
    (usd, aic)
}

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

    let r = palette.reset;
    let d = palette.dim;
    let lbl = palette.label;

    let (usd, aic) = calculate_month_spend(total_month_nano);
    let cost_str = format!(
        "{}{}{:.*}{}",
        palette.spend, config.currency_symbol, config.decimal_places, usd, r
    );

    let icon = crate::icons::month_icon(icon_set, config.prefix.as_deref());
    let mut out = format!("{}{}{} {}", lbl, icon, r, cost_str);

    if config.show_aic && aic >= 1.0 {
        out.push_str(&format!(" ({}{:.0} AIC{})", d, aic, r));
    }

    Some(out)
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
}
