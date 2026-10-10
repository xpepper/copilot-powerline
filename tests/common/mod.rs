//! Fixture for the binary-level wiring tests: a throwaway `HOME` holding an
//! optional config and Copilot database, and a way to run one status line
//! refresh against it. Each test crate uses a different subset of it.
#![allow(dead_code)]

use rusqlite::Connection;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// SQLite date arguments for `insert_usage`. A fixed past date is always in
/// an earlier month, unlike `-1 month`, which can land in the current one.
pub const NOW: &str = "now";
pub const LONG_AGO: &str = "2000-01-01";

/// A refresh payload for session `session_id`, with the given session spend.
/// The token totals are non-zero so lookups that watch them (the idle clock)
/// take effect.
pub fn payload(session_id: &str, session_nano_aiu: u64) -> String {
    format!(
        r#"{{"session_id":"{session_id}","context_window":{{"current_context_tokens":1000,"displayed_context_limit":200000,"current_context_used_percentage":1,"total_input_tokens":1000,"total_output_tokens":100}},"ai_used":{{"total_nano_aiu":{session_nano_aiu}}}}}"#
    )
}

pub struct Sandbox {
    home: TempDir,
}

impl Sandbox {
    pub fn new() -> Self {
        Self {
            home: tempfile::tempdir().unwrap(),
        }
    }

    pub fn home(&self) -> &Path {
        self.home.path()
    }

    pub fn copilot_dir(&self) -> PathBuf {
        self.home().join(".copilot")
    }

    pub fn state_dir(&self) -> PathBuf {
        let cache = if cfg!(target_os = "macos") {
            self.home().join("Library/Caches")
        } else {
            self.home().join(".cache")
        };
        cache.join("copilot-powerline")
    }

    pub fn write_config(&self, toml: &str) {
        fs::create_dir_all(self.copilot_dir()).unwrap();
        fs::write(self.copilot_dir().join("powerline.toml"), toml).unwrap();
    }

    /// Creates an empty Copilot database at `path` with the one table the
    /// month-to-date query reads.
    pub fn create_db(&self, path: &Path) -> Connection {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let conn = Connection::open(path).unwrap();
        conn.execute(
            "CREATE TABLE assistant_usage_events (
                session_id TEXT,
                total_nano_aiu INTEGER,
                created_at TEXT
            )",
            [],
        )
        .unwrap();
        conn
    }

    /// Creates the database at the default location, `~/.copilot/session-store.db`.
    pub fn create_default_db(&self) -> Connection {
        self.create_db(&self.copilot_dir().join("session-store.db"))
    }

    /// Runs one refresh and returns its stdout without the trailing newline.
    ///
    /// `HOME` and the working directory are set on the child only, so tests
    /// can run in parallel without sharing process state.
    pub fn refresh(&self, payload: &str, args: &[&str]) -> String {
        let mut child = Command::new(env!("CARGO_BIN_EXE_copilot-powerline"))
            .args(args)
            .env("HOME", self.home())
            .env_remove("XDG_CACHE_HOME")
            .current_dir(self.home())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(payload.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "refresh failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .trim_end()
            .to_string()
    }

    /// State files whose name starts with `prefix`, e.g. `month_`.
    pub fn state_files(&self, prefix: &str) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(self.state_dir()) else {
            return Vec::new();
        };
        entries
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(prefix))
            })
            .collect()
    }
}

pub fn insert_usage(conn: &Connection, session_id: &str, nano_aiu: i64, created_at: &str) {
    conn.execute(
        "INSERT INTO assistant_usage_events (session_id, total_nano_aiu, created_at)
         VALUES (?1, ?2, datetime(?3))",
        (session_id, nano_aiu, created_at),
    )
    .unwrap();
}
