use crate::config::{IconSet, PrConfig};
use crate::github::PullRequestInfo;
use crate::icons::pr_icon;
use crate::theme::Palette;

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
    } else if config.hyperlinks && !pr.url.is_empty() {
        // OSC 8 terminal hyperlink with underline:
        // underline + color + OSC8-open + text + OSC8-close + reset
        format!(
            "\x1b[4m{}\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\{}",
            palette.pr, pr.url, pr_text, palette.reset
        )
    } else {
        format!("{}{}{}", palette.pr, pr_text, palette.reset)
    };

    Some(format!("{}{}", icon_part, formatted_pr))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let pr = PullRequestInfo {
            number: 50,
            url: "https://github.com/xpepper/copilot-powerline/pull/50".to_string(),
        };
        assert!(render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).is_none());
    }

    #[test]
    fn test_render_pr_plain_theme() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("plain");
        let pr = PullRequestInfo {
            number: 50,
            url: "https://github.com/xpepper/copilot-powerline/pull/50".to_string(),
        };
        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "PR #50");
    }

    #[test]
    fn test_render_pr_nerd_icon() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("plain");
        let pr = PullRequestInfo {
            number: 50,
            url: "https://github.com/xpepper/copilot-powerline/pull/50".to_string(),
        };
        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Nerd, &p).unwrap();
        assert_eq!(rendered, " #50");
    }

    #[test]
    fn test_render_pr_emoji_icon() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("plain");
        let pr = PullRequestInfo {
            number: 50,
            url: "https://github.com/xpepper/copilot-powerline/pull/50".to_string(),
        };
        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Emoji, &p).unwrap();
        assert_eq!(rendered, "🔀 #50");
    }

    #[test]
    fn test_render_pr_hyperlink_and_colors() {
        let cfg = PrConfig::default();
        let p = Palette::for_theme("colorblind");
        let pr = PullRequestInfo {
            number: 50,
            url: "https://github.com/xpepper/copilot-powerline/pull/50".to_string(),
        };
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
    fn test_render_pr_no_hyperlinks() {
        let cfg = PrConfig {
            hyperlinks: false,
            ..Default::default()
        };
        let p = Palette::for_theme("colorblind");
        let pr = PullRequestInfo {
            number: 50,
            url: "https://github.com/xpepper/copilot-powerline/pull/50".to_string(),
        };
        let rendered = render_pr_segment(Some(&pr), &cfg, IconSet::Plain, &p).unwrap();

        // Must NOT contain OSC 8 sequence.
        assert!(!rendered.contains("\x1b]8;;"));
        assert!(rendered.contains("#50"));
    }
}
