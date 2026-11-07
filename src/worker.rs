//! Shared plumbing for the detached background workers (`--fetch-pr-cache`,
//! `--fetch-cycle-usage`, `--fetch-branch-size`): a check that the program a
//! worker runs is installed, a throttled claim so only one worker runs per
//! cache file, starting the worker detached, and a deadline on the external
//! command it runs.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant, UNIX_EPOCH};

/// Minimum time between two background refreshes of one cache, even while
/// they keep failing.
pub const THROTTLE_SECONDS: u64 = 15;

/// How long a worker's external command may run. Shorter than the refresh
/// throttle, so a hung call is killed before the next worker can start.
pub const WORKER_TIMEOUT: Duration = Duration::from_secs(10);
const _: () = assert!(WORKER_TIMEOUT.as_secs() < THROTTLE_SECONDS);

/// One `WORKER_TIMEOUT` shared by all of a worker's external commands, so
/// the worker as a whole ends before the throttle lets the next one start.
pub struct Deadline(Instant);

impl Deadline {
    pub fn start() -> Self {
        Self(Instant::now() + WORKER_TIMEOUT)
    }

    /// The time left for the next command; zero once the deadline passed.
    pub fn remaining(&self) -> Duration {
        self.0.saturating_duration_since(Instant::now())
    }
}

/// Whether `program` is a file in one of the `PATH` directories, checked
/// without spawning it.
pub fn is_on_path(program: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| {
        #[cfg(windows)]
        let candidate = dir.join(format!("{program}.exe"));
        #[cfg(not(windows))]
        let candidate = dir.join(program);
        candidate.is_file()
    })
}

/// Re-invokes this binary with `args` as a detached worker. Its stdio is
/// null, so it never holds the status line's pipes open.
pub fn spawn_detached(args: &[&OsStr]) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

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

/// Starts a background refresh of `cache_path` through `spawn_refresh` when
/// the cache, written at `cached_at`, is missing or `cache_ttl_seconds` old. At
/// most one refresh starts per throttle window.
pub fn refresh_if_stale(
    cache_path: &Path,
    cached_at: Option<u64>,
    cache_ttl_seconds: u64,
    now: u64,
    spawn_refresh: impl FnOnce(&Path),
) {
    let fresh = cached_at.is_some_and(|at| now.saturating_sub(at) < cache_ttl_seconds);
    if !fresh && try_claim_spawn(cache_path, now, THROTTLE_SECONDS) {
        spawn_refresh(cache_path);
    }
}

pub fn lock_file_path(cache_path: &Path) -> PathBuf {
    cache_path.with_extension("lock")
}

/// Returns true if a background refresh claim for `cache_path` was taken
/// recently (within `throttle_seconds`), based on the claim lock file's
/// mtime. The lock file is a separate marker from the cache entry itself, so
/// throttling never touches (or misrepresents the freshness of) cached data.
pub fn should_throttle_spawn(cache_path: &Path, now: u64, throttle_seconds: u64) -> bool {
    is_lock_fresh(&lock_file_path(cache_path), now, throttle_seconds)
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

    if create_lock(&lock_path) {
        return true;
    }

    // A lock exists and looked stale (e.g. left behind by a crashed worker).
    // Only the holder of a short-lived reclaim guard may delete it, and it
    // re-checks staleness under the guard, so a lock freshly created by a
    // concurrent claimer is never removed.
    let guard_path = lock_path.with_extension("lock.reclaim");
    if !create_lock(&guard_path) {
        if !is_lock_fresh(&guard_path, now, throttle_seconds) {
            // A crashed reclaimer left its guard behind; clear it so a
            // later refresh can reclaim.
            let _ = fs::remove_file(&guard_path);
        }
        return false;
    }
    let won = !is_lock_fresh(&lock_path, now, throttle_seconds) && {
        let _ = fs::remove_file(&lock_path);
        create_lock(&lock_path)
    };
    let _ = fs::remove_file(&guard_path);
    won
}

