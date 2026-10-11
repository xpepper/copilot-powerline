//! Runs the built binary against a throwaway `HOME` to check the
//! `cycle_cost` wiring: the `--fetch-cycle-usage` worker must write what
//! `gh api` returns to the cache, and a refresh must read that cache from
//! the state dir. The unit tests in `src/cycle_usage.rs` cover parsing and
//! the cache rules, but call these functions with paths of their own.
//!
//! A fake `gh` on `PATH` stands in for the network call.
#![cfg(unix)]

mod common;

use common::{SESSION, SESSION_SPEND, Sandbox, assert_shows, payload};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const UNLIMITED: &str = include_str!("fixtures/copilot-user-unlimited.json");

/// Puts a `gh` that prints `response` in a bin dir under `HOME`, and returns
/// that dir, to be used as the whole `PATH`.
fn fake_gh(sandbox: &Sandbox, response: &str) -> PathBuf {
    let bin = sandbox.home().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let response_path = sandbox.home().join("gh-response.json");
    fs::write(&response_path, response).unwrap();
    let gh = bin.join("gh");
    fs::write(
        &gh,
        format!("#!/bin/sh\nexec /bin/cat '{}'\n", response_path.display()),
    )
    .unwrap();
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

fn run_fetch_worker(sandbox: &Sandbox, path: &Path, cache_path: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_copilot-powerline"))
        .arg("--fetch-cycle-usage")
        .arg(cache_path)
        .env("PATH", path)
        .env("HOME", sandbox.home())
        .env_remove("XDG_CACHE_HOME")
        .env_remove("COPILOT_GITHUB_TOKEN")
        .current_dir(sandbox.home())
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_fetched_cycle_usage_shows_on_the_next_refresh() {
    let sandbox = Sandbox::new();
    sandbox.write_config("theme = \"plain\"\nsegments = [\"cycle_cost\"]\n");
    // A cycle that has not ended, so the cached usage is shown.
    let response = UNLIMITED.replace("2026-11-01", "2999-01-01");
    let path = fake_gh(&sandbox, &response);

    run_fetch_worker(
        &sandbox,
        &path,
        &sandbox.state_dir().join("cycle_usage.json"),
    );
    let line = sandbox.refresh_with_env(
        &payload(SESSION, SESSION_SPEND),
        &["--style", "plain", "--icon-set", "plain"],
        &[("PATH", &path)],
    );

    assert_shows(&line, "Cycle: $156.49");
}
