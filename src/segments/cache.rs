use crate::config::{CacheConfig, IconSet};
use crate::icons::cache_icon;
use crate::input::ContextWindow;
use crate::segments::tokens::format_tokens;
use crate::theme::Palette;

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

    let value_str = if config.show_as_percentage {
        let total_in = ctx.total_input_tokens.unwrap_or(0);
        let denom = if total_in > 0 {
            total_in
        } else {
            cache_read + cache_write
        };

        if denom > 0 {
            let pct = ((cache_read as f64 / denom as f64) * 100.0).clamp(0.0, 100.0);
            format!("{:.0}%", pct)
        } else {
            "0%".to_string()
        }
    } else {
        format_tokens(Some(cache_read))
    };

    Some(format!(
        "{}{}{} {}{}{}",
        lbl, icon, r, palette.tokens_normal, value_str, r
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
}
