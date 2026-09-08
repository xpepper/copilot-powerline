use crate::config::{IconSet, TotalTokensConfig};
use crate::icons::total_tokens_icon;
use crate::input::ContextWindow;
use crate::segments::tokens::format_tokens;
use crate::theme::Palette;

pub fn render_total_tokens_segment(
    ctx: &ContextWindow,
    config: &TotalTokensConfig,
    icon_set: IconSet,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }

    let total = ctx.total_tokens.unwrap_or_else(|| {
        let input = ctx.total_input_tokens.unwrap_or(0);
        let output = ctx.total_output_tokens.unwrap_or(0);
        input + output
    });

    let icon = total_tokens_icon(icon_set, config.prefix.as_deref());
    let r = palette.reset;
    let lbl = palette.label;
    let val_str = format_tokens(Some(total));

    Some(format!(
        "{}{}{} {}{}{}",
        lbl, icon, r, palette.tokens_normal, val_str, r
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_tokens_direct() {
        let ctx = ContextWindow {
            total_tokens: Some(1_500_000),
            ..Default::default()
        };
        let cfg = TotalTokensConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_total_tokens_segment(&ctx, &cfg, IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Total: 1.5M");
    }

    #[test]
    fn test_total_tokens_fallback_sum() {
        let ctx = ContextWindow {
            total_input_tokens: Some(50_000),
            total_output_tokens: Some(15_000),
            ..Default::default()
        };
        let cfg = TotalTokensConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_total_tokens_segment(&ctx, &cfg, IconSet::Emoji, &p).unwrap();
        assert_eq!(rendered, "📊 65k");
    }

    #[test]
    fn test_total_tokens_nerd() {
        let ctx = ContextWindow {
            total_tokens: Some(250_000),
            ..Default::default()
        };
        let cfg = TotalTokensConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_total_tokens_segment(&ctx, &cfg, IconSet::Nerd, &p).unwrap();
        assert_eq!(rendered, "󰓅 250k");
    }
}
