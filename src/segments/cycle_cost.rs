use super::spend_limit;
use crate::config::CycleCostConfig;
use crate::cycle_usage::CycleUsage;
use crate::theme::Palette;

pub fn render_cycle_cost_segment(
    usage: Option<&CycleUsage>,
    config: &CycleCostConfig,
    icon_set: crate::config::IconSet,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }
    let aic = usage?.credits;

    let r = palette.reset;
    let d = palette.dim;
    let lbl = palette.label;

    let usd = aic * 0.01;
    let (limit_icon, color) =
        if spend_limit::exceeds(usd, config.decimal_places, config.alert_above_usd) {
            (config.alert_icon.as_str(), palette.tokens_alert)
        } else {
            ("", palette.spend)
        };
    let cost_str = format!(
        "{}{}{:.*}{}",
        color, config.currency_symbol, config.decimal_places, usd, r
    );

    let icon = crate::icons::cycle_icon(icon_set, config.prefix.as_deref());
    let mut out = format!("{}{}{} {}{}", lbl, icon, r, limit_icon, cost_str);

    if config.show_aic && aic >= 1.0 {
        out.push_str(&format!(" ({}{:.0} AIC{})", d, aic, r));
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IconSet;

    fn usage(credits: f64) -> CycleUsage {
        CycleUsage {
            credits,
            resets_at: 1_793_491_200,
        }
    }

    fn render(credits: f64, cfg: &CycleCostConfig, icon_set: IconSet) -> Option<String> {
        let p = Palette::for_theme("plain");
        render_cycle_cost_segment(Some(&usage(credits)), cfg, icon_set, &p)
    }

    #[test]
    fn test_cycle_cost_in_usd() {
        let cfg = CycleCostConfig::default();
        assert_eq!(
            render(15649.0, &cfg, IconSet::Plain).as_deref(),
            Some("Cycle: $156.49")
        );
        assert_eq!(
            render(15649.0, &cfg, IconSet::Emoji).as_deref(),
            Some("🐙 $156.49")
        );
    }

    #[test]
    fn test_cycle_cost_with_aic() {
        let cfg = CycleCostConfig {
            show_aic: true,
            ..Default::default()
        };
        assert_eq!(
            render(15649.0, &cfg, IconSet::Plain).as_deref(),
            Some("Cycle: $156.49 (15649 AIC)")
        );
    }

    #[test]
    fn test_cycle_cost_custom_prefix_and_precision() {
        let cfg = CycleCostConfig {
            prefix: Some("GitHub:".to_string()),
            decimal_places: 0,
            ..Default::default()
        };
        assert_eq!(
            render(15649.0, &cfg, IconSet::Plain).as_deref(),
            Some("GitHub: $156")
        );
    }

    #[test]
    fn test_cycle_cost_over_limit_is_flagged() {
        let cfg = CycleCostConfig {
            alert_above_usd: Some(150.0),
            ..Default::default()
        };
        assert_eq!(
            render(15649.0, &cfg, IconSet::Plain).as_deref(),
            Some("Cycle: 💸 $156.49")
        );

        let p = Palette::for_theme("github");
        let rendered =
            render_cycle_cost_segment(Some(&usage(15649.0)), &cfg, IconSet::Plain, &p).unwrap();
        assert!(rendered.contains(&format!("{}$156.49{}", p.tokens_alert, p.reset)));
    }

    #[test]
    fn test_cycle_cost_at_limit_is_not_flagged() {
        let cfg = CycleCostConfig {
            alert_above_usd: Some(156.49),
            ..Default::default()
        };
        assert_eq!(
            render(15649.0, &cfg, IconSet::Plain).as_deref(),
            Some("Cycle: $156.49")
        );
    }

    #[test]
    fn test_hidden_without_usage_or_when_disabled() {
        let p = Palette::for_theme("plain");
        let cfg = CycleCostConfig::default();
        assert_eq!(
            render_cycle_cost_segment(None, &cfg, IconSet::Plain, &p),
            None
        );

        let disabled = CycleCostConfig {
            enabled: false,
            ..Default::default()
        };
        assert_eq!(render(15649.0, &disabled, IconSet::Plain), None);
    }
}
