use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "copilot-powerline")]
#[command(author, version, about = "Customizable statusline for GitHub Copilot CLI", long_about = None)]
pub struct Cli {
    /// Path to TOML configuration file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Override the display style (minimal, powerline, capsule, plain)
    #[arg(short, long)]
    pub style: Option<String>,

    /// Override the color theme (colorblind, github, nord, tokyo-night, plain)
    #[arg(short, long)]
    pub theme: Option<String>,

    /// Override the icon set (plain, nerd, emoji)
    #[arg(short = 'i', long = "icon-set")]
    pub icon_set: Option<String>,

    /// Write a default config to --config or ~/.copilot/powerline.toml (never overwrites)
    #[arg(long)]
    pub init: bool,

    /// Switch between full and compact mode (takes effect on the next refresh)
    #[arg(long)]
    pub toggle: bool,

    /// Internal worker flag to update the PR cache in the background
    #[arg(long, hide = true)]
    pub fetch_pr_cache: Option<PathBuf>,

    /// Internal worker flag to update the cycle usage cache in the background
    #[arg(long, hide = true)]
    pub fetch_cycle_usage: Option<PathBuf>,

    /// Internal worker flag indicating the target repo directory for PR fetching
    #[arg(long, hide = true)]
    pub repo_dir: Option<PathBuf>,

    /// Internal worker flag: also count the PR's unresolved review threads
    #[arg(long, hide = true)]
    pub fetch_unresolved: bool,
}
