use crate::config::MonthCostConfig;
use crate::theme::Palette;

pub fn calculate_month_spend(total_month_nano: u64) -> (f64, f64) {
    let aic = total_month_nano as f64 / 1e9;
    let usd = aic * 0.01;
    (usd, aic)
}

pub fn render_month_cost_segment(
    total_month_nano: u64,
    config: &MonthCostConfig,
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

    let mut out = format!("{}{} {}", lbl, config.prefix, cost_str);

    if config.show_aic && aic >= 1.0 {
        out.push_str(&format!(" ({}{:.0} AIC{})", d, aic, r));
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_month_cost_no_aic() {
        let cfg = MonthCostConfig::default(); // show_aic: false
        let p = Palette::for_theme("plain");

        // 25938000000000 nano aiu = 25938 AIC = $259.38
        let rendered = render_month_cost_segment(25_938_000_000_000, &cfg, &p).unwrap();
        assert_eq!(rendered, "Month: $259.38");
    }

    #[test]
    fn test_month_cost_with_aic() {
        let mut cfg = MonthCostConfig::default();
        cfg.show_aic = true;
        let p = Palette::for_theme("plain");

        let rendered = render_month_cost_segment(25_938_000_000_000, &cfg, &p).unwrap();
        assert_eq!(rendered, "Month: $259.38 (25938 AIC)");
    }
}
