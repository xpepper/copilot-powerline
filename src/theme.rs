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
    }
}
