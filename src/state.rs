//! Small private files the status line keeps between invocations (PR lookups,
//! display mode, spend snapshots). Every refresh is a fresh process, so any
//! cross-refresh memory has to live on disk.

use serde::{Serialize, de::DeserializeOwned};
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};

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
}
