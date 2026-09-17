# ADR-108: gallery touch-target sizing meets the 48-logical-unit minimum

## Status

Accepted, 2026-09-17. Physically re-verified on Pixel 7.

## Context

VUI-02's Acceptance checklist: "Primary touch targets are at least 48×48
logical units" -- `saai_ui_core::MIN_TOUCH_TARGET`. Checking ADR-105's
gallery against this directly found two of the four interactive-looking
primitives sized below it:

- `Button`'s rect used `CONTROL_VISUAL_HEIGHT` (40 logical, 120 physical)
  directly -- the anatomy component-library-v1.md section 6.6 describes
  ("48 minimum hit height ... 40 visual height") assumes a hit region
  wrapping a shorter visual fill, a distinction `draw_gallery_button`
  (like every other gallery primitive) does not implement -- it fills
  exactly the one `Rect` it is given, nothing more.
- `Disclosure`'s rect used a flat `32` logical units -- well under the
  floor section 6.10 states explicitly: "Hit region is at least 48x48 even
  when the chevron is 16-20 units."

`DataRow` was already correct (`physical(TWO_LINE_ROW_HEIGHT)`, matching
its own `min_hit_height()` method). `Field`'s height cleared the minimum on
the real 2400px screen but only incidentally, via a `row_height.
saturating_sub(20)` computed for other reasons, not because anyone checked
it against `MIN_TOUCH_TARGET`.

## Decision

- `Button`: sized to `physical(MIN_TOUCH_TARGET)` directly. Since this
  renderer has no hit-rect/visual-rect split, sizing the one rect it
  actually has to the real minimum is the honest choice here, not
  `CONTROL_VISUAL_HEIGHT` alone -- a real hit-region wrapper around a
  shorter visual fill is future work if/when this primitive gets a real
  interactive consumer with its own hit-testing.
- `Disclosure`: sized to `physical(MIN_TOUCH_TARGET)`, replacing the flat
  `32`.
- `Field`: height now explicitly clamped to
  `.max(physical(MIN_TOUCH_TARGET))` (with an upper bound so it does not
  grow unboundedly on a taller screen), rather than incidentally clearing
  the floor.
- `DataRow` unchanged -- already correct.

## Verification

- `cargo test -p saai-shell`: 91/91, unchanged (no new host-testable
  surface -- this is a sizing constant change, already covered by
  `gallery_rows_never_overlap`/`gallery_rows_stay_within_the_screen`, both
  of which still pass with the taller rects).
- `cargo clippy -p saai-shell --all-targets -- -D warnings`: clean. Removed
  the now-unused `CONTROL_VISUAL_HEIGHT` import after `Button` stopped
  referencing it.
- Hot-swapped onto the live device, rollback still `saai-shell.pre-gallery`.
  Real device screenshot confirms the visibly taller `Button` fill, no
  overlap introduced with the `Field` row below it or anywhere else in the
  gallery.

## Consequences

- VUI-02's touch-target acceptance line is satisfied for the gallery as it
  exists today. It remains, as ADR-106 already noted for hit-test/
  press-state coverage, not something a *test* enforces -- there is no
  automated check that a future gallery row addition respects
  `MIN_TOUCH_TARGET`, only this one-time review and fix.
- `Button`'s visual fill is now taller (144 physical) than section 6.6's
  stated 40-unit visual height. This is a known, accepted gap between the
  spec's intended hit-region/visual-fill split and what this renderer
  currently implements, not silently resolved -- a real fix needs the
  renderer to draw a shorter visual fill centered within a taller
  (invisible) hit region, which requires `draw_gallery_button` to
  distinguish the two, not attempted here.

## Rollback

`saai-shell.pre-gallery` remains on-device. Pure sizing-constant change; no
entity, protocol, or storage change.

## Links

- ADR-105 -- the gallery these sizes belong to.
- ADR-106 -- the row-layout tests that still pass with the new sizes.
- `docs/os/ui/component-library-v1.md` sections 6.6, 6.8, 6.10.
- `docs/os/sprints/VISUAL-ROADMAP.md` -- VUI-02 Acceptance.
