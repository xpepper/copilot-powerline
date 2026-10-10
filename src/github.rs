use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestInfo {
    pub number: u64,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrCacheEntry {
    pub timestamp: u64,
    pub pr: Option<PullRequestInfo>,
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn is_gh_available() -> bool {
    if let Some(paths) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&paths) {
            let candidate = path.join("gh");
            #[cfg(windows)]
            let candidate = path.join("gh.exe");

            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

pub fn get_cache_file_path(repo_dir: &Path, branch: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    repo_dir.to_string_lossy().hash(&mut hasher);
    branch.hash(&mut hasher);
    let hash = hasher.finish();

    crate::state::base_dir().join(format!("pr_{:016x}.json", hash))
}

pub fn read_cache_entry(cache_path: &Path) -> Option<PrCacheEntry> {
    let content = fs::read_to_string(cache_path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn write_cache_entry(cache_path: &Path, entry: &PrCacheEntry) -> std::io::Result<()> {
    let json = serde_json::to_string(entry)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    crate::state::write_private_atomic(cache_path, json.as_bytes())
}

/// How long a background `gh` call may run. Shorter than the refresh
/// throttle, so a hung call is killed before the next worker can start.
pub const GH_TIMEOUT: Duration = Duration::from_secs(10);

/// Runs `command` and returns its stdout, or `None` when it cannot start or
/// is still running after `timeout` (it is killed). stderr is discarded.
///
/// Without a deadline, a `gh` call stuck on the network would hold the
/// refresh claim until it goes stale, and the next worker would pile on.
pub fn output_with_timeout(command: &mut Command, timeout: Duration) -> Option<Output> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut pipe = child.stdout.take()?;
    // Drained on its own thread so a large response cannot fill the pipe and
    // stall the child while this thread polls.
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf);
        buf
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    Some(Output {
        status,
        stdout: reader.join().ok()?,
        stderr: Vec::new(),
    })
}

pub fn is_cache_fresh(entry: &PrCacheEntry, ttl_seconds: u64, now: u64) -> bool {
    now.saturating_sub(entry.timestamp) < ttl_seconds
}

pub fn lock_file_path(cache_path: &Path) -> PathBuf {
    cache_path.with_extension("lock")
}

/// Returns true if a background refresh claim for `cache_path` was taken
/// recently (within `throttle_seconds`), based on the claim lock file's
/// mtime. The lock file is a separate marker from the cache entry itself, so
/// throttling never touches (or misrepresents the freshness of) cached data.
pub fn should_throttle_spawn(cache_path: &Path, now: u64, throttle_seconds: u64) -> bool {
    if let Ok(metadata) = fs::metadata(lock_file_path(cache_path))
        && let Ok(modified) = metadata.modified()
        && let Ok(dur) = modified.duration_since(UNIX_EPOCH)
    {
        return now.saturating_sub(dur.as_secs()) < throttle_seconds;
    }
    false
}

/// Atomically claims the right to spawn a background refresh for
/// `cache_path`. Returns true if this call won the claim, false if another
/// process already holds a live claim (or won a concurrent race for a new
/// one). The claim never reads or writes the cache entry, so a losing (or
/// failing) refresh attempt can never mark stale cached data as fresh.
pub fn try_claim_spawn(cache_path: &Path, now: u64, throttle_seconds: u64) -> bool {
    if should_throttle_spawn(cache_path, now, throttle_seconds) {
        return false;
    }

    let lock_path = lock_file_path(cache_path);
    if let Some(parent) = lock_path.parent() {
        let _ = crate::state::ensure_private_dir(parent);
    }

    // A stale lock (older than throttle_seconds, e.g. left behind by a
    // crashed worker) is removed before claiming; the final `create_new`
    // below is what actually arbitrates a concurrent race atomically, since
    // it fails if another process's `create_new` won in the meantime.
    let _ = fs::remove_file(&lock_path);

    match File::options()
        .write(true)
        .create_new(true)
        .open(&lock_path)
    {
        Ok(_file) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&lock_path, fs::Permissions::from_mode(0o600));
            }
            true
        }
        Err(_) => false,
    }
}

