//! Reads Copilot CLI's event log (`events.jsonl`) for what the status line
//! payload does not carry.
//!
//! The file is internal and undocumented, so every read fails quietly: a
//! missing or unreadable file, or a line in an unexpected shape, gives `None`
//! and the segment that asked simply hides.

use serde::Deserialize;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Every Usage Record line starts with this, so other lines are skipped
/// without parsing them.
const USAGE_RECORD_PREFIX: &[u8] = br#"{"type":"session.usage_record""#;

/// The size of the tool definitions on a main-agent Step, from its Usage
/// Record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsageRecord {
    pub tool_tokens: u64,
}

/// How much of the end of the Event Log is read. The latest main-agent Usage
/// Record sat within 317 KB of the end in every one of 138 recent sessions,
/// so a window of 1 MB finds it without reading the whole log.
const TAIL_WINDOW: u64 = 1024 * 1024;

/// The latest main-agent Usage Record in the Event Log in `dir`.
pub fn latest_main_usage(dir: &Path) -> Option<UsageRecord> {
    latest_main_usage_in(dir, TAIL_WINDOW)
}

fn latest_main_usage_in(dir: &Path, window: u64) -> Option<UsageRecord> {
    let tail = read_tail(&dir.join("events.jsonl"), window)?;
    tail.rsplit(|&b| b == b'\n')
        .filter(|line| line.starts_with(USAGE_RECORD_PREFIX))
        .find_map(main_agent_usage)
}

/// The whole lines in the last `window` bytes of the file at `path`.
fn read_tail(path: &Path, window: u64) -> Option<Vec<u8>> {
    let mut file = fs::File::open(path).ok()?;
    let start = file.metadata().ok()?.len().saturating_sub(window);
    // Begin one byte early: if that byte is a newline, the first line is whole.
    let from = start.saturating_sub(1);
    file.seek(SeekFrom::Start(from)).ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    if from > 0 {
        // The window starts inside a line, so drop its cut-off beginning.
        let line_end = bytes.iter().position(|&b| b == b'\n')?;
        bytes.drain(..=line_end);
    }
    Some(bytes)
}

/// The record on `line`, unless it is a sub-agent's (those carry an
/// `agentId`, and run with a tool set of their own) or has no tool count.
fn main_agent_usage(line: &[u8]) -> Option<UsageRecord> {
    let event: UsageEvent = serde_json::from_slice(line).ok()?;
    if event.agent_id.is_some() {
        return None;
    }
    Some(UsageRecord {
        tool_tokens: event.data.usage.tool_token_count?,
    })
}

#[derive(Deserialize)]
struct UsageEvent {
    data: UsageData,
    #[serde(rename = "agentId")]
    agent_id: Option<String>,
}

