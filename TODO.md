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
      default) in real sessions. Seen live on 2026-10-09: it appeared
      during an 8-minute break with Claude and cleared on the next step,
      which really did rewrite the whole context. The 300 s lifetime is
      wrong for GPT models and the size shown is too low: tracked in #67.
      Still open: is the 50k `min_tokens` floor right; a long tool run
      (e.g. a 10-minute build) also triggers it, which is accurate but may
      read oddly mid-turn; it could silence the spike warning's expected
      alarm on the first turn after idle.

- [ ] Validate state-file pruning live (shipped in 0.7.1, PR #51).
      Installed locally late on 2026-10-09, after that day's last status
      line refresh, so the cache dir has no `last_prune` marker yet; the
      first refresh with 0.7.1 creates it. The dir (148 files, oldest
      2026-09-15) should first shrink after 2026-10-15; confirm the files
      of live sessions survive, and the `mode` override if one is set (it
      only exists while toggled away from the configured mode). Open
      question: the 30-day cutoff deletes the `idle_*` snapshot of a
      session idle that long, so resuming it hides the cache-expiry
      warning for one step; raise `MAX_AGE` in `src/state.rs` (e.g. 90
      days) if that ever matters.

## CI

- [ ] Check `publish-crates.yml` on Ubuntu 26 before the next release.
      GitHub rolls `ubuntu-latest` over to Ubuntu 26.04 gradually, from
      2026-10-19 to 2026-11-19. `ci.yml` was verified green on
      `ubuntu-26.04` on 2026-10-09 (all jobs, PR #69), so nothing to do
      there. `publish-crates.yml` only runs on release, so its steps
      (`jq`, `curl`, `cargo metadata`, the crates.io auth action) are
      unverified on 26.04; the image ships jq 1.8.1 and curl 8.18. If it
      breaks, pin `ubuntu-24.04` there. `release.yml` runs on
      `ubuntu-22.04` (dist's default, not set in `dist-workspace.toml`).
      GitHub retires that image on 2027-04-17, with brownouts from
      2027-03-23: upgrade dist or set a custom runner before then.
