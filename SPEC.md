# Add an optional branch size segment

GitHub Issue: https://github.com/xpepper/copilot-powerline/issues/109

## Context

Copilot CLI's built-in `changes` item counts the lines changed in this session, not on the branch.
A branch built over several sessions looks small in each of them, so a pull request can grow too
big to review without anyone noticing. Codex CLI has a similar `branch-changes` item.

## Intent

An optional `branch_size` segment shows how big the current branch is compared with the remote
default branch, e.g. `+420 -80 · 12 files vs main · +35 -2 uncommitted`, while the agent is still
adding to it. The committed part matches what the pull request will show. The uncommitted part
shows what is still coming. Working out the size never slows down the status line.

## Out of scope

- Sharing one background `git` refresh with #108 (time since last commit). Each can be merged into
  one worker later.
- Counting untracked files (never added with `git add`) in the uncommitted part.
- Comparing with the local default branch, or using the PR size from `gh pr view`.
- A file count for the uncommitted part.
- A spend-style alert icon: the alert is a color only.

## Acceptance criteria

Showing the size

- AC1: When `branch_size` is listed in `segments` and the branch has commits not on
  `origin/<default>`, it shows `+<added> -<removed> · <n> files vs <default>`.
- AC2: When exactly one file changed, the count reads `1 file`, not `1 files`.
- AC3: The committed size is measured from the merge base with `origin/<default>`, so commits merged
  into `origin/<default>` after the branch started do not count.
- AC4: When tracked files have uncommitted changes, the segment adds `· +<added> -<removed>
  uncommitted`, staged and unstaged changes together.
- AC5: When only uncommitted changes exist, the committed part reads `+0 -0 · 0 files vs <default>`
  and the uncommitted part follows it.
- AC6: `branch_size` is not in the default `segments` list, and `enabled = false` under
  `[branch_size]` hides it.

Default branch

- AC7: When `default_branch` is set under `[branch_size]`, the segment compares with
  `origin/<default_branch>`.
- AC8: When `default_branch` is not set, the default branch is read from `origin/HEAD`, also in a
  linked worktree.
- AC9: When `default_branch` is not set and `origin/HEAD` does not exist, the segment is hidden.

When it is hidden

- AC10: When the current branch is the default branch, or HEAD is detached, the segment is hidden.
- AC11: When the branch has no committed and no uncommitted changes, the segment is hidden.
- AC12: Outside a git repository, when `git` is not installed, or when `origin/<default>` does not
  exist, the segment is hidden and nothing errors.

Refreshing without blocking

- AC13: A refresh never runs `git diff` itself: it reads a cached size, and a missing or stale
  cache starts a throttled background refresh.
- AC14: Until the first background refresh finishes, the segment is hidden. After that, a stale
  size is shown until the next refresh replaces it.
- AC15: A cached size is fresh for `cache_ttl_seconds` (default 60), and each repository and branch
  has its own cache.
- AC16: A background `git` call that hangs is stopped before the next refresh may start.

Alert

- AC17: When `alert_above_lines` is set and committed plus uncommitted added and removed lines go
  strictly above it, the segment shows in the alert color.
- AC18: When `alert_above_lines` is not set, which is the default, the segment never shows the
  alert color.

Docs

- AC19 (manual): The README and `examples/powerline.toml` describe the segment, its settings, and
  when it is hidden.

## Plan

1. **refactor: move the background-refresh code out of `github.rs`** (enabling) — infrastructure.
   Move `try_claim_spawn`, `should_throttle_spawn`, `lock_file_path`, `is_lock_fresh`,
   `create_lock`, `output_with_timeout` and `GH_TIMEOUT` (renamed `WORKER_TIMEOUT`) with their
   tests to a new `src/worker.rs`. `github.rs` and `cycle_usage.rs` import from it. A pure move.
   In the same commit, update AGENTS.md: a Non-Obvious Locations line for `src/worker.rs`, and
   rule 1 points to it for the throttle, lock and timeout helpers.
2. **refactor: one throttle window** (enabling) — infrastructure. A single
   `worker::THROTTLE_SECONDS` replaces the local `15` in `github::get_pr_info` and
   `cycle_usage::THROTTLE_SECONDS`, next to `WORKER_TIMEOUT`.
3. **refactor: name per-branch state files in `state.rs`** (enabling) — infrastructure.
   `state::branch_file(kind, repo_dir, branch)`, twin of `session_file`, with the same
   `PRUNABLE_KINDS` check. `github::get_cache_file_path` delegates to it with the same hash inputs,
   order and `pr_` prefix, so existing caches keep their names.
