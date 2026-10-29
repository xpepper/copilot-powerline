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

pub struct FusionRouting {
    id: Option<String>,
    pub pattern: String,
    pub selected_model: Option<String>,
    pub observed: Vec<(String, String)>,
    pub handoff_model: Option<String>,
    pub final_model: Option<String>,
    pub outcome: Option<String>,
}

impl FusionRouting {
    fn new(id: Option<&str>, pattern: Option<&str>) -> Option<Self> {
        let id = id.filter(|id| !id.is_empty())?;
        let pattern = pattern?;
        matches!(pattern, "single" | "cascade" | "critique").then(|| Self {
            id: Some(id.to_string()),
            pattern: pattern.to_string(),
            selected_model: None,
            observed: Vec::new(),
            handoff_model: None,
            final_model: None,
            outcome: None,
        })
    }

    fn auto(model: Option<String>) -> Option<Self> {
        let model = model.filter(|model| !model.is_empty())?;
        Some(Self {
            id: None,
            pattern: "auto".to_string(),
            selected_model: Some(model),
            observed: Vec::new(),
            handoff_model: None,
            final_model: None,
            outcome: None,
        })
    }
}

/// The latest Fusion routing decision in the bounded tail of the Event Log.
pub fn latest_fusion(dir: &Path) -> Option<FusionRouting> {
    let tail = read_tail(&dir.join("events.jsonl"), TAIL_WINDOW)?;
    let mut routing: Option<FusionRouting> = None;
    // Fusion IDs that were replaced, reset or unsupported: late events for
    // them must not bring their routing back.
    let mut retired: Vec<String> = Vec::new();
    for line in tail.split(|&b| b == b'\n') {
        let Ok(line) = std::str::from_utf8(line) else {
            continue;
        };
        if !line.contains("\"session.fusion_")
            && !line.contains("\"assistant.fusion_phase_completed\"")
            && !line.contains("\"session.usage_record\"")
            && !line.contains("\"session.model_change\"")
            && !line.contains("\"session.auto_mode_resolved\"")
        {
            continue;
        }
        let Ok(event) = serde_json::from_str::<FusionEvent>(line) else {
            continue;
        };
        if event.agent_id.is_some() {
            continue;
        }
        let data = event.data;
        if event.kind == "session.auto_mode_resolved" {
            retire(&mut routing, &mut retired);
            routing = FusionRouting::auto(data.chosen_model);
        } else if event.kind == "session.model_change"
            || (event.kind == "session.usage_record"
                && data
                    .usage
                    .as_ref()
                    .is_none_or(|usage| usage.fusion.is_none()))
        {
            retire(&mut routing, &mut retired);
            if let Some(usage) = data.usage
                && usage.is_auto == Some(true)
            {
                routing = FusionRouting::auto(usage.model.clone());
                if let Some(current) = routing.as_mut()
                    && let Some(model) = usage.model
                {
                    current.observed.push(("model".to_string(), model));
                }
            }
        } else if event.kind == "session.fusion_resolved" {
            retire(&mut routing, &mut retired);
            routing = FusionRouting::new(data.fusion_id.as_deref(), data.pattern.as_deref());
            if routing.is_none()
                && let Some(id) = data.fusion_id.clone()
            {
                retired.push(id);
            }
        } else {
            // A decision line can be missing (torn write, unknown shape), so
            // a completion or usage line for an unseen Fusion starts its own.
            let started = match event.kind.as_str() {
                "session.fusion_completed" => {
                    FusionRouting::new(data.fusion_id.as_deref(), data.pattern.as_deref())
                }
                "session.usage_record" => data
                    .usage
                    .as_ref()
                    .and_then(|usage| usage.fusion.as_ref())
                    .and_then(|fusion| {
                        FusionRouting::new(fusion.fusion_id.as_deref(), fusion.pattern.as_deref())
                    }),
                _ => None,
            };
            if let Some(started) = started
                && routing
                    .as_ref()
                    .is_none_or(|current| current.id != started.id)
                && !started.id.as_ref().is_some_and(|id| retired.contains(id))
            {
                retire(&mut routing, &mut retired);
                routing = Some(started);
            }
            let Some(current) = routing.as_mut() else {
                continue;
            };
            if data.fusion_id.is_some() && data.fusion_id == current.id {
                match event.kind.as_str() {
                    "session.fusion_handoff" => current.handoff_model = data.target_model,
                    "session.fusion_completed" => {
                        current.final_model = data.final_source_model;
                        current.outcome = data.outcome;
                    }
                    _ => {}
                }
            }
            let observation = match event.kind.as_str() {
                "assistant.fusion_phase_completed" => {
                    Some((data.fusion_id, data.phase_kind, data.model))
                }
                "session.usage_record" => data.usage.and_then(|usage| {
                    let fusion = usage.fusion?;
                    Some((fusion.fusion_id, fusion.phase_kind, usage.model))
                }),
                _ => None,
            };
            if let Some((Some(id), Some(phase), Some(model))) = observation
                && Some(&id) == current.id.as_ref()
                && !current.observed.contains(&(phase.clone(), model.clone()))
            {
                current.observed.push((phase, model));
            }
        }
    }
    routing
}

