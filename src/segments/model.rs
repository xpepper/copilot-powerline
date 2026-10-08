use crate::config::{IconSet, ModelConfig};
use crate::icons::model_icon;
use crate::input::ModelInfo;
use crate::theme::Palette;

pub fn render_model_segment(
    model: Option<&ModelInfo>,
    config: &ModelConfig,
    icon_set: IconSet,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }

    let model = model?;
    let non_blank = |s: &Option<String>| {
        s.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    let name = non_blank(&model.display_name).or_else(|| non_blank(&model.id))?;

    let icon = model_icon(icon_set, config.prefix.as_deref());
    let r = palette.reset;

    Some(format!(
        "{}{}{} {}{}{}",
        palette.label, icon, r, palette.tokens_normal, name, r
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: Option<&str>, display_name: Option<&str>) -> ModelInfo {
        ModelInfo {
            id: id.map(String::from),
            display_name: display_name.map(String::from),
        }
    }

    #[test]
    fn test_model_shows_display_name() {
        let m = model(Some("claude-sonnet-4.5"), Some("Claude Sonnet 4.5"));
        let p = Palette::for_theme("plain");

        let rendered =
            render_model_segment(Some(&m), &ModelConfig::default(), IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Model: Claude Sonnet 4.5");
    }

    #[test]
    fn test_model_shows_auto_routing_target() {
        let m = model(Some("auto"), Some("Auto → Claude Opus 4.5"));
        let p = Palette::for_theme("plain");

        let rendered =
            render_model_segment(Some(&m), &ModelConfig::default(), IconSet::Emoji, &p).unwrap();
        assert_eq!(rendered, "🤖 Auto → Claude Opus 4.5");
    }

    #[test]
    fn test_model_falls_back_to_id() {
        let m = model(Some("gpt-5"), Some("  "));
        let p = Palette::for_theme("plain");

        let rendered =
            render_model_segment(Some(&m), &ModelConfig::default(), IconSet::Plain, &p).unwrap();
        assert_eq!(rendered, "Model: gpt-5");
    }

    #[test]
    fn test_model_hidden_when_unknown() {
        let p = Palette::for_theme("plain");
        let cfg = ModelConfig::default();

        assert!(render_model_segment(None, &cfg, IconSet::Plain, &p).is_none());
        assert!(render_model_segment(Some(&model(None, None)), &cfg, IconSet::Plain, &p).is_none());
    }

    #[test]
    fn test_model_disabled() {
        let m = model(Some("gpt-5"), None);
        let cfg = ModelConfig {
            enabled: false,
            ..Default::default()
        };
        let p = Palette::for_theme("plain");

        assert!(render_model_segment(Some(&m), &cfg, IconSet::Plain, &p).is_none());
    }
}
