use super::is_listed;
use crate::config::{IconSet, ToolOverheadConfig};
use crate::events::UsageRecord;
use crate::icons::tool_overhead_icon;
use crate::input::ContextWindow;
use crate::segments::tokens::format_tokens;
use crate::theme::Palette;

/// The name that lists this segment in `segments` and `compact_segments`.
pub const NAME: &str = "tool_overhead";

/// Whether the segment will render, so callers can skip reading the event log
/// when nothing would show it.
pub fn is_visible(segments: &[String], config: &ToolOverheadConfig) -> bool {
    config.enabled && is_listed(segments, NAME)
}

/// Renders the size of the tool definitions, e.g. `Tools: 68k (34%)`, the
/// percent being their share of the context window. Without a usage record
/// (no event log yet, or no model call so far) there is nothing to show.
pub fn render_tool_overhead_segment(
    usage: Option<UsageRecord>,
    ctx: &ContextWindow,
    config: &ToolOverheadConfig,
    icon_set: IconSet,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }

    let tool_tokens = usage?.tool_tokens;
    if tool_tokens < config.min_tokens {
        return None;
    }

    let icon = tool_overhead_icon(icon_set, config.prefix.as_deref());
    let r = palette.reset;
    let share = ctx
        .displayed_context_limit
        .and_then(|limit| percent_of(tool_tokens, limit))
        .map(|pct| format!(" {}({pct}%){r}", palette.dim))
        .unwrap_or_default();

    Some(format!(
        "{}{}{} {}{}{}{}",
        palette.label,
        icon,
        r,
        palette.tokens_normal,
        format_tokens(Some(tool_tokens)),
        r,
        share
    ))
}

/// `part` as a whole percent of `whole`, rounding halves up; `None` for an
/// empty `whole`. Integer maths, so a half like 4.5% is not at the mercy of
/// float rounding.
fn percent_of(part: u64, whole: u64) -> Option<u64> {
    (whole > 0).then(|| (part * 100 + whole / 2) / whole)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segments::names;

    fn ctx_with_limit(limit: u64) -> ContextWindow {
        ContextWindow {
            displayed_context_limit: Some(limit),
            ..Default::default()
        }
    }

    fn usage(tool_tokens: u64) -> Option<UsageRecord> {
        Some(UsageRecord { tool_tokens })
    }

    fn render(
        usage: Option<UsageRecord>,
        ctx: &ContextWindow,
        cfg: &ToolOverheadConfig,
    ) -> Option<String> {
        render_tool_overhead_segment(
            usage,
            ctx,
            cfg,
            IconSet::Plain,
            &Palette::for_theme("plain"),
        )
    }

    #[test]
    fn test_shows_the_tool_tokens_and_their_share_of_the_window() {
        let cfg = ToolOverheadConfig::default();
        assert_eq!(
            render(usage(68_000), &ctx_with_limit(200_000), &cfg).as_deref(),
            Some("Tools: 68k (34%)")
        );
    }

    #[test]
    fn test_quiet_mode_hides_it_below_min_tokens() {
        let cfg = ToolOverheadConfig {
            min_tokens: 30_000,
            ..Default::default()
        };
        let ctx = ctx_with_limit(200_000);
        assert_eq!(render(usage(29_999), &ctx, &cfg), None);
        assert_eq!(
            render(usage(30_000), &ctx, &cfg).as_deref(),
            Some("Tools: 30k (15%)")
        );
    }

    #[test]
    fn test_quiet_when_disabled() {
        let cfg = ToolOverheadConfig {
            enabled: false,
            ..Default::default()
        };
        assert_eq!(render(usage(68_000), &ctx_with_limit(200_000), &cfg), None);
    }

    #[test]
    fn test_shows_a_low_count_too() {
        let cfg = ToolOverheadConfig::default();
        // 9k of 200k is 4.5%: halves round up.
        assert_eq!(
            render(usage(9_000), &ctx_with_limit(200_000), &cfg).as_deref(),
            Some("Tools: 9.0k (5%)")
        );
    }

    #[test]
    fn test_percent_is_a_whole_number_of_the_window() {
        let cfg = ToolOverheadConfig::default();
        assert_eq!(
            render(usage(116_000), &ctx_with_limit(200_000), &cfg).as_deref(),
            Some("Tools: 116k (58%)")
        );
        assert_eq!(
            render(usage(68_000), &ctx_with_limit(1_000_000), &cfg).as_deref(),
            Some("Tools: 68k (7%)")
        );
    }

    #[test]
    fn test_omits_the_percent_without_a_usable_limit() {
        let cfg = ToolOverheadConfig::default();
        assert_eq!(
            render(usage(68_000), &ContextWindow::default(), &cfg).as_deref(),
            Some("Tools: 68k")
        );
        assert_eq!(
            render(usage(68_000), &ctx_with_limit(0), &cfg).as_deref(),
            Some("Tools: 68k")
        );
    }

    #[test]
    fn test_quiet_without_a_usage_record() {
        let cfg = ToolOverheadConfig::default();
        assert_eq!(render(None, &ctx_with_limit(200_000), &cfg), None);
    }

    #[test]
    fn test_prefix_replaces_the_label() {
        let cfg = ToolOverheadConfig {
            prefix: Some("MCP:".to_string()),
            ..Default::default()
        };
        assert_eq!(
            render(usage(68_000), &ctx_with_limit(200_000), &cfg).as_deref(),
            Some("MCP: 68k (34%)")
        );
    }

    #[test]
    fn test_emoji_icon_and_theme_colors() {
        let p = Palette::for_theme("github");
        let rendered = render_tool_overhead_segment(
            usage(68_000),
            &ctx_with_limit(200_000),
            &ToolOverheadConfig::default(),
            IconSet::Emoji,
            &p,
        )
        .unwrap();
        assert!(rendered.contains("🧰"));
        assert!(rendered.contains(&format!("{}68k{}", p.tokens_normal, p.reset)));
        assert!(rendered.contains(&format!("{}(34%){}", p.dim, p.reset)));
    }

    #[test]
    fn test_visible_when_listed_and_enabled() {
        let cfg = ToolOverheadConfig::default();
        assert!(is_visible(&names(&["tokens", "tool_overhead"]), &cfg));
    }

    #[test]
    fn test_not_visible_when_unlisted_or_disabled() {
        let cfg = ToolOverheadConfig::default();
        assert!(!is_visible(&names(&["tokens"]), &cfg));
        let disabled = ToolOverheadConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!is_visible(&names(&["tool_overhead"]), &disabled));
    }
}
