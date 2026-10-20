pub struct Palette {
    pub reset: &'static str,
    pub dim: &'static str,
    pub label: &'static str,
    pub sep: &'static str,
    pub spend: &'static str,
    pub tokens_normal: &'static str,
    pub tokens_alert: &'static str,
    #[allow(dead_code)]
    pub badge: &'static str,
    pub pr: &'static str,
    pub pr_approved: &'static str,
    pub pr_changes_requested: &'static str,
    pub pr_draft: &'static str,
    pub trend_up: &'static str,
    pub trend_down: &'static str,
}

impl Palette {
    pub fn for_theme(theme_name: &str) -> Self {
        match theme_name.to_lowercase().as_str() {
            "github" | "default" => Self {
                reset: "\x1b[0m",
                dim: "\x1b[90m",
                label: "\x1b[90m",
                sep: "\x1b[90m  │  \x1b[0m",
                spend: "\x1b[92m", // Bright green
                tokens_normal: "\x1b[97m", // Bright white
                tokens_alert: "\x1b[1;31m", // Bold red
                badge: "\x1b[93m", // Bright yellow
                pr: "\x1b[33m",    // Yellow
                pr_approved: "\x1b[92m", // Bright green
                pr_changes_requested: "\x1b[1;31m", // Bold red
                pr_draft: "\x1b[90m", // Grey
                trend_up: "\x1b[92m",   // Bright green
                trend_down: "\x1b[91m", // Bright red
            },
            "nord" => Self {
                reset: "\x1b[0m",
                dim: "\x1b[90m",
                label: "\x1b[90m",
                sep: "\x1b[90m  │  \x1b[0m",
                spend: "\x1b[36m", // Cyan (Frost)
                tokens_normal: "\x1b[97m",
                tokens_alert: "\x1b[1;31m",
                badge: "\x1b[93m",
                pr: "\x1b[33m", // Yellow
                pr_approved: "\x1b[92m", // Bright green
                pr_changes_requested: "\x1b[1;31m", // Bold red
                pr_draft: "\x1b[90m", // Grey
                trend_up: "\x1b[92m",
                trend_down: "\x1b[91m",
            },
            "tokyo-night" => Self {
                reset: "\x1b[0m",
                dim: "\x1b[90m",
                label: "\x1b[90m",
                sep: "\x1b[90m  │  \x1b[0m",
                spend: "\x1b[95m", // Bright magenta / violet
                tokens_normal: "\x1b[97m",
                tokens_alert: "\x1b[1;31m",
                badge: "\x1b[93m",
                pr: "\x1b[93m", // Bright yellow
                pr_approved: "\x1b[92m", // Bright green
                pr_changes_requested: "\x1b[1;31m", // Bold red
                pr_draft: "\x1b[90m", // Grey
                trend_up: "\x1b[92m",
                trend_down: "\x1b[91m",
            },
            "plain" => Self {
                reset: "",
                dim: "",
                label: "",
                sep: "  |  ",
                spend: "",
                tokens_normal: "",
                tokens_alert: "",
                badge: "",
                pr: "",
                pr_approved: "",
                pr_changes_requested: "",
                pr_draft: "",
                trend_up: "",
                trend_down: "",
            },
            _ /* "colorblind" or any other */ => Self {
                reset: "\x1b[0m",
                dim: "\x1b[90m",
                label: "\x1b[90m",
                sep: "\x1b[90m  │  \x1b[0m",
                spend: "\x1b[94m", // Bright blue
                tokens_normal: "\x1b[97m", // Bright white
                tokens_alert: "\x1b[1;31m", // Bold red
                badge: "\x1b[93m", // Bright yellow
                pr: "\x1b[93m",    // Bright yellow
                // Blue/orange instead of green/red, like the trend arrows.
                pr_approved: "\x1b[94m", // Bright blue
                pr_changes_requested: "\x1b[38;5;208m", // Orange
                pr_draft: "\x1b[90m", // Grey
                // Blue/orange instead of green/red, distinguishable with
                // red-green color blindness; the arrow carries the meaning too.
                trend_up: "\x1b[94m",          // Bright blue
                trend_down: "\x1b[38;5;208m",  // Orange
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_selection() {
        let cb = Palette::for_theme("colorblind");
        assert_eq!(cb.spend, "\x1b[94m");

        let gh = Palette::for_theme("github");
        assert_eq!(gh.spend, "\x1b[92m");

        let plain = Palette::for_theme("plain");
        assert_eq!(plain.reset, "");
        assert_eq!(plain.sep, "  |  ");
        assert_eq!(plain.pr, "");

        assert_eq!(cb.pr, "\x1b[93m");
    }

    #[test]
    fn test_review_state_colours_are_blue_and_orange_for_colorblind() {
        let cb = Palette::for_theme("colorblind");
        assert_eq!(cb.pr_approved, "\x1b[94m");
        assert_eq!(cb.pr_changes_requested, "\x1b[38;5;208m");

        let gh = Palette::for_theme("github");
        assert_eq!(gh.pr_approved, "\x1b[92m");
        assert_eq!(gh.pr_changes_requested, "\x1b[1;31m");
        assert_eq!(gh.pr_draft, "\x1b[90m");

        let plain = Palette::for_theme("plain");
        assert_eq!(plain.pr_approved, "");
        assert_eq!(plain.pr_changes_requested, "");
        assert_eq!(plain.pr_draft, "");
    }
}
