# Agent Guidelines for `copilot-powerline`

Welcome! This document provides architecture overviews, design constraints, and development guidelines for AI coding agents and contributors collaborating on `copilot-powerline`.

---

## 1. Project Overview

`copilot-powerline` is a fast, modular, and customizable status line tool for GitHub Copilot CLI written in Rust.

### Data Flow
1. **Stdin Input**: GitHub Copilot CLI passes a JSON payload via standard input on statusline refreshes (containing `context_window`, `ai_used`, `session_id`, `model`, etc.).
2. **Configuration**: The tool loads `~/.copilot/powerline.toml` (or a path provided via `--config`), falling back to built-in defaults.
3. **Database Query**: Queries the local SQLite database (`~/.copilot/session-store.db`) to calculate month-to-date spend across previous sessions.
4. **Segment Assembly**: Iterates through enabled segments (`tokens`, `session_cost`, `month_cost`), formatting each.
5. **Rendering**: The `renderer` applies the configured style (`minimal`, `powerline`, `capsule`, `plain`) and theme ANSI colors.
6. **Stdout Output**: Emits the single-line formatted status line to standard output.

---

## 2. Codebase Map

```
copilot-powerline/
├── Cargo.toml                 # Dependencies, release profile optimizations, and crates.io metadata
├── README.md                  # User-facing documentation and installation guide
├── LICENSE                    # MIT License
├── src/
│   ├── main.rs                # Entry point, CLI orchestration, and stdin reading
│   ├── cli.rs                 # Clap CLI arguments (--init, --style, --theme, --icon-set, --config)
│   ├── config.rs              # TOML config structures, defaults, and file loading
│   ├── icons.rs               # Icon set resolver (Nerd, Emoji, Plain)
│   ├── input.rs               # Deserialization of Copilot CLI stdin JSON payloads
│   ├── db.rs                  # Read-only SQLite query helper for session-store.db
│   ├── renderer.rs            # Separators and powerline/capsule glyph formatting
│   ├── theme.rs               # ANSI color palettes (colorblind, github, nord, tokyo-night, plain)
│   └── segments/              # Modular statusline components
│       ├── mod.rs             # Module declarations
│       ├── tokens.rs          # Token counts, formatting (e.g. 150k, 1.2M), and alert thresholds
│       ├── session_cost.rs    # Real-time session spend calculation (USD and optional AIC)
│       ├── month_cost.rs      # Month-to-date spend calculation (USD and optional AIC)
│       ├── cache.rs           # Prompt cache hit rate or token counts
│       ├── reasoning.rs       # Model reasoning/thinking token tracking
│       └── total_tokens.rs    # Accumulated session token volume
```

---

## 3. Non-Negotiable Engineering Rules

1. **Sub-15ms Execution**:
   - This tool runs inside an interactive terminal status line loop.
   - Do not introduce heavy dependencies, async runtimes (e.g. Tokio), or network requests during status line execution.
2. **Bundled SQLite**:
   - `rusqlite` must always be configured with `features = ["bundled"]` in `Cargo.toml`. This ensures the binary remains self-contained with zero external C library dependencies on user machines.
3. **Defensive Stdin Parsing**:
   - Copilot CLI payloads may change over time or be empty. Never assume a field is always present.
   - All input deserialization in `src/input.rs` must use `Option<T>` or `#[serde(default)]` and fall back gracefully without panicking.
4. **Safe, Read-Only Database Access**:
   - When querying `~/.copilot/session-store.db`, always open the connection with `OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI`.
   - Never write to or lock the Copilot database.
   - Always set a short timeout (`busy_timeout(500ms)`) to prevent blocking the status line if Copilot is writing to SQLite.
5. **Zero Compiler Warnings**:
   - All code must compile cleanly with `cargo check` and `cargo test` without warnings.
6. **No Machine-Specific Paths**:
   - Never hardcode or leak absolute paths from the developer's local machine (e.g. `/Users/...` or `/home/...`) in documentation, code comments, or tests.
   - Use `dirs::home_dir()` or relative paths.

---

## 4. How to Add a New Segment

When adding a new segment (e.g., `git`, `model`, `duration`):

1. **Create Segment Logic**:
   - Add `src/segments/<segment_name>.rs` with:
     - Pure formatting/calculation functions.
     - A `render_<segment_name>_segment(...) -> Option<String>` function.
     - Comprehensive unit tests covering edge cases.
2. **Update Module Exports**:
   - Register the module in `src/segments/mod.rs`.
3. **Add Segment Configuration**:
   - In `src/config.rs`, add the segment's configuration struct and include it in `Config`.
   - Add sensible defaults so existing user configs continue working without schema errors.
4. **Wire into Renderer**:
   - In `src/main.rs`, match the segment name in the rendering loop and append the formatted string to `rendered_segments`.

---

## 5. Development & Verification Workflow

Always follow test-first development for behavioural changes:

```bash
# 1. Run all unit tests
cargo test

# 2. Run linter
cargo clippy --all-targets -- -D warnings

# 3. Test with a sample Copilot CLI stdin payload
echo '{"context_window":{"current_context_tokens":0,"displayed_context_limit":200000,"current_context_used_percentage":0},"ai_used":{"total_nano_aiu":0}}' | cargo run --

# 4. Test style overrides
echo '{"context_window":{"current_context_tokens":120000,"displayed_context_limit":200000,"current_context_used_percentage":60},"ai_used":{"total_nano_aiu":500000000000}}' | cargo run -- --style capsule --theme nord

# 5. Build optimized release binary
cargo build --release
```

---

## 6. Commit Conventions

Use Conventional Commits:
- `feat(<scope>): <description>` — New features or segment additions
- `fix(<scope>): <description>` — Bug fixes or edge-case handling
- `perf(<scope>): <description>` — Performance optimizations
- `test(<scope>): <description>` — Adding or updating unit tests
- `docs: <description>` — Documentation or README updates
- `chore: <description>` — Maintenance, dependency bumps, or metadata changes
