# copilot-powerline

[![CI](https://github.com/xpepper/copilot-powerline/actions/workflows/ci.yml/badge.svg)](https://github.com/xpepper/copilot-powerline/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/copilot-powerline.svg)](https://crates.io/crates/copilot-powerline)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **A fast, beautiful status line for GitHub Copilot CLI.**
>
> Keep context usage, session and monthly spend, prompt-cache efficiency, reasoning tokens, and total session activity visible while you work. `copilot-powerline` is a native Copilot CLI `statusLine` command: a small Rust executable with bundled SQLite that reads the data Copilot already provides.

## Install and connect it to Copilot CLI

Install a prebuilt binary (macOS and Linux, no Rust toolchain needed):

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/xpepper/copilot-powerline/releases/latest/download/copilot-powerline-installer.sh | sh
```

Or with Homebrew:

```bash
brew install xpepper/tap/copilot-powerline
```

Or install from crates.io:

```bash
cargo install copilot-powerline
```

Then add it to `~/.copilot/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "copilot-powerline",
    "refreshInterval": 30
  }
}
```

If the install directory is not on your `PATH`, use the full path as the command: `~/.local/bin/copilot-powerline` for the installer, or `~/.cargo/bin/copilot-powerline` for Cargo. Run `copilot-powerline --init` at any time to create the default configuration at `~/.copilot/powerline.toml`.

## See it in action

![Animated Copilot CLI status line updating context usage, session and monthly spend, cache rate, reasoning tokens, and total tokens](assets/copilot-powerline-demo.gif)

<details>
<summary>View a static status-line preview</summary>

![Copilot CLI status line showing context usage, session and monthly spend, cache rate, reasoning tokens, and total tokens](assets/copilot-powerline-statusline.png)
</details>

```text
⚠️ 145k/400k (36%)  │  󰄬 $8.64  │  󰠠 $281.66  │  󰘸 95%  │  󰚩 6.2k  │  󰓅 4.5M
```

Text preview: a Copilot CLI status line showing a context warning, current-session and month-to-date spend, prompt-cache hit rate, reasoning tokens, and total session tokens.

Choose the presentation that fits your terminal:

```text
# Minimal (Nerd icons)
󰮚 129k/400k (32%)  │  󰄬 $2.20  │  󰠠 $273.88  │  󰘸 80%  │  󰚩 4.2k  │  󰓅 175k

# Capsule (Nerd icons)
󰮚 129k/400k (32%)  󰄬 $2.20  󰠠 $273.88  󰘸 80%  󰚩 4.2k  󰓅 175k 

# Plain (text labels)
Tokens: 129k/400k (32%)  │  Session: $2.20  │  Month: $273.88  │  Cache: 80%  │  Think: 4.2k  │  Total: 175k
```

## Why copilot-powerline?

- **Stay in flow.** See context pressure before it becomes disruptive, with configurable warnings and a choice of compact layouts.
- **Understand usage.** Follow session spend, month-to-date spend, cache hits, reasoning tokens, and total token volume in the terminal.
- **Make it yours.** Select `minimal`, `powerline`, `capsule`, or `plain` layouts; `nerd`, `emoji`, or text icons; and a built-in color theme.
- **Keep it lightweight.** A compiled Rust executable runs as Copilot refreshes the native status line and queries Copilot's local session database read-only.

Inspired by [`claude-powerline`](https://github.com/Owloops/claude-powerline).

---

## Features

- **Context Window Monitoring**: Real-time context tracking with percentage and warning alert threshold (`>100k`).
- **Prompt Cache Tracking**: Real-time cache hit rate (or token count), automatically hidden when zero.
- **Reasoning Tokens**: Tracks thinking tokens for reasoning models (e.g. o3-mini, Claude 3.7 Sonnet thinking).
- **Total Session Tokens**: Displays total accumulated token volume across all turns and compactions.
- **Spend Tracking**: Real-time session spend and month-to-date aggregation from Copilot's local SQLite database.
- **GitHub Usage Refresh**: An experimental helper scrapes the authenticated GitHub Copilot features page to cache the more complete personal AI-credit counter outside the status-line refresh loop.
- **Pull Request Reference** *(optional)*: Shows the current branch's pull request (e.g. `PR #50`) as a clickable link, via the `gh` CLI. Disabled from the default segment list; opt in by adding `pr` to `segments`.
- **Configurable AIC Display**: Toggle whether AI Credits (`... AIC`) appear alongside dollar amounts.
- **Multiple Styles & Icon Sets**: Choose from `minimal`, `powerline`, `capsule`, or `plain`, with `nerd`, `emoji`, or `plain` icons.
- **Themes**: Built-in support for `colorblind`, `github`, `nord`, `tokyo-night`, and `plain`. Automatically syncs with your Copilot CLI theme if set to default.
- **Bundled SQLite**: SQLite and JSON parsing ship with the executable; no status-line service is required.