#[derive(Deserialize)]
struct GhPrViewPayload {
    number: u64,
    url: String,
}

/// Runs `gh pr view` synchronously and writes the result to the cache file.
///
/// This blocks on a network call, so it must only ever run in the detached
/// background worker process spawned by `spawn_background_fetch`, never on
/// the status-line's hot path.
pub fn fetch_and_write_pr_cache(repo_dir: &Path, cache_path: &Path) {
    let now = current_timestamp();

    let output = output_with_timeout(
        Command::new("gh")
            .args(["pr", "view", "--json", "number,url"])
            .current_dir(repo_dir),
        GH_TIMEOUT,
    );

    let entry = match output {
        Some(out) if out.status.success() => {
            if let Ok(payload) = serde_json::from_slice::<GhPrViewPayload>(&out.stdout) {
                PrCacheEntry {
                    timestamp: now,
                    pr: Some(PullRequestInfo {
                        number: payload.number,
                        url: payload.url,
                    }),
                }
            } else {
                PrCacheEntry {
                    timestamp: now,
                    pr: None,
                }
            }
        }
        _ => PrCacheEntry {
            timestamp: now,
            pr: None,
        },
    };

    let _ = write_cache_entry(cache_path, &entry);
    let _ = fs::remove_file(lock_file_path(cache_path));
}

