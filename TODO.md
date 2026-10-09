# TODO: copilot-powerline

Open work only, highest priority first. Completed items live in git history.

## Validate new features

- [ ] Tune the session-cost spike warning, then decide whether to turn it
      on by default. It ships opt-in (`spike_alert = false`) because the
      thresholds (`spike_ratio = 2.0`, `spike_min_usd = 0.05`) are guesses.
      Enable it for a few days of real sessions and note how often it fires.
      Expected false alarms: the first turn after ~5 idle minutes (the prompt
      cache TTL is 300 s, so the whole context is rewritten) and
      output-heavy steps (output tokens cost more than input). Not yet seen
      live; the cache trend arrow and `--toggle` are verified live.

- [ ] Verify the crates.io publish job on the next release tag
      (`.github/workflows/publish-crates.yml`, see RELEASING.md). Before
      tagging, configure the trusted publisher on crates.io (owner
      `xpepper`, repo `copilot-powerline`, workflow `release.yml`). Then
      check that the job published and that `cargo binstall` picks up the
      new version.

## Spend alerts

- [ ] One distinct icon per kind of warning. The context token threshold
      (`[tokens] alert_icon`) and the spend spike (`[session_cost]
      spike_icon`) both default to `⚠️`, so the icon alone does not say
      which alert fired. Pick distinct defaults (candidates: `🔥`, `🚨`,
      `📈`; `⚡` is taken by the emoji cache icon) and update the README.
      Changing a default changes what existing users see, so call it out in
      the release notes. Prefer emoji with an unambiguous width: `⚠️` is
      `U+26A0` plus a VS16 selector, which terminals and TUIs measure as 1
      or 2 columns. A suspected (unconfirmed) cause of a garbled status line
      in Copilot CLI seen with 0.5.0, after frames with `⚠️`/`💸` were
      replaced by one without; a terminal resize fixed it. Our output was
      verified correct for that state.

## Housekeeping

- [ ] Prune stale state files in the cache dir: per-session
      `spend_*.json`, `cache_*.json` and `month_*.json` snapshots and
      `pr_*.json` entries are never removed. Tiny, but they grow forever.
      Prune opportunistically (e.g. files older than 30 days, only when
      creating a new one) to keep the hot path cheap.
- [ ] Binary-level test for `main.rs` wiring. Nothing checks that `main`
      actually skips the month query when `month_cost` is hidden, or reuses
      the cached total; only `hyperfine` and manual runs showed it. A test
      could run the built binary (`env!("CARGO_BIN_EXE_copilot-powerline")`)
      against a temp config whose `[month_cost] db_path` points at a temp
      SQLite file, with no new dependency.
- [ ] `calculate_session_spend` and `calculate_month_spend` are the same
      nano AIU to (USD, AIC) conversion; keep one. A natural home is
      `segments/spend_limit.rs`, renamed to something like `spend_format`.
- [ ] Extract the repeated "is this segment shown" check in `main.rs`
      (`segments.iter().any(|s| s == "...")` for `pr`, `session_cost` and
      `cache`), as done with `month_cost::is_visible`.

## New optional segments (from unused payload fields)

- [ ] Allow-all indicator from `allow_all_enabled` (safety signal).
- [ ] Code churn from `cost.total_lines_added` / `total_lines_removed`
      (e.g. `+120 -30`).
- [ ] Premium requests from `cost.total_premium_requests`.
- [ ] Session and API time from `cost.total_duration_ms` /
      `total_api_duration_ms`.

## `fetch-github-copilot-usage`

- [ ] Optional spike detection: compare each new reading against the
      previous history entry and warn (stderr, and/or a flag in the JSON
      output) when the increase exceeds a configurable threshold. Deferred
      until real history data exists to pick a sensible default threshold.

## Distribution

- [ ] Trim the published crate with an `include` list in `Cargo.toml`.
      `cargo package` ships 48 files (1.4 MiB), mostly `assets/` (the demo
      GIF and screenshot used only by the README), plus `.github/`,
      `TODO.md` and `scripts/`.
- [ ] macOS: consider signing/notarization for browser-downloaded binaries.
- [ ] aqua / eget: not tested; document only if someone asks.
- [ ] Later, on demand: Scoop/winget (needs Windows CI first), AUR, Nix,
      `.deb`/`.rpm`.
