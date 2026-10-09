//! Runs the built binary against a throwaway `HOME` to check that a status
//! line refresh prunes stale state files. The unit tests in `src/state.rs`
//! cover the pruning rules; this covers the call in `main`.
#![cfg(unix)]

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);
const PAYLOAD: &str = r#"{"session_id":"wiring","context_window":{"current_context_tokens":1000,"displayed_context_limit":200000,"current_context_used_percentage":1},"ai_used":{"total_nano_aiu":0}}"#;

fn state_dir(home: &Path) -> PathBuf {
    let cache = if cfg!(target_os = "macos") {
        home.join("Library/Caches")
    } else {
        home.join(".cache")
    };
    cache.join("copilot-powerline")
}

fn touch(dir: &Path, name: &str, age: Duration) -> PathBuf {
    let path = dir.join(name);
    File::create(&path)
        .unwrap()
        .set_modified(SystemTime::now() - age)
        .unwrap();
    path
}

fn refresh(home: &Path) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_copilot-powerline"))
        .env("HOME", home)
        .env_remove("XDG_CACHE_HOME")
        .current_dir(home)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(PAYLOAD.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());
}

#[test]
fn test_refresh_prunes_stale_state_files_and_keeps_the_rest() {
    let home = tempfile::tempdir().unwrap();
    let state = state_dir(home.path());
    fs::create_dir_all(&state).unwrap();
    let stale = touch(&state, "spend_0123456789abcdef.json", 40 * DAY);
    let recent = touch(&state, "spend_fedcba9876543210.json", DAY);
    let mode = touch(&state, "mode", 40 * DAY);

    refresh(home.path());

    assert!(!stale.exists());
    assert!(recent.exists() && mode.exists());
    assert!(state.join("last_prune").exists());
}

#[test]
fn test_refresh_scans_at_most_once_a_day() {
    let home = tempfile::tempdir().unwrap();
    let state = state_dir(home.path());
    fs::create_dir_all(&state).unwrap();

    refresh(home.path());
    assert!(state.join("last_prune").exists());
    let stale = touch(&state, "cache_0123456789abcdef.json", 40 * DAY);
    refresh(home.path());

    assert!(stale.exists());
}