fn spawn_background_fetch(repo_dir: &Path, cache_path: &Path) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe)
            .arg("--fetch-pr-cache")
            .arg(cache_path)
            .arg("--repo-dir")
            .arg(repo_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

/// Returns the current branch's PR info, if any is cached and fresh.
///
/// Never blocks on the network: a stale or missing cache triggers a
/// throttled, detached background refresh (via re-invoking this binary with
/// `--fetch-pr-cache`) and returns immediately with whatever was cached.
pub fn get_pr_info(
    repo_dir: &Path,
    branch: &str,
    ttl_seconds: u64,
    gh_available: bool,
) -> Option<PullRequestInfo> {
    if !gh_available {
        return None;
    }

    let cache_path = get_cache_file_path(repo_dir, branch);
    let now = current_timestamp();
    let throttle_seconds = 15;

    if let Some(entry) = read_cache_entry(&cache_path) {
        if is_cache_fresh(&entry, ttl_seconds, now) {
            return entry.pr;
        }

        // Cache is stale. Claim the right to refresh atomically; the loser
        // of a concurrent race does not spawn a duplicate worker, and the
        // stale entry on disk is left untouched (never marked fresh) until
        // a real refresh completes.
        if try_claim_spawn(&cache_path, now, throttle_seconds) {
            spawn_background_fetch(repo_dir, &cache_path);
        }

        entry.pr
    } else {
        // No cache exists yet. If not throttled, claim and spawn.
        if try_claim_spawn(&cache_path, now, throttle_seconds) {
            spawn_background_fetch(repo_dir, &cache_path);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_output_with_timeout_returns_stdout_of_a_finished_command() {
        let mut command = Command::new("echo");
        command.arg("hello");
        let out = output_with_timeout(&mut command, Duration::from_secs(5)).unwrap();
        assert!(out.status.success());
        assert_eq!(out.stdout, b"hello\n");
    }

    #[cfg(unix)]
    #[test]
    fn test_output_with_timeout_kills_a_command_that_hangs() {
        let started = std::time::Instant::now();
        let mut command = Command::new("sleep");
        command.arg("30");
        assert!(output_with_timeout(&mut command, Duration::from_millis(200)).is_none());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn test_output_with_timeout_is_none_when_the_command_is_missing() {
        let mut command = Command::new("copilot-powerline-no-such-command");
        assert!(output_with_timeout(&mut command, Duration::from_secs(1)).is_none());
    }

    #[test]
    fn test_cache_entry_roundtrip() {
        let temp_file = std::env::temp_dir().join(format!(
            "test_cache_{}_{}.json",
            std::process::id(),
            current_timestamp()
        ));

        let entry = PrCacheEntry {
            timestamp: 1000,
            pr: Some(PullRequestInfo {
                number: 50,
                url: "https://github.com/org/repo/pull/50".to_string(),
            }),
        };

        write_cache_entry(&temp_file, &entry).unwrap();
        let loaded = read_cache_entry(&temp_file).unwrap();
        assert_eq!(loaded, entry);

        let _ = fs::remove_file(&temp_file);
    }

    #[test]
    fn test_cache_entry_none_roundtrip() {
        let temp_file = std::env::temp_dir().join(format!(
            "test_cache_none_{}_{}.json",
            std::process::id(),
            current_timestamp()
        ));

        let entry = PrCacheEntry {
            timestamp: 12345,
            pr: None,
        };

        write_cache_entry(&temp_file, &entry).unwrap();
        let loaded = read_cache_entry(&temp_file).unwrap();
        assert_eq!(loaded, entry);

        let _ = fs::remove_file(&temp_file);
    }

    #[test]
    fn test_cache_freshness() {
        let entry = PrCacheEntry {
            timestamp: 100,
            pr: None,
        };
        assert!(is_cache_fresh(&entry, 60, 150));
        assert!(!is_cache_fresh(&entry, 60, 160));
        assert!(!is_cache_fresh(&entry, 60, 200));
    }

    #[test]
    fn test_get_cache_file_path_avoids_shared_temp_dir_when_private_cache_available() {
        if let Some(cache_dir) = dirs::cache_dir() {
            let path = get_cache_file_path(Path::new("/some/repo"), "feature-x");
            assert!(path.starts_with(&cache_dir));
            assert!(!path.starts_with(std::env::temp_dir()));
        }
    }

    #[test]
    fn test_get_cache_file_path_deterministic() {
        let path1 = get_cache_file_path(Path::new("/some/repo"), "feature-x");
        let path2 = get_cache_file_path(Path::new("/some/repo"), "feature-x");
        let path3 = get_cache_file_path(Path::new("/some/repo"), "feature-y");
        assert_eq!(path1, path2);
        assert_ne!(path1, path3);
    }

    #[test]
    fn test_get_pr_info_when_gh_not_available() {
        let res = get_pr_info(Path::new("/some/repo"), "main", 60, false);
        assert!(res.is_none());
    }

    fn unique_cache_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "copilot_powerline_test_{}_{}_{}.json",
            label,
            std::process::id(),
            current_timestamp()
        ))
    }

    #[test]
    fn test_try_claim_spawn_only_one_winner_when_racing() {
        let cache_path = unique_cache_path("claim_race");
        let now = current_timestamp();

        let first = try_claim_spawn(&cache_path, now, 15);
        let second = try_claim_spawn(&cache_path, now, 15);

        let _ = fs::remove_file(lock_file_path(&cache_path));

        assert!(first, "first claim attempt should win");
        assert!(!second, "second concurrent claim attempt should lose");
    }

    #[test]
    fn test_try_claim_spawn_allows_reclaim_after_throttle_window() {
        let cache_path = unique_cache_path("claim_reclaim");
        let now = current_timestamp();

        assert!(try_claim_spawn(&cache_path, now, 15));
        assert!(try_claim_spawn(&cache_path, now + 20, 15));

        let _ = fs::remove_file(lock_file_path(&cache_path));
    }

    #[test]
    fn test_try_claim_spawn_does_not_touch_cache_entry() {
        let cache_path = unique_cache_path("claim_no_touch");
        let original = PrCacheEntry {
            timestamp: 100,
            pr: Some(PullRequestInfo {
                number: 7,
                url: "https://github.com/org/repo/pull/7".to_string(),
            }),
        };
        write_cache_entry(&cache_path, &original).unwrap();

        let now = current_timestamp();
        assert!(try_claim_spawn(&cache_path, now, 15));

        let unchanged = read_cache_entry(&cache_path).unwrap();

        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(lock_file_path(&cache_path));

        assert_eq!(unchanged, original);
    }
}