4. **refactor: tell a branch from a detached HEAD in `git.rs`** (enabling) — infrastructure.
   `git::read_head` returns `Head::Branch(name)` or `Head::Detached(short_sha)`;
   `get_git_branch` maps both to today's strings, so `pr` is unchanged.
5. **refactor: check any program on PATH** (enabling) — infrastructure. `worker::is_on_path(program)`
   keeps the Windows `.exe` case; `github::is_gh_available` delegates to it.
6. **show the committed branch size** (AC1, AC3, AC6, AC7, AC8, AC9, AC13, AC14, AC15, AC16) —
   outside-in: nobody has run this repository's worker pattern against real `git` (merge base,
   `git symbolic-ref` for `origin/HEAD` in a linked worktree, `--shortstat` output when a part is
   zero). Touches infrastructure (new `src/branch_size.rs` cache and worker, `--fetch-branch-size`
   flag, `main.rs` wiring, `BranchSizeConfig`) and the new `src/segments/branch_size.rs`. The
   wiring test uses a real `git` repository with a local bare repository as `origin`.
   - slice 1: with `branch_size` listed, a branch with commits shows `+a -r · n files vs main`
     after the background refresh; the refresh itself never runs `git`, and nothing shows before
     the first refresh (AC1, AC3, AC6, AC13, AC14)
   - slice 2: the cache is per repository and branch, stays fresh for `cache_ttl_seconds`, a stale
     size shows until replaced, and a hung `git` is stopped by `WORKER_TIMEOUT` (AC14, AC15, AC16)
   - slice 3: `default_branch` set, `origin/HEAD` read in a linked worktree, and hidden when neither
     gives a default branch (AC7, AC8, AC9)
7. **add the uncommitted part** (AC4, AC5) — inside-out: follows the worker and cache entry built
   in item 6, plus one more `git diff --shortstat HEAD`. Touches infrastructure and the segment.
8. **hide the segment and spell `1 file`** (AC2, AC10, AC11, AC12) — inside-out: follows
   `segments::pr::is_visible` and the `Head` from item 4; the `git` check uses `is_on_path` from
   item 5. Touches the segment and `main.rs`.
9. **alert above a line count** (AC17, AC18) — inside-out: follows the optional limit of
   `alert_above_usd` and the `Palette::tokens_alert` colour used in `segments/spend_total.rs`.
   Touches the segment and `BranchSizeConfig`.
10. **document the segment** (AC19) — README segment table and background-lookups section,
    `examples/powerline.toml`, `GLOSSARY.md` entries for the branch size and its committed and
    uncommitted parts, and a `.cargo/mutants.toml` exclusion for the new spawn function, as
    `cycle_usage` has.

## Design notes

- **Where it belongs:** like `cycle_cost`. `src/branch_size.rs` owns the cache entry, the hot-path
  read (`get_branch_size(cache_path, ttl, now, spawn_refresh)` with an injected spawn, as
  `src/cycle_usage.rs` — `get_cycle_usage`), the worker action and the `--shortstat` parsing.
  `src/segments/branch_size.rs` owns `NAME`, `is_visible` and `render_branch_size_segment`, as
  `src/segments/cycle_cost.rs`. `BranchSizeConfig` sits in `src/config.rs` next to `PrConfig`.
  The worker resolves the default branch (`default_branch`, else `git symbolic-ref
  refs/remotes/origin/HEAD`) and stores it in the cache entry, so the hot path parses no refs.
- **Existing behaviour and abstractions to reuse:** `src/github.rs` — `try_claim_spawn`,
  `output_with_timeout`, `get_cache_file_path`, `is_cache_fresh`, `current_timestamp` (moved or
  wrapped by items 1 to 5); `src/cycle_usage.rs` — `get_cycle_usage`, `spawn_background_fetch`,
  `fetch_and_write_cycle_cache`; `src/git.rs` — `find_repo_root`, `get_git_branch`;
  `src/state.rs` — `base_dir`, `write_private_atomic`, `PRUNABLE_KINDS`;
  `src/segments/mod.rs` — `is_listed`, `test_default_segments_use_segment_names`;
  `src/theme.rs` — `Palette::tokens_alert`; `tests/pr_wiring.rs` — `wait_for_worker`;
  `tests/common/mod.rs` — `Sandbox`, `state_files`.
- **Boundaries that must hold:** the refresh never spawns `git` (AGENTS.md rule 1); only the
  re-invoked worker does. `WORKER_TIMEOUT` stays shorter than `THROTTLE_SECONDS`. The new cache
  kind is added to `PRUNABLE_KINDS`. Top-level modules never depend on a segment module. One
  setting, one name (`default_branch` → `--default-branch`), and at most one `bool` parameter per
  function (CODING_STANDARDS.md).
