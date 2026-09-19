# ADR-148: VUI-07 lock PIN entry -- height-proportional scaling fix

## Status

Accepted, 2026-09-19. Host-verified only (two-height regression test).
Not physically re-confirmed on Pixel 7 -- see Verification for why, and
what would close that gap.

## Context

VUI-07's "Restyle remaining lock chrome" task, continuing the sprint's
own ContextHeader/Field migration wave (ADR-127 through ADR-147).
`draw_lock_pin_entry` (the PIN-dots unlock screen, distinct from
`draw_lock_idle`'s no-PIN clock screen ADR-134 already restyled) had a
real, unrelated-to-ContextHeader bug: its dot row and title both used
bare 2400-reference pixel literals (`dot_y = 420`, title `y = 330`) that
never scaled with a real `height`, unlike:

- its own `keys` parameter, already `pin_keypad_rect(index, width,
  height)`-computed by the caller;
- its sibling `draw_lock_idle`, which already scales `time_y`/`hint_y`
  proportionally off `height`.

Found while auditing this file for an unrelated pre-merge review; not
something any of ADR-127 through ADR-147 touched, since none of them
migrate this specific screen.

## Decision

**Fixed**: `draw_lock_pin_entry` gained a `height: u32` parameter (both
call sites in `main.rs`'s `present_lock_pin_entry` already had `height`
in scope, just weren't passing it). `dot_y`/title `y` are now
`((height as u64 * LITERAL) / 2400) as u32`, the same proportional
pattern `draw_lock_idle` already established. Sizes (`dot_size`, `gap`,
the title's `32.0`) are deliberately left as literals -- this fixes the
real scaling gap, not the reference-device visual, which stays
pixel-identical at `height == 2400` (Pixel 7's own resolution).

**Deliberately not done**: this screen does not gain a `ContextHeader`
the way every other migrated screen in this wave did. The lock PIN entry
screen is shown *before* authentication -- `draw_lock_idle`'s own doc
comment already establishes the boundary this respects ("does not
invent attention or Inbox content"). A `ContextHeader` would show the
selected Space's name, which is exactly the kind of pre-auth context
exposure that boundary exists to prevent. "Restyle remaining lock
chrome" is read here as "fix what's actually broken" (the scaling gap),
not "apply the ContextHeader pattern indiscriminately to every screen
this sprint hasn't touched yet."

**Also found and fixed in the same pass, unrelated to lock chrome**: a
pre-existing clippy debt this same file had accumulated (`cargo clippy
--all-targets -D warnings` was not clean before this ADR):
- Four functions (`root_navigation_rect`, `intent_summary_from_entity`,
  `ensure_me_row_cache`, `me_fixture_facts`) are real, host-tested logic
  with no call site outside their own tests -- not evidence of
  abandonment, but genuinely staged/unwired work. Marked
  `#[allow(dead_code)]` with a comment explaining exactly that, rather
  than silently deleted (which would destroy tested logic on a guess
  about its intent) or silently left to fail the build.
- `draw_remote_pair`'s 8-argument signature: added the same
  `#[allow(clippy::too_many_arguments)]` this codebase's other
  8-argument draw functions (e.g. `draw_root`) already carry.
- Five `&[value.clone()]` call sites (three real, two duplicated under
  the test target) rewritten to `std::slice::from_ref(&value)` per
  clippy's own suggestion -- pure micro-cleanup, no behavior change.
- One real doc-comment drift, unrelated to any of the above: the
  paragraph describing `stacked_row_rect`'s own 190-tall/220-apart
  stacking convention had been stranded next to `NOW_GRID_COLUMNS` by
  an earlier reorg (content-matched, not guessed), leaving
  `stacked_row_rect` itself with no doc comment and `NOW_GRID_COLUMNS`
  documented by an unrelated paragraph. Moved to precede
  `stacked_row_rect`, its real, content-verified target.

## Verification

- `cargo test -p saai-shell`: 193/193 (192 existing + 1 new --
  `lock_pin_entry_dots_scale_with_height_not_a_2400_reference_literal`,
  which renders at `height` 2400 and 1200 and confirms the entered dot
  lands at the height-scaled position in both, not just the reference
  resolution).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean (was
  not, before this ADR -- 9 real findings across dead code, an
  arg-count lint, redundant clones, and the doc-comment drift above, all
  fixed in this same pass since this file was already open).
- `cargo test --workspace`: unchanged pass count everywhere else.
- Hot-swapped onto the live device (backup `saai-shell.pre-lockfix`,
  hash-verified). **Not physically confirmed**: reproducing this screen
  requires a real PIN configured (`ShellSettings.pin_code`), and the
  development device currently has none set (`pin_code: null`) --
  confirmed by reading the live settings file rather than assumed.
  Setting one specifically to screenshot this fix was judged a
  disproportionate, real change to the device's actual security
  configuration for a scaling-only fix already caught precisely by a
  two-height host test. `draw_lock_idle` (the currently-reachable lock
  screen with no PIN set) was screenshotted and confirmed unaffected --
  same canvas fill, same clock/hint layout as before this ADR.

## Consequences

- The lock PIN entry screen will now render correctly on any real
  panel height, not just the 2400 reference -- a real correctness
  fix, not just a lint satisfaction.
- This file's `cargo clippy --all-targets -D warnings` is clean again,
  restoring the bar this project has held throughout its own history.
- **Known gap, flagged rather than silently closed**: this screen has
  not been seen on a real device since this fix. If a PIN is ever
  configured on the development device for other testing, re-running
  `cargo test`'s own two-height check is not a substitute for one real
  screenshot -- add it then, and update this ADR's own Verification
  section rather than opening a new one for the same fix.
- The four `#[allow(dead_code)]` functions remain a real, open question
  for whoever next has context on what they were staged for -- wire them
  in, or remove them and their tests together, next time this specific
  code is touched.

## Rollback

`saai-shell.pre-lockfix` remains on-device. No settings/entity changes
involved -- purely rendering math and lint-satisfaction edits.

## Links

- ADR-134 -- `draw_lock_idle`, the sibling function whose own
  height-proportional pattern this fix brings `draw_lock_pin_entry` up
  to parity with.
- ADR-127 through ADR-147 -- the ContextHeader/Field migration wave this
  ADR continues numerically but deliberately does not extend to this
  specific pre-auth screen.
