//! Spend spike detection: flags a refresh where the session's cost grew much
//! faster per token than its running average, a hint of expensive routing or
//! repeated uncached retries before the bill becomes a surprise.
//!
//! Each refresh is a fresh process, so the previous counters are kept in a
//! small per-session snapshot file.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 1 USD = 100 AIC = 1e11 nano AIU.
const NANO_AIU_PER_USD: f64 = 1e11;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub nano_aiu: u64,
    pub total_tokens: u64,
    /// Verdict for the last step, kept while the counters don't move so
    /// timer-driven refreshes don't make the warning flicker.
    pub alert: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpikeRule {
    /// How many times the session's average cost per token a step must reach.
    pub ratio: f64,
    /// Ignore steps cheaper than this, however steep their rate.
    pub min_usd: f64,
}

/// Decides whether the step from `prev` to the current counters is a spike,
/// returning the snapshot to persist.
pub fn assess(
    prev: Option<Snapshot>,
    nano_aiu: u64,
    total_tokens: u64,
    rule: SpikeRule,
) -> Snapshot {
    let baseline = |alert| Snapshot {
        nano_aiu,
        total_tokens,
        alert,
    };

    let Some(prev) = prev else {
        return baseline(false);
    };
    if nano_aiu == prev.nano_aiu && total_tokens == prev.total_tokens {
        return prev;
    }
    if nano_aiu < prev.nano_aiu || total_tokens < prev.total_tokens || prev.total_tokens == 0 {
        return baseline(false);
    }

    let step_nano = (nano_aiu - prev.nano_aiu) as f64;
    let step_tokens = (total_tokens - prev.total_tokens) as f64;
    if step_nano / NANO_AIU_PER_USD < rule.min_usd {
        return baseline(false);
    }

    // Compare rates as cross-products to avoid dividing by a zero token step.
    let session_rate = prev.nano_aiu as f64 / prev.total_tokens as f64;
    baseline(step_nano > rule.ratio * session_rate * step_tokens)
}

pub fn snapshot_path(session_id: &str) -> PathBuf {
    crate::state::session_file("spend", session_id)
}

/// Reads the session snapshot at `path`, assesses the current counters and
/// persists the result. Returns whether the session cost should be flagged.
pub fn check_spike(path: &Path, nano_aiu: u64, total_tokens: u64, rule: SpikeRule) -> bool {
    crate::state::update_snapshot(path, |prev| assess(prev, nano_aiu, total_tokens, rule)).alert
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULE: SpikeRule = SpikeRule {
        ratio: 2.0,
        min_usd: 0.05,
    };
    /// $0.01 in nano AIU.
    const CENT: u64 = 1_000_000_000;

    #[test]
    fn test_usd_rate_matches_segment_conversion_constants() {
        use crate::segments::spend_format::{NANO_AIU_PER_AIC, USD_PER_AIC};
        // Compared as constants: the computed values differ for some amounts.
        assert_eq!(NANO_AIU_PER_AIC / USD_PER_AIC, NANO_AIU_PER_USD);
    }

    fn snap(nano_aiu: u64, total_tokens: u64, alert: bool) -> Snapshot {
        Snapshot {
            nano_aiu,
            total_tokens,
            alert,
        }
    }

    #[test]
    fn test_first_observation_is_baseline_only() {
        let s = assess(None, 50 * CENT, 100_000, RULE);
        assert_eq!(s, snap(50 * CENT, 100_000, false));
    }

    #[test]
    fn test_step_at_session_rate_is_not_a_spike() {
        // Baseline $0.50 per 100k tokens; step adds $0.25 per 50k tokens.
        let prev = snap(50 * CENT, 100_000, false);
        assert!(!assess(Some(prev), 75 * CENT, 150_000, RULE).alert);
    }

    #[test]
    fn test_step_far_above_session_rate_is_a_spike() {
        // Step adds $0.30 for 10k tokens: 6x the baseline rate.
        let prev = snap(50 * CENT, 100_000, false);
        let s = assess(Some(prev), 80 * CENT, 110_000, RULE);
        assert_eq!(s, snap(80 * CENT, 110_000, true));
    }

    #[test]
    fn test_cheap_steep_step_is_ignored() {
        // 6x the rate, but only $0.03 spent.
        let prev = snap(50 * CENT, 100_000, false);
        assert!(!assess(Some(prev), 53 * CENT, 101_000, RULE).alert);
    }

    #[test]
    fn test_spend_without_new_tokens_is_a_spike() {
        let prev = snap(50 * CENT, 100_000, false);
        assert!(assess(Some(prev), 60 * CENT, 100_000, RULE).alert);
    }

    #[test]
    fn test_unchanged_counters_keep_previous_verdict() {
        let prev = snap(80 * CENT, 110_000, true);
        assert_eq!(assess(Some(prev), 80 * CENT, 110_000, RULE), prev);
    }

    #[test]
    fn test_counters_going_down_rebaseline() {
        let prev = snap(80 * CENT, 110_000, true);
        let s = assess(Some(prev), 5 * CENT, 2_000, RULE);
        assert_eq!(s, snap(5 * CENT, 2_000, false));
    }

    #[test]
    fn test_no_baseline_without_prior_tokens() {
        let prev = snap(0, 0, false);
        assert!(!assess(Some(prev), 90 * CENT, 1_000, RULE).alert);
    }

    #[test]
    fn test_check_spike_persists_between_calls() {
        let path = std::env::temp_dir().join(format!(
            "copilot-powerline-spend-{}-{}.json",
            std::process::id(),
            crate::github::current_timestamp()
        ));

        assert!(!check_spike(&path, 50 * CENT, 100_000, RULE));
        assert!(check_spike(&path, 80 * CENT, 110_000, RULE));
        // Timer refresh, nothing new: the warning stays up.
        assert!(check_spike(&path, 80 * CENT, 110_000, RULE));
        // A normal step clears it.
        assert!(!check_spike(&path, 90 * CENT, 140_000, RULE));

        let _ = std::fs::remove_file(&path);
    }
}
