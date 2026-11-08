use super::is_listed;
use crate::branch_size::{BranchSize, DiffStat};
use crate::config::BranchSizeConfig;
use crate::theme::Palette;

/// The name that lists this segment in `segments` and `compact_segments`.
pub const NAME: &str = "branch_size";

/// Whether the segment will render, so callers can skip the branch size
/// lookup (a disk cache read and a possible background `git` refresh) when
/// nothing would show it.
pub fn is_visible(segments: &[String], config: &BranchSizeConfig) -> bool {
    config.enabled && is_listed(segments, NAME)
}

pub fn render_branch_size_segment(
    size: Option<&BranchSize>,
    current_branch: &str,
    config: &BranchSizeConfig,
    palette: &Palette,
) -> Option<String> {
    let size = size?;
    if current_branch == size.default_branch {
        return None;
    }
    if size.is_empty() {
        return None;
    }
    if config
        .alert_above_lines
        .is_some_and(|limit| size.changed_lines() > limit)
    {
        let uncolored = describe(size, &Palette::for_theme("plain"));
        return Some(format!(
            "{}{uncolored}{}",
            palette.tokens_alert, palette.reset
        ));
    }
    Some(describe(size, palette))
}

fn describe(size: &BranchSize, palette: &Palette) -> String {
    let committed = format!(
        "{} · {} vs {}",
        lines(&size.committed, palette),
        files(size.committed.files),
        size.default_branch
    );
    if size.uncommitted.is_empty() {
        return committed;
    }
    format!(
        "{committed} · {} uncommitted",
        lines(&size.uncommitted, palette)
    )
}

fn lines(stat: &DiffStat, palette: &Palette) -> String {
    let r = palette.reset;
    format!(
        "{}+{}{r} {}-{}{r}",
        palette.trend_up, stat.added, palette.trend_down, stat.removed
    )
}

