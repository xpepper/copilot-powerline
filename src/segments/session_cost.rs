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
    // Compare the amount as shown, so the icon never contradicts the number
    // and float noise (0.57 computing to 0.5700000000000001) cannot trip it.
    let scale = 10f64.powi(config.decimal_places as i32);
    let shown_usd = (usd * scale).round() / scale;
    let over_limit = config
        .alert_above_usd
        .is_some_and(|limit| shown_usd > limit);
    let limit_icon = if over_limit {
        config.alert_icon.as_str()
    } else {
        ""
    };
    let spike_icon = if spike {
        config.spike_icon.as_str()
    } else {
        ""
    };
    let color = if over_limit || spike {
        palette.tokens_alert
    } else {
        palette.spend
    };
    let cost_str = format!(
        "{}{}{:.*}{}",
        color, config.currency_symbol, config.decimal_places, usd, r
    );

    let icon = crate::icons::session_icon(icon_set, config.prefix.as_deref());
    let mut out = format!(
        "{}{}{} {}{}{}",
        lbl, icon, r, limit_icon, spike_icon, cost_str
    );

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

    fn with_limit(usd: f64) -> CostConfig {
        CostConfig {
            alert_above_usd: Some(usd),
            ..Default::default()
        }
    }

    #[test]
    fn test_spend_under_limit_is_not_flagged() {
        let p = Palette::for_theme("plain");

        // 400 billion nano AIU = $4.00
        let rendered = render_session_cost_segment(
            400_000_000_000,
            false,
            &with_limit(5.0),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Session: $4.00");
    }

    #[test]
    fn test_spend_exactly_at_limit_is_not_flagged() {
        let p = Palette::for_theme("plain");

        let rendered = render_session_cost_segment(
            500_000_000_000,
            false,
            &with_limit(5.0),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Session: $5.00");
    }

    #[test]
    fn test_spend_at_limit_ignores_float_error() {
        let p = Palette::for_theme("plain");

        // 57 billion nano AIU computes to $0.5700000000000001 in f64.
        let rendered = render_session_cost_segment(
            57_000_000_000,
            false,
            &with_limit(0.57),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Session: $0.57");
    }

    #[test]
    fn test_spend_over_limit_only_once_the_shown_amount_is() {
        let p = Palette::for_theme("plain");

        // $5.004 is shown as $5.00: the icon must not contradict the number.
        let rendered = render_session_cost_segment(
            500_400_000_000,
            false,
            &with_limit(5.0),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Session: $5.00");
    }

    #[test]
    fn test_spend_over_limit_is_flagged() {
        let p = Palette::for_theme("plain");

        // 620 billion nano AIU = $6.20
        let rendered = render_session_cost_segment(
            620_000_000_000,
            false,
            &with_limit(5.0),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Session: 💸 $6.20");
    }

    #[test]
    fn test_spend_over_limit_uses_alert_color() {
        let p = Palette::for_theme("github");

        let rendered = render_session_cost_segment(
            620_000_000_000,
            false,
            &with_limit(5.0),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert!(rendered.contains(&format!("{}$6.20{}", p.tokens_alert, p.reset)));
    }

    #[test]
    fn test_spend_without_limit_is_never_flagged() {
        let cfg = CostConfig::default(); // alert_above_usd: None
        let p = Palette::for_theme("plain");

        let rendered =
            render_session_cost_segment(620_000_000_000_000, false, &cfg, IconSet::Plain, &p)
                .unwrap();
        assert_eq!(rendered, "Session: $6200.00");
    }

    #[test]
    fn test_spike_over_limit_shows_both_icons() {
        let p = Palette::for_theme("plain");

        let rendered = render_session_cost_segment(
            620_000_000_000,
            true,
            &with_limit(5.0),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Session: 💸 ⚠️ $6.20");
    }
}
