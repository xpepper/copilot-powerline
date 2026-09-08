# copilot-powerline

[![Crates.io](https://img.shields.io/crates/v/copilot-powerline.svg)](https://crates.io/crates/copilot-powerline)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A blazingly fast, modular, and customizable powerline status line for GitHub Copilot CLI, written in Rust. Inspired by [`claude-powerline`](https://github.com/Owloops/claude-powerline).

---

## Previews

```text
# Minimal Style (Nerd Icons)
󰮚 129k/400k (32%)  │  󰄬 $2.20  │  󰠠 $273.88  │  󰘸 80%  │  󰚩 4.2k  │  󰓅 175k

# Minimal Style (Emoji Icons)
🪙 129k/400k (32%)  │  💰 $2.20  │  📅 $273.88  │  ⚡ 80%  │  🧠 4.2k  │  📊 175k

# Capsule Style (Nerd Icons)
󰮚 129k/400k (32%)  󰄬 $2.20  󰠠 $273.88  󰘸 80%  󰚩 4.2k  󰓅 175k 

# Plain Style (Text Labels)
Tokens: 129k/400k (32%)  │  Session: $2.20  │  Month: $273.88  │  Cache: 80%  │  Think: 4.2k  │  Total: 175k
```

---

## Features

- ⚡️ **Sub-5ms Execution**: Compiled Rust binary with bundled SQLite; zero terminal latency or lag.
- 🎯 **Context Window Monitoring**: Real-time context tracking with percentage and warning alert threshold (`>100k`).
- 🧠 **Prompt Cache Tracking**: Real-time cache hit rate (or token count), automatically hidden when zero.
- 💡 **Reasoning Tokens**: Tracks thinking tokens for reasoning models (e.g. o3-mini, Claude 3.7 Sonnet thinking).
- 📊 **Total Session Tokens**: Displays total accumulated token volume across all turns and compactions.
- 💰 **Spend Tracking**: Real-time session spend and month-to-date aggregation (read directly from Copilot's local SQLite database).
- ⚙️ **Configurable AIC Display**: Easily toggle whether AI Credits (`... AIC`) are displayed alongside dollar amounts.
- 🎨 **Multiple Styles & Icon Sets**: Choose from `minimal`, `powerline`, `capsule`, or `plain`, with `nerd`, `emoji`, or `plain` icons.
- 🌈 **Themes**: Built-in support for `colorblind`, `github`, `nord`, `tokyo-night`, and `plain`. Automatically syncs with your Copilot CLI theme if set to default.
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

## Available Segments

| Segment | Icon (`nerd`) | Icon (`emoji`) | Description |
|---|:---:|:---:|---|
| `tokens` | `󰮚` | `🪙` | Active context window tokens vs limit (with percentage and alert icon) |
| `session_cost` | `󰄬` | `💰` | Current session cost in USD (optional AIC suffix) |
| `month_cost` | `󰠠` | `📅` | Month-to-date spend read from `~/.copilot/session-store.db` |
| `cache` | `󰘸` | `⚡` | Prompt cache hit percentage or read token count (auto-hides when 0) |
| `reasoning` | `󰚩` | `🧠` | Model reasoning/thinking tokens (auto-hides when 0) |
| `total_tokens` | `󰓅` | `📊` | Total accumulated tokens exchanged across the entire session |

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
# prefix = "Tokens:"   # Optional custom override

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
copilot-powerline --icon-set plain

# Override style on the fly
copilot-powerline --style capsule
copilot-powerline --style powerline
copilot-powerline --style minimal

# Override theme on the fly
copilot-powerline --theme nord
copilot-powerline --theme tokyo-night

# Use a custom configuration file
copilot-powerline --config /path/to/custom-powerline.toml
```

---

## Troubleshooting

- **Icons look like question marks or boxes?**
  Make sure your terminal font is a [Nerd Font](https://www.nerdfonts.com/) (such as *JetBrainsMono Nerd Font*, *Hack Nerd Font*, or *FiraCode Nerd Font*). Alternatively, switch to emoji icons by setting `icon_set = "emoji"` or plain text with `icon_set = "plain"` in `powerline.toml`.

- **Copilot shows a blank statusline?**
  Ensure `copilot-powerline` is in your `PATH` or use the full path `~/.cargo/bin/copilot-powerline` in `~/.copilot/settings.json`.

---

## License

MIT
