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
