//! Runs the built binary against a throwaway `HOME` to check that the
//! `tool_overhead` segment finds the session's event log through the
//! payload's `transcript_path`. The unit tests in `src/events.rs` and
//! `src/segments/tool_overhead.rs` cover the rules; this covers what `main`
//! wires up.

mod common;

use common::{Sandbox, assert_shows};
use std::fs;
use std::path::PathBuf;

const PLAIN: [&str; 6] = [
    "--style",
    "plain",
    "--icon-set",
    "plain",
    "--theme",
    "plain",
];

/// A main-agent Usage Record line with the given tool definition size.
fn usage_record(tool_tokens: u64) -> String {
    format!(
        r#"{{"type":"session.usage_record","data":{{"usage":{{"model":"claude-sonnet-5.5","initiator":"agent","toolTokenCount":{tool_tokens},"availableToolCount":45}}}},"id":"a1","parentId":"a0","timestamp":"2026-10-18T10:00:00.000Z"}}"#
    )
}

/// Creates the session directory under `~/.copilot/session-state` and
/// returns it. With `tool_tokens`, its event log holds one Usage Record.
fn session_dir(sandbox: &Sandbox, tool_tokens: Option<u64>) -> PathBuf {
    let dir = sandbox.copilot_dir().join("session-state").join("wiring");
    fs::create_dir_all(&dir).unwrap();
    if let Some(tokens) = tool_tokens {
        fs::write(dir.join("events.jsonl"), usage_record(tokens) + "\n").unwrap();
    }
    dir
}

/// A refresh payload, with the session directory when there is one.
fn payload(transcript_path: Option<&PathBuf>) -> String {
    serde_json::json!({
        "session_id": "wiring",
        "transcript_path": transcript_path,
        "context_window": {
            "current_context_tokens": 140000,
            "displayed_context_limit": 200000,
            "current_context_used_percentage": 70
        },
        "ai_used": { "total_nano_aiu": 0 }
    })
    .to_string()
}

const LISTED: &str = "segments = [\"tokens\", \"tool_overhead\"]\n";

#[test]
fn test_shows_the_tool_overhead_from_the_event_log() {
    let sandbox = Sandbox::new();
    sandbox.write_config(LISTED);
    let dir = session_dir(&sandbox, Some(68_000));

    let line = sandbox.refresh(&payload(Some(&dir)), &PLAIN);

    assert_shows(&line, "Tools: 68k (34%)");
}

#[test]
fn test_is_not_shown_unless_listed() {
    let sandbox = Sandbox::new();
    let dir = session_dir(&sandbox, Some(68_000));

    let line = sandbox.refresh(&payload(Some(&dir)), &PLAIN);

    assert_shows(&line, "Tokens:");
    assert!(!line.contains("Tools:"), "unexpected Tools in {line:?}");
}

#[test]
fn test_min_tokens_keeps_a_low_count_quiet() {
    let sandbox = Sandbox::new();
    sandbox.write_config(&format!("{LISTED}[tool_overhead]\nmin_tokens = 30000\n"));
    let dir = session_dir(&sandbox, Some(9_000));

    let line = sandbox.refresh(&payload(Some(&dir)), &PLAIN);

    assert!(!line.contains("Tools:"), "unexpected Tools in {line:?}");
}

#[test]
fn test_hides_quietly_without_a_transcript_path() {
    let sandbox = Sandbox::new();
    sandbox.write_config(LISTED);

    let line = sandbox.refresh(&payload(None), &PLAIN);

    assert_shows(&line, "Tokens:");
    assert!(!line.contains("Tools:"), "unexpected Tools in {line:?}");
}

#[test]
fn test_hides_quietly_without_an_event_log() {
    let sandbox = Sandbox::new();
    sandbox.write_config(LISTED);
    let dir = session_dir(&sandbox, None);

    let line = sandbox.refresh(&payload(Some(&dir)), &PLAIN);

    assert_shows(&line, "Tokens:");
    assert!(!line.contains("Tools:"), "unexpected Tools in {line:?}");
}
