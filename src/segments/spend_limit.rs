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
