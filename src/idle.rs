//! Idle time since the session's last step, used by the long-break reminder
//! (`cache_expiry`). A step is any model call, sub-agents included; typing,
//! reading and long tool runs all count as idle.
//!
//! The payload carries no timestamps, so the time the session's token counter
//! last moved is kept in a per-session snapshot. Copilot CLI refreshes the
//! status line on a timer while idle (when `statusLine.refreshInterval` is
//! set), which is what lets the idle time grow between steps.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Cumulative session tokens; any change means a new step happened.
    pub tokens: u64,
    /// Unix time (seconds) when `tokens` was first seen at its current value.
    pub changed_at: u64,
}

/// Restarts the idle clock when the counter moved, returning the snapshot to
/// persist.
pub fn assess(prev: Option<Snapshot>, tokens: u64, now: u64) -> Snapshot {
    match prev {
        Some(prev) if prev.tokens == tokens => prev,
        _ => Snapshot {
            tokens,
            changed_at: now,
        },
    }
}

pub fn snapshot_path(session_id: &str) -> PathBuf {
    crate::state::session_file("idle", session_id)
}

/// Reads the session snapshot at `path`, records the current counter and
/// returns how many seconds it has gone unchanged. `None` while the counter
/// is zero: with no steps to watch, idle time would only measure the payload
/// lacking token totals.
pub fn idle_seconds(path: &Path, tokens: u64, now: u64) -> Option<u64> {
    if tokens == 0 {
        return None;
    }
    let snapshot = crate::state::update_snapshot(path, |prev| assess(prev, tokens, now));
    Some(now.saturating_sub(snapshot.changed_at))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(tokens: u64, changed_at: u64) -> Snapshot {
        Snapshot { tokens, changed_at }
    }

    #[test]
    fn test_first_observation_starts_the_clock() {
        assert_eq!(assess(None, 90_000, 1_000), snap(90_000, 1_000));
    }

    #[test]
    fn test_unchanged_counter_keeps_the_clock_running() {
        let prev = snap(90_000, 1_000);
        assert_eq!(assess(Some(prev), 90_000, 1_400), prev);
    }

    #[test]
    fn test_new_step_restarts_the_clock() {
        let prev = snap(90_000, 1_000);
        assert_eq!(assess(Some(prev), 95_000, 1_400), snap(95_000, 1_400));
    }

    #[test]
    fn test_counter_going_down_restarts_the_clock() {
        let prev = snap(90_000, 1_000);
        assert_eq!(assess(Some(prev), 5_000, 1_400), snap(5_000, 1_400));
    }

    #[test]
    fn test_idle_seconds_grow_across_timer_refreshes() {
        let path = std::env::temp_dir().join(format!(
            "copilot-powerline-idle-{}-{}.json",
            std::process::id(),
            crate::github::current_timestamp()
        ));

        assert_eq!(idle_seconds(&path, 90_000, 1_000), Some(0));
        // Timer refreshes, nothing new.
        assert_eq!(idle_seconds(&path, 90_000, 1_030), Some(30));
        assert_eq!(idle_seconds(&path, 90_000, 1_310), Some(310));
        // A new step resets it.
        assert_eq!(idle_seconds(&path, 95_000, 1_320), Some(0));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_unknown_without_a_counter_to_watch() {
        let path = std::env::temp_dir().join(format!(
            "copilot-powerline-idle-none-{}-{}.json",
            std::process::id(),
            crate::github::current_timestamp()
        ));

        assert_eq!(idle_seconds(&path, 0, 1_000), None);
        assert_eq!(idle_seconds(&path, 0, 1_400), None);
        assert!(!path.exists());
    }

    #[test]
    fn test_clock_going_backwards_is_not_idle() {
        let path = std::env::temp_dir().join(format!(
            "copilot-powerline-idle-back-{}-{}.json",
            std::process::id(),
            crate::github::current_timestamp()
        ));

        assert_eq!(idle_seconds(&path, 90_000, 1_000), Some(0));
        assert_eq!(idle_seconds(&path, 90_000, 900), Some(0));

        let _ = std::fs::remove_file(&path);
    }
}
