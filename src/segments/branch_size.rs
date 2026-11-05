use super::is_listed;
use crate::branch_size::{BranchSize, DiffStat};
use crate::config::BranchSizeConfig;

/// The name that lists this segment in `segments` and `compact_segments`.
pub const NAME: &str = "branch_size";

/// Whether the segment will render, so callers can skip the branch size
/// lookup (a disk cache read and a possible background `git` refresh) when
/// nothing would show it.
pub fn is_visible(segments: &[String], config: &BranchSizeConfig) -> bool {
    config.enabled && is_listed(segments, NAME)
}

pub fn render_branch_size_segment(size: Option<&BranchSize>) -> Option<String> {
    let size = size?;
    let DiffStat {
        added,
        removed,
        files,
    } = size.committed;
    Some(format!(
        "+{added} -{removed} · {files} files vs {}",
        size.default_branch
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segments::names;

    fn size(added: u64, removed: u64, files: u64) -> BranchSize {
        BranchSize {
            default_branch: "main".to_string(),
            committed: DiffStat {
                added,
                removed,
                files,
            },
        }
    }

    #[test]
    fn test_shows_lines_and_files_against_the_default_branch() {
        let size = size(420, 80, 12);

        let rendered = render_branch_size_segment(Some(&size));

        assert_eq!(rendered.as_deref(), Some("+420 -80 · 12 files vs main"));
    }

    #[test]
    fn test_visible_when_listed_and_enabled() {
        let cfg = BranchSizeConfig::default();

        let visible = is_visible(&names(&["tokens", "branch_size"]), &cfg);

        assert!(visible);
    }

    #[test]
    fn test_hidden_when_not_listed() {
        let cfg = BranchSizeConfig::default();

        let visible = is_visible(&names(&["tokens"]), &cfg);

        assert!(!visible);
    }

    #[test]
    fn test_hidden_when_disabled() {
        let cfg = BranchSizeConfig {
            enabled: false,
            ..Default::default()
        };

        let visible = is_visible(&names(&["branch_size"]), &cfg);

        assert!(!visible);
    }
}
