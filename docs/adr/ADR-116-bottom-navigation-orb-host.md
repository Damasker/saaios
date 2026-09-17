# ADR-116: `BottomNavigation`/`OrbHost` composites, real Orb states

## Status

Accepted, 2026-09-17. Physically verified on Pixel 7 (Idle state
confirmed by screenshot; Active/Running/Attention/Offline confirmed by
priority-ordered unit tests and code-path inspection, not yet each seen
individually on a live device -- see Verification for exactly what was
and was not screenshotted).

## Context

VUI-04's first, third, and fifth tasks: a shared bottom-navigation
component with real interaction states (selected/pressed/disabled/
attention/badge), and a restrained Orb host with real
quiet/active/progress/attention/offline states. `component-library-v1.md`
section 3's own inventory table already named both composites
(`BottomNavigation`, `OrbHost`), deferred until this sprint.

## Decision

**`BottomNavigation`/`NavigationItem`** (component-library-v1.md section
7.4): one `NavigationItem` per destination (`id`, `label`, optional
`icon`, `selected`/`pressed`/`disabled`/`attention`/`badge: Option<u32>`).
`icon` stays optional rather than forced: this project's current icon set
(the Feather build, ADR-100) has no glyph yet for any of these
destinations, so the type does not pretend otherwise.

Wired into real rendering: `draw_tab_bar` (`render.rs`) now takes
`&[(Rect, NavigationItem)]` instead of `&[(Rect, &str)]` plus a separate
`selected: usize` index -- `selected`/`disabled` drive per-item color
directly from the item's own data, and a real `badge` count (when
`Some(n)` with `n > 0`) draws a small numeral swatch, colored with the
universal Attention color when `attention` is set. A new `Shell::
root_navigation_items(width, height)` builds the real list once, shared
by both `Frame::Root` and `Frame::Now`'s construction -- one navigation
strip, not two implementations that could drift apart. "Входящие"'s
badge is the exact same `inbox_rows(...).len()` count "Входящие" itself
already lists, not a separate tally.

`draw_root` keeps its own `selected: usize` parameter (used for its own
unrelated title-lookup and diagnostic-scaffold row-count logic) but no
longer forwards it to `draw_tab_bar`; `draw_now` drops the parameter
entirely, since it never used it for anything else.

**Honest, not silently dropped**: `pressed` and `disabled` are real
fields in the contract but have no real trigger anywhere in `saai-shell`
yet -- no touch-down tracking feeds `pressed`, and no tab is ever
actually disabled today. Read by `draw_tab_bar` for completeness (a
`disabled` item does render in muted colors, tested), but always `false`
in practice currently. Flagged as real follow-up, not claimed solved.

**`OrbHost`** (section 7.5): reuses `UniversalState` directly rather than
inventing a parallel five-state enum -- `Idle`/`Active`/`Running`/
`Attention`/`Offline` map onto the task's own "quiet/active/progress/
attention/offline" wording exactly, matching section 4's own rule against
inventing a local vocabulary. `reduced_motion` is a separate flag, not a
sixth state, matching `Progress`'s and `StatusIndicator`'s own treatment
of reduced motion as a rendering modifier.

Wired into real rendering: `draw_orb`'s old binary `is_attention: bool`
(a hand-rolled solid-square-or-hollow-ring) is replaced with a real
`mark: StatusMark`, drawn via the *existing* `draw_calibration_mark` --
the same function `StatusIndicator`'s own compact mark and the
calibration fixture already use, so every real Orb state gets a genuine,
distinct shape (`Outline`/`ActiveDot`/`Activity`/`Alert`/`Offline`)
instead of the old two-way split. A new pure function,
`orb_visual_state(appd_connected, entityd_connected,
has_pending_notifications, has_in_progress_work, menu_open) ->
UniversalState`, replaces the old shell-private `OrbState` enum entirely
(removed, along with `orb_state()` -- confirmed genuinely dead after the
replacement, not just unused in this one call site). Priority, most
urgent first: `Offline` (a live connection is required to trust any
other signal -- without one this shell cannot honestly claim to know
whether there is real pending attention or real in-progress work
either), `Attention` (undismissed notifications, unchanged data source),
`Running` (`in_progress_work`, the exact same VUI-03 query "Продолжается"
already uses), `Active` (the menu is open), else `Idle`.

