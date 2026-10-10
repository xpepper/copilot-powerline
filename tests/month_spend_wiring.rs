//! Runs the built binary to check the wiring around the month-to-date query,
//! which scans the whole usage table: `main` must skip it when `month_cost`
//! is not shown and reuse the cached total for 60 s when it is. The unit
//! tests cover `month_cost::is_visible` and the cache rules; if `main`
//! stopped calling them, only the status line would get slower.
//!
//! The query leaves no trace of its own (the database is opened read-only),
//! so the tests watch the per-session `month_*.json` snapshot, which `main`
//! only writes on the path that reaches the query.
#![cfg(unix)]

mod common;

use common::{NOW, Sandbox, insert_usage};

/// $250.00 and $100.00 in nano AIU.
const FIRST_OTHER_SPEND: i64 = 25_000_000_000_000;
const SECOND_OTHER_SPEND: i64 = 10_000_000_000_000;
const SNAPSHOT_PREFIX: &str = "month_";

fn config(segments: &str, month_cost: &str) -> String {
    format!("theme = \"plain\"\nsegments = [{segments}]\n\n[month_cost]\n{month_cost}\n")
}

/// Refreshes with data in the database and checks the query was never reached.
fn assert_query_skipped(config: &str) {
    let sandbox = Sandbox::new();
    sandbox.write_config(config);
    insert_usage(
        &sandbox.create_default_db(),
        "other",
        FIRST_OTHER_SPEND,
        NOW,
    );

    let line = sandbox.refresh_plain();

    assert!(!line.contains("Month"), "unexpected month segment: {line}");
    // `cache_expiry` is another gated lookup that writes its own snapshot
    // when shown. Seeing it proves the refresh got as far as the lookups and
    // that this is the state dir the binary uses, so the missing month
    // snapshot means the query was skipped.
    assert_eq!(sandbox.state_files("idle_").len(), 1);
    assert_eq!(
        sandbox.state_files(SNAPSHOT_PREFIX),
        Vec::<std::path::PathBuf>::new()
    );
}

#[test]
fn test_query_is_skipped_when_month_cost_is_not_in_segments() {
    assert_query_skipped(&config(r#""session_cost", "cache_expiry""#, ""));
}

#[test]
fn test_query_is_skipped_when_month_cost_is_disabled() {
    assert_query_skipped(&config(
        r#""month_cost", "cache_expiry""#,
        "enabled = false",
    ));
}

#[test]
fn test_cached_total_is_reused_until_the_snapshot_goes() {
    let sandbox = Sandbox::new();
    sandbox.write_config(&config(r#""month_cost""#, ""));
    let db = sandbox.create_default_db();
    insert_usage(&db, "other", FIRST_OTHER_SPEND, NOW);

    assert_eq!(sandbox.refresh_plain(), "Month: $300.00");
    let snapshots = sandbox.state_files(SNAPSHOT_PREFIX);
    assert_eq!(snapshots.len(), 1);

    // Another session spends more, but the total was queried moments ago.
    insert_usage(&db, "other", SECOND_OTHER_SPEND, NOW);
    assert_eq!(sandbox.refresh_plain(), "Month: $300.00");

    // Without the cache the same database does show the new spend, so the
    // line above is the cache at work and not a row the query cannot see.
    std::fs::remove_file(&snapshots[0]).unwrap();
    assert_eq!(sandbox.refresh_plain(), "Month: $400.00");
}