- **Other modules or consumers affected:** `src/main.rs` (worker dispatch, and the repo root and
  branch lookup now inside the `pr` block is shared by both segments), `src/cli.rs` (new flag;
  `--repo-dir` becomes shared), `src/config.rs` (default TOML, so `tests/init_wiring.rs` may move),
  `src/state.rs` tests, `README.md`, `examples/powerline.toml`, `GLOSSARY.md`,
  `.cargo/mutants.toml`, and the architecture map (refresh after merge).
- **Enabling refactorings (planned):** items 1 to 5.
- **Deferred:** `current_timestamp` living in `github.rs` while seven other modules use it. (The
  read-cache, claim, spawn flow was extracted in session 2: `worker::refresh_if_stale`.)
- **Open questions:** none blocking. Changing `default_branch` shows the old comparison until the
  cache expires (at most `cache_ttl_seconds`), which AC15 accepts.

## Progress

Session 1 (enabling items 1 to 5, all committed):

- [x] 1. `src/worker.rs` holds the claim, lock and timeout helpers (`GH_TIMEOUT` is now
  `WORKER_TIMEOUT`). AGENTS.md points to it.
- [x] 2. `worker::THROTTLE_SECONDS` is the one throttle window. A compile-time assert keeps
  `WORKER_TIMEOUT` below it.
- [x] 3. `state::branch_file`. PR cache names are unchanged: a temporary test, since removed,
  compared them with the old function. Two `github` path tests that `branch_file`'s test now covers
  were removed. Their private-directory check moved to a `base_dir` test.
- [x] 4. `git::read_head` returns `Head::Branch` or `Head::Detached`. `get_git_branch` maps both to
  the same strings as before.
- [x] 5. `worker::is_on_path(program)`. `github::is_gh_available` delegates to it.

Session 2 (item 6, committed):

- [x] 6. The committed branch size, built outside-in.
  - Item 6 slices built: slice 1, the committed size through the real worker and `git`; slice 2,
    the per-branch cache, TTL, stale size and hung `git`; slice 3, `default_branch`, `origin/HEAD`
    in a linked worktree, hidden without a default branch.
  - Item 6 slice 2 found nothing left to build: slice 1 already reused the shared refresh helper
    and the worker timeout. Its tests passed on first run and confirm the behaviour.

Session 3 (items 7 to 10, and the final design review, committed):

- [x] 7. The uncommitted part: one more `git diff --numstat HEAD` in the worker, rendered as
  `· +a -r uncommitted`. A change with no lines (a binary file) still shows it, because AC4 asks
  whether tracked files changed, not whether lines did.
- [x] 8. Hidden on the default branch, on a detached HEAD, with no changes, and without `git` (no
  worker starts). `1 file` is singular. The "on the default branch" rule lives in the segment: only
  the cache entry knows the default branch, so `render_branch_size_segment` takes the current
  branch. `main` now reads `git::Head` itself, and `git::get_git_branch` is removed: `pr` maps both
  kinds of `Head` to a name in `main`, as before.
- [x] 9. `alert_above_lines` in `[branch_size]`: the whole segment in `tokens_alert` once committed
  plus uncommitted lines go strictly above it.
- [x] 10. README (feature bullet, segment row, "Branch size lookups"), `examples/powerline.toml`,
  three `GLOSSARY.md` entries. `default_branch` stays commented out in the example, because a wrong
  value hides the segment.
- Final design review, five findings. Applied: `BranchSize::is_empty` and `changed_lines`;
  `worker::spawn_detached` for all three workers (only the `cycle_usage` mutants exclusion is left);
  `worker::Deadline` shared across a worker's commands; `Sandbox::wait_for_worker` in
  `tests/common`. AGENTS.md's `src/worker.rs` line names `refresh_if_stale`, `spawn_detached` and
  `Deadline`. Deferred: moving `current_timestamp` (below).
- Each item again landed as one `feat` commit then its `test` commit, except item 9, whose config
  commit stands alone. The review refactors are separate `refactor` commits: the `BranchSize` one
  belongs with the item 8 and 9 feature commits; the other three change code from before the branch.

Decisions:

- Design notes that no longer hold: `try_claim_spawn` and `output_with_timeout` live in
  `src/worker.rs`, `is_cache_fresh` and `get_git_branch` are gone, `tests/init_wiring.rs` did not
  change, and `tests/pr_wiring.rs`'s `wait_for_worker` became `Sandbox::wait_for_worker`.

- The worker parses `git diff --numstat`, not `--shortstat`: `git` translates the `--shortstat`
  line into the user's language. A binary file counts as a file with no lines, as on GitHub.
- New stand-alone refactor `worker::refresh_if_stale`: the stale check, claim and spawn were
  written three times (`github::get_pr_info`, `cycle_usage::get_cycle_usage`, and the new
  `branch_size::get_branch_size`). The user chose to refactor now. `github::is_cache_fresh` is gone.
