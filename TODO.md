# TODO: copilot-powerline

## Goal
Migrate Python status line to a standalone, configurable Rust tool (`copilot-powerline`) supporting TOML configuration, modular segments, styles (minimal, powerline, capsule), and themes.

## Steps
- [x] Project Scaffolding
  - [x] Initialize Cargo binary and git repository
  - [x] Add dependencies to `Cargo.toml` (`serde`, `serde_json`, `toml`, `rusqlite`, `clap`, `dirs`)
- [x] Core Logic (TDD)
  - [x] Token formatting & alert detection tests & implementation
  - [x] Cost calculation & currency/AIC formatting tests & implementation
  - [x] SQLite month usage query tests & implementation
  - [x] TOML configuration parsing and defaults tests & implementation
- [x] Rendering Engine
  - [x] Minimal style separator (`│`)
  - [x] Themes (colorblind, github, plain, etc.)
  - [x] Powerline & capsule styling
- [x] CLI & Stdin Integration
  - [x] Read JSON from stdin with graceful fallback
  - [x] CLI flags: `--init`, `--config`, `--style`, `--theme`
- [x] Verification & Integration
  - [x] Integration tests with actual Copilot stdin payloads
  - [x] Compare output against Python statusline
  - [x] Build release binary and install to ~/.cargo/bin
  - [x] Create comprehensive README.md for Git publishing

## v0.2.0 Enhancements
- [x] Extend `ContextWindow` input model (`cache_read`, `cache_write`, `reasoning`, `total_tokens`)
- [x] Add `icon_set` configuration (`nerd`, `emoji`, `plain`) and segment configs
- [x] Implement `cache` segment (cache read tokens / hit rate)
- [x] Implement `reasoning` segment (reasoning tokens, auto-hidden when 0)
- [x] Implement `total_tokens` segment (accumulated session token volume)
- [x] Update existing segments with icon support
- [x] Unit tests for new segments and icon sets
- [x] Clippy & cargo test verification
- [x] Update README.md, AGENTS.md, bump version to 0.2.0

## Follow-ups
- [x] `fetch-github-copilot-usage`: append each run to a local JSONL history
      log (`COPILOT_USAGE_HISTORY_FILE`, `--no-history`) for tracking credit
      consumption over time.
- [ ] `fetch-github-copilot-usage`: optional spike detection — compare each
      new reading against the previous history entry and warn (stderr, and/or
      a flag in the JSON output) when the increase exceeds a configurable
      threshold. Deferred until real history data exists to pick a sensible
      default threshold.
- [x] Build & reinstall binary to ~/.cargo/bin
- [x] Commit, push, and publish to crates.io

## Distribution beyond crates.io
Goal: let Copilot CLI users install without a Rust toolchain.

- [ ] Prebuilt binaries on GitHub Releases (via `dist`), with shell
      installer and `cargo binstall` support
- [ ] npm package wrapping the prebuilt binaries (`npm i -g copilot-powerline`);
      Copilot CLI users usually already have Node. Prefer a global install
      over `npx` in `statusLine.command` to keep refreshes fast.
- [ ] Homebrew tap (`xpepper/homebrew-tap`), updated automatically on release
- [ ] Document installs via version managers that read GitHub Releases
      (mise `ubi:`/`github:` backend, aqua, eget)
- [ ] Later, on demand: Scoop/winget (needs Windows CI first), AUR, Nix,
      `.deb`/`.rpm`
- [ ] macOS: consider signing/notarization for browser-downloaded binaries

