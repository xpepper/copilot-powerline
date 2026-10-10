//! Cache hit trend: compares the hit rate of the latest step (the tokens
//! added since the previous refresh where the counters moved) with the
//! session average before that step. Also remembers the latest large cache
//! miss, which a cumulative rate and a one-step arrow both hide (#83).
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

/// A step that sent many input tokens without reading them from the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Miss {
    /// Input tokens of the step not served from the cache.
    pub tokens: u64,
    /// Unix time (seconds) when the step was seen.
    pub at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Cumulative input tokens, including cache reads and writes.
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    /// Verdict for the last step, kept while the counters don't move so
    /// timer-driven refreshes don't make the arrow flicker.
    pub trend: Option<Trend>,
    /// Latest step with at least `miss_min_tokens` uncached input tokens.
    /// Defaults so snapshots written before it existed still load.
    #[serde(default)]
    pub last_miss: Option<Miss>,
}

impl Snapshot {
    /// Size of the last miss while it is at most `visible_seconds` old.
    pub fn recent_miss(&self, now: u64, visible_seconds: u64) -> Option<u64> {
        self.last_miss
            .filter(|miss| now.saturating_sub(miss.at) <= visible_seconds)
            .map(|miss| miss.tokens)
    }
}

/// Decides the trend of the step from `prev` to the current counters, and
/// whether it was a miss of at least `miss_min_tokens` uncached tokens,
/// returning the snapshot to persist.
pub fn assess(
    prev: Option<Snapshot>,
    input_tokens: u64,
    cache_read_tokens: u64,
    now: u64,
    miss_min_tokens: u64,
) -> Snapshot {
    let baseline = |trend, last_miss| Snapshot {
        input_tokens,
        cache_read_tokens,
        trend,
        last_miss,
    };

    let Some(prev) = prev else {
        return baseline(None, None);
    };
    if input_tokens == prev.input_tokens && cache_read_tokens == prev.cache_read_tokens {
        return prev;
    }
    if input_tokens <= prev.input_tokens
        || cache_read_tokens < prev.cache_read_tokens
        || prev.input_tokens == 0
    {
        return baseline(None, None);
    }

    let step_input = input_tokens - prev.input_tokens;
    let step_read = cache_read_tokens - prev.cache_read_tokens;

    let uncached = step_input.saturating_sub(step_read);
    let last_miss = if uncached >= miss_min_tokens {
        Some(Miss {
            tokens: uncached,
            at: now,
        })
    } else {
        prev.last_miss
    };

    let pct = |read: u64, input: u64| read as f64 / input as f64 * 100.0;
    let delta = pct(step_read, step_input) - pct(prev.cache_read_tokens, prev.input_tokens);
    let trend = if delta > TREND_TOLERANCE_PCT {
        Some(Trend::Up)
    } else if delta < -TREND_TOLERANCE_PCT {
        Some(Trend::Down)
    } else {
        None
    };
    baseline(trend, last_miss)
}

pub fn snapshot_path(session_id: &str) -> PathBuf {
    crate::state::session_file("cache", session_id)
}

/// When a step counts as a miss, and how long it stays visible.
#[derive(Debug, Clone, Copy)]
pub struct MissRule {
    pub min_tokens: u64,
    pub visible_seconds: u64,
}

/// What the cache segment shows from the session snapshot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Signals {
    pub trend: Option<Trend>,
    /// Size of a miss at most `visible_seconds` old.
    pub recent_miss: Option<u64>,
}

