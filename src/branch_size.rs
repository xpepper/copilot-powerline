//! How big the current branch is compared with the remote default branch.
//!
//! Measuring it runs `git diff`, so it follows the `pr` segment's pattern:
//! the status line only reads a disk cache, and a stale cache triggers a
//! throttled, detached refresh (this binary re-invoked with
//! `--fetch-branch-size`).

use crate::worker::{WORKER_TIMEOUT, output_with_timeout};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

/// How much the branch changed since it left the default branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchSize {
    /// The default branch it was compared with, without `origin/`.
    pub default_branch: String,
    /// What the branch's commits changed, as its pull request counts it.
    pub committed: DiffStat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CacheEntry {
    timestamp: u64,
    /// `None` when the last refresh could not measure the branch.
    size: Option<BranchSize>,
}

/// Where the size of `branch` in the repository at `repo_dir` is cached.
pub fn cache_path(repo_dir: &Path, branch: &str) -> PathBuf {
    crate::state::branch_file("branch_size", repo_dir, branch)
}

/// The branch size, if it is cached.
///
/// Never runs `git`: a stale or missing cache calls `spawn_refresh` (at
/// most once per throttle window) and returns whatever was cached.
pub fn get_branch_size(
    cache_path: &Path,
    ttl_seconds: u64,
    now: u64,
    spawn_refresh: impl FnOnce(&Path),
) -> Option<BranchSize> {
    let entry = read_entry(cache_path);
    let cached_at = entry.as_ref().map(|e| e.timestamp);
    crate::worker::refresh_if_stale(cache_path, cached_at, ttl_seconds, now, spawn_refresh);

    entry.and_then(|e| e.size)
}

fn read_entry(cache_path: &Path) -> Option<CacheEntry> {
    std::fs::read_to_string(cache_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
}

/// Re-invokes this binary as a detached `--fetch-branch-size` worker.
pub fn spawn_background_fetch(repo_dir: &Path, cache_path: &Path) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe)
            .arg("--fetch-branch-size")
            .arg(cache_path)
            .arg("--repo-dir")
            .arg(repo_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

/// Measures the branch with `git` and writes the result to the cache.
///
/// This runs `git diff`, so it must only ever run in the detached worker
/// started by `spawn_background_fetch`, never on the hot path.
pub fn fetch_and_write_branch_size(repo_dir: &Path, cache_path: &Path) {
    let entry = CacheEntry {
        timestamp: crate::github::current_timestamp(),
        size: measure(repo_dir),
    };
    if let Ok(json) = serde_json::to_string(&entry) {
        let _ = crate::state::write_private_atomic(cache_path, json.as_bytes());
    }
    let _ = std::fs::remove_file(crate::worker::lock_file_path(cache_path));
}

/// The committed size since the merge base with `origin/<default>`, as a
/// pull request counts it.
fn measure(repo_dir: &Path) -> Option<BranchSize> {
    let started = Instant::now();
    // All calls share one WORKER_TIMEOUT, so the worker as a whole still
    // ends before the throttle lets the next one start.
    let git = |args: &[&str]| {
        let remaining = WORKER_TIMEOUT.saturating_sub(started.elapsed());
        let output = output_with_timeout(
            Command::new("git").args(args).current_dir(repo_dir),
            remaining,
        )?;
        output
            .status
            .success()
            .then(|| String::from_utf8(output.stdout).ok())
            .flatten()
    };

    let default_branch = default_branch_from(&git(&[
        "symbolic-ref",
        "--short",
        "refs/remotes/origin/HEAD",
    ])?)?;
    let range = format!("origin/{default_branch}...HEAD");
    let committed = parse_numstat(&git(&["diff", "--numstat", &range])?)?;
    Some(BranchSize {
        default_branch,
        committed,
    })
}

/// The default branch named by the stdout of `git symbolic-ref --short
/// refs/remotes/origin/HEAD`, e.g. `main` from `origin/main`.
fn default_branch_from(stdout: &str) -> Option<String> {
    stdout.trim().strip_prefix("origin/").map(str::to_string)
}

/// Lines added and removed, and files changed, in a diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DiffStat {
    pub added: u64,
    pub removed: u64,
    pub files: u64,
}

