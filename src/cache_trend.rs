//! Cache hit trend: compares the hit rate of the latest step (the tokens
//! added since the previous refresh where the counters moved) with the
//! session average before that step.
//!
//! Copilot CLI's payload has no per-call cache counts: `current_usage` holds
//! session totals, and `last_call_*` has no cache split. So the latest step
//! is derived from the cumulative counters kept in a per-session snapshot.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Percentage points the latest step's hit rate must differ from the session
/// average before a trend is shown, so small fluctuations don't flicker.
const TREND_TOLERANCE_PCT: f64 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trend {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Cumulative input tokens, including cache reads and writes.
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    /// Verdict for the last step, kept while the counters don't move so
    /// timer-driven refreshes don't make the arrow flicker.
    pub trend: Option<Trend>,
}

/// Decides the trend of the step from `prev` to the current counters,
/// returning the snapshot to persist.
pub fn assess(prev: Option<Snapshot>, input_tokens: u64, cache_read_tokens: u64) -> Snapshot {
    let baseline = |trend| Snapshot {
        input_tokens,
        cache_read_tokens,
        trend,
    };

    let Some(prev) = prev else {
        return baseline(None);
    };
    if input_tokens == prev.input_tokens && cache_read_tokens == prev.cache_read_tokens {
        return prev;
    }
    if input_tokens <= prev.input_tokens
        || cache_read_tokens < prev.cache_read_tokens
        || prev.input_tokens == 0
    {
        return baseline(None);
    }

    let pct = |read: u64, input: u64| read as f64 / input as f64 * 100.0;
    let session_pct = pct(prev.cache_read_tokens, prev.input_tokens);
    let step_pct = pct(
        cache_read_tokens - prev.cache_read_tokens,
        input_tokens - prev.input_tokens,
    );

    let delta = step_pct - session_pct;
    baseline(if delta > TREND_TOLERANCE_PCT {
        Some(Trend::Up)
    } else if delta < -TREND_TOLERANCE_PCT {
        Some(Trend::Down)
    } else {
        None
    })
}

pub fn snapshot_path(session_id: &str) -> PathBuf {
    crate::state::session_file("cache", session_id)
}

/// Reads the session snapshot at `path`, assesses the current counters and
/// persists the result. Returns the trend to display, if any.
pub fn check_trend(path: &Path, input_tokens: u64, cache_read_tokens: u64) -> Option<Trend> {
    crate::state::update_snapshot(path, |prev| assess(prev, input_tokens, cache_read_tokens)).trend
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(input_tokens: u64, cache_read_tokens: u64, trend: Option<Trend>) -> Snapshot {
        Snapshot {
            input_tokens,
            cache_read_tokens,
            trend,
        }
    }

    #[test]
    fn test_first_observation_is_baseline_only() {
        assert_eq!(assess(None, 100_000, 60_000), snap(100_000, 60_000, None));
    }

    #[test]
    fn test_step_hitting_cache_more_than_average_trends_up() {
        // Session at 60%; the step adds 10k tokens, 9k from cache (90%).
        let prev = snap(100_000, 60_000, None);
        assert_eq!(
            assess(Some(prev), 110_000, 69_000),
            snap(110_000, 69_000, Some(Trend::Up))
        );
    }

    #[test]
    fn test_step_missing_cache_trends_down() {
        // Session at 60%; the step adds 10k tokens, 1k from cache (10%).
        let prev = snap(100_000, 60_000, None);
        assert_eq!(assess(Some(prev), 110_000, 61_000).trend, Some(Trend::Down));
    }

    #[test]
    fn test_step_close_to_average_has_no_trend() {
        // Session at 60%; the step hits 62%.
        let prev = snap(100_000, 60_000, Some(Trend::Down));
        assert_eq!(assess(Some(prev), 110_000, 66_200).trend, None);
    }

    #[test]
    fn test_unchanged_counters_keep_previous_verdict() {
        let prev = snap(110_000, 61_000, Some(Trend::Down));
        assert_eq!(assess(Some(prev), 110_000, 61_000), prev);
    }

    #[test]
    fn test_counters_going_down_rebaseline() {
        let prev = snap(110_000, 61_000, Some(Trend::Down));
        assert_eq!(assess(Some(prev), 5_000, 0), snap(5_000, 0, None));
    }

    #[test]
    fn test_no_baseline_without_prior_input() {
        let prev = snap(0, 0, None);
        assert_eq!(assess(Some(prev), 10_000, 0).trend, None);
    }

    /// Regression: replays counters from a live Copilot CLI 1.0.93 payload,
    /// where `current_usage` mirrored the session totals and the arrow never
    /// appeared. A 50k-token step written to (not read from) the cache must
    /// show as a drop.
    #[test]
    fn test_live_payload_cache_write_step_trends_down_and_sticks() {
        let path = std::env::temp_dir().join(format!(
            "copilot-powerline-cache-trend-{}-{}.json",
            std::process::id(),
            crate::github::current_timestamp()
        ));

        assert_eq!(check_trend(&path, 96_202, 96_192), None);
        assert_eq!(check_trend(&path, 146_242, 96_192), Some(Trend::Down));
        // Timer refresh, nothing new: the arrow stays.
        assert_eq!(check_trend(&path, 146_242, 96_192), Some(Trend::Down));

        let _ = std::fs::remove_file(&path);
    }
}