- `BranchSize` holds a `committed: DiffStat`, so item 7 can add the uncommitted part beside it.
- `main` looks up the repo root and branch once for `pr` and `branch_size`. It still uses
  `git::get_git_branch`, so a detached HEAD is measured until item 8 hides it.
- Each slice lands as one `feat` commit then its `test` commit. A split by module would leave
  unused code, which `clippy -D warnings` rejects.

Acceptance:

- AC1: met — `test_shows_the_committed_size_after_the_background_refresh`
- AC3: met — the same test, with a commit pushed to `origin/main` after the branch started
- AC6: met — `test_branch_size_is_opt_in_and_fresh_for_a_minute`,
  `segments::branch_size::test_hidden_when_disabled`
- AC7: met — `test_compares_with_the_configured_default_branch`
- AC8: met — `test_reads_the_default_branch_in_a_linked_worktree`
- AC9: met — `test_hidden_without_a_default_branch`
- AC13: met — the first wiring test (every `git` call comes from the worker),
  `worker::test_a_missing_cache_is_refreshed_once_per_throttle_window`
- AC14: met — the first wiring test (nothing before the first refresh),
  `test_a_stale_size_is_shown_until_the_next_refresh_replaces_it`
- AC15: met — `test_a_fresh_size_is_shown_without_measuring_again`,
  `test_each_branch_shows_its_own_size`, the 60 s default in the config test
- AC16: met — `test_a_hung_git_is_stopped_before_the_next_refresh_may_start`
- AC2: met — `segments::branch_size::test_one_changed_file_is_singular`, and the wiring tests
  expecting `1 file`
- AC4: met — `test_adds_staged_and_unstaged_changes_as_uncommitted`
- AC5: met — `test_only_uncommitted_changes_show_an_empty_committed_part`
- AC10: met — `test_hidden_on_the_default_branch`, `test_hidden_on_a_detached_head`
- AC11: met — `test_hidden_without_changes` (wiring and segment)
- AC12: met — `test_hidden_outside_a_repository`, `test_hidden_without_git_and_never_starts_a_worker`,
  `test_hidden_when_the_default_branch_is_not_on_origin`
- AC17: met — `test_shows_the_alert_color_above_the_line_limit`,
  `segments::branch_size::test_alert_color_above_the_line_limit`, `test_no_alert_color_at_the_line_limit`
- AC18: met — `segments::branch_size::test_no_alert_color_without_a_line_limit`, the default in
  `config::test_branch_size_is_opt_in_and_fresh_for_a_minute`
- AC19: met — confirmed by the user: the README bullet, segment row and "Branch size lookups"
  section, the `[branch_size]` example section, and the three glossary entries

Verification (session 3): `cargo fmt --check`, `cargo clippy --all-targets -D warnings`,
`cargo test` (378 passed), `scripts/check-machine-paths`, both smoke runs from AGENTS.md, and the
segment on this branch, which matched `git diff --shortstat origin/main...HEAD`.
`scripts/mutants-diff origin/main` (run in a clean worktree, as the untracked `.claude/` folder
blocks it here): 116 mutants, 101 caught, 10 unviable, 5 missed. Three were real gaps and now have
killing tests (`pr` lookup when only `branch_size` is listed, no PR worker without `gh`, a claim
exactly one throttle window old); each was checked by applying the mutant by hand. Two are
equivalent and excluded in `.cargo/mutants.toml` with the reason (`should_throttle_spawn`, the
`output_with_timeout` deadline comparison). The full run was not repeated after those fixes.

Deferred refactorings:

- Move `current_timestamp` out of `src/github.rs`, into `src/worker.rs` or `state.rs`.
  - Observation: modules with no GitHub concern call `crate::github::current_timestamp`, and the
    shared `worker.rs` tests import it from one of the modules that depend on `worker.rs`.
  - Evidence: `src/github.rs` — `current_timestamp`; `src/worker.rs` — `tests`;
    `src/branch_size.rs` — `fetch_and_write_branch_size`; `src/main.rs` — its call sites
  - Competing explanations: harmless as is (test build only for `worker.rs`); not part of this
    feature, so it would widen the diff over many call sites
  - Assumptions: none. Confidence: high on the evidence.
  - What would change it: nothing new; `worker.rs` is now the obvious home.
  - Recommendation: a move and rename of the one function. Risk: mechanical, no behaviour change.
  - Why deferred: already parked before this session; unrelated to the branch size.

Noticed, not changed: `git::read_head` slices the detached SHA by bytes (`line[..7]`), which
panics on a `HEAD` with a multi-byte character in its first 7 bytes. This predates the branch.
