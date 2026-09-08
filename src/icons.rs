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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_set_plain() {
        assert_eq!(tokens_icon(IconSet::Plain, None), "Tokens:");
        assert_eq!(cache_icon(IconSet::Plain, None), "Cache:");
        assert_eq!(reasoning_icon(IconSet::Plain, None), "Think:");
        assert_eq!(total_tokens_icon(IconSet::Plain, None), "Total:");
    }

    #[test]
    fn test_icon_set_nerd() {
        assert_eq!(tokens_icon(IconSet::Nerd, None), "󰮚");
        assert_eq!(cache_icon(IconSet::Nerd, None), "󰘸");
        assert_eq!(reasoning_icon(IconSet::Nerd, None), "󰚩");
        assert_eq!(total_tokens_icon(IconSet::Nerd, None), "󰓅");
    }

    #[test]
    fn test_icon_set_emoji() {
        assert_eq!(tokens_icon(IconSet::Emoji, None), "🪙");
        assert_eq!(session_icon(IconSet::Emoji, None), "💰");
        assert_eq!(month_icon(IconSet::Emoji, None), "📅");
        assert_eq!(cache_icon(IconSet::Emoji, None), "⚡");
        assert_eq!(reasoning_icon(IconSet::Emoji, None), "🧠");
        assert_eq!(total_tokens_icon(IconSet::Emoji, None), "📊");
    }

    #[test]
    fn test_custom_override() {
        assert_eq!(
            cache_icon(IconSet::Emoji, Some("CustomCache:")),
            "CustomCache:"
        );
    }
}
