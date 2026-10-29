use crate::config::{FusionConfig, IconSet};
use crate::events::FusionRouting;
use crate::icons::fusion_icon;
use crate::theme::Palette;

pub const NAME: &str = "fusion";

pub fn render_fusion_segment(
    routing: Option<&FusionRouting>,
    config: &FusionConfig,
    icon_set: IconSet,
    palette: &Palette,
) -> Option<String> {
    if !config.enabled {
        return None;
    }
    let routing = routing?;
    let icon = fusion_icon(icon_set, config.prefix.as_deref());
    let r = palette.reset;
    let mut models = routing
        .observed
        .iter()
        .map(|(phase, model)| format!("{phase}={model}"))
        .collect::<Vec<_>>();
    if models.is_empty()
        && let Some(model) = &routing.selected_model
    {
        models.push(format!("selected={model}"));
    }
    if let Some(model) = &routing.final_model {
        models.push(format!("final={model}"));
    } else if let Some(model) = &routing.handoff_model {
        models.push(format!("handoff={model}"));
    }
    let models = if models.is_empty() {
        String::new()
    } else {
        format!(": {}", models.join(", "))
    };
    let state = routing.outcome.as_deref().unwrap_or_else(|| {
        if routing.handoff_model.is_some() {
            "handoff"
        } else if routing.observed.is_empty() {
            "planned"
        } else {
            "observed"
        }
    });
    let escalated = if routing.outcome.is_some() && routing.handoff_model.is_some() {
        ", escalated"
    } else {
        ""
    };
    Some(format!(
        "{}{icon}{r} {}{}{models} ({state}{escalated}){r}",
        palette.label, palette.tokens_normal, routing.pattern
    ))
}