fn is_lock_fresh(lock_path: &Path, now: u64, throttle_seconds: u64) -> bool {
    fs::metadata(lock_path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .is_some_and(|dur| now.saturating_sub(dur.as_secs()) < throttle_seconds)
}

/// `create_new` fails if another process's creation won, which is what
/// arbitrates the race.
fn create_lock(lock_path: &Path) -> bool {
    match File::options().write(true).create_new(true).open(lock_path) {
        Ok(_file) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(lock_path, fs::Permissions::from_mode(0o600));
            }
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::current_timestamp;

    #[cfg(unix)]
    #[test]
    fn test_a_program_in_a_path_directory_is_found() {
        let found = is_on_path("sh");

        assert!(found);
    }

    #[test]
    fn test_a_program_in_no_path_directory_is_not_found() {
        let found = is_on_path("copilot-powerline-no-such-command");

        assert!(!found);
    }

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

    fn unique_cache_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "copilot_powerline_test_{}_{}_{}.json",
            label,
            std::process::id(),
            current_timestamp()
        ))
    }

    /// Calls `refresh_if_stale` with a 60 s TTL and reports whether it
    /// started a refresh.
    fn refreshed(cache_path: &Path, cached_at: Option<u64>, now: u64) -> bool {
        let mut spawned = false;
        refresh_if_stale(cache_path, cached_at, 60, now, |_| spawned = true);
        spawned
    }

    #[test]
    fn test_a_cache_one_ttl_old_is_refreshed() {
        let cache_path = unique_cache_path("refresh_stale");

        let spawned = refreshed(&cache_path, Some(100), 160);

        let _ = fs::remove_file(lock_file_path(&cache_path));
        assert!(spawned);
    }

    #[test]
    fn test_a_cache_younger_than_its_ttl_is_not_refreshed() {
        let cache_path = unique_cache_path("refresh_fresh");

        let spawned = refreshed(&cache_path, Some(100), 159);

        assert!(!spawned);
    }

    #[test]
    fn test_a_missing_cache_is_refreshed_once_per_throttle_window() {
        let cache_path = unique_cache_path("refresh_missing");

        let first = refreshed(&cache_path, None, 100);
        let second = refreshed(&cache_path, None, 101);

        let _ = fs::remove_file(lock_file_path(&cache_path));
        assert!(first);
        assert!(!second);
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
    fn test_a_claim_exactly_one_throttle_window_old_can_be_reclaimed() {
        let cache_path = unique_cache_path("claim_boundary");
        assert!(try_claim_spawn(&cache_path, current_timestamp(), 15));
        let claimed_at = fs::metadata(lock_file_path(&cache_path))
            .and_then(|m| m.modified())
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let reclaimed = try_claim_spawn(&cache_path, claimed_at + 15, 15);

        assert!(reclaimed);
        let _ = fs::remove_file(lock_file_path(&cache_path));
    }

    #[test]
    fn test_try_claim_spawn_single_winner_when_reclaiming_stale_lock_concurrently() {
        use std::sync::{Arc, Barrier};
        use std::time::Duration;

        let now = current_timestamp();
        for round in 0..50 {
            let cache_path = unique_cache_path(&format!("claim_stale_race_{round}"));
            let lock_path = lock_file_path(&cache_path);
            let stale = File::create(&lock_path).unwrap();
            stale
                .set_modified(UNIX_EPOCH + Duration::from_secs(now - 100))
                .unwrap();
            drop(stale);

            let barrier = Arc::new(Barrier::new(8));
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    let (barrier, cache_path) = (barrier.clone(), cache_path.clone());
                    std::thread::spawn(move || {
                        barrier.wait();
                        try_claim_spawn(&cache_path, now, 15)
                    })
                })
                .collect();
            let winners = handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .filter(|won| *won)
                .count();

            let _ = fs::remove_file(&lock_path);
            assert_eq!(winners, 1, "round {round}: exactly one claim must win");
        }
    }

    #[test]
    fn test_a_guard_left_by_a_crashed_reclaimer_is_cleared_for_the_next_claim() {
        let cache_path = unique_cache_path("claim_stale_guard");
        let lock_path = lock_file_path(&cache_path);
        let guard_path = lock_path.with_extension("lock.reclaim");
        let now = current_timestamp();
        for path in [&lock_path, &guard_path] {
            File::create(path)
                .unwrap()
                .set_modified(UNIX_EPOCH + Duration::from_secs(now - 100))
                .unwrap();
        }

        let blocked = try_claim_spawn(&cache_path, now, 15);
        let next = try_claim_spawn(&cache_path, now, 15);

        let _ = fs::remove_file(&lock_path);
        let _ = fs::remove_file(&guard_path);
        assert!(!blocked, "a claim that finds a guard must not reclaim");
        assert!(next, "the stale guard should be gone for the next claim");
    }

    #[test]
    fn test_try_claim_spawn_does_not_touch_cache_entry() {
        let cache_path = unique_cache_path("claim_no_touch");
        fs::write(&cache_path, b"cached").unwrap();

        let now = current_timestamp();
        assert!(try_claim_spawn(&cache_path, now, 15));

        let unchanged = fs::read(&cache_path).unwrap();

        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(lock_file_path(&cache_path));

        assert_eq!(unchanged, b"cached");
    }
}
