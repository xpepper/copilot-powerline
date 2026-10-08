use crate::config::{CacheConfig, IconSet};
use crate::icons::cache_icon;
use crate::input::ContextWindow;
use crate::segments::tokens::format_tokens;
use crate::theme::Palette;

/// Percentage points the last call's hit rate must differ from the session
/// average before a trend is shown, so small fluctuations don't flicker.
const TREND_TOLERANCE_PCT: f64 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trend {
    Up,
    Down,
}

fn hit_rate(cache_read: u64, input: u64) -> Option<f64> {
    (input > 0).then(|| ((cache_read as f64 / input as f64) * 100.0).clamp(0.0, 100.0))
}

/// Compares the most recent call's cache hit rate with the session average.
fn last_call_trend(ctx: &ContextWindow, session_pct: f64) -> Option<Trend> {
    let usage = ctx.current_usage.as_ref()?;
    let last_pct = hit_rate(usage.cache_read_input_tokens?, usage.input_tokens?)?;
    let delta = last_pct - session_pct;
    if delta > TREND_TOLERANCE_PCT {
        Some(Trend::Up)
    } else if delta < -TREND_TOLERANCE_PCT {
        Some(Trend::Down)
    } else {
        None
    }
}

pub fn render_cache_segment(
    ctx: &ContextWindow,
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

        match hit_rate(cache_read, denom) {
            Some(pct) => {
                let trend = config
                    .show_trend
                    .then(|| last_call_trend(ctx, pct))
                    .flatten();
                (format!("{:.0}%", pct), trend)
            }
            None => ("0%".to_string(), None),
        }
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
    use crate::input::CurrentUsage;

    #[test]
    fn test_cache_hidden_when_zero() {
        let ctx = ContextWindow::default();
        let cfg = CacheConfig::default();
        let p = Palette::for_theme("plain");

        assert!(render_cache_segment(&ctx, &cfg, IconSet::Plain, &p).is_none());
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

        let rendered = render_cache_segment(&ctx, &cfg, IconSet::Plain, &p).unwrap();
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

        let rendered = render_cache_segment(&ctx, &cfg, IconSet::Emoji, &p).unwrap();
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

        let rendered = render_cache_segment(&ctx, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Cache: 85k");
    }

    fn ctx_with_last_call(last_input: u64, last_read: u64) -> ContextWindow {
        ContextWindow {
            total_cache_read_tokens: Some(60_000),
            total_input_tokens: Some(100_000),
            current_usage: Some(CurrentUsage {
                input_tokens: Some(last_input),
                cache_read_input_tokens: Some(last_read),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_cache_trend_up_when_last_call_beats_session_average() {
        let ctx = ctx_with_last_call(10_000, 9_000); // 90% vs 60% session
        let cfg = CacheConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_cache_segment(&ctx, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Cache: 60% ↑");
    }

    #[test]
    fn test_cache_trend_down_when_last_call_misses_cache() {
        let ctx = ctx_with_last_call(10_000, 1_000); // 10% vs 60% session
        let cfg = CacheConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_cache_segment(&ctx, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Cache: 60% ↓");
    }

    #[test]
    fn test_cache_trend_steady_within_tolerance() {
        let ctx = ctx_with_last_call(10_000, 6_200); // 62% vs 60% session
        let cfg = CacheConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_cache_segment(&ctx, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Cache: 60%");
    }

    #[test]
    fn test_cache_trend_absent_without_last_call_usage() {
        let ctx = ContextWindow {
            total_cache_read_tokens: Some(60_000),
            total_input_tokens: Some(100_000),
            ..Default::default()
        };
        let cfg = CacheConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_cache_segment(&ctx, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Cache: 60%");
    }

    #[test]
    fn test_cache_trend_can_be_disabled() {
        let ctx = ctx_with_last_call(10_000, 1_000);
        let cfg = CacheConfig {
            show_trend: false,
            ..Default::default()
        };
        let p = Palette::for_theme("plain");

        let rendered = render_cache_segment(&ctx, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Cache: 60%");
    }

    #[test]
    fn test_cache_trend_colors_value_with_theme() {
        let ctx = ctx_with_last_call(10_000, 1_000);
        let cfg = CacheConfig::default();
        let p = Palette::for_theme("github");

        let rendered = render_cache_segment(&ctx, &cfg, IconSet::Plain, &p).unwrap();
        assert!(rendered.contains(&format!("{}60% ↓{}", p.trend_down, p.reset)));
    }
}
