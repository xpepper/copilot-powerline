use crate::config::TokensConfig;
use crate::input::ContextWindow;
use crate::theme::Palette;

pub fn format_tokens(n: Option<u64>) -> String {
    let count = match n {
        Some(v) if v > 0 => v,
        _ => return "0".to_string(),
    };

    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 10_000 {
        format!("{:.0}k", count as f64 / 1_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

pub fn render_tokens_segment(
    ctx: &ContextWindow,
    config: &TokensConfig,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }

    let r = palette.reset;
    let d = palette.dim;
    let lbl = palette.label;

    let curr_tokens = ctx.current_context_tokens.unwrap_or(0);
    let max_tokens = ctx.displayed_context_limit.unwrap_or(0);
    let pct = ctx.current_context_used_percentage;

    let tok_curr_str = format_tokens(Some(curr_tokens));
    let tok_max_str = format_tokens(Some(max_tokens));

    let is_alert = curr_tokens > config.alert_threshold;
    let (alert_icon, curr_styled) = if is_alert {
        (
            config.alert_icon.as_str(),
            format!("{}{}{}", palette.tokens_alert, tok_curr_str, r),
        )
    } else {
        (
            "",
            format!("{}{}{}", palette.tokens_normal, tok_curr_str, r),
        )
    };

    let max_styled = format!("{}{}{}", d, tok_max_str, r);
    let pct_styled = if config.show_percentage {
        pct.map(|p| format!(" ({}{}%{})", d, p, r))
            .unwrap_or_default()
    } else {
        String::new()
    };

    Some(format!(
        "{}Tokens:{} {}{}/{}{}",
        lbl, r, alert_icon, curr_styled, max_styled, pct_styled
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(None), "0");
        assert_eq!(format_tokens(Some(0)), "0");
        assert_eq!(format_tokens(Some(450)), "450");
        assert_eq!(format_tokens(Some(1_200)), "1.2k");
        assert_eq!(format_tokens(Some(10_000)), "10k");
        assert_eq!(format_tokens(Some(200_000)), "200k");
        assert_eq!(format_tokens(Some(1_500_000)), "1.5M");
    }

    #[test]
    fn test_render_tokens_normal() {
        let ctx = ContextWindow {
            current_context_tokens: Some(0),
            displayed_context_limit: Some(200_000),
            current_context_used_percentage: Some(0.0),
        };
        let cfg = TokensConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_tokens_segment(&ctx, &cfg, &p).unwrap();
        assert_eq!(rendered, "Tokens: 0/200k (0%)");
    }

    #[test]
    fn test_render_tokens_alert() {
        let ctx = ContextWindow {
            current_context_tokens: Some(150_000),
            displayed_context_limit: Some(200_000),
            current_context_used_percentage: Some(75.0),
        };
        let cfg = TokensConfig::default();
        let p = Palette::for_theme("plain");

        let rendered = render_tokens_segment(&ctx, &cfg, &p).unwrap();
        assert_eq!(rendered, "Tokens: ⚠️ 150k/200k (75%)");
    }
}