/// Reads the session snapshot at `path`, assesses the current counters and
/// persists the result. Returns what to display.
pub fn check(
    path: &Path,
    input_tokens: u64,
    cache_read_tokens: u64,
    now: u64,
    rule: MissRule,
) -> Signals {
    let snapshot = crate::state::update_snapshot(path, |prev| {
        assess(prev, input_tokens, cache_read_tokens, now, rule.min_tokens)
    });
    Signals {
        trend: snapshot.trend,
        recent_miss: snapshot.recent_miss(now, rule.visible_seconds),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_000_000;
    const MISS_MIN: u64 = 50_000;
    const RULE: MissRule = MissRule {
        min_tokens: MISS_MIN,
        visible_seconds: 300,
    };

    fn snap(input_tokens: u64, cache_read_tokens: u64, trend: Option<Trend>) -> Snapshot {
        Snapshot {
            input_tokens,
            cache_read_tokens,
            trend,
            last_miss: None,
        }
    }

    fn assess(prev: Option<Snapshot>, input_tokens: u64, cache_read_tokens: u64) -> Snapshot {
        super::assess(prev, input_tokens, cache_read_tokens, NOW, MISS_MIN)
    }

    fn miss(tokens: u64, at: u64) -> Option<Miss> {
        Some(Miss { tokens, at })
    }

    #[test]
    fn test_large_uncached_step_is_recorded_as_miss() {
        // The #83 cold restart: a 2.0M-token session at 94%, then a step of
        // 133k tokens with nothing read from the cache.
        let prev = snap(2_000_000, 1_886_000, None);
        let next = assess(Some(prev), 2_133_000, 1_886_000);
        assert_eq!(next.last_miss, miss(133_000, NOW));
    }

    #[test]
    fn test_miss_counts_only_the_uncached_part_of_the_step() {
        // 200k step, 120k of it read from the cache: 80k missed.
        let prev = snap(1_000_000, 900_000, None);
        let next = assess(Some(prev), 1_200_000, 1_020_000);
        assert_eq!(next.last_miss, miss(80_000, NOW));
    }

    #[test]
    fn test_step_below_the_floor_is_not_a_miss_and_keeps_the_last_one() {
        let prev = Snapshot {
            last_miss: miss(133_000, NOW - 60),
            ..snap(2_133_000, 1_886_000, None)
        };
        // 140k step, 130k cached: only 10k missed.
        let next = assess(Some(prev), 2_273_000, 2_016_000);
        assert_eq!(next.last_miss, miss(133_000, NOW - 60));
    }

    #[test]
    fn test_newer_miss_replaces_the_last_one() {
        let prev = Snapshot {
            last_miss: miss(133_000, NOW - 60),
            ..snap(2_133_000, 1_886_000, None)
        };
        let next = assess(Some(prev), 2_193_000, 1_886_000);
        assert_eq!(next.last_miss, miss(60_000, NOW));
    }

    #[test]
    fn test_first_step_of_a_session_is_not_a_miss() {
        // Nothing can be cached before the first call.
        let prev = snap(0, 0, None);
        assert_eq!(assess(Some(prev), 80_000, 0).last_miss, None);
    }

    #[test]
    fn test_counters_going_down_forget_the_miss() {
        let prev = Snapshot {
            last_miss: miss(133_000, NOW - 60),
            ..snap(2_133_000, 1_886_000, None)
        };
        assert_eq!(assess(Some(prev), 5_000, 0).last_miss, None);
    }

    #[test]
    fn test_unchanged_counters_keep_the_miss() {
        let prev = Snapshot {
            last_miss: miss(133_000, NOW - 60),
            ..snap(2_133_000, 1_886_000, None)
        };
        assert_eq!(assess(Some(prev), 2_133_000, 1_886_000), prev);
    }

    #[test]
    fn test_miss_is_recent_within_the_visible_window() {
        let s = Snapshot {
            last_miss: miss(133_000, NOW),
            ..snap(2_133_000, 1_886_000, None)
        };
        assert_eq!(s.recent_miss(NOW, 300), Some(133_000));
        assert_eq!(s.recent_miss(NOW + 300, 300), Some(133_000));
        assert_eq!(s.recent_miss(NOW + 301, 300), None);
    }

    #[test]
    fn test_no_recent_miss_without_one() {
        assert_eq!(snap(100, 60, None).recent_miss(NOW, 300), None);
    }

    #[test]
    fn test_snapshot_written_before_last_miss_existed_still_loads() {
        let old = r#"{"input_tokens":100,"cache_read_tokens":60,"trend":"Down"}"#;
        let loaded: Snapshot = serde_json::from_str(old).unwrap();
        assert_eq!(loaded, snap(100, 60, Some(Trend::Down)));
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

        let trend = |input, read| check(&path, input, read, NOW, RULE).trend;
        assert_eq!(trend(96_202, 96_192), None);
        assert_eq!(trend(146_242, 96_192), Some(Trend::Down));
        // Timer refresh, nothing new: the arrow stays.
        assert_eq!(trend(146_242, 96_192), Some(Trend::Down));

        let _ = std::fs::remove_file(&path);
    }
}