fn files(count: u64) -> String {
    match count {
        1 => "1 file".to_string(),
        n => format!("{n} files"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segments::names;

    fn plain() -> Palette {
        Palette::for_theme("plain")
    }

    fn alert_above(lines: u64) -> BranchSizeConfig {
        BranchSizeConfig {
            alert_above_lines: Some(lines),
            ..Default::default()
        }
    }

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

        let rendered = render_branch_size_segment(
            Some(&size),
            "feature",
            &BranchSizeConfig::default(),
            &plain(),
        );

        assert_eq!(rendered.as_deref(), Some("+420 -80 · 12 files vs main"));
    }

    #[test]
    fn test_one_changed_file_is_singular() {
        let size = size(3, 0, 1);

        let rendered = render_branch_size_segment(
            Some(&size),
            "feature",
            &BranchSizeConfig::default(),
            &plain(),
        );

        assert_eq!(rendered.as_deref(), Some("+3 -0 · 1 file vs main"));
    }

    #[test]
    fn test_adds_the_uncommitted_lines() {
        let size = with_uncommitted(size(420, 80, 12), 35, 2, 3);

        let rendered = render_branch_size_segment(
            Some(&size),
            "feature",
            &BranchSizeConfig::default(),
            &plain(),
        );

        assert_eq!(
            rendered.as_deref(),
            Some("+420 -80 · 12 files vs main · +35 -2 uncommitted")
        );
    }

    #[test]
    fn test_an_uncommitted_binary_file_shows_no_lines() {
        let size = with_uncommitted(size(4, 1, 1), 0, 0, 1);

        let rendered = render_branch_size_segment(
            Some(&size),
            "feature",
            &BranchSizeConfig::default(),
            &plain(),
        );

        assert_eq!(
            rendered.as_deref(),
            Some("+4 -1 · 1 file vs main · +0 -0 uncommitted")
        );
    }

    #[test]
    fn test_colors_the_added_lines_green_and_the_removed_lines_red() {
        let size = size(420, 80, 12);

        for theme in ["github", "nord", "tokyo-night"] {
            let rendered = render_branch_size_segment(
                Some(&size),
                "feature",
                &BranchSizeConfig::default(),
                &Palette::for_theme(theme),
            );

            assert_eq!(
                rendered.as_deref(),
                Some("\x1b[92m+420\x1b[0m \x1b[91m-80\x1b[0m · 12 files vs main"),
                "{theme}"
            );
        }
    }

    #[test]
    fn test_colors_the_counts_blue_and_orange_in_the_colorblind_theme() {
        let size = size(420, 80, 12);

        let rendered = render_branch_size_segment(
            Some(&size),
            "feature",
            &BranchSizeConfig::default(),
            &Palette::for_theme("colorblind"),
        );

        assert_eq!(
            rendered.as_deref(),
            Some("\x1b[94m+420\x1b[0m \x1b[38;5;208m-80\x1b[0m · 12 files vs main")
        );
    }

    #[test]
    fn test_colors_the_uncommitted_counts_like_the_committed_ones() {
        let size = with_uncommitted(size(420, 80, 12), 35, 2, 3);

        let rendered = render_branch_size_segment(
            Some(&size),
            "feature",
            &BranchSizeConfig::default(),
            &Palette::for_theme("github"),
        );

        assert_eq!(
            rendered.as_deref(),
            Some(
                "\x1b[92m+420\x1b[0m \x1b[91m-80\x1b[0m · 12 files vs main · \
                 \x1b[92m+35\x1b[0m \x1b[91m-2\x1b[0m uncommitted"
            )
        );
    }

    #[test]
    fn test_colors_a_zero_count_like_any_other() {
        let size = with_uncommitted(size(3, 0, 1), 0, 2, 1);

        let rendered = render_branch_size_segment(
            Some(&size),
            "feature",
            &BranchSizeConfig::default(),
            &Palette::for_theme("github"),
        );

        assert_eq!(
            rendered.as_deref(),
            Some(
                "\x1b[92m+3\x1b[0m \x1b[91m-0\x1b[0m · 1 file vs main · \
                 \x1b[92m+0\x1b[0m \x1b[91m-2\x1b[0m uncommitted"
            )
        );
    }

    #[test]
    fn test_hidden_without_changes() {
        let size = size(0, 0, 0);

        let rendered = render_branch_size_segment(
            Some(&size),
            "feature",
            &BranchSizeConfig::default(),
            &plain(),
        );

        assert_eq!(rendered, None);
    }

    #[test]
    fn test_hidden_on_the_default_branch() {
        let size = size(3, 0, 1);

        let rendered =
            render_branch_size_segment(Some(&size), "main", &BranchSizeConfig::default(), &plain());

        assert_eq!(rendered, None);
    }

    #[test]
    fn test_alert_color_above_the_line_limit() {
        let size = with_uncommitted(size(3, 1, 1), 1, 0, 1);
        let p = Palette::for_theme("github");

        let rendered = render_branch_size_segment(Some(&size), "feature", &alert_above(4), &p);

        assert_eq!(
            rendered,
            Some(format!(
                "{}+3 -1 · 1 file vs main · +1 -0 uncommitted{}",
                p.tokens_alert, p.reset
            ))
        );
    }

    #[test]
    fn test_no_alert_color_at_the_line_limit() {
        let size = with_uncommitted(size(3, 1, 1), 1, 0, 1);

        let rendered = render_branch_size_segment(
            Some(&size),
            "feature",
            &alert_above(5),
            &Palette::for_theme("github"),
        );

        assert_eq!(
            rendered.as_deref(),
            Some(
                "\x1b[92m+3\x1b[0m \x1b[91m-1\x1b[0m · 1 file vs main · \
                 \x1b[92m+1\x1b[0m \x1b[91m-0\x1b[0m uncommitted"
            )
        );
    }

    #[test]
    fn test_no_alert_color_without_a_line_limit() {
        let size = size(100_000, 0, 1);

        let rendered = render_branch_size_segment(
            Some(&size),
            "feature",
            &BranchSizeConfig::default(),
            &Palette::for_theme("github"),
        );

        assert_eq!(
            rendered.as_deref(),
            Some("\x1b[92m+100000\x1b[0m \x1b[91m-0\x1b[0m · 1 file vs main")
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
