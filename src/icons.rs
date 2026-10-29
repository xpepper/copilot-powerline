use crate::config::IconSet;

pub fn tokens_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "Tokens:",
        IconSet::Nerd => "󰮚",
        IconSet::Emoji => "🪙",
    }
}

pub fn session_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "Session:",
        IconSet::Nerd => "󰄬",
        IconSet::Emoji => "💰",
    }
}

pub fn month_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "Month:",
        IconSet::Nerd => "󰠠",
        IconSet::Emoji => "📅",
    }
}

pub fn cycle_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "Cycle:",
        IconSet::Nerd => "\u{f02a4}", // nf-md-github
        IconSet::Emoji => "🐙",
    }
}

pub fn cache_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "Cache:",
        IconSet::Nerd => "󰘸",
        IconSet::Emoji => "⚡",
    }
}

pub fn reasoning_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "Think:",
        IconSet::Nerd => "󰚩",
        IconSet::Emoji => "🧠",
    }
}

pub fn total_tokens_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "Total:",
        IconSet::Nerd => "󰓅",
        IconSet::Emoji => "📊",
    }
}

pub fn model_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "Model:",
        IconSet::Nerd => "\u{f061a}", // nf-md-chip
        IconSet::Emoji => "🤖",
    }
}

pub fn fusion_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "Fusion:",
        IconSet::Nerd => "\u{f061a}",
        IconSet::Emoji => "\u{1f500}",
    }
}

pub fn pr_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "PR",
        IconSet::Nerd => "",
        IconSet::Emoji => "🔀",
    }
}

pub fn cache_expiry_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "Idle:",
        IconSet::Nerd => "\u{f051f}", // nf-md-timer_sand
        IconSet::Emoji => "⏳",
    }
}

pub fn tool_overhead_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => "Tools:",
        IconSet::Nerd => "\u{f1064}", // nf-md-tools
        IconSet::Emoji => "🧰",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_set_plain() {
        assert_eq!(tokens_icon(IconSet::Plain, None), "Tokens:");
        assert_eq!(cache_icon(IconSet::Plain, None), "Cache:");
        assert_eq!(reasoning_icon(IconSet::Plain, None), "Think:");
        assert_eq!(total_tokens_icon(IconSet::Plain, None), "Total:");
        assert_eq!(pr_icon(IconSet::Plain, None), "PR");
        assert_eq!(model_icon(IconSet::Plain, None), "Model:");
        assert_eq!(cache_expiry_icon(IconSet::Plain, None), "Idle:");
        assert_eq!(tool_overhead_icon(IconSet::Plain, None), "Tools:");
    }

    #[test]
    fn test_icon_set_nerd() {
        assert_eq!(tokens_icon(IconSet::Nerd, None), "󰮚");
        assert_eq!(cache_icon(IconSet::Nerd, None), "󰘸");
        assert_eq!(reasoning_icon(IconSet::Nerd, None), "󰚩");
        assert_eq!(total_tokens_icon(IconSet::Nerd, None), "󰓅");
        assert_eq!(pr_icon(IconSet::Nerd, None), "");
        assert_eq!(tool_overhead_icon(IconSet::Nerd, None), "\u{f1064}");
    }

    #[test]
    fn test_icon_set_emoji() {
        assert_eq!(tokens_icon(IconSet::Emoji, None), "🪙");
        assert_eq!(session_icon(IconSet::Emoji, None), "💰");
        assert_eq!(month_icon(IconSet::Emoji, None), "📅");
        assert_eq!(cache_icon(IconSet::Emoji, None), "⚡");
        assert_eq!(reasoning_icon(IconSet::Emoji, None), "🧠");
        assert_eq!(total_tokens_icon(IconSet::Emoji, None), "📊");
        assert_eq!(pr_icon(IconSet::Emoji, None), "🔀");
        assert_eq!(model_icon(IconSet::Emoji, None), "🤖");
        assert_eq!(cache_expiry_icon(IconSet::Emoji, None), "⏳");
        assert_eq!(tool_overhead_icon(IconSet::Emoji, None), "🧰");
    }

    #[test]
    fn test_custom_override() {
        assert_eq!(
            cache_icon(IconSet::Emoji, Some("CustomCache:")),
            "CustomCache:"
        );
        assert_eq!(pr_icon(IconSet::Plain, Some("Pull:")), "Pull:");
    }
}
