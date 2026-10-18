use super::is_listed;
use crate::config::{IconSet, PrConfig};
use crate::github::{PullRequestInfo, ReviewState};
use crate::icons::pr_icon;
use crate::theme::Palette;

/// The name that lists this segment in `segments` and `compact_segments`.
pub const NAME: &str = "pr";

/// Whether the segment will render, so callers can skip the PR lookup (a
/// disk cache read and a possible background `gh` refresh) when nothing
/// would show it.
pub fn is_visible(segments: &[String], config: &PrConfig) -> bool {
    config.enabled && is_listed(segments, NAME)
}

/// Returns true if `url` is safe to splice into a raw terminal escape
/// sequence: a `https://github.com` URL containing no control characters.
///
/// The URL comes from an on-disk cache (see `github::PrCacheEntry`) that a
/// local writer could tamper with, so it must never be trusted verbatim when
/// building an OSC 8 hyperlink.
fn is_safe_github_url(url: &str) -> bool {
    if url.contains(|c: char| c.is_control()) {
        return false;
    }

    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };

    rest == "github.com"
        || rest.starts_with("github.com/")
        || rest == "www.github.com"
        || rest.starts_with("www.github.com/")
}

pub fn render_pr_segment(
    pr_info: Option<&PullRequestInfo>,
    config: &PrConfig,
    icon_set: IconSet,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }

    let pr = pr_info?;

    let icon = pr_icon(icon_set, config.prefix.as_deref());
    let icon_part = if icon.is_empty() {
        String::new()
    } else {
        format!("{}{}{} ", palette.label, icon, palette.reset)
    };

    let pr_text = format!("#{}", pr.number);

    let formatted_pr = if palette.reset.is_empty() {
        // Plain theme without escape codes.
        pr_text
    } else if config.hyperlinks && is_safe_github_url(&pr.url) {
        // OSC 8 terminal hyperlink with underline:
        // underline + color + OSC8-open + text + OSC8-close + reset
        format!(
            "\x1b[4m{}\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\{}",
            palette.pr, pr.url, pr_text, palette.reset
        )
    } else {
        format!("{}{}{}", palette.pr, pr_text, palette.reset)
    };

    let mut rendered = format!("{}{}", icon_part, formatted_pr);
    for (text, color) in review_parts(pr, config, palette) {
        rendered.push_str(&format!(" · {}{}{}", color, text, palette.reset));
    }
    Some(rendered)
}

