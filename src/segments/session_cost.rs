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
    config: &CostConfig,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }

    let r = palette.reset;
    let d = palette.dim;
    let lbl = palette.label;

    let (usd, aic) = calculate_session_spend(total_nano_aiu);
    let cost_str = format!(
        "{}{}{:.*}{}",
        palette.spend, config.currency_symbol, config.decimal_places, usd, r
    );

    let mut out = format!("{}{} {}", lbl, config.prefix, cost_str);

    if config.show_aic && aic >= 0.1 {
        out.push_str(&format!(" ({}{:.1} AIC{})", d, aic, r));
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_spend_no_aic() {
        let cfg = CostConfig::default(); // show_aic: false
        let p = Palette::for_theme("plain");

        let rendered = render_session_cost_segment(0, &cfg, &p).unwrap();
        assert_eq!(rendered, "Session: $0.00");
    }

    #[test]
    fn test_spend_with_show_aic_enabled() {
        let mut cfg = CostConfig::default();
        cfg.show_aic = true;
        let p = Palette::for_theme("plain");

        // 500 billion nano AIU = 500 AIC = $5.00
        let rendered = render_session_cost_segment(500_000_000_000, &cfg, &p).unwrap();
        assert_eq!(rendered, "Session: $5.00 (500.0 AIC)");
    }

    #[test]
    fn test_spend_with_show_aic_disabled() {
        let mut cfg = CostConfig::default();
        cfg.show_aic = false;
        let p = Palette::for_theme("plain");

        let rendered = render_session_cost_segment(500_000_000_000, &cfg, &p).unwrap();
        assert_eq!(rendered, "Session: $5.00");
    }
}
