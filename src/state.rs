//! Small private files the status line keeps between invocations (PR lookups,
//! display mode, spend snapshots). Every refresh is a fresh process, so any
//! cross-refresh memory has to live on disk.

use std::fs::{self, File};
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