/// The Review State and Unresolved Threads shown after the number, each with
/// its colour: the alert colour for what needs action, the segment's own
/// colour otherwise.
fn review_parts(
    pr: &PullRequestInfo,
    config: &PrConfig,
    palette: &Palette,
) -> Vec<(String, &'static str)> {
    let mut parts = Vec::new();
    if let Some(state) = pr.review_state.filter(|_| config.show_review_state) {
        parts.push(match state {
            ReviewState::Draft => ("draft".to_string(), palette.pr),
            ReviewState::Approved => ("approved".to_string(), palette.pr),
            ReviewState::ChangesRequested => {
                ("changes requested".to_string(), palette.tokens_alert)
            }
        });
    }
    if let Some(threads) = pr
        .unresolved_threads
        .filter(|t| config.show_unresolved && (t.count > 0 || t.more))
    {
        // Only the first page of threads is counted, so with more pages the
        // count is a lower bound, and none on the first page says nothing.
        let count = match (threads.count, threads.more) {
            (0, _) => "?".to_string(),
            (count, true) => format!("{count}+"),
            (count, false) => count.to_string(),
        };
        parts.push((format!("{count} unresolved"), palette.tokens_alert));
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::UnresolvedThreads;
    use crate::segments::names;

    #[test]
    fn test_render_pr_none() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("plain");
        assert!(render_pr_segment(None, &cfg, IconSet::Plain, &p).is_none());
    }

    #[test]
    fn test_render_pr_disabled() {
        let cfg = PrConfig {
            enabled: false,
            ..Default::default()
        };
        let p = Palette::for_theme("plain");
        let pr = reviewed_pr(None, None);
        assert!(render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).is_none());
    }

    #[test]
    fn test_render_pr_plain_theme() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("plain");
        let pr = reviewed_pr(None, None);
        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "PR #50");
    }

    #[test]
    fn test_render_pr_nerd_icon() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("plain");
        let pr = reviewed_pr(None, None);
        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Nerd, &p).unwrap();
        assert_eq!(rendered, " #50");
    }

    #[test]
    fn test_render_pr_emoji_icon() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("plain");
        let pr = reviewed_pr(None, None);
        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Emoji, &p).unwrap();
        assert_eq!(rendered, "🔀 #50");
    }

    #[test]
    fn test_render_pr_hyperlink_and_colors() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("colorblind");
        let pr = reviewed_pr(None, None);
        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        // Must contain OSC 8 hyperlink sequence.
        assert!(rendered.contains(
            "\x1b]8;;https://github.com/xpepper/copilot-powerline/pull/50\x1b\\#50\x1b]8;;\x1b\\"
        ));
        // Must contain underline.
        assert!(rendered.contains("\x1b[4m"));
        // Must contain PR text.
        assert!(rendered.contains("PR"));
    }

    #[test]
    fn test_render_pr_rejects_malicious_url() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("colorblind");
        let pr = PullRequestInfo {
            number: 50,
            url: "https://github.com/org/repo/pull/50\x1b]52;c;ZXZpbA==\x07".to_string(),
            review_state: None,
            unresolved_threads: None,
        };
        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        // Must not carry the injected escape sequence through to stdout.
        assert!(!rendered.contains("\x1b]52"));
        // Falls back to non-hyperlinked text.
        assert!(!rendered.contains("\x1b]8;;"));
        assert!(rendered.contains("#50"));
    }

    #[test]
    fn test_render_pr_rejects_non_github_url() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("colorblind");
        let pr = PullRequestInfo {
            url: "https://evil.example.com/pull/50".to_string(),
            ..reviewed_pr(None, None)
        };
        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        assert!(!rendered.contains("\x1b]8;;"));
        assert!(rendered.contains("#50"));
    }

    #[test]
    fn test_render_pr_no_hyperlinks() {
        let cfg = PrConfig {
            hyperlinks: false,
            ..Default::default()
        };
        let p = Palette::for_theme("colorblind");
        let pr = reviewed_pr(None, None);
        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        // Must NOT contain OSC 8 sequence.
        assert!(!rendered.contains("\x1b]8;;"));
        assert!(rendered.contains("#50"));
    }

    fn reviewed_pr(
        review_state: Option<ReviewState>,
        unresolved: Option<(u32, bool)>,
    ) -> PullRequestInfo {
        PullRequestInfo {
            number: 50,
            url: "https://github.com/xpepper/copilot-powerline/pull/50".to_string(),
            review_state,
            unresolved_threads: unresolved.map(|(count, more)| UnresolvedThreads { count, more }),
        }
    }

    /// A config with both review switches on, as they are off by default.
    fn reviews_shown() -> PrConfig {
        PrConfig {
            show_review_state: true,
            show_unresolved: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_render_pr_shows_no_review_info_by_default() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("plain");
        let pr = reviewed_pr(Some(ReviewState::ChangesRequested), Some((3, false)));

        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        assert_eq!(rendered, "PR #50");
    }

    #[test]
    fn test_render_pr_review_state_and_unresolved_threads() {
        let cfg = reviews_shown();
        let p = Palette::for_theme("plain");
        let pr = reviewed_pr(Some(ReviewState::ChangesRequested), Some((3, false)));

        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        assert_eq!(rendered, "PR #50 · changes requested · 3 unresolved");
    }

    #[test]
    fn test_render_pr_alert_colour_only_on_what_needs_action() {
        let cfg = reviews_shown();
        let p = Palette::for_theme("colorblind");

        let changes = reviewed_pr(Some(ReviewState::ChangesRequested), Some((3, false)));
        let rendered = render_pr_segment(Some(&changes), &cfg, IconSet::Plain, &p).unwrap();
        assert!(rendered.contains(&format!(
            " · {}changes requested{}",
            p.tokens_alert, p.reset
        )));
        assert!(rendered.contains(&format!(" · {}3 unresolved{}", p.tokens_alert, p.reset)));

        let draft = reviewed_pr(Some(ReviewState::Draft), None);
        let rendered = render_pr_segment(Some(&draft), &cfg, IconSet::Plain, &p).unwrap();
        assert!(rendered.contains(&format!(" · {}draft{}", p.pr, p.reset)));

        let approved = reviewed_pr(Some(ReviewState::Approved), None);
        let rendered = render_pr_segment(Some(&approved), &cfg, IconSet::Plain, &p).unwrap();
        assert!(rendered.contains(&format!(" · {}approved{}", p.pr, p.reset)));
    }

    #[test]
    fn test_render_pr_hyperlink_stays_on_the_number() {
        let cfg = reviews_shown();
        let p = Palette::for_theme("colorblind");
        let pr = reviewed_pr(Some(ReviewState::Approved), Some((1, false)));

        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        assert!(rendered.contains("\x1b\\#50\x1b]8;;\x1b\\"));
        assert_eq!(rendered.matches("\x1b]8;;").count(), 2);
    }

    #[test]
    fn test_render_pr_unresolved_count_past_the_first_page_is_a_lower_bound() {
        let cfg = reviews_shown();
        let p = Palette::for_theme("plain");
        let pr = reviewed_pr(None, Some((3, true)));

        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        assert_eq!(rendered, "PR #50 · 3+ unresolved");
    }

    #[test]
    fn test_render_pr_unknown_count_when_only_later_pages_could_hold_unresolved_threads() {
        let cfg = reviews_shown();
        let p = Palette::for_theme("plain");
        let pr = reviewed_pr(None, Some((0, true)));

        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        assert_eq!(rendered, "PR #50 · ? unresolved");
    }

    #[test]
    fn test_render_pr_nothing_extra_without_reviews_or_open_threads() {
        let cfg = reviews_shown();
        let p = Palette::for_theme("plain");
        let pr = reviewed_pr(None, Some((0, false)));

        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        assert_eq!(rendered, "PR #50");
    }

    #[test]
    fn test_render_pr_switches_hide_review_state_and_unresolved_threads() {
        let p = Palette::for_theme("plain");
        let pr = reviewed_pr(Some(ReviewState::ChangesRequested), Some((3, false)));
        let render =
            |cfg: PrConfig| render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        let no_state = PrConfig {
            show_review_state: false,
            ..reviews_shown()
        };
        assert_eq!(render(no_state), "PR #50 · 3 unresolved");

        let no_threads = PrConfig {
            show_unresolved: false,
            ..reviews_shown()
        };
        assert_eq!(render(no_threads), "PR #50 · changes requested");
    }

    #[test]
    fn test_visible_when_listed_and_enabled() {
        let cfg = PrConfig::default();
        assert!(is_visible(&names(&["tokens", "pr"]), &cfg));
    }

    #[test]
    fn test_hidden_when_not_listed() {
        let cfg = PrConfig::default();
        assert!(!is_visible(&names(&["tokens"]), &cfg));
    }

    #[test]
    fn test_hidden_when_disabled() {
        let cfg = PrConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!is_visible(&names(&["pr"]), &cfg));
    }
}
