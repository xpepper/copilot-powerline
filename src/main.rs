use clap::Parser;
use std::io::{self, Read};
use std::path::PathBuf;

mod cache_trend;
mod cli;
mod config;
mod db;
mod git;
mod github;
mod icons;
mod input;
mod mode;
mod renderer;
mod segments;
mod spend;
mod state;
mod theme;

use cli::Cli;
use config::{Config, IconSet, Style};
use input::CopilotInput;
use renderer::render_segments;
use segments::cache::render_cache_segment;
use segments::model::render_model_segment;
use segments::month_cost::{self, render_month_cost_segment};
use segments::pr::render_pr_segment;
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

    if let Some(cache_path) = cli.fetch_pr_cache {
        let repo_dir = cli.repo_dir.unwrap_or_else(|| PathBuf::from("."));
        github::fetch_and_write_pr_cache(&repo_dir, &cache_path);
        return;
    }

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

    let mode_path = mode::override_file_path();

    if cli.toggle {
        match mode::toggle(&mode_path, config.mode) {
            Ok(m) => println!("copilot-powerline: {} mode", m.as_str()),
            Err(e) => eprintln!("Failed to save display mode: {e}"),
        }
        return;
    }

    let segments = config
        .segments_for(mode::effective(&mode_path, config.mode))
        .to_vec();

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

    // Copilot CLI spawns the status line command in the session's current
    // working directory (it follows `/cwd` and session switches), so the
    // process cwd is the right place for PR lookups. The payload's `cwd`
    // field carries the same value, so it is not parsed.
    let pr_segment_enabled = config.pr.enabled && segments.iter().any(|s| s == "pr");

    let pr_info = if pr_segment_enabled {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // Use the resolved repository root (not the raw cwd) so the cache
        // key and gh working directory stay stable regardless of which
        // subdirectory of the repo the status line was invoked from.
        git::find_repo_root(&cwd).and_then(|repo_dir| {
            git::get_git_branch(&repo_dir).and_then(|branch| {
                github::get_pr_info(
                    &repo_dir,
                    &branch,
                    config.pr.cache_ttl_seconds,
                    github::is_gh_available(),
                )
            })
        })
    } else {
        None
    };

    let other_nano = if month_cost::is_visible(&segments, &config.month_cost) {
        db::get_month_other_sessions_nano(&db_path, input.session_id.as_deref())
    } else {
        0
    };
    let session_nano = input.ai_used.total_nano_aiu;
    let total_month_nano = other_nano + session_nano;

    let spend_spike = config.session_cost.spike_alert
        && segments.iter().any(|s| s == "session_cost")
        && input.session_id.as_deref().is_some_and(|id| {
            let ctx = &input.context_window;
            let total_tokens = ctx.total_tokens.unwrap_or_else(|| {
                ctx.total_input_tokens.unwrap_or(0) + ctx.total_output_tokens.unwrap_or(0)
            });
            spend::check_spike(
                &spend::snapshot_path(id),
                session_nano,
                total_tokens,
                spend::SpikeRule {
                    ratio: config.session_cost.spike_ratio,
                    min_usd: config.session_cost.spike_min_usd,
                },
            )
        });

    let latest_cache_trend = if config.cache.show_trend
        && segments.iter().any(|s| s == "cache")
        && let Some(id) = input.session_id.as_deref()
        && let Some(input_tokens) = input.context_window.total_input_tokens
    {
        cache_trend::check_trend(
            &cache_trend::snapshot_path(id),
            input_tokens,
            input.context_window.total_cache_read_tokens.unwrap_or(0),
        )
    } else {
        None
    };

    let mut rendered_segments = Vec::new();

    for seg in &segments {
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
                    spend_spike,
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
                    latest_cache_trend,
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
            "model" => {
                if let Some(s) = render_model_segment(
                    input.model.as_ref(),
                    &config.model,
                    config.icon_set,
                    &palette,
                ) {
                    rendered_segments.push(s);
                }
            }
            "pr" => {
                if let Some(s) =
                    render_pr_segment(pr_info.as_ref(), &config.pr, config.icon_set, &palette)
                {
                    rendered_segments.push(s);
                }
            }
            _ => {}
        }
    }

    let output = render_segments(&rendered_segments, config.style, &palette);
    println!("{output}");
}
