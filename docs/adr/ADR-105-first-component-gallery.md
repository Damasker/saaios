# ADR-105: first device component gallery, and a real logical/physical unit bug it caught

## Status

Accepted, 2026-09-17. Physically verified on Pixel 7.

## Context

VUI-02's task list: "Build the first device component-gallery surface covering
all primitive states, long Russian strings, and scaled text." ADR-102/103/104
shipped ten component contracts with zero rendering -- none of them had ever
been drawn.

## Decision

Added `render::draw_gallery` (`services/saai-shell/src/render.rs`) with one
draw function per primitive from ADR-102/103, each converting a component
contract into pixels via the existing `Fonts`/`Canvas`/`draw_text` machinery
-- no new drawing primitives, no new asset formats. Wired into `main.rs`
behind the exact same developer-only gate VUI-01's calibration fixture uses:
`SAAIOS_UI_GALLERY` env var or the `/run/saaios/ui-gallery` runtime marker,
checked once at startup, dispatched from the same early-return point in
`paint_frame` calibration already uses. Never a normal navigation
destination; the marker lives under `/run`, so it cannot survive a reboot or
become a persistent setting, same as calibration's own.

Also extracted `Fonts::icon()` (a `None`-tolerant accessor mirroring the
existing `resolve()`) and `text_width` (previously a closure local to
`draw_status_bar`, now a shared function both the status bar and the gallery
call) rather than duplicating either.

## A real bug, caught by the first physical screenshot

The first build compiled clean, passed all tests, and clippy was silent --
none of that catches a rendering bug, only a real screenshot does (ADR-099's
own point). The first screenshot showed `StatusIndicator`'s reason line
overlapping its own label, and `DataRow`'s secondary line overlapping its own
primary line.

Root cause: `saai_ui_core`'s size tokens (`IconSize`, `MIN_TOUCH_TARGET`,
`TWO_LINE_ROW_HEIGHT`, `Progress::MIN_TRACK_HEIGHT`, spacing tokens) are all
`LogicalUnit` values -- component-library-v1.md section 1 is explicit that
"components consume logical units; only a surface/backend converts them to
physical pixels." The first gallery draft used these logical numbers (24,
48, 64...) directly as physical pixel offsets and rect dimensions, without
ever calling `SurfaceScale::PIXEL_7.logical_to_physical()` -- exactly the
conversion boundary section 1 describes, just skipped. On Pixel 7's 3x
physical-per-logical scale, every one of those values was 3x too small,
and two hand-picked line-spacing constants (`+28`, `+30`) were guesses that
happened to be smaller than `TextRole::Body`'s real physical line height
(72px), so a second line landed on top of the first instead of below it.

Fixed by adding `physical_line_height(role)` and `physical(value)` helpers
(both thin wrappers around `SurfaceScale::PIXEL_7.logical_to_physical`) and
routing every size in the gallery through one of them -- mark size, icon
size, button dimensions, row heights, and every stacked-line offset. Also
pushed the gallery's top margin from `height / 24` to `height / 10`, matching
`draw_calibration`'s own `palette_top` -- the status bar is a separately
composited overlay above the main surface regardless of which fixture that
surface draws, so both fixtures need the same top clearance, and the first
screenshot showed the gallery's title genuinely obscured by it.

## Verification

- `cargo test -p saai-shell`: 87/87, unchanged.
- `cargo clippy -p saai-shell --all-targets -- -D warnings`: clean.
- Hot-swapped onto the live device twice (buggy build, then the fix), same
  `mv`-over-running-binary/`kill`/native-init-respawn pattern as every prior
  shell swap this session, rollback preserved as `saai-shell.pre-gallery`.
- Real device screenshots (`screencap`, ADR-099) of both the buggy and fixed
  builds, gallery mode enabled via the runtime marker: the buggy build
  visibly showed `StatusIndicator`'s reason text colliding with its label
  and `DataRow`'s secondary text colliding with its primary text; the fixed
  build shows all ten primitives with correct, non-overlapping spacing --
  `SemanticText`, `Wifi` icon, divider, `StatusIndicator` (mark + label +
  reason, now on separate lines), `Progress` (62% filled), `Button`
  ("Сохранить"), `Field` (masked PIN), `DataRow` (Wi-Fi/Wallbox/99%),
  `Metric` (Батарея/87 %), `Disclosure` (Подробности + chevron).
- Gallery marker removed and shell restarted back to normal mode after
  verification, matching the same "leave no dev fixture active" practice
  used for every prior device test this session.

## Not verified by this ADR

Explicitly a first pass, not the full section 7 gallery matrix:

- Only each primitive's default state is shown -- pressed, focused,
  disabled, and busy variants are not in this gallery yet.
- Only the compact variant is shown where a component has one
  (`StatusIndicator`'s normal variant with a reason IS shown, since that
  was the more informative default choice for a first pass, but not both
  variants side by side).
- No long-Russian-text stress case or increased text-scale (100%/125%/150%)
  coverage.
- No golden render, layout, hit-test, or overflow tests -- the remaining
  VUI-02 task-list item, and this gallery's own next real step.
- No accessible-overlay mode showing layout/hit-test bounds (section 7's
  "optional developer overlay").

## Consequences

- VUI-02 now has one real, physically-verified rendering path proving all
  ten Experimental component contracts actually draw correctly, not just
  compile and pass unit tests -- the gap ADR-102/103/104 explicitly left
  open.
- The logical/physical unit conversion boundary (`physical`/
  `physical_line_height`) is now a real, reusable pattern in `render.rs` for
  any future code drawing from `saai_ui_core`'s logical-unit tokens --
  worth reusing rather than re-deriving when the remaining primitives
  (state variants, other screens) get wired up.
- `text_width` and `Fonts::icon()` are now shared, not gallery-private --
  available to any future `render.rs` code without re-deriving either.

## Rollback

`saai-shell.pre-gallery` remains on-device for atomic rollback. The gallery
is unreachable without deliberately setting `SAAIOS_UI_GALLERY=1` or
creating `/run/saaios/ui-gallery`, so normal device behavior is unaffected
by this change existing in the binary at all.

## Links

- `docs/os/ui/component-library-v1.md` section 7 -- the full gallery matrix
  this is a first pass toward.
- ADR-102, ADR-103, ADR-104 -- the component contracts this draws.
- ADR-099 -- the screenshot tool; also the precedent for "a real screenshot
  catches what tests and clippy cannot."
- ADR-101's own note on the physical/logical unit tension this ADR resolves
  for the gallery specifically, not the whole `saai-ui-core` layout tree.
