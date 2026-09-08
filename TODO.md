# TODO: copilot-powerline

## Goal
Migrate Python status line to a standalone, configurable Rust tool (`copilot-powerline`) supporting TOML configuration, modular segments, styles (minimal, powerline, capsule), and themes.

## Steps
- [x] Project Scaffolding
  - [x] Initialize Cargo binary and git repository
  - [ ] Add dependencies to `Cargo.toml` (`serde`, `serde_json`, `toml`, `rusqlite`, `clap`, `dirs`)
- [ ] Core Logic (TDD)
  - [ ] Token formatting & alert detection tests & implementation
  - [ ] Cost calculation & currency/AIC formatting tests & implementation
  - [ ] SQLite month usage query tests & implementation
  - [ ] TOML configuration parsing and defaults tests & implementation
- [ ] Rendering Engine
  - [ ] Minimal style separator (`│`)
  - [ ] Themes (colorblind, github, plain, etc.)
  - [ ] Powerline & capsule Nerd Font styling
- [ ] CLI & Stdin Integration
  - [ ] Read JSON from stdin with graceful fallback
  - [ ] CLI flags: `--init`, `--config`, `--style`, `--theme`
- [ ] Verification & Integration
  - [ ] Integration tests with actual Copilot stdin payloads
  - [ ] Compare output against Python statusline
  - [ ] Build release binary
