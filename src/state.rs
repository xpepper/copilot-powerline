//! Small private files the status line keeps between invocations (PR lookups,
//! display mode, spend snapshots). Every refresh is a fresh process, so any
//! cross-refresh memory has to live on disk.

use serde::{Serialize, de::DeserializeOwned};
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Kinds of per-session or per-branch files that pruning may delete. Anything
/// else in the state directory (the `mode` override, the prune marker) is
/// never touched. `session_file` asserts its kind is listed here.
const PRUNABLE_KINDS: [&str; 5] = ["spend", "cache", "idle", "month", "pr"];

/// A session idle longer than this loses its snapshots; resuming it only
/// re-creates the baselines (idle time restarts from zero).
const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Minimum time between two directory scans.
const PRUNE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

const PRUNE_MARKER: &str = "last_prune";

/// Base directory for state files. Prefers the user-private cache directory
/// (e.g. `~/.cache` on Linux, `~/Library/Caches` on macOS) over the shared
/// system temp dir, since the temp dir's predictable, world-writable path
/// would let another local user pre-create or race the state files.
pub fn base_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("copilot-powerline")
}

pub fn ensure_private_dir(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

/// Writes `contents` to `path` with owner-only permissions, via a temp file
/// and rename so concurrent readers never see a partial write.
pub fn write_private_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }

    let tmp_path = path.with_extension(format!("tmp.{}", std::process::id()));
    {
        let mut file = File::create(&tmp_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(contents)?;
        file.flush()?;
    }

    fs::rename(&tmp_path, path)
}

/// Path of a per-session state file, e.g. `spend_<hash>.json`. The session id
/// is hashed so arbitrary ids can never escape the state directory.
pub fn session_file(kind: &str, session_id: &str) -> PathBuf {
    debug_assert!(
        PRUNABLE_KINDS.contains(&kind),
        "unknown state kind {kind:?}: add it to PRUNABLE_KINDS or it is never pruned"
    );
    let mut hasher = DefaultHasher::new();
    session_id.hash(&mut hasher);
    base_dir().join(format!("{kind}_{:016x}.json", hasher.finish()))
}

/// Loads the snapshot at `path` (if readable), lets `assess` derive the next
/// one, and persists it only when it changed. Write failures are ignored: the
/// status line must render even when the cache dir is read-only.
pub fn update_snapshot<T>(path: &Path, assess: impl FnOnce(Option<T>) -> T) -> T
where
    T: Serialize + DeserializeOwned + PartialEq + Copy,
{
    let prev = fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok());
    let next = assess(prev);

    if prev != Some(next)
        && let Ok(json) = serde_json::to_string(&next)
    {
        let _ = write_private_atomic(path, json.as_bytes());
    }
    next
}

fn is_prunable_name(name: &str) -> bool {
    PRUNABLE_KINDS.iter().any(|kind| {
        name.strip_prefix(kind)
            .is_some_and(|rest| rest.starts_with('_'))
    })
}

