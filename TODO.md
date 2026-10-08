# TODO: copilot-powerline

Open work only, highest priority first. Completed items live in git history.

## Correctness

- [ ] `pr` segment: use the payload's `cwd` (also sent as
      `workspace.current_dir`) instead of the process cwd. Copilot CLI has
      sent it since at least 1.0.90; the process cwd is only the directory
      Copilot was launched from, so the PR shown can belong to the wrong repo
      or branch if the session's working directory changes. Also fix the
      stale "payload has no working directory" note in AGENTS.md (Data Flow,
      step 1).

## Validate new features

- [ ] Tune the session-cost spike thresholds (`spike_ratio = 2.0`,
      `spike_min_usd = 0.05`) against a few days of real sessions. They are
      reasonable guesses, not measured. Watch for false alarms in agent loops
      and cache-miss turns.
- [ ] Try `--toggle` and watch for a spike warning in a live Copilot CLI
      session (only verified with piped payloads so far). The cache trend
      arrow is verified live.

## Performance

- [ ] Re-measure refresh time with `hyperfine`. A Python-driven timing
      (spawn overhead included) showed ~16-17 ms median on both `main` and
      the reddit-feedback branch, above the sub-15 ms rule; the earlier
      hyperfine figure was ~12.6 ms. If it is really over, profile the
      month-to-date SQLite query against a large `session-store.db`.

## Housekeeping

- [ ] Prune stale state files in the cache dir: per-session
      `spend_*.json` snapshots and `pr_*.json` entries are never removed.
      Tiny, but they grow forever. Prune opportunistically (e.g. files older
      than 30 days, only when creating a new one) to keep the hot path cheap.

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