fn retire(routing: &mut Option<FusionRouting>, retired: &mut Vec<String>) {
    if let Some(old) = routing.take()
        && let Some(id) = old.id
    {
        retired.push(id);
    }
}

#[derive(Deserialize)]
struct FusionEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "agentId")]
    agent_id: Option<String>,
    data: FusionData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FusionData {
    chosen_model: Option<String>,
    fusion_id: Option<String>,
    pattern: Option<String>,
    phase_kind: Option<String>,
    model: Option<String>,
    usage: Option<FusionUsage>,
    target_model: Option<String>,
    final_source_model: Option<String>,
    outcome: Option<String>,
}

#[derive(Deserialize)]
struct FusionUsage {
    model: Option<String>,
    #[serde(rename = "isAuto")]
    is_auto: Option<bool>,
    fusion: Option<FusionPhase>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FusionPhase {
    fusion_id: Option<String>,
    phase_kind: Option<String>,
    pattern: Option<String>,
}

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

    #[test]
    fn test_fusion_uses_only_complete_recent_lines_and_skips_partial_writes() {
        let decision =
            r#"{"type":"session.fusion_resolved","data":{"fusionId":"f","pattern":"single"}}"#;
        let outside = session_dir(&[decision.to_string(), filler(1_200_000)]);
        assert!(latest_fusion(outside.path()).is_none());

        let inside = session_dir(&[
            filler(1_200_000),
            decision.to_string(),
            r#"{"type":"session.fusion_resolved","data":{"fusionId":"partial"#.to_string(),
        ]);
        assert_eq!(latest_fusion(inside.path()).unwrap().pattern, "single");
        assert!(latest_fusion(&inside.path().join("missing")).is_none());
    }

    fn resolved(id: &str, pattern: &str) -> String {
        format!(
            r#"{{"type":"session.fusion_resolved","data":{{"fusionId":"{id}","pattern":"{pattern}"}}}}"#
        )
    }

    fn completed(id: &str, pattern: &str) -> String {
        format!(
            r#"{{"type":"session.fusion_completed","data":{{"fusionId":"{id}","pattern":"{pattern}","finalSourceModel":"m-{id}","outcome":"ok"}}}}"#
        )
    }

    fn fusion_usage(id: &str, pattern: &str) -> String {
        format!(
            r#"{{"type":"session.usage_record","data":{{"usage":{{"model":"m-{id}","fusion":{{"fusionId":"{id}","pattern":"{pattern}","phaseKind":"primary"}}}}}}}}"#
        )
    }

    const MODEL_CHANGE: &str = r#"{"type":"session.model_change","data":{}}"#;

    #[test]
    fn test_late_events_do_not_revive_a_route_hidden_by_an_unsupported_decision() {
        let dir = session_dir(&[
            resolved("old", "single"),
            resolved("new", "unknown"),
            completed("old", "single"),
            fusion_usage("old", "single"),
        ]);
        assert!(latest_fusion(dir.path()).is_none());
    }

    #[test]
    fn test_late_events_do_not_revive_a_route_cleared_by_a_reset() {
        let dir = session_dir(&[
            resolved("old", "single"),
            MODEL_CHANGE.to_string(),
            completed("old", "single"),
        ]);
        assert!(latest_fusion(dir.path()).is_none());
    }

    #[test]
    fn test_late_events_do_not_revive_a_replaced_route() {
        let dir = session_dir(&[
            resolved("old", "single"),
            resolved("new", "cascade"),
            completed("old", "single"),
        ]);
        let routing = latest_fusion(dir.path()).unwrap();
        assert_eq!(routing.pattern, "cascade");
        assert_eq!(routing.final_model, None);
    }

    #[test]
    fn test_a_new_fusion_id_replaces_the_route_when_its_decision_line_is_missing() {
        let usage = session_dir(&[resolved("old", "single"), fusion_usage("new", "cascade")]);
        let routing = latest_fusion(usage.path()).unwrap();
        assert_eq!(routing.pattern, "cascade");
        assert_eq!(
            routing.observed,
            vec![("primary".to_string(), "m-new".to_string())]
        );

        let done = session_dir(&[resolved("old", "single"), completed("new", "critique")]);
        let routing = latest_fusion(done.path()).unwrap();
        assert_eq!(routing.pattern, "critique");
        assert_eq!(routing.final_model.as_deref(), Some("m-new"));
    }
}
