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
    let committed = format!(
        "{} · {} files vs {}",
        lines(&size.committed),
        size.committed.files,
        size.default_branch
    );
    if size.uncommitted.files == 0 {
        return Some(committed);
    }
    Some(format!(
        "{committed} · {} uncommitted",
        lines(&size.uncommitted)
    ))
}

fn lines(stat: &DiffStat) -> String {
    format!("+{} -{}", stat.added, stat.removed)
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
            uncommitted: DiffStat::default(),
        }
    }

    fn with_uncommitted(size: BranchSize, added: u64, removed: u64, files: u64) -> BranchSize {
        BranchSize {
            uncommitted: DiffStat {
                added,
                removed,
                files,
            },
            ..size
        }
    }

    #[test]
    fn test_shows_lines_and_files_against_the_default_branch() {
        let size = size(420, 80, 12);

        let rendered = render_branch_size_segment(Some(&size));

        assert_eq!(rendered.as_deref(), Some("+420 -80 · 12 files vs main"));
    }

    #[test]
    fn test_adds_the_uncommitted_lines() {
        let size = with_uncommitted(size(420, 80, 12), 35, 2, 3);

        let rendered = render_branch_size_segment(Some(&size));

        assert_eq!(
            rendered.as_deref(),
            Some("+420 -80 · 12 files vs main · +35 -2 uncommitted")
        );
    }

    #[test]
    fn test_an_uncommitted_binary_file_shows_no_lines() {
        let size = with_uncommitted(size(4, 1, 1), 0, 0, 1);

        let rendered = render_branch_size_segment(Some(&size));

        assert_eq!(
            rendered.as_deref(),
            Some("+4 -1 · 1 files vs main · +0 -0 uncommitted")
        );
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
