use clap::Parser;
use std::io::{self, Read};
use std::path::PathBuf;

mod cli;
mod config;
mod db;
mod icons;
mod input;
mod renderer;
mod segments;
mod theme;

use cli::Cli;
use config::{Config, IconSet, Style};
use input::CopilotInput;
use renderer::render_segments;
use segments::cache::render_cache_segment;
use segments::month_cost::render_month_cost_segment;
use segments::reasoning::render_reasoning_segment;
use segments::session_cost::render_session_cost_segment;
use segments::tokens::render_tokens_segment;
use segments::total_tokens::render_total_tokens_segment;
use theme::Palette;

fn read_stdin() -> String {
    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let _ = handle.read_to_string(&mut buffer);
    buffer
}

fn detect_copilot_theme() -> Option<String> {
    let settings_path = dirs::home_dir()?.join(".copilot").join("settings.json");
    let content = std::fs::read_to_string(settings_path).ok()?;
    let val: serde_json::Value = serde_json::from_str(&content).ok()?;
    val.get("theme")
        .and_then(|t| t.as_str())
        .map(ToString::to_string)
}

fn main() {
    let cli = Cli::parse();

    if cli.init {
        let default_cfg = Config::default();
        let toml_str = default_cfg
            .to_toml_string()
            .unwrap_or_else(|_| String::new());

        if let Some(cfg_path) = Config::default_config_path() {
            if cfg_path.exists() {
                eprintln!("Config file already exists at {}", cfg_path.display());
            } else {
                if let Some(parent) = cfg_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(&cfg_path, &toml_str) {
                    Ok(_) => println!("Initialized config at {}", cfg_path.display()),
                    Err(e) => eprintln!("Failed to write config: {e}"),
                }
            }
        } else {
            println!("{toml_str}");
        }
        return;
    }

    let mut config = Config::load_from_file_or_default(cli.config.as_deref());

    if let Some(s) = cli.style.and_then(|s| s.parse::<Style>().ok()) {
        config.style = s;
    }

    if let Some(i) = cli.icon_set.and_then(|i| i.parse::<IconSet>().ok()) {
        config.icon_set = i;
    }

    if let Some(theme_override) = cli.theme {
        config.theme = theme_override;
    } else if config.theme == "colorblind" {
        // Check if Copilot settings has a theme preference
        if let Some(copilot_theme) = detect_copilot_theme() {
            config.theme = copilot_theme;
        }
    }

    let palette = Palette::for_theme(&config.theme);

    let raw_input = read_stdin();
    let input = if raw_input.trim().is_empty() {
        CopilotInput::default()
    } else {
        CopilotInput::from_json(&raw_input)
    };

    let db_path = config
        .month_cost
        .db_path
        .clone()
        .or_else(db::default_db_path)
        .unwrap_or_else(|| PathBuf::from("session-store.db"));

    let other_nano = db::get_month_other_sessions_nano(&db_path, input.session_id.as_deref());
    let session_nano = input.ai_used.total_nano_aiu;
    let total_month_nano = other_nano + session_nano;

    let mut rendered_segments = Vec::new();

    for seg in &config.segments {
        match seg.as_str() {
            "tokens" => {
                if let Some(s) = render_tokens_segment(
                    &input.context_window,
                    &config.tokens,
                    config.icon_set,
                    &palette,
                ) {
                    rendered_segments.push(s);
                }
            }
            "session_cost" => {
                if let Some(s) = render_session_cost_segment(
                    session_nano,
                    &config.session_cost,
                    config.icon_set,
                    &palette,
                ) {
                    rendered_segments.push(s);
                }
            }
            "month_cost" => {
                if let Some(s) = render_month_cost_segment(
                    total_month_nano,
                    &config.month_cost,
                    config.icon_set,
                    &palette,
                ) {
                    rendered_segments.push(s);
                }
            }
            "cache" => {
                if let Some(s) = render_cache_segment(
                    &input.context_window,
                    &config.cache,
                    config.icon_set,
                    &palette,
                ) {
                    rendered_segments.push(s);
                }
            }
            "reasoning" => {
                if let Some(s) = render_reasoning_segment(
                    &input.context_window,
                    &config.reasoning,
                    config.icon_set,
                    &palette,
                ) {
                    rendered_segments.push(s);
                }
            }
            "total_tokens" => {
                if let Some(s) = render_total_tokens_segment(
                    &input.context_window,
                    &config.total_tokens,
                    config.icon_set,
                    &palette,
                ) {
                    rendered_segments.push(s);
                }
            }
            _ => {}
        }
    }

    let output = render_segments(&rendered_segments, config.style, &palette);
    println!("{output}");
}