Color still means context for the two non-urgent states (`Idle`/`Active`
keep the selected Space's own color) -- Context Light's own "context=
color, state=shape" split, state now fully carried by the mark alone, not
by also changing color for every state the way `Attention` alone used to.

## Verification

- `cargo test -p saai-ui-core`: 45/45 (42 existing + 3 new --
  `bottom_navigation_selected_index_finds_the_one_real_selection`,
  `bottom_navigation_selected_index_is_none_when_nothing_is_selected`,
  `orb_host_reuses_universal_state_and_is_busy_only_when_running_and_
  not_reduced`).
- `cargo test -p saai-shell`: 96/96, unchanged count (two `orb_state`
  tests removed alongside the dead enum, two `orb_visual_state` tests
  and a rewritten tab-bar render test added in their place; net zero).
- `cargo clippy --all-targets -- -D warnings`: clean on both crates.
- Hot-swapped onto the live device (backup `saai-shell.pre-bottomnav`,
  hash-verified). Real screenshot confirms: the tab bar is visually
  unchanged from before this ADR (no regression -- same highlight,
  colors, labels), and the Orb dot now shows `StatusMark::Outline` (a
  visible hollow-ring shape) for `Idle`, a real, physically-observed
  change from the old plain solid square.
- **Not individually screenshotted this pass**: `Active`/`Running`/
  `Attention`/`Offline`'s own distinct marks. `Attention`/`Offline` were
  already exercised as real states in this session (ADR-114's own
  `saai-entityd` disconnect test, and every space with an undismissed
  notification), just not re-screenshotted specifically for their new
  shape after this ADR; `Running` requires selecting a space with a
  genuinely in-progress task at the moment of capture (`personal` has one
  today, but the "first frame can predate entityd" characteristic ADR-112
  already documented made this awkward to catch reliably this pass) --
  `Active` (menu open) was not attempted at all. All four are covered by
  `orb_mark_shapes_differ_between_states`'s structural pixel-diff test
  and by `orb_visual_state`'s own priority-ordered unit tests, but that is
  not the same as a real device photo of each. Flagged rather than
  claimed complete.

## Consequences

- Both composites are real, tested, and running in production (not
  behind any dev gate) -- `RootPage`'s tab bar and the Orb dot both use
  them unconditionally now.
- `component-library-v1.md` section 3's inventory table entries for
  `BottomNavigation`/`OrbHost` move from "Deferred to VUI-04" to
  "Experimental (VUI-04)."
- The `OrbState`/`orb_state` removal is a clean deletion, not a
  deprecation -- nothing else referenced either name after the
  replacement went in, confirmed before deleting.
- VUI-04's remaining tasks (the `Я`→`Система` stage, the full Context
  Light grammar audit beyond what this ADR's color/shape split already
  gives Orb, layering-during-scroll guarantees, and rotation/inset/
  keyboard/rapid-tab-switch interaction tests) are unstarted -- this ADR
  covers tasks 1, 3, and 5 only, and even those three carry the
  `pressed`/reduced-motion-wiring/five-states-screenshotted gaps noted
  above.

## Rollback

`saai-shell.pre-bottomnav` remains on-device for a full binary rollback.
No data or protocol changes -- both composites are pure rendering/data
contracts.

## Links

- ADR-111 -- the composite pattern (`ContextHeader`/`SystemSection`/
  `ObjectSummary`) this ADR extends to `BottomNavigation`/`OrbHost`.
- ADR-112 -- `in_progress_work`, reused unchanged for the Orb's
  `Running` signal.
- ADR-114 -- the `saai-entityd` disconnect test method reused (in spirit)
  for reasoning about `Offline`'s real trigger condition.
- HIA-16 -- the original "Attention must be distinguishable by shape"
  requirement this ADR generalizes to every real Orb state.
