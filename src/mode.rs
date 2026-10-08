//! Runtime override of the display mode, flipped by `--toggle`.
//!
//! Copilot CLI re-runs the status line command on every refresh, so writing
//! the chosen mode to a state file takes effect on the next refresh without
//! restarting Copilot or editing `powerline.toml`.

use crate::config::DisplayMode;
use std::path::{Path, PathBuf};

pub fn override_file_path() -> PathBuf {
    crate::state::base_dir().join("mode")
}

pub fn read_override(path: &Path) -> Option<DisplayMode> {
    std::fs::read_to_string(path).ok()?.parse().ok()
}

/// The mode to render: the toggled override if one exists, else the config's.
pub fn effective(path: &Path, configured: DisplayMode) -> DisplayMode {
    read_override(path).unwrap_or(configured)
}

/// Flips the effective mode and returns it. The override is only kept while
/// it differs from the configured mode, so it never masks later config edits.
pub fn toggle(path: &Path, configured: DisplayMode) -> std::io::Result<DisplayMode> {
    let next = effective(path, configured).toggled();
    if next == configured {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    } else {
        crate::state::write_private_atomic(path, next.as_str().as_bytes())?;
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "copilot-powerline-mode-{}-{}-{}",
            label,
            std::process::id(),
            crate::github::current_timestamp()
        ))
    }

    #[test]
    fn test_no_override_uses_configured_mode() {
        let path = unique_path("missing");
        assert_eq!(read_override(&path), None);
        assert_eq!(effective(&path, DisplayMode::Compact), DisplayMode::Compact);
    }

    #[test]
    fn test_toggle_flips_and_persists() {
        let path = unique_path("toggle");

        assert_eq!(
            toggle(&path, DisplayMode::Full).unwrap(),
            DisplayMode::Compact
        );
        assert_eq!(effective(&path, DisplayMode::Full), DisplayMode::Compact);

        assert_eq!(toggle(&path, DisplayMode::Full).unwrap(), DisplayMode::Full);
        assert_eq!(effective(&path, DisplayMode::Full), DisplayMode::Full);
    }

    #[test]
    fn test_toggling_back_to_configured_mode_clears_override() {
        let path = unique_path("clear");

        toggle(&path, DisplayMode::Full).unwrap();
        assert!(path.exists());
        toggle(&path, DisplayMode::Full).unwrap();

        // A leftover "full" override would silently mask a later
        // `mode = "compact"` edit in powerline.toml.
        assert!(!path.exists());
        assert_eq!(effective(&path, DisplayMode::Compact), DisplayMode::Compact);
    }

    #[test]
    fn test_unreadable_override_is_ignored() {
        let path = unique_path("garbage");
        fs::write(&path, "sideways").unwrap();

        assert_eq!(read_override(&path), None);
        assert_eq!(effective(&path, DisplayMode::Full), DisplayMode::Full);

        let _ = fs::remove_file(&path);
    }
}
