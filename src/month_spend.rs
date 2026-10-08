//! Month-to-date spend of the user's other sessions, cached per session for a
//! short TTL. The query behind it scans every usage event (the Copilot
//! database has no index on `created_at`, and it must stay read-only), which
//! dominated refresh time on large histories. Other sessions' spend changes
//! rarely compared to the current one, which comes live from the payload, so
//! a value up to a minute old is good enough.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// How long a queried total is reused before the database is read again.
pub const TTL_SECS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub nano_aiu: u64,
    /// Unix seconds when `nano_aiu` was queried.
    pub fetched_at: u64,
}

/// Keeps `prev` while it is younger than the TTL, otherwise runs `query`.
pub fn assess(prev: Option<Snapshot>, now: u64, query: impl FnOnce() -> u64) -> Snapshot {
    match prev {
        Some(prev) if now >= prev.fetched_at && now - prev.fetched_at < TTL_SECS => prev,
        _ => Snapshot {
            nano_aiu: query(),
            fetched_at: now,
        },
    }
}

pub fn snapshot_path(session_id: &str) -> PathBuf {
    crate::state::session_file("month", session_id)
}

/// Returns the other sessions' month-to-date spend from the snapshot at
/// `path`, running `query` and persisting its result when the snapshot is
/// missing or stale.
pub fn other_sessions_nano(path: &Path, now: u64, query: impl FnOnce() -> u64) -> u64 {
    crate::state::update_snapshot(path, |prev| assess(prev, now, query)).nano_aiu
}

#[cfg(test)]
mod tests {
    use super::*;

    fn never_queried() -> u64 {
        panic!("the database must not be queried while the snapshot is fresh")
    }

    const FETCHED_AT: u64 = 1_000_000;
    const PREV: Snapshot = Snapshot {
        nano_aiu: 42,
        fetched_at: FETCHED_AT,
    };

    #[test]
    fn test_queries_when_no_snapshot() {
        let next = assess(None, FETCHED_AT, || 7);
        assert_eq!(
            next,
            Snapshot {
                nano_aiu: 7,
                fetched_at: FETCHED_AT
            }
        );
    }

    #[test]
    fn test_reuses_snapshot_within_ttl() {
        let now = FETCHED_AT + TTL_SECS - 1;
        assert_eq!(assess(Some(PREV), now, never_queried), PREV);
    }

    #[test]
    fn test_requeries_once_ttl_has_passed() {
        let now = FETCHED_AT + TTL_SECS;
        let next = assess(Some(PREV), now, || 99);
        assert_eq!(
            next,
            Snapshot {
                nano_aiu: 99,
                fetched_at: now
            }
        );
    }

    #[test]
    fn test_requeries_when_snapshot_is_from_the_future() {
        // The clock went backwards: the age is unknown, so don't trust it.
        let now = FETCHED_AT - 1;
        assert_eq!(assess(Some(PREV), now, || 99).nano_aiu, 99);
    }

    #[test]
    fn test_other_sessions_nano_persists_between_calls() {
        let path = std::env::temp_dir().join(format!(
            "copilot-powerline-month-test-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        assert_eq!(other_sessions_nano(&path, FETCHED_AT, || 5), 5);
        assert_eq!(other_sessions_nano(&path, FETCHED_AT + 1, never_queried), 5);

        let _ = std::fs::remove_file(&path);
    }
}
