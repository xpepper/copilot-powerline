# Color the added and removed lines in the Branch Size segment

## Context

The `branch_size` segment shows its line counts in the terminal's default color, for example `+420 -80 · 12 files vs main`. Git and GitHub show added lines in green and removed lines in red, so people expect that code. Without it, the counts are slower to read.

## Intent

Show the added count in green and the removed count in red, in both the Committed Part and the Uncommitted Part. Follow what the Themes already do for color: `colorblind` uses blue and orange instead of green and red, and `plain` has no color. Above the line limit, the segment keeps those colors and starts with an alert icon, in place of today's alert color.

## Out of scope

- Changing the segment's text, its order, or when it is hidden, apart from the alert icon.
- A setting to turn the colors off or to choose them.
- Coloring the file count, `vs <branch>` or the word `uncommitted`.

## Acceptance criteria

- AC1: When a Branch Size shows in the `github`, `nord` or `tokyo-night` Theme, each added count, with its `+`, is bright green.
- AC2: When a Branch Size shows in the `github`, `nord` or `tokyo-night` Theme, each removed count, with its `-`, is bright red.
- AC3: When a Branch Size shows in the `colorblind` Theme, each added count is bright blue and each removed count is orange.
- AC4: When a Branch Size shows in the `plain` Theme, the output has no color codes and its text is unchanged.
- AC5: When the Uncommitted Part shows, its added and removed counts are colored like those of the Committed Part.
- AC6: When the changed lines go above `alert_above_lines`, the segment starts with the alert icon and its counts keep their normal colors.
- AC7: When a count is zero, such as `-0`, it is colored like any other count.
- AC8: The file count, `vs <branch>` and `uncommitted` keep the terminal's default color.
- AC9 (manual): The README's Branch Size description says which counts are colored and that going above the limit adds an icon.
- AC10: The alert icon is 🐘 by default, and `alert_icon` under `[branch_size]` replaces it.

## Plan

1. **color the line counts in the segment** (AC1, AC2, AC3, AC4, AC5, AC6, AC7, AC8): inside-out, follows how `src/segments/cache.rs` uses `palette.trend_up` and `palette.trend_down`. Touches domain.
   - Those two Palette fields already hold the wanted colors in every Theme, so `src/theme.rs` does not change.
   - The change is in `describe` and `lines` in `src/segments/branch_size.rs`, with unit tests beside them.
   - The acceptance test goes in `tests/branch_size_wiring.rs`: a real refresh in the `github` Theme shows the colored counts. The existing `plain` and alert wiring tests pin AC4 and AC6.
2. **replace the alert color with an icon** (AC6, AC10): inside-out, follows `alert_icon` in `src/config.rs` and its use in `src/segments/tokens.rs`. Touches domain.
3. **document the colors and the icon** (AC9): inside-out, follows the alert sentence in the README's Branch Size row. Touches documentation only.

The enabling refactoring analysis was skipped.

## Progress

- [x] Item 1: color the line counts in the segment.
- [x] Item 2: replace the alert color with an icon.
- [x] Item 3: document the colors and the icon.

Acceptance:

- AC1: met — `test_colors_the_added_lines_green_and_the_removed_lines_red` (all three Themes), `test_colors_the_added_and_removed_lines` (real refresh)
- AC2: met — the same two tests
- AC3: met — `test_colors_the_counts_blue_and_orange_in_the_colorblind_theme`
- AC4: met — `test_shows_lines_and_files_against_the_default_branch`, and the wiring tests that use the `plain` Theme
- AC5: met — `test_colors_the_uncommitted_counts_like_the_committed_ones`, `test_colors_the_added_and_removed_lines`
- AC6: changed — the user asked for an icon in place of the alert color when reviewing the README. Met by `test_starts_with_the_alert_icon_above_the_line_limit`, `test_shows_the_alert_icon_above_the_line_limit`, and the `test_no_alert_icon_*` tests for the limit itself
- AC7: met — `test_colors_a_zero_count_like_any_other`
- AC8: met — the tests above assert the whole line, so the file count, `vs <branch>` and `uncommitted` carry no color
- AC9: changed — it now names the icon instead of the alert color. Met: confirmed by the user, who read the README's Branch Size row
- AC10: met — `test_branch_size_alert_icon_defaults_to_an_elephant`, `test_shows_the_configured_alert_icon_in_the_plain_theme`, `test_shows_the_configured_alert_icon`

Decisions:

- Item 2 and AC10 were added during implementation, with the user's agreement. The Intent and Out of scope were updated to match.
- The icon is 🐘, chosen by the user. It goes first, like the other alert icons, and shows in every Theme.
- `alert_icon` under `[branch_size]` matches the same setting on `tokens`, `session_cost`, `month_cost` and `cycle_cost`.
- This segment no longer uses the alert color, unlike those segments. The user chose this.

The final design review ran over the whole feature and found no design problems in the code.

Deferred refactorings: none.
