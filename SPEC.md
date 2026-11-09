# Add a warning level to the tokens segment

GitHub Issue: https://github.com/xpepper/copilot-powerline/issues/113

## Context

The `tokens` segment has one `alert_threshold` (default 100k). Below it the count is plain, above it the count turns to the alert color with an icon. One level forces a choice between an early alert that stays on for most of a long session and a late alert with no heads-up.

## Intent

Give the segment two levels: a warning, then an alert. A growing context gets a heads-up before the alert. Both thresholds stay absolute token counts, as today. The `alert_threshold` default moves from 100k to 200k, and the new `warning_threshold` defaults to 100k.

## Out of scope

- Thresholds as a percentage of the context window.
- Other segments' thresholds.
- A warning color for the `plain` theme, which has no colors: there a warning looks like a normal count.

## Acceptance criteria

- AC1: When the count is at or below `warning_threshold`, it shows in the normal color with no icon.
- AC2: When the count is above `warning_threshold` and at or below `alert_threshold`, it shows in the warning color with no icon.
- AC3: When the count is above `alert_threshold`, it shows in the alert color with `alert_icon`.
- AC4: When `warning_threshold` is above `alert_threshold`, a count above `alert_threshold` is an alert and no count is a warning.
- AC5: Every theme except `plain` has a warning color.
- AC6: When a config sets only `alert_threshold`, it loads and that value keeps its meaning.
- AC7 (manual): The example config and the README describe both thresholds and the new defaults.

## Plan

1. **refactor: use the default helpers in `TokensConfig::default`** (enabling) — infrastructure (config). The alert default is written twice, as a literal and in `default_alert_threshold`. Adding a second threshold would repeat that.
2. **warning color in every theme** (AC5) — inside-out: follows `tokens_alert` in `Palette::for_theme`. Touches presentation. Yellow in each colored theme; empty in `plain`.
3. **warning level in the tokens segment** (AC1, AC2, AC3, AC4, AC6) — inside-out: follows the existing `alert_threshold` and `alert_icon` handling in `TokensConfig` and `render_tokens_segment`. Touches application, infrastructure (config).
   - `warning_threshold` defaults to 100k and `alert_threshold` to 200k, for a missing or partial `[tokens]` section alike
   - alert is checked first, then warning, then normal, with strict `>` on the raw count
4. **document both thresholds** (AC7) — inside-out: edits `examples/powerline.toml` and the `tokens` row and features line in `README.md`. A config test already checks the example's keys and default comments.

## Design notes

- **Where it belongs:** level choice in `render_tokens_segment` (`src/segments/tokens.rs`), thresholds and defaults in `TokensConfig` (`src/config.rs`), colors in `Palette` (`src/theme.rs`, new `tokens_warning`).
- **Existing behaviour and abstractions to reuse:** `format_tokens`, `alert_icon` handling, Serde field defaults, `Palette::for_theme`, and the config tests `test_example_config_uses_only_known_keys` and `test_example_config_default_comments_match_the_defaults`.
- **Boundaries that must hold:** compare the raw token count with strict `>`; the alert wins when the thresholds are reversed, with no rejecting or reordering; only an alert adds `alert_icon`; the shared `tokens_alert` color stays as it is (spend alerts, the long-break reminder and unresolved threads also use it); no level logic in `src/main.rs` or `src/renderer.rs`; no new I/O or dependencies.
- **Other modules or consumers affected:** `src/main.rs` dispatch needs no change; `--init` picks up the new setting through config serialization (`tests/init_wiring.rs`); `test_render_tokens_alert` expects an alert at 150k and must change with the new default; README features line, `tokens` legend row and the 145k alert preview.
- **Enabling refactorings (planned):** item 1.
- **Deferred:** none. A shared alert-level abstraction across segments was ruled out: spend compares rounded currency, branch size adds an icon only, and the long-break reminder controls visibility.
- **Open questions:** exact warning colors; yellow is distinct from the red alert, but check it in `colorblind`.

## Progress

- [x] 1. refactor: use the default helpers in `TokensConfig::default`
  - `TokensConfig::default` now calls `default_alert_threshold()`; behaviour unchanged, covered by the existing default tests
- [x] 2. warning color in every theme (AC5)
  - `Palette::tokens_warning` is bright yellow (`\x1b[93m`) in github, nord, tokyo-night and colorblind, empty in `plain`. Read by the segment in item 3, so no dead-code allowance
- [x] 3. warning level in the tokens segment (AC1, AC2, AC3, AC4, AC6)
  - `TokensConfig` gained `warning_threshold` (default 100000). `alert_threshold` default moved from 100000 to 200000
  - Levels are strict `>`: alert is checked first, then warning, then normal. Reversed thresholds never warn (AC4)
  - Decision: items 2 and 3 were built together, because an unread palette field fails `clippy -D warnings`
- [x] 4. document both thresholds (AC7)
  - `examples/powerline.toml` lists `warning_threshold` with its default; README feature line and `tokens` row updated

- The final design review was skipped

### Acceptance

- AC1: met. `test_render_tokens_normal`, `test_render_tokens_at_warning_threshold_is_normal`
- AC2: met. `test_render_tokens_above_warning_threshold_is_warning_without_icon`
- AC3: met. `test_render_tokens_alert`, `test_render_tokens_above_alert_threshold_is_alert_with_icon`
- AC4: met. `test_render_tokens_reversed_thresholds_have_no_warning`
- AC5: met. `test_tokens_warning_colour_is_set_in_every_theme_except_plain`
- AC6: met. `test_tokens_section_with_only_alert_threshold_keeps_it`
- AC7: met (manual, confirmed by the user)
- Open: colorblind suitability of the yellow warning was not checked visually. The user chose to keep it
- Note: the `alert_threshold = 100000` in the user's own `~/.copilot/powerline.toml` overrides the new default. Their warning and alert then both start at 100k, so no count shows as a warning there
