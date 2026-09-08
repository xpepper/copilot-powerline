use crate::config::{IconSet, ReasoningConfig};
use crate::icons::reasoning_icon;
use crate::input::ContextWindow;
use crate::segments::tokens::format_tokens;
use crate::theme::Palette;

pub fn render_reasoning_segment(
    ctx: &ContextWindow,
    config: &ReasoningConfig,
    icon_set: IconSet,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }

    let reasoning = ctx.total_reasoning_tokens.unwrap_or(0);

    if config.auto_hide_zero && reasoning == 0 {
        return None;
    }

    let icon = reasoning_icon(icon_set, config.prefix.as_deref());
    let r = palette.reset;
    let lbl = palette.label;
    let val_str = format_tokens(Some(reasoning));

    Some(format!(
        "{}{}{} {}{}{}",
        lbl, icon, r, palette.tokens_normal, val_str, r
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_hidden_when_zero() {
        let ctx = ContextWindow::default();
        let cfg = ReasoningConfig::default();
        let p = Palette::for_theme("plain");

        assert!(render_reasoning_segment(&ctx, &cfg, IconSet::Plain, &p).is_none());
    }

    #[test]
    fn test_reasoning_plain() {
        let ctx = ContextWindow {
            total_reasoning_tokens: Some(3_200),
            ..Default::default()
        };
        let cfg = ReasoningConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_reasoning_segment(&ctx, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Think: 3.2k");
    }

    #[test]
    fn test_reasoning_emoji() {
        let ctx = ContextWindow {
            total_reasoning_tokens: Some(4_500),
            ..Default::default()
        };
        let cfg = ReasoningConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_reasoning_segment(&ctx, &cfg, IconSet::Emoji, &p).unwrap();
        assert_eq!(rendered, "🧠 4.5k");
    }

    #[test]
    fn test_reasoning_nerd() {
        let ctx = ContextWindow {
            total_reasoning_tokens: Some(12_000),
            ..Default::default()
        };
        let cfg = ReasoningConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_reasoning_segment(&ctx, &cfg, IconSet::Nerd, &p).unwrap();
        assert_eq!(rendered, "󰚩 12k");
    }
}
