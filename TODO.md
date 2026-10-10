# TODO: copilot-powerline

Short-lived checks and memos only, highest priority first. The backlog lives
in [GitHub issues](https://github.com/xpepper/copilot-powerline/issues);
completed items live in git history.

## Validate new features

- [ ] Tune the session-cost spike warning, then decide whether to turn it
      on by default. It ships opt-in (`spike_alert = false`) because the
      thresholds (`spike_ratio = 2.0`, `spike_min_usd = 0.05`) are guesses.
      Enable it for a few days of real sessions and note how often it fires.
      Expected false alarms: the first turn after the prompt cache expires
      (about 5 idle minutes for Claude, 30 for GPT-5.6 and later, see #67;
      the whole context is rewritten) and output-heavy steps (output tokens
      cost more than input). The cache trend arrow and `--toggle` are
      verified live. Enabled 2026-10-08, not yet seen live, likely because
      the verdict only lasts until the next step: replayed through 0.7.1,
      the 2026-10-09 cold restart (one call at 19x the usual cost) shows
      `📈`, and the next call 9 s later clears it.

- [ ] Validate the idle cache-expiry warning (`cache_expiry`, on by
      default). Confirmed live with Claude (2026-10-09). Still open: the
      300 s lifetime is wrong for GPT models and the size shown is too
      low (#67); is the 50k `min_tokens` floor right; a long tool run
      (e.g. a 10-minute build) also triggers it, which is accurate but may
      read oddly mid-turn; it could silence the spike warning's expected
      alarm on the first turn after idle.

- [ ] Validate state-file pruning live (shipped in 0.7.1, PR #51). The
      `last_prune` marker exists (2026-10-09). The cache dir (177 files,
      oldest 2026-09-15) should first shrink after 2026-10-15; confirm the
      files of live sessions survive, and the `mode` override if one is
      set (it only exists while toggled away from the configured mode).
      Open question: the 30-day cutoff deletes the `idle_*` snapshot of a
      session idle that long, so resuming it hides the cache-expiry
      warning for one step; raise `MAX_AGE` in `src/state.rs` (e.g. 90
      days) if that ever matters.

- [ ] Validate the opt-in `cycle_cost` segment (PR #73, #74, unreleased)
      against the GitHub billing page: the total, the budget vs unlimited
      display, and the behaviour after the cycle resets.

## CI

- [ ] Check the first release after the runner changes (PR #69, merged;
      no release cut since 0.7.1). Neither
      workflow can run its release path in a PR, so both are unverified there:
      - `release.yml` now builds on `ubuntu-24.04` / `ubuntu-24.04-arm`
        (set in `dist-workspace.toml`; 22.04 is retired 2027-04-17).
        Linux binaries now need glibc 2.39 instead of 2.35: confirm the
        shell installer still works on the oldest distro you care about.
      - `publish-crates.yml` runs on `ubuntu-latest`, which GitHub moves to
        26.04 between 2026-10-19 and 2026-11-19 (`ci.yml` is green on it).
        Its steps (`jq`, `curl`, `cargo metadata`, the crates.io auth
        action) are untested there; if they break, pin `ubuntu-24.04`.
