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

- [ ] Validate the long-break reminder (`cache_expiry`, on by default,
      30 idle minutes since #67). Check it stays quiet through ordinary
      pauses, shows when you come back after 30+ minutes, and clears with
      the first new model call. Still open: is the 50k `min_tokens` floor
      right.

- [ ] Validate the last-miss label (`cache`, `· miss 133k`, on by
      default, #83). Check it appears after the first turn following a
      long break, survives the next well-cached call, and clears after
      5 minutes. Watch for false alarms from large tool outputs or file
      reads (they count too, as they cost the same), and whether the 50k
      `miss_min_tokens` floor and 5-minute window feel right.

- [ ] Watch for stale status line payloads. On 2026-10-10 the status line
      still showed the totals from before a 54-minute break ($0.62, 94%,
      2.0M) after a 2-minute turn had finished, so the idle clock kept
      running through active work (#81). Copilot CLI 1.0.95 should refresh
      on every usage event; cause not found. If it recurs, log each payload
      with a timestamp (a wrapper around the `statusLine` command) to tell
      a stale payload from a missed repaint.

- [ ] Validate state-file pruning live (shipped in 0.7.1, PR #51). The
      `last_prune` marker exists (2026-10-09). The cache dir (177 files,
      oldest 2026-09-15) should first shrink after 2026-10-15; confirm the
      files of live sessions survive, and the `mode` override if one is
      set (it only exists while toggled away from the configured mode).
      Open question: the 30-day cutoff deletes the `idle_*` snapshot of a
      session idle that long, so resuming it hides the long-break
      reminder for one step; raise `MAX_AGE` in `src/state.rs` (e.g. 90
      days) if that ever matters.

- [ ] Validate the opt-in `cycle_cost` segment (shipped in 0.8.0, PRs
      #73 and #74) against the GitHub billing page: the total, the budget
      vs unlimited display, and the behaviour after the cycle resets.

## CI

- [ ] Finish checking the runner changes (PR #69). The 0.8.0 release ran
      `release.yml` on `ubuntu-24.04` / `ubuntu-24.04-arm` (set in
      `dist-workspace.toml`; 22.04 is retired 2027-04-17) and every job
      passed, including the crates.io and Homebrew publish. Still open:
      - Linux binaries now need glibc 2.39 instead of 2.35: confirm the
        shell installer still works on the oldest distro you care about.
      - `publish-crates.yml` runs on `ubuntu-latest`, still 24.04 for
        0.8.0. GitHub moves it to 26.04 between 2026-10-19 and 2026-11-19
        (`ci.yml` is green on it). Check the first release after the move:
        its steps (`jq`, `curl`, `cargo metadata`, the crates.io auth
        action) are untested there; if they break, pin `ubuntu-24.04`.
