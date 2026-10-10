//! Spend helpers shared by the cost segments: the nano AIU to AIC to USD
//! conversion and the `alert_above_usd` check.

/// 1 AIC (AI credit) = 1e9 nano AIU.
const NANO_AIU_PER_AIC: f64 = 1e9;
/// List price of one AIC, in USD.
const USD_PER_AIC: f64 = 0.01;

/// AI credits for a count of nano AIU.
pub fn nano_to_aic(nano_aiu: u64) -> f64 {
    nano_aiu as f64 / NANO_AIU_PER_AIC
}

/// USD list price of `aic` AI credits. Callers convert nano AIU to AIC first,
/// then to USD, so float results stay identical to what the segments showed
/// before this was shared.
pub fn aic_to_usd(aic: f64) -> f64 {
    aic * USD_PER_AIC
}

/// Whether `usd` is strictly above `limit`, compared at the precision it is
/// shown with, so the alert never contradicts the number on screen and float
/// noise (0.57 computing to 0.5700000000000001) cannot trip it. No limit set
/// means never.
pub fn exceeds(usd: f64, decimal_places: usize, limit: Option<f64>) -> bool {
    let scale = 10f64.powi(decimal_places as i32);
    let shown_usd = (usd * scale).round() / scale;
    limit.is_some_and(|limit| shown_usd > limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nano_to_aic() {
        assert_eq!(nano_to_aic(0), 0.0);
        assert_eq!(nano_to_aic(1_000_000_000), 1.0);
        assert_eq!(nano_to_aic(57_000_000_000), 57.0);
    }

    #[test]
    fn test_aic_to_usd() {
        assert_eq!(aic_to_usd(0.0), 0.0);
        assert_eq!(aic_to_usd(100.0), 1.0);
    }

    #[test]
    fn test_rate_matches_the_spike_check_rate() {
        // `spend` keeps its own constant; compared as constants because the
        // computed values differ for some amounts.
        assert_eq!(
            NANO_AIU_PER_AIC / USD_PER_AIC,
            crate::spend::NANO_AIU_PER_USD
        );
    }

    #[test]
    fn test_conversion_keeps_float_operation_order() {
        // `exceeds` relies on this: 57 AIC computes to $0.5700000000000001.
        let usd = aic_to_usd(nano_to_aic(57_000_000_000));
        assert_eq!(usd, 0.5700000000000001);
    }

    #[test]
    fn test_no_limit_never_exceeds() {
        assert!(!exceeds(1_000_000.0, 2, None));
    }

    #[test]
    fn test_strictly_above_limit_exceeds() {
        assert!(!exceeds(5.0, 2, Some(5.0)));
        assert!(exceeds(5.01, 2, Some(5.0)));
    }

    #[test]
    fn test_compares_at_shown_precision() {
        // 57 AIC computes to $0.5700000000000001.
        assert!(!exceeds(57.0 * 0.01, 2, Some(0.57)));
        // $5.004 is shown as $5.00.
        assert!(!exceeds(5.004, 2, Some(5.0)));
        // With no decimals, $5.40 is shown as $5.
        assert!(!exceeds(5.4, 0, Some(5.0)));
    }
}
