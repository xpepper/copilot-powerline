//! Runs the built binary against a throwaway `HOME` to check that a status
//! line refresh prunes stale state files. The unit tests in `src/state.rs`
//! cover the pruning rules; this covers the call in `main`.
#![cfg(unix)]

mod common;

use common::{Sandbox, payload};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

fn touch(dir: &Path, name: &str, age: Duration) -> PathBuf {
    let path = dir.join(name);
    File::create(&path)
        .unwrap()
        .set_modified(SystemTime::now() - age)
        .unwrap();
    path
}

fn refresh(sandbox: &Sandbox) {
    sandbox.refresh(&payload("wiring", 0), &[]);
}

#[test]
fn test_refresh_prunes_stale_state_files_and_keeps_the_rest() {
    let sandbox = Sandbox::new();
    let state = sandbox.state_dir();
    fs::create_dir_all(&state).unwrap();
    let stale = touch(&state, "spend_0123456789abcdef.json", 40 * DAY);
    let recent = touch(&state, "spend_fedcba9876543210.json", DAY);
    let mode = touch(&state, "mode", 40 * DAY);

    refresh(&sandbox);

    assert!(!stale.exists());
    assert!(recent.exists() && mode.exists());
    assert!(state.join("last_prune").exists());
}

#[test]
fn test_refresh_scans_at_most_once_a_day() {
    let sandbox = Sandbox::new();
    let state = sandbox.state_dir();
    fs::create_dir_all(&state).unwrap();

    refresh(&sandbox);
    assert!(state.join("last_prune").exists());
    let stale = touch(&state, "cache_0123456789abcdef.json", 40 * DAY);
    refresh(&sandbox);

    assert!(stale.exists());
}
