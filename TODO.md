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
      Enabled 2026-10-08; no spike seen as of 2026-10-09.

- [ ] Validate the idle cache-expiry warning (`cache_expiry`, on by
      default) in real sessions: does it appear after ~5 idle minutes, and
      clear on the next step? Verified only with a backdated snapshot. Open
      questions: is the 50k `min_tokens` floor right; is 300 s right for
      non-Anthropic models (their cache lifetimes may differ); a long tool run
      (e.g. a 10-minute build) also triggers it, which is accurate but may
      read oddly mid-turn. If it fires reliably, it could also silence the
      spike warning's expected false alarm on the first turn after idle.
      Installed locally (0.7.0, `cache_expiry` added to the pinned
      `segments`) on 2026-10-09.

- [ ] Watch whether the garbled status line seen in Copilot CLI with 0.5.0
      comes back. 0.6.0 replaced the default `⚠️` alert icons (`U+26A0
      U+FE0F`, measured as 1 or 2 columns) with `🔥` and `📈`, single
      wide code points. If it stays away, the selector was the likely cause.
      Only meaningful once the local config stops pinning `[tokens]
      alert_icon = "⚠️ "` (copied from the pre-0.6.0 README sample).

## Agent environment (from the 2026-10-09 retro)

- [ ] Make `scripts/release.sh` testable without a real release: a
      `--dry-run` flag that prints the commands, or a test with fake `git`
      and `gh`. Only worth it if more releases are coming. Both
      subcommands ran cleanly for real on the 0.6.0 release. A tag cannot
      be undone.

## CI

- [ ] Check CI after GitHub moves `ubuntu-latest` to Ubuntu 26, from
      2026-10-19 (annotation on the v0.7.0 release run). Affects `ci.yml`
      and `publish-crates.yml`; `release.yml` pins `ubuntu-22.04`. Nothing
      to do before then. A `publish-crates.yml` break would only surface
      on the next release, so check it before tagging one.

## Housekeeping

- [ ] Prune stale state files in the cache dir: per-session
      `spend_*.json`, `cache_*.json`, `idle_*.json` and `month_*.json`
      snapshots and `pr_*.json` entries are never removed. Tiny, but they
      grow forever.
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

- [ ] macOS: consider signing/notarization for browser-downloaded binaries.
- [ ] aqua / eget: not tested; document only if someone asks.
- [ ] Later, on demand: Scoop/winget (needs Windows CI first), AUR, Nix,
      `.deb`/`.rpm`.
