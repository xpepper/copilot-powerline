# TODO: copilot-powerline

Short-lived checks and memos only, highest priority first. The backlog lives
in [GitHub issues](https://github.com/xpepper/copilot-powerline/issues);
completed items live in git history.

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

- [ ] Validate state-file pruning live (shipped in 0.7.1, PR #51; not yet
      installed locally). Install 0.7.1, then the local cache dir (~145
      files, oldest 2026-09-15) should first shrink after 2026-10-15;
      confirm `mode` and the files of live sessions survive. Open question:
      the 30-day cutoff deletes the `idle_*` snapshot of a session idle
      that long, so resuming it hides the cache-expiry warning for one
      step; raise `MAX_AGE` in `src/state.rs` (e.g. 90 days) if that ever
      matters.

## CI

- [ ] Check CI after GitHub moves `ubuntu-latest` to Ubuntu 26, from
      2026-10-19 (annotation on the v0.7.0 release run). Affects `ci.yml`
      and `publish-crates.yml`; `release.yml` pins `ubuntu-22.04`. Nothing
      to do before then. A `publish-crates.yml` break would only surface
      on the next release, so check it before tagging one.
