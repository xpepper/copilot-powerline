# copilot-powerline

[![Crates.io](https://img.shields.io/crates/v/copilot-powerline.svg)](https://crates.io/crates/copilot-powerline)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A fast, modular, and customizable powerline status line for GitHub Copilot CLI, written in Rust. Inspired by [`claude-powerline`](https://github.com/Owloops/claude-powerline).

## Features

- ⚡️ **Sub-5ms Execution**: Compiled Rust binary with bundled SQLite; zero terminal latency or lag.
- 🎯 **Context & Token Monitoring**: Displays current vs max tokens with a percentage and a high-usage alert threshold (>100k).
- 🧠 **Prompt Cache & Reasoning Tokens**: Displays prompt cache hit rate / cache tokens and model reasoning/thinking tokens.
- 📊 **Total Session Tokens**: Tracks total accumulated token volume across the entire session.
- 💰 **Spend Tracking**: Real-time session spend and month-to-date aggregation (read directly from Copilot's local SQLite database).
- ⚙️ **Configurable AIC Display**: Easily toggle whether AI Credits (`... AIC`) are displayed alongside dollar amounts.
- 🎨 **Multiple Styles & Icon Sets**: Choose from `minimal` (pipe separator), `powerline` (Nerd Font arrows), `capsule` (rounded pills), or `plain`, and select from `nerd`, `emoji`, or `plain` icon sets.
- 🌈 **Themes**: Built-in support for `colorblind`, `github`, `nord`, `tokyo-night`, and `plain`. Automatically syncs with your Copilot theme if set to default.
- 🛠 **Zero Dependencies**: Bundles SQLite and JSON parsing directly into a single static binary.

---

## Quick Start

### 1. Install

Install directly via Cargo:

```bash
cargo install copilot-powerline
```

Or build from source:

```bash
git clone https://github.com/xpepper/copilot-powerline.git
cd copilot-powerline
cargo install --path .
```

### 2. Generate Default Configuration

```bash
copilot-powerline --init
```

This creates a default configuration file at `~/.copilot/powerline.toml`.

### 3. Connect to GitHub Copilot CLI

Update your `~/.copilot/settings.json` to use `copilot-powerline`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "copilot-powerline",
    "refreshInterval": 30
  }
}
```

*(If `~/.cargo/bin` is not in your global system `PATH`, specify `~/.cargo/bin/copilot-powerline`)*.

---

## Configuration (`~/.copilot/powerline.toml`)

`copilot-powerline` is configured via a simple TOML file:

```toml
style = "minimal"      # Options: "minimal", "powerline", "capsule", "plain"
icon_set = "nerd"      # Options: "nerd", "emoji", "plain"
theme = "colorblind"   # Options: "colorblind", "github", "nord", "tokyo-night", "plain"
segments = [
    "tokens",
    "session_cost",
    "month_cost",
    "cache",
    "reasoning",
    "total_tokens",
]

[tokens]
enabled = true
show_percentage = true
alert_threshold = 100000
alert_icon = "⚠️ "

[session_cost]
enabled = true
currency_symbol = "$"
show_aic = false       # Set to true to show "(X.X AIC)"
decimal_places = 2

[month_cost]
enabled = true
currency_symbol = "$"
show_aic = false       # Set to true to show "(X AIC)"
decimal_places = 2

[cache]
enabled = true
show_as_percentage = true  # Set to false to show token count (e.g. 85k)
auto_hide_zero = true      # Automatically hide if 0 cache reads

[reasoning]
enabled = true
auto_hide_zero = true      # Automatically hide if model has no reasoning tokens

[total_tokens]
enabled = true
```

---

## Command Line Options

```bash
# Test with sample JSON from stdin
echo '{"context_window":{"current_context_tokens":0,"displayed_context_limit":200000,"current_context_used_percentage":0},"ai_used":{"total_nano_aiu":0}}' | copilot-powerline

# Override icon set on the fly
copilot-powerline --icon-set nerd
copilot-powerline --icon-set emoji

# Override style on the fly
copilot-powerline --style capsule
copilot-powerline --style powerline

# Override theme on the fly
copilot-powerline --theme nord

# Use a custom configuration file
copilot-powerline --config /path/to/custom-powerline.toml
```

---

## License

MIT