---

## Installation and compatibility

### Prebuilt binaries

Each [GitHub Release](https://github.com/xpepper/copilot-powerline/releases) ships binaries for macOS (Apple Silicon and Intel) and Linux (x86_64 and arm64, glibc 2.35 or newer), with SHA-256 checksums. The shell installer from the [quick start](#install-and-connect-it-to-copilot-cli) picks the right one and installs it to `~/.local/bin`, adding that directory to your `PATH` if needed.

Homebrew (macOS or Linux) installs the same binaries from the [xpepper/homebrew-tap](https://github.com/xpepper/homebrew-tap) tap:

```bash
brew install xpepper/tap/copilot-powerline
```

If you use [cargo-binstall](https://github.com/cargo-bins/cargo-binstall), it downloads the same binaries instead of compiling:

```bash
cargo binstall copilot-powerline
```

If you manage tools with [mise](https://mise.jdx.dev/), install the release binary directly from GitHub:

```bash
mise use -g github:xpepper/copilot-powerline
```

For the status-line command, use the real binary path printed by `mise which copilot-powerline` rather than the mise shim: the shim starts `mise` on every refresh, roughly doubling run time. That path includes the version, so update `~/.copilot/settings.json` after upgrading. mise may hide a release for a while after it is published; if it reports no matching versions, pin one explicitly (for example `github:xpepper/copilot-powerline@0.3.2`).

### Install with Cargo

```bash
cargo install copilot-powerline
```

It requires a current stable Rust toolchain with Cargo.

However you install it, the executable runs locally during Copilot CLI status-line refreshes and never makes network requests itself. The only exception is the optional `pr` segment, which runs `gh pr view` in a detached background process, never inline in the refresh.

### Verified environments and terminal support

CI tests and builds release binaries on GitHub Actions' current macOS and Ubuntu runner images. Windows is not currently verified in CI and has no prebuilt binary; use `cargo install` there at your own risk.

The default `nerd` icon set requires a [Nerd Font](https://www.nerdfonts.com/). Use `--icon-set emoji` or `--icon-set plain` when your terminal does not support Nerd Font glyphs.

### Build from source

For local development or unreleased changes:

```bash
git clone https://github.com/xpepper/copilot-powerline.git
cd copilot-powerline
cargo install --path .
```

To configure a generated default, run:

```bash
copilot-powerline --init
```

This creates `~/.copilot/powerline.toml`.

---

## Status Line Legend & Segments

| Segment | Icon (`nerd`) | Icon (`emoji`) | Text (`plain`) | Example Value | Description |
|---|:---:|:---:|---|---|---|
| `tokens` | `󰮚` / `⚠️` | `🪙` / `⚠️` | `Tokens:` | `145k/400k (36%)` | **Context Window**: Active context tokens vs model limit (and percentage used). Automatically switches to `⚠️` when crossing the configured alert threshold (default `>100k`). |
| `session_cost` | `󰄬` | `💰` | `Session:` | `$8.64` | **Current Session Cost**: Real-time spend accumulated in the active session in USD (optional AIC credit display). |
| `month_cost` | `󰠠` | `📅` | `Month:` | `$281.66` | **Month-to-Date Cost**: Total cumulative monthly spend across all sessions, queried directly from Copilot's `~/.copilot/session-store.db`. |
| `cache` | `󰘸` | `⚡` | `Cache:` | `95%` | **Prompt Cache Hit Rate**: Percentage of prompt tokens served from cache (or raw token count). Automatically hidden when 0. |
| `reasoning` | `󰚩` | `🧠` | `Think:` | `6.2k` | **Reasoning Tokens**: Cumulative tokens used by thinking models (e.g. o3-mini, Claude 3.7 Sonnet). Automatically hidden when 0. |
| `total_tokens` | `󰓅` | `📊` | `Total:` | `4.5M` | **Total Session Tokens**: Total cumulative token throughput (input + output + cached) exchanged across all turns and compactions in the session. |
| `pr` | `` | `🔀` | `PR` | `PR #50` | **Pull Request Reference** *(optional, not in the default `segments` list)*: The current branch's open pull request, as a clickable hyperlink. Requires an authenticated `gh` CLI; hidden when the branch has no open PR or `gh` is unavailable. |

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
    # "pr",   # Uncomment to show the current branch's PR (requires the `gh` CLI)
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

[pr]
enabled = true
hyperlinks = true      # Set to false to print "PR #50" as plain text
cache_ttl_seconds = 60 # How long a cached PR lookup is considered fresh
# prefix = "Pull:"      # Optional custom override
```

The `pr` segment shells out to `gh pr view --json number,url` for the current branch. To avoid blocking the status line on a network call, lookups are cached to disk and refreshed by a throttled, detached background process; the segment is hidden until the first refresh completes, and again whenever the branch has no open PR or `gh` is not installed/authenticated.

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
  Ensure `copilot-powerline` is in your `PATH` or use the full path in `~/.copilot/settings.json`: `~/.local/bin/copilot-powerline` (shell installer), `~/.cargo/bin/copilot-powerline` (Cargo), or the output of `which copilot-powerline`.

---

## Experimental: exact GitHub usage refresh

The `month_cost` segment calculates an estimate from the local Copilot CLI
database. GitHub's **Settings > Copilot > Features** page can show a more
complete personal usage counter, including usage that is absent from the local
database.

The experimental helper is a separate script, not part of the status-line
binary, so it is only available from a clone of this repository. It fetches
that displayed counter through a dedicated local browser profile. It never sends the profile or cookies anywhere, and it
does not run as part of the status-line refresh loop.

```bash
# Install the browser dependency once.
npm install --global agent-browser
agent-browser install

# Open a visible browser once and complete GitHub login, SSO, and 2FA.
./scripts/fetch-github-copilot-usage --login

# Fetch the current value and save a private local cache.
./scripts/fetch-github-copilot-usage
# {"ai_credits_used":49854,"usd":"498.54","cycle":"September 1-30, 2026"}
```

By default, browser state is stored in
`~/.copilot/github-usage-profile` and the output cache in
`~/.copilot/github-usage.json`. Set `COPILOT_USAGE_BROWSER_PROFILE` or
`COPILOT_USAGE_CACHE_FILE` to use different paths. The helper depends on an
undocumented GitHub settings page and may need updating if GitHub changes its
markup.

Every run also appends a timestamped entry to a local JSON Lines history log
at `~/.copilot/github-usage-history.jsonl` (one JSON object per line: `ts`,
`ai_credits_used`, `usd`, `cycle`), so you can track how your credit
consumption changes over time. Set `COPILOT_USAGE_HISTORY_FILE` to use a
different path, or pass `--no-history` to skip it for a single run. Inspect
it with `jq`, for example:

```bash
# Show the last 10 readings.
tail -n 10 ~/.copilot/github-usage-history.jsonl | jq .

# Credit delta between the two most recent runs.
jq -s '.[-1].ai_credits_used - .[-2].ai_credits_used' \
    ~/.copilot/github-usage-history.jsonl
```

---

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for local setup, verification commands, supported scope, and the pull-request workflow.

---

## License

MIT
