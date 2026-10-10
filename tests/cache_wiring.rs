//! Runs the built binary against a throwaway `HOME` to check that the cache
//! segment keeps a large cache miss visible across refreshes. The unit tests
//! in `src/cache_trend.rs` cover the rules; this covers the snapshot wiring
//! in `main`.

mod common;

use common::{Sandbox, assert_shows};

/// A refresh payload with the given cumulative input and cache read totals.
fn payload(input_tokens: u64, cache_read_tokens: u64) -> String {
    format!(
        r#"{{"session_id":"wiring","context_window":{{"current_context_tokens":140000,"displayed_context_limit":200000,"current_context_used_percentage":70,"total_input_tokens":{input_tokens},"total_output_tokens":10000,"total_cache_read_tokens":{cache_read_tokens}}},"ai_used":{{"total_nano_aiu":0}}}}"#
    )
}

fn refresh(sandbox: &Sandbox, input_tokens: u64, cache_read_tokens: u64) -> String {
    sandbox.refresh(
        &payload(input_tokens, cache_read_tokens),
        &[
            "--style",
            "plain",
            "--icon-set",
            "plain",
            "--theme",
            "plain",
        ],
    )
}

/// Replays the #83 cold restart: a 2.0M-token session at 94%, a 133k step
/// with nothing read from the cache, then a well-cached call.
fn replay_cold_restart(sandbox: &Sandbox) -> (String, String) {
    refresh(sandbox, 2_000_000, 1_886_000);
    let after_miss = refresh(sandbox, 2_133_000, 1_886_000);
    let next_call = refresh(sandbox, 2_143_000, 1_895_900);
    (after_miss, next_call)
}

#[test]
fn test_cache_miss_stays_visible_after_the_next_call() {
    let sandbox = Sandbox::new();

    let (after_miss, next_call) = replay_cold_restart(&sandbox);

    assert_shows(&after_miss, "Cache: 88% ↓ · miss 133k");
    assert_shows(&next_call, "Cache: 88% ↑ · miss 133k");
}

#[test]
fn test_cache_miss_shown_without_the_trend_arrow() {
    let sandbox = Sandbox::new();
    sandbox.write_config("[cache]\nshow_trend = false\n");

    let (_, next_call) = replay_cold_restart(&sandbox);

    assert_shows(&next_call, "Cache: 88% · miss 133k");
}

#[test]
fn test_cache_miss_can_be_turned_off() {
    let sandbox = Sandbox::new();
    sandbox.write_config("[cache]\nshow_last_miss = false\n");

    let (_, next_call) = replay_cold_restart(&sandbox);

    assert_shows(&next_call, "Cache: 88% ↑");
    assert!(
        !next_call.contains("miss"),
        "unexpected miss in {next_call:?}"
    );
}
