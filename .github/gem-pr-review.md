# Review guidelines for copilot-powerline

A Rust status-line tool for GitHub Copilot CLI. Every refresh is a fresh process, so the hot path must stay fast and quiet.

## Global

- The hot path stays under 15 ms: no network, no heavy dependencies, no async runtime. Slow work (a `gh` call) runs in a detached background worker that only writes a disk cache.
- Stdin payloads can be empty or change shape. Deserialize with `Option<T>` or `#[serde(default)]` and never panic.
- Copilot's SQLite database is opened read-only (`SQLITE_OPEN_READ_ONLY`, `busy_timeout` 500 ms) and never written.
- State kept across refreshes goes through `src/state.rs` (private cache dir, atomic writes).
- No absolute machine paths in code, docs, comments or tests.

## Contracts

- The files in `tests/fixtures/` are captures of real GitHub responses and are the contract for parsing. Check a parsing or format claim against them before reporting it. For example, `quota_reset_date` is `YYYY-MM-DD` and `quota_reset_date_utc` is the timestamp.
- New config fields need serde defaults, so existing `powerline.toml` files keep loading.

## Performance

- Only the main refresh path is latency-sensitive. A background worker may block on `gh`, but it needs a bounded runtime.
- `gh` output is a small JSON document from a trusted local binary, so buffering it is fine.

## Conventions

- A new segment follows the steps in `AGENTS.md` ("How to Add a New Segment"): a file in `src/segments/`, registered in `mod.rs`, a config struct with defaults, wired into the `main.rs` match.
- Behaviour changes come with a test that failed first.
