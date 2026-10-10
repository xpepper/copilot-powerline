//! Rendering shared by the running spend totals, `month_cost` and
//! `cycle_cost`, so their alert and AIC formatting stay the same.

use super::spend_format::{self, aic_to_usd};
use crate::theme::Palette;

/// The display settings both totals' configs carry.
pub struct SpendTotal<'a> {
    pub currency_symbol: &'a str,
    pub decimal_places: usize,
    pub show_aic: bool,
    pub alert_above_usd: Option<f64>,
    pub alert_icon: &'a str,
}

/// Renders `aic` AI credits (1 AIC = USD 0.01) as `<icon> $12.34`, flagged
/// with the alert icon and color once above `alert_above_usd`.
pub fn render(icon: &str, aic: f64, total: &SpendTotal, palette: &Palette) -> String {
    let r = palette.reset;
    let d = palette.dim;
    let lbl = palette.label;

    let usd = aic_to_usd(aic);
    let (limit_icon, color) =
        if spend_format::exceeds(usd, total.decimal_places, total.alert_above_usd) {
            (total.alert_icon, palette.tokens_alert)
        } else {
            ("", palette.spend)
        };
    let cost_str = format!(
        "{}{}{:.*}{}",
        color, total.currency_symbol, total.decimal_places, usd, r
    );

    let mut out = format!("{}{}{} {}{}", lbl, icon, r, limit_icon, cost_str);

    if total.show_aic && aic >= 1.0 {
        out.push_str(&format!(" ({}{:.0} AIC{})", d, aic, r));
    }

    out
}
