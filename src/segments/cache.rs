use super::is_listed;
use crate::cache_trend::{Signals, Trend};
use crate::config::{CacheConfig, IconSet};
use crate::icons::cache_icon;
use crate::input::ContextWindow;
use crate::segments::tokens::format_tokens;
use crate::theme::Palette;

/// The name that lists this segment in `segments` and `compact_segments`.
pub const NAME: &str = "cache";

/// Whether the segment needs the per-session snapshot: it is listed and
/// shows the trend arrow or the last miss. Like it always has, this ignores
/// `enabled`, so a disabled but listed segment still records snapshots.
pub fn needs_snapshot(segments: &[String], config: &CacheConfig) -> bool {
    (config.show_trend || config.show_last_miss) && is_listed(segments, NAME)
}

fn hit_rate(cache_read: u64, input: u64) -> Option<f64> {
    (input > 0).then(|| ((cache_read as f64 / input as f64) * 100.0).clamp(0.0, 100.0))
}

/// Renders the session cache hit rate. `signals.trend` is the latest step's
/// direction (see `cache_trend`), shown only in percentage mode.
/// `signals.recent_miss` is the size of a recent large cache miss, shown in
/// both modes.
pub fn render_cache_segment(
    ctx: &ContextWindow,
    signals: Signals,
    config: &CacheConfig,
    icon_set: IconSet,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }

    let cache_read = ctx.total_cache_read_tokens.unwrap_or(0);
    let cache_write = ctx.total_cache_write_tokens.unwrap_or(0);

    if config.auto_hide_zero && cache_read == 0 && cache_write == 0 {
        return None;
    }

    let icon = cache_icon(icon_set, config.prefix.as_deref());
    let r = palette.reset;
    let lbl = palette.label;

    let (value_str, trend) = if config.show_as_percentage {
        let total_in = ctx.total_input_tokens.unwrap_or(0);
        let denom = if total_in > 0 {
            total_in
        } else {
            cache_read + cache_write
        };

        let pct = hit_rate(cache_read, denom).map_or("0%".to_string(), |p| format!("{:.0}%", p));
        (pct, signals.trend.filter(|_| config.show_trend))
    } else {
        (format_tokens(Some(cache_read)), None)
    };

    let (color, arrow) = match trend {
        Some(Trend::Up) => (palette.trend_up, " ↑"),
        Some(Trend::Down) => (palette.trend_down, " ↓"),
        None => (palette.tokens_normal, ""),
    };
    let miss = signals
        .recent_miss
        .filter(|_| config.show_last_miss)
        .map_or(String::new(), |tokens| {
            format!(
                " · {}miss {}{}",
                palette.trend_down,
                format_tokens(Some(tokens)),
                r
            )
        });

    Some(format!(
        "{}{}{} {}{}{}{}{}",
        lbl, icon, r, color, value_str, arrow, r, miss
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segments::names;

    fn signals(trend: Option<Trend>, recent_miss: Option<u64>) -> Signals {
        Signals { trend, recent_miss }
    }

    #[test]
    fn test_cache_hidden_when_zero() {
        let ctx = ContextWindow::default();
        let cfg = CacheConfig::default();
        let p = Palette::for_theme("plain");

        assert!(
            render_cache_segment(&ctx, signals(None, None), &cfg, IconSet::Plain, &p).is_none()
        );
    }

    #[test]
    fn test_cache_percentage_plain() {
        let ctx = ContextWindow {
            total_cache_read_tokens: Some(85_000),
            total_input_tokens: Some(100_000),
            ..Default::default()
        };
        let cfg = CacheConfig::default();
        let p = Palette::for_theme("plain");

        let rendered =
            render_cache_segment(&ctx, signals(None, None), &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Cache: 85%");
    }

    #[test]
    fn test_cache_emoji_icon() {
        let ctx = ContextWindow {
            total_cache_read_tokens: Some(85_000),
            total_input_tokens: Some(100_000),
            ..Default::default()
        };
        let cfg = CacheConfig::default();
        let p = Palette::for_theme("plain");

        let rendered =
            render_cache_segment(&ctx, signals(None, None), &cfg, IconSet::Emoji, &p).unwrap();
        assert_eq!(rendered, "⚡ 85%");
    }

    #[test]
    fn test_cache_token_count_mode() {
        let ctx = ContextWindow {
            total_cache_read_tokens: Some(85_000),
            total_input_tokens: Some(100_000),
            ..Default::default()
        };
        let cfg = CacheConfig {
            show_as_percentage: false,
            ..Default::default()
        };
        let p = Palette::for_theme("plain");

        let rendered =
            render_cache_segment(&ctx, signals(None, None), &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Cache: 85k");
    }

    fn ctx_at_60_pct() -> ContextWindow {
        ContextWindow {
            total_cache_read_tokens: Some(60_000),
            total_input_tokens: Some(100_000),
            ..Default::default()
        }
    }

    #[test]
    fn test_cache_trend_up() {
        let p = Palette::for_theme("plain");
        let rendered = render_cache_segment(
            &ctx_at_60_pct(),
            signals(Some(Trend::Up), None),
            &CacheConfig::default(),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Cache: 60% ↑");
    }

    #[test]
    fn test_cache_trend_down() {
        let p = Palette::for_theme("plain");
        let rendered = render_cache_segment(
            &ctx_at_60_pct(),
            signals(Some(Trend::Down), None),
            &CacheConfig::default(),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Cache: 60% ↓");
    }

    #[test]
    fn test_cache_without_trend() {
        let p = Palette::for_theme("plain");
        let rendered = render_cache_segment(
            &ctx_at_60_pct(),
            signals(None, None),
            &CacheConfig::default(),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Cache: 60%");
    }

    #[test]
    fn test_cache_trend_can_be_disabled() {
        let cfg = CacheConfig {
            show_trend: false,
            ..Default::default()
        };
        let p = Palette::for_theme("plain");
        let rendered = render_cache_segment(
            &ctx_at_60_pct(),
            signals(Some(Trend::Down), None),
            &cfg,
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Cache: 60%");
    }

    #[test]
    fn test_cache_trend_not_shown_in_token_count_mode() {
        let cfg = CacheConfig {
            show_as_percentage: false,
            ..Default::default()
        };
        let p = Palette::for_theme("plain");
        let rendered = render_cache_segment(
            &ctx_at_60_pct(),
            signals(Some(Trend::Down), None),
            &cfg,
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Cache: 60k");
    }

    #[test]
    fn test_cache_trend_colors_value_with_theme() {
        let p = Palette::for_theme("github");
        let rendered = render_cache_segment(
            &ctx_at_60_pct(),
            signals(Some(Trend::Down), None),
            &CacheConfig::default(),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert!(rendered.contains(&format!("{}60% ↓{}", p.trend_down, p.reset)));
    }

    fn ctx_after_miss() -> ContextWindow {
        ContextWindow {
            total_cache_read_tokens: Some(2_016_000),
            total_input_tokens: Some(2_143_000),
            ..Default::default()
        }
    }

    #[test]
    fn test_recent_miss_shown_after_the_rate() {
        let p = Palette::for_theme("plain");
        let rendered = render_cache_segment(
            &ctx_after_miss(),
            signals(None, Some(133_000)),
            &CacheConfig::default(),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Cache: 94% · miss 133k");
    }

    #[test]
    fn test_recent_miss_follows_the_trend_arrow() {
        // The call after a miss is well cached: the arrow says so, and the
        // miss stays visible next to it.
        let p = Palette::for_theme("plain");
        let rendered = render_cache_segment(
            &ctx_after_miss(),
            signals(Some(Trend::Up), Some(133_000)),
            &CacheConfig::default(),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Cache: 94% ↑ · miss 133k");
    }

    #[test]
    fn test_recent_miss_colored_as_a_drop() {
        let p = Palette::for_theme("github");
        let rendered = render_cache_segment(
            &ctx_after_miss(),
            signals(None, Some(133_000)),
            &CacheConfig::default(),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert!(rendered.ends_with(&format!(" · {}miss 133k{}", p.trend_down, p.reset)));
    }

    #[test]
    fn test_recent_miss_shown_in_token_count_mode() {
        let cfg = CacheConfig {
            show_as_percentage: false,
            ..Default::default()
        };
        let p = Palette::for_theme("plain");
        let rendered = render_cache_segment(
            &ctx_after_miss(),
            signals(None, Some(133_000)),
            &cfg,
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Cache: 2.0M · miss 133k");
    }

    #[test]
    fn test_recent_miss_can_be_disabled() {
        let cfg = CacheConfig {
            show_last_miss: false,
            ..Default::default()
        };
        let p = Palette::for_theme("plain");
        let rendered = render_cache_segment(
            &ctx_after_miss(),
            signals(None, Some(133_000)),
            &cfg,
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert_eq!(rendered, "Cache: 94%");
    }

    #[test]
    fn test_snapshot_needs_a_flag_and_a_listed_segment() {
        let cfg = CacheConfig::default();
        assert!(needs_snapshot(&names(&["cache"]), &cfg));
        assert!(!needs_snapshot(&names(&["tokens"]), &cfg));
        let trend_only = CacheConfig {
            show_last_miss: false,
            ..Default::default()
        };
        assert!(needs_snapshot(&names(&["cache"]), &trend_only));
        let miss_only = CacheConfig {
            show_trend: false,
            ..Default::default()
        };
        assert!(needs_snapshot(&names(&["cache"]), &miss_only));
        let off = CacheConfig {
            show_trend: false,
            show_last_miss: false,
            ..Default::default()
        };
        assert!(!needs_snapshot(&names(&["cache"]), &off));
    }

    #[test]
    fn test_snapshot_ignores_enabled() {
        // Gated on the display flags only, as it was in main.
        let cfg = CacheConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(needs_snapshot(&names(&["cache"]), &cfg));
    }
}
