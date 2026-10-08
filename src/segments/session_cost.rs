use crate::config::CostConfig;
use crate::theme::Palette;

pub fn calculate_session_spend(total_nano_aiu: u64) -> (f64, f64) {
    // 1 AIC = $0.01 USD; 1 nano AIU = 1e-9 AIC = 1e-11 USD
    let aic = total_nano_aiu as f64 / 1e9;
    let usd = aic * 0.01;
    (usd, aic)
}

pub fn render_session_cost_segment(
    total_nano_aiu: u64,
    spike: bool,
    config: &CostConfig,
    icon_set: crate::config::IconSet,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }

    let r = palette.reset;
    let d = palette.dim;
    let lbl = palette.label;

    let (usd, aic) = calculate_session_spend(total_nano_aiu);
    let (spike_icon, color) = if spike {
        (config.spike_icon.as_str(), palette.tokens_alert)
    } else {
        ("", palette.spend)
    };
    let cost_str = format!(
        "{}{}{:.*}{}",
        color, config.currency_symbol, config.decimal_places, usd, r
    );

    let icon = crate::icons::session_icon(icon_set, config.prefix.as_deref());
    let mut out = format!("{}{}{} {}{}", lbl, icon, r, spike_icon, cost_str);

    if config.show_aic && aic >= 0.1 {
        out.push_str(&format!(" ({}{:.1} AIC{})", d, aic, r));
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IconSet;

    #[test]
    fn test_zero_spend_no_aic() {
        let cfg = CostConfig::default(); // show_aic: false
        let p = Palette::for_theme("plain");

        let rendered = render_session_cost_segment(0, false, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Session: $0.00");
    }

    #[test]
    fn test_spend_with_emoji() {
        let cfg = CostConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_session_cost_segment(0, false, &cfg, IconSet::Emoji, &p).unwrap();
        assert_eq!(rendered, "💰 $0.00");
    }

    #[test]
    fn test_spend_with_show_aic_enabled() {
        let cfg = CostConfig {
            show_aic: true,
            ..Default::default()
        };
        let p = Palette::for_theme("plain");

        // 500 billion nano AIU = 500 AIC = $5.00
        let rendered =
            render_session_cost_segment(500_000_000_000, false, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Session: $5.00 (500.0 AIC)");
    }

    #[test]
    fn test_spend_with_show_aic_disabled() {
        let cfg = CostConfig {
            show_aic: false,
            ..Default::default()
        };
        let p = Palette::for_theme("plain");

        let rendered =
            render_session_cost_segment(500_000_000_000, false, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Session: $5.00");
    }

    #[test]
    fn test_spend_spike_is_flagged() {
        let cfg = CostConfig::default();
        let p = Palette::for_theme("plain");

        let rendered =
            render_session_cost_segment(80_000_000_000, true, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Session: ⚠️ $0.80");
    }

    #[test]
    fn test_spend_spike_uses_alert_color() {
        let cfg = CostConfig::default();
        let p = Palette::for_theme("github");

        let rendered =
            render_session_cost_segment(80_000_000_000, true, &cfg, IconSet::Plain, &p).unwrap();
        assert!(rendered.contains(&format!("{}$0.80{}", p.tokens_alert, p.reset)));
    }
}
