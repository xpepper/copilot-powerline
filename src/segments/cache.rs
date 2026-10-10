use super::is_listed;
use crate::cache_trend::Trend;
use crate::config::{CacheConfig, IconSet};
use crate::icons::cache_icon;
use crate::input::ContextWindow;
use crate::segments::tokens::format_tokens;
use crate::theme::Palette;

/// Whether the trend arrow needs the per-session hit rate snapshot: the
/// segment is listed and `show_trend` is on. Like it always has, this ignores
/// `enabled`, so a disabled but listed segment still records snapshots.
pub fn needs_trend(segments: &[String], config: &CacheConfig) -> bool {
    config.show_trend && is_listed(segments, "cache")
}

fn hit_rate(cache_read: u64, input: u64) -> Option<f64> {
    (input > 0).then(|| ((cache_read as f64 / input as f64) * 100.0).clamp(0.0, 100.0))
}

/// Renders the session cache hit rate. `trend` is the latest step's
/// direction (see `cache_trend`), shown only in percentage mode.
pub fn render_cache_segment(
    ctx: &ContextWindow,
    trend: Option<Trend>,
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
        (pct, trend.filter(|_| config.show_trend))
    } else {
        (format_tokens(Some(cache_read)), None)
    };

    let (color, arrow) = match trend {
        Some(Trend::Up) => (palette.trend_up, " ↑"),
        Some(Trend::Down) => (palette.trend_down, " ↓"),
        None => (palette.tokens_normal, ""),
    };

    Some(format!(
        "{}{}{} {}{}{}{}",
        lbl, icon, r, color, value_str, arrow, r
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hidden_when_zero() {
        let ctx = ContextWindow::default();
        let cfg = CacheConfig::default();
        let p = Palette::for_theme("plain");

        assert!(render_cache_segment(&ctx, None, &cfg, IconSet::Plain, &p).is_none());
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

        let rendered = render_cache_segment(&ctx, None, &cfg, IconSet::Plain, &p).unwrap();
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

        let rendered = render_cache_segment(&ctx, None, &cfg, IconSet::Emoji, &p).unwrap();
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

        let rendered = render_cache_segment(&ctx, None, &cfg, IconSet::Plain, &p).unwrap();
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
            Some(Trend::Up),
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
            Some(Trend::Down),
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
            None,
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
            Some(Trend::Down),
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
            Some(Trend::Down),
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
            Some(Trend::Down),
            &CacheConfig::default(),
            IconSet::Plain,
            &p,
        )
        .unwrap();
        assert!(rendered.contains(&format!("{}60% ↓{}", p.trend_down, p.reset)));
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn test_trend_needs_the_flag_and_a_listed_segment() {
        let cfg = CacheConfig::default();
        assert!(needs_trend(&names(&["cache"]), &cfg));
        assert!(!needs_trend(&names(&["tokens"]), &cfg));
        let off = CacheConfig {
            show_trend: false,
            ..Default::default()
        };
        assert!(!needs_trend(&names(&["cache"]), &off));
    }

    #[test]
    fn test_trend_ignores_enabled() {
        // Gated on `show_trend` only, as it was in main.
        let cfg = CacheConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(needs_trend(&names(&["cache"]), &cfg));
    }
}