/// Adds up the output of `git diff --numstat`, one `added<TAB>removed<TAB>path`
/// line per file. A binary file shows `-` for both counts, and adds no lines.
fn parse_numstat(stdout: &str) -> Option<DiffStat> {
    let count = |field: &str| match field {
        "-" => Some(0),
        number => number.parse::<u64>().ok(),
    };
    stdout.lines().try_fold(DiffStat::default(), |stat, line| {
        let mut fields = line.split('\t');
        let added = count(fields.next()?)?;
        let removed = count(fields.next()?)?;
        Some(DiffStat {
            added: stat.added + added,
            removed: stat.removed + removed,
            files: stat.files + 1,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_790_000_000;
    const TTL: u64 = 60;

    fn cache_with(entry: Option<CacheEntry>) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("branch_size.json");
        if let Some(entry) = entry {
            std::fs::write(&path, serde_json::to_string(&entry).unwrap()).unwrap();
        }
        (dir, path)
    }

    /// Calls `get_branch_size` and reports whether it asked for a refresh.
    fn get(path: &Path, now: u64) -> (Option<BranchSize>, bool) {
        let mut spawned = false;
        let size = get_branch_size(path, TTL, now, |_| spawned = true);
        (size, spawned)
    }

    #[test]
    fn test_missing_cache_starts_a_refresh_and_shows_nothing() {
        let (_dir, path) = cache_with(None);

        let result = get(&path, NOW);

        assert_eq!(result, (None, true));
    }

    fn size() -> BranchSize {
        BranchSize {
            default_branch: "main".to_string(),
            committed: DiffStat {
                added: 5,
                removed: 1,
                files: 2,
            },
        }
    }

    #[test]
    fn test_fresh_cache_is_shown_without_refreshing() {
        let (_dir, path) = cache_with(Some(CacheEntry {
            timestamp: NOW - 10,
            size: Some(size()),
        }));

        let result = get(&path, NOW);

        assert_eq!(result, (Some(size()), false));
    }

    #[test]
    fn test_stale_cache_is_shown_while_a_refresh_starts() {
        let (_dir, path) = cache_with(Some(CacheEntry {
            timestamp: NOW - TTL,
            size: Some(size()),
        }));

        let result = get(&path, NOW);

        assert_eq!(result, (Some(size()), true));
    }

    #[test]
    fn test_each_branch_of_a_repository_has_its_own_cache() {
        let repo = Path::new("/some/repo");

        let feature = cache_path(repo, "feature");

        assert_ne!(feature, cache_path(repo, "other"));
        assert_ne!(feature, cache_path(Path::new("/other/repo"), "feature"));
        assert!(
            feature
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("branch_size_")
        );
    }

    #[test]
    fn test_numstat_lines_add_up() {
        let stdout = "2\t1\ta.txt\n3\t0\tc.txt\n";

        let stat = parse_numstat(stdout);

        assert_eq!(
            stat,
            Some(DiffStat {
                added: 5,
                removed: 1,
                files: 2
            })
        );
    }

    #[test]
    fn test_a_binary_file_counts_as_a_file_with_no_lines() {
        let stdout = "-\t-\timage.png\n4\t2\ta.txt\n";

        let stat = parse_numstat(stdout);

        assert_eq!(
            stat,
            Some(DiffStat {
                added: 4,
                removed: 2,
                files: 2
            })
        );
    }

    #[test]
    fn test_an_empty_diff_has_no_lines_and_no_files() {
        let stat = parse_numstat("");

        assert_eq!(stat, Some(DiffStat::default()));
    }

    #[test]
    fn test_unexpected_output_gives_no_size() {
        let stat = parse_numstat("2 files changed\n");

        assert_eq!(stat, None);
    }

    #[test]
    fn test_the_default_branch_drops_the_remote_name() {
        let branch = default_branch_from("origin/main\n");

        assert_eq!(branch.as_deref(), Some("main"));
    }

    #[test]
    fn test_no_default_branch_without_an_origin_ref() {
        let branch = default_branch_from("");

        assert_eq!(branch, None);
    }
}
