//! Opt-in log of what each status line refresh received, to tell a stale
//! payload from a missed repaint (#81). Off unless `COPILOT_POWERLINE_LOG`
//! names a file. One line per refresh: time, hashed session id, the session
//! counters and the computed idle time. No paths, no prompt content.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const ENV_VAR: &str = "COPILOT_POWERLINE_LOG";

pub struct Entry<'a> {
    pub now: u64,
    pub session_id: Option<&'a str>,
    pub total_tokens: u64,
    pub total_nano_aiu: u64,
    pub idle_seconds: Option<u64>,
}

/// The log file named by `COPILOT_POWERLINE_LOG`, if set and not empty.
pub fn path_from_env() -> Option<PathBuf> {
    std::env::var_os(ENV_VAR)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// `YYYY-MM-DDTHH:MM:SSZ` for a Unix time, so lines compare directly with
/// the timestamps in Copilot's own session data.
fn format_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

fn format_line(entry: &Entry) -> String {
    let session = entry
        .session_id
        .map_or_else(|| "-".to_string(), crate::state::session_hash);
    let idle = entry
        .idle_seconds
        .map_or_else(|| "-".to_string(), |secs| secs.to_string());
    format!(
        "{} session={session} total_tokens={} nano_aiu={} idle_seconds={idle}",
        format_utc(entry.now),
        entry.total_tokens,
        entry.total_nano_aiu,
    )
}

/// Appends one line for `entry` to `path`. Failures are ignored: diagnostics
/// must never break or slow the status line.
pub fn append(path: &Path, entry: &Entry) {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    if let Ok(mut file) = options.open(path) {
        // A single write keeps concurrent refreshes from interleaving lines.
        let _ = file.write_all(format!("{}\n", format_line(entry)).as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_utc_matches_known_dates() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_utc(1_791_618_639), "2026-10-10T07:50:39Z");
    }

    #[test]
    fn test_line_holds_counters_and_a_hashed_session_id() {
        let line = format_line(&Entry {
            now: 0,
            session_id: Some("secret-session-id"),
            total_tokens: 1_971_130,
            total_nano_aiu: 623_000_000_000,
            idle_seconds: Some(3_240),
        });

        assert_eq!(
            line,
            format!(
                "1970-01-01T00:00:00Z session={} total_tokens=1971130 \
                 nano_aiu=623000000000 idle_seconds=3240",
                crate::state::session_hash("secret-session-id")
            )
        );
        assert!(!line.contains("secret-session-id"));
    }

    #[test]
    fn test_line_marks_missing_session_and_idle_with_a_dash() {
        let line = format_line(&Entry {
            now: 0,
            session_id: None,
            total_tokens: 0,
            total_nano_aiu: 0,
            idle_seconds: None,
        });

        assert!(line.contains("session=- "));
        assert!(line.ends_with("idle_seconds=-"));
    }

    #[test]
    fn test_append_adds_one_line_per_call() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("payloads.log");
        let entry = Entry {
            now: 10,
            session_id: Some("s"),
            total_tokens: 1,
            total_nano_aiu: 2,
            idle_seconds: Some(3),
        };

        append(&path, &entry);
        append(&path, &entry);

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), 2);
    }

    #[test]
    fn test_append_ignores_an_unwritable_path() {
        let dir = tempfile::tempdir().unwrap();
        let entry = Entry {
            now: 0,
            session_id: None,
            total_tokens: 0,
            total_nano_aiu: 0,
            idle_seconds: None,
        };

        append(&dir.path().join("missing").join("payloads.log"), &entry);
    }
}