#[derive(Deserialize)]
struct UsageData {
    usage: Usage,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(rename = "toolTokenCount")]
    tool_token_count: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A main-agent Usage Record line, shaped like Copilot CLI 1.0.98 writes it.
    fn main_record(tool_tokens: u64) -> String {
        format!(
            r#"{{"type":"session.usage_record","data":{{"usage":{{"model":"claude-sonnet-5.5","initiator":"agent","toolTokenCount":{tool_tokens},"availableToolCount":45}}}},"id":"a1","parentId":"a0","timestamp":"2026-10-18T10:00:00.000Z"}}"#
        )
    }

    /// A sub-agent Usage Record line: same shape, plus a top-level `agentId`.
    fn sub_agent_record(tool_tokens: u64) -> String {
        format!(
            r#"{{"type":"session.usage_record","data":{{"usage":{{"model":"claude-sonnet-5.5","initiator":"sub-agent","toolTokenCount":{tool_tokens},"availableToolCount":43}}}},"id":"b1","parentId":"b0","timestamp":"2026-10-18T10:00:05.000Z","agentId":"9b2dbb13-730f-4cb3-b88c-740acc684772"}}"#
        )
    }

    /// A session directory whose `events.jsonl` holds `lines`.
    fn session_dir(lines: &[String]) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("events.jsonl"), lines.join("\n") + "\n").unwrap();
        dir
    }

    #[test]
    fn test_reads_the_tool_tokens_of_a_main_agent_record() {
        let dir = session_dir(&[main_record(68_000)]);
        assert_eq!(
            latest_main_usage(dir.path()),
            Some(UsageRecord {
                tool_tokens: 68_000
            })
        );
    }

    #[test]
    fn test_ignores_a_later_sub_agent_record() {
        let dir = session_dir(&[main_record(68_000), sub_agent_record(12_000)]);
        assert_eq!(
            latest_main_usage(dir.path()),
            Some(UsageRecord {
                tool_tokens: 68_000
            })
        );
    }

    #[test]
    fn test_the_last_main_agent_record_wins() {
        let dir = session_dir(&[main_record(9_000), main_record(68_000)]);
        assert_eq!(
            latest_main_usage(dir.path()),
            Some(UsageRecord {
                tool_tokens: 68_000
            })
        );
    }

    #[test]
    fn test_skips_a_record_without_a_tool_count() {
        let no_count = main_record(1).replace(r#""toolTokenCount":1,"#, "");
        let dir = session_dir(&[main_record(9_000), no_count]);
        assert_eq!(
            latest_main_usage(dir.path()),
            Some(UsageRecord { tool_tokens: 9_000 })
        );
    }

    #[test]
    fn test_skips_a_half_written_last_line() {
        let half_written = &main_record(68_000)[..60];
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("events.jsonl"),
            format!("{}\n{half_written}", main_record(9_000)),
        )
        .unwrap();
        assert_eq!(
            latest_main_usage(dir.path()),
            Some(UsageRecord { tool_tokens: 9_000 })
        );
    }

    #[test]
    fn test_no_usage_record_gives_none() {
        let message = r#"{"type":"assistant.message","data":{"content":"hi"},"id":"c1","timestamp":"2026-10-18T10:00:00.000Z"}"#;
        let dir = session_dir(&[message.to_string()]);
        assert_eq!(latest_main_usage(dir.path()), None);
    }

    #[test]
    fn test_missing_event_log_gives_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(latest_main_usage(dir.path()), None);
        assert_eq!(latest_main_usage(&dir.path().join("gone")), None);
    }

    /// A line of another event type, `len` bytes of padding long.
    fn filler(len: usize) -> String {
        format!(
            r#"{{"type":"hook.start","data":{{"pad":"{}"}}}}"#,
            "x".repeat(len)
        )
    }

    #[test]
    fn test_ignores_a_record_beyond_the_tail_window() {
        let dir = session_dir(&[main_record(68_000), filler(2_000)]);
        assert_eq!(latest_main_usage_in(dir.path(), 1_000), None);
        assert_eq!(
            latest_main_usage_in(dir.path(), 10_000),
            Some(UsageRecord {
                tool_tokens: 68_000
            })
        );
    }

    #[test]
    fn test_a_record_cut_by_the_window_start_is_ignored() {
        let record = main_record(68_000);
        let dir = session_dir(&[filler(500), record.clone()]);
        let whole_line = record.len() as u64 + 1;
        assert_eq!(
            latest_main_usage_in(dir.path(), whole_line),
            Some(UsageRecord {
                tool_tokens: 68_000
            })
        );
        assert_eq!(latest_main_usage_in(dir.path(), whole_line - 1), None);
    }

    #[test]
    fn test_reads_about_the_last_megabyte_by_default() {
        let near = session_dir(&[main_record(68_000), filler(500_000)]);
        assert_eq!(
            latest_main_usage(near.path()),
            Some(UsageRecord {
                tool_tokens: 68_000
            })
        );
        let far = session_dir(&[main_record(68_000), filler(1_200_000)]);
        assert_eq!(latest_main_usage(far.path()), None);
    }
}
