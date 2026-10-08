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

## Spend alerts

- [ ] Absolute session spend threshold, e.g. `alert_above_usd = 5.0` under
      `[session_cost]`: show the session cost with `⚠️` and the alert color
      once it passes the limit. Stateless and predictable, and it covers what
      the relative spike warning cannot: sessions that are expensive from the
      start, and overall budget surprises. Consider the same for
      `month_cost`.

## Housekeeping

- [ ] Prune stale state files in the cache dir: per-session
      `spend_*.json`, `cache_*.json` and `month_*.json` snapshots and
      `pr_*.json` entries are never removed. Tiny, but they grow forever.
      Prune opportunistically
      (e.g. files older than 30 days, only when creating a new one) to keep
      the hot path cheap.

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