/// Deletes per-session state files (and their `.lock` / leftover `.tmp.<pid>`
/// siblings) last modified more than `MAX_AGE` ago. Errors are ignored: a
/// file that cannot be removed is simply tried again on the next scan, and a
/// racing writer at worst loses one baseline.
fn prune_stale(dir: &Path, now: SystemTime) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if !entry.file_name().to_str().is_some_and(is_prunable_name) {
            continue;
        }
        let is_stale = entry
            .metadata()
            .ok()
            .filter(|metadata| metadata.is_file())
            .and_then(|metadata| metadata.modified().ok())
            .is_some_and(|modified| now.duration_since(modified).is_ok_and(|age| age > MAX_AGE));
        if is_stale {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Prunes `dir` about once per `PRUNE_INTERVAL`, tracked by the mtime of a
/// marker file, so the common case costs a single `stat`. Refreshes that
/// overlap at the moment the marker goes stale may each scan once; the scan
/// is idempotent, so that is cheaper than a lock needing stale-lock handling.
/// Only ever call it with the state directory.
pub fn prune_if_due(dir: &Path, now: SystemTime) {
    let marker = dir.join(PRUNE_MARKER);
    let recently_pruned = fs::metadata(&marker)
        .and_then(|metadata| metadata.modified())
        .is_ok_and(|modified| {
            now.duration_since(modified)
                .is_ok_and(|age| age < PRUNE_INTERVAL)
        });
    if recently_pruned {
        return;
    }

    // Stamp before scanning, and skip the scan when stamping fails, so an
    // unwritable directory is not rescanned on every refresh.
    if File::create(&marker)
        .and_then(|file| file.set_modified(now))
        .is_ok()
    {
        prune_stale(dir, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_file_is_private_and_per_session_and_kind() {
        let a = session_file("spend", "sess-a");
        assert!(a.starts_with(base_dir()));
        assert_eq!(a, session_file("spend", "sess-a"));
        assert_ne!(a, session_file("spend", "sess-b"));
        assert_ne!(a, session_file("cache", "sess-a"));
    }

    const DAY: Duration = Duration::from_secs(24 * 60 * 60);

    fn touch(dir: &Path, name: &str, age: Duration, now: SystemTime) -> PathBuf {
        let path = dir.join(name);
        let file = File::create(&path).unwrap();
        file.set_modified(now - age).unwrap();
        path
    }

    #[test]
    fn test_prune_stale_removes_only_old_per_session_files() {
        let dir = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        let old = 31 * DAY;

        let removed: Vec<PathBuf> = [
            "spend_0123456789abcdef.json",
            "cache_0123456789abcdef.json",
            "idle_0123456789abcdef.json",
            "month_0123456789abcdef.json",
            "pr_0123456789abcdef.json",
            "pr_0123456789abcdef.lock",
            "spend_0123456789abcdef.tmp.4242",
        ]
        .iter()
        .map(|name| touch(dir.path(), name, old, now))
        .collect();
        let recent = touch(dir.path(), "spend_fedcba9876543210.json", 29 * DAY, now);
        let mode = touch(dir.path(), "mode", old, now);
        let unrelated = touch(dir.path(), "notes.txt", old, now);
        let unseparated = touch(dir.path(), "spendthrift.json", old, now);

        prune_stale(dir.path(), now);

        assert!(removed.iter().all(|path| !path.exists()));
        assert!(recent.exists() && mode.exists() && unrelated.exists() && unseparated.exists());
    }

    #[test]
    fn test_prune_stale_keeps_files_dated_in_the_future() {
        let dir = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        let path = dir.path().join("spend_0123456789abcdef.json");
        File::create(&path)
            .unwrap()
            .set_modified(now + 40 * DAY)
            .unwrap();

        prune_stale(dir.path(), now);

        assert!(path.exists());
    }

    #[test]
    fn test_prune_stale_skips_directories_and_tolerates_a_missing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        let subdir = dir.path().join("spend_0123456789abcdef.json");
        fs::create_dir(&subdir).unwrap();
        File::open(&subdir)
            .unwrap()
            .set_modified(now - 40 * DAY)
            .unwrap();

        prune_stale(dir.path(), now);
        prune_stale(&dir.path().join("missing"), now);

        assert!(subdir.is_dir());
    }

    #[test]
    fn test_prune_if_due_prunes_once_then_waits_for_the_interval() {
        let dir = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        let first = touch(dir.path(), "spend_aaaaaaaaaaaaaaaa.json", 31 * DAY, now);

        prune_if_due(dir.path(), now);
        assert!(!first.exists());
        assert!(dir.path().join(PRUNE_MARKER).exists());

        let second = touch(dir.path(), "spend_bbbbbbbbbbbbbbbb.json", 31 * DAY, now);
        prune_if_due(dir.path(), now + DAY / 2);
        assert!(second.exists());

        prune_if_due(dir.path(), now + DAY + DAY / 2);
        assert!(!second.exists());
    }

    #[test]
    fn test_prune_if_due_treats_a_future_marker_as_due_and_resets_it() {
        let dir = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        touch(dir.path(), PRUNE_MARKER, Duration::ZERO, now + 10 * DAY);
        let old = touch(dir.path(), "cache_aaaaaaaaaaaaaaaa.json", 31 * DAY, now);

        prune_if_due(dir.path(), now);
        assert!(!old.exists());

        let next = touch(dir.path(), "cache_bbbbbbbbbbbbbbbb.json", 31 * DAY, now);
        prune_if_due(dir.path(), now + DAY / 2);
        assert!(next.exists());
    }

    #[test]
    fn test_prune_if_due_skips_the_scan_when_the_marker_cannot_be_written() {
        let dir = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        fs::create_dir(dir.path().join(PRUNE_MARKER)).unwrap();
        let old = touch(dir.path(), "spend_aaaaaaaaaaaaaaaa.json", 31 * DAY, now);

        prune_if_due(dir.path(), now);
        prune_if_due(&dir.path().join("missing"), now);

        assert!(old.exists());
    }

    #[test]
    fn test_every_state_file_kind_is_prunable() {
        let names = [
            crate::spend::snapshot_path("s"),
            crate::cache_trend::snapshot_path("s"),
            crate::idle::snapshot_path("s"),
            crate::month_spend::snapshot_path("s"),
            crate::github::get_cache_file_path(Path::new("repo"), "branch"),
        ]
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned());

        for name in &names {
            assert!(is_prunable_name(name), "{name} would never be pruned");
        }
        assert!(!is_prunable_name(
            &crate::mode::override_file_path()
                .file_name()
                .unwrap()
                .to_string_lossy()
        ));
        assert!(!is_prunable_name(PRUNE_MARKER));
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "unknown state kind")]
    fn test_session_file_rejects_a_kind_that_pruning_does_not_know() {
        session_file("brand_new", "sess-a");
    }
}
