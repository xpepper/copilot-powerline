use crate::config::{CacheExpiryConfig, IconSet};
use crate::icons::cache_expiry_icon;
use crate::input::ContextWindow;
use crate::segments::tokens::format_tokens;
use crate::theme::Palette;

/// Renders the idle cache-expiry warning: the context size the next turn
/// will rewrite uncached, plus a hint. `idle_seconds` is the time since the
/// session's last step (see `idle`), `None` when unknown.
pub fn render_cache_expiry_segment(
    ctx: &ContextWindow,
    idle_seconds: Option<u64>,
    config: &CacheExpiryConfig,
    icon_set: IconSet,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled || idle_seconds? < config.ttl_seconds {
        return None;
    }

    let context_tokens = ctx.current_context_tokens.unwrap_or(0);
    if context_tokens < config.min_tokens {
        return None;
    }

    let icon = cache_expiry_icon(icon_set, config.prefix.as_deref());
    let r = palette.reset;
    let hint = if config.hint.is_empty() {
        String::new()
    } else {
        format!("{} · {}{}", palette.dim, config.hint, r)
    };

    Some(format!(
        "{}{}{} {}~{} uncached{}{}",
        palette.label,
        icon,
        r,
        palette.tokens_alert,
        format_tokens(Some(context_tokens)),
        r,
        hint
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_with(tokens: u64) -> ContextWindow {
        ContextWindow {
            current_context_tokens: Some(tokens),
            ..Default::default()
        }
    }

    fn render(ctx: &ContextWindow, idle: Option<u64>, cfg: &CacheExpiryConfig) -> Option<String> {
        render_cache_expiry_segment(ctx, idle, cfg, IconSet::Plain, &Palette::for_theme("plain"))
    }

    #[test]
    fn test_warns_once_idle_past_the_ttl() {
        let cfg = CacheExpiryConfig::default();
        assert_eq!(
            render(&ctx_with(92_000), Some(300), &cfg).as_deref(),
            Some("Idle: ~92k uncached · /clear to start fresh")
        );
    }

    #[test]
    fn test_quiet_while_the_cache_is_warm() {
        let cfg = CacheExpiryConfig::default();
        assert_eq!(render(&ctx_with(92_000), Some(299), &cfg), None);
    }

    #[test]
    fn test_quiet_when_idle_time_is_unknown() {
        let cfg = CacheExpiryConfig::default();
        assert_eq!(render(&ctx_with(92_000), None, &cfg), None);
    }

    #[test]
    fn test_quiet_below_the_token_floor() {
        let cfg = CacheExpiryConfig::default();
        assert_eq!(render(&ctx_with(49_999), Some(3_600), &cfg), None);
        assert!(render(&ctx_with(50_000), Some(3_600), &cfg).is_some());
    }

    #[test]
    fn test_quiet_when_disabled() {
        let cfg = CacheExpiryConfig {
            enabled: false,
            ..Default::default()
        };
        assert_eq!(render(&ctx_with(92_000), Some(3_600), &cfg), None);
    }

    #[test]
    fn test_custom_ttl_and_floor() {
        let cfg = CacheExpiryConfig {
            ttl_seconds: 3_600,
            min_tokens: 10_000,
            ..Default::default()
        };
        assert_eq!(render(&ctx_with(20_000), Some(600), &cfg), None);
        assert!(render(&ctx_with(20_000), Some(3_600), &cfg).is_some());
    }

    #[test]
    fn test_empty_hint_shows_only_the_size() {
        let cfg = CacheExpiryConfig {
            hint: String::new(),
            ..Default::default()
        };
        assert_eq!(
            render(&ctx_with(92_000), Some(300), &cfg).as_deref(),
            Some("Idle: ~92k uncached")
        );
    }

    #[test]
    fn test_emoji_icon_and_theme_colors() {
        let p = Palette::for_theme("github");
        let rendered = render_cache_expiry_segment(
            &ctx_with(92_000),
            Some(300),
            &CacheExpiryConfig::default(),
            IconSet::Emoji,
            &p,
        )
        .unwrap();
        assert!(rendered.contains("⏳"));
        assert!(rendered.contains(&format!("{}~92k uncached{}", p.tokens_alert, p.reset)));
        assert!(rendered.contains(&format!("{} · /clear to start fresh{}", p.dim, p.reset)));
    }
}
