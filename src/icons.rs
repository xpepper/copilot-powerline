use crate::config::IconSet;

/// One segment's icon in each icon set.
struct Icons {
    plain: &'static str,
    nerd: &'static str,
    emoji: &'static str,
}

/// The icon to show: the user's custom prefix when set, otherwise the
/// segment's icon in `icon_set`.
fn pick(icon_set: IconSet, custom: Option<&str>, icons: Icons) -> &str {
    if let Some(c) = custom {
        return c;
    }
    match icon_set {
        IconSet::Plain => icons.plain,
        IconSet::Nerd => icons.nerd,
        IconSet::Emoji => icons.emoji,
    }
}

pub fn tokens_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "Tokens:",
            nerd: "󰮚",
            emoji: "🪙",
        },
    )
}

pub fn session_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "Session:",
            nerd: "󰄬",
            emoji: "💰",
        },
    )
}

pub fn month_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "Month:",
            nerd: "󰠠",
            emoji: "📅",
        },
    )
}

pub fn cycle_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "Cycle:",
            nerd: "\u{f02a4}", // nf-md-github
            emoji: "🐙",
        },
    )
}

pub fn cache_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "Cache:",
            nerd: "󰘸",
            emoji: "⚡",
        },
    )
}

pub fn reasoning_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "Think:",
            nerd: "󰚩",
            emoji: "🧠",
        },
    )
}

pub fn total_tokens_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "Total:",
            nerd: "󰓅",
            emoji: "📊",
        },
    )
}

pub fn model_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "Model:",
            nerd: "\u{f061a}", // nf-md-chip
            emoji: "🤖",
        },
    )
}

pub fn fusion_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "Fusion:",
            nerd: "\u{f061a}",
            emoji: "\u{1f500}",
        },
    )
}

pub fn pr_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "PR",
            nerd: "",
            emoji: "🔀",
        },
    )
}

pub fn cache_expiry_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "Idle:",
            nerd: "\u{f051f}", // nf-md-timer_sand
            emoji: "⏳",
        },
    )
}

pub fn tool_overhead_icon(icon_set: IconSet, custom: Option<&str>) -> &str {
    pick(
        icon_set,
        custom,
        Icons {
            plain: "Tools:",
            nerd: "\u{f1064}", // nf-md-tools
            emoji: "🧰",
        },
    )
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
