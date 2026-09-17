# ADR-110: fix wrap/line-stacking measurement to account for accessibility text scale

## Status

Accepted, 2026-09-17. Physically verified on Pixel 7 at 100% and 150% text scale.

## Context

VUI-02 acceptance: "Components look like one family at normal and increased
text scale." Tested this directly by setting `shell-settings.json`'s
`text_scale_pct` to `150` (backed up first as
`shell-settings.json.pre-scale-test`, restored afterward) and capturing a
real device screenshot of the component gallery via `screencap`.

Found two real bugs, both the same root cause: `render::text_scale()` is
applied inside `draw_text`/`draw_text_centered` at render time (`let size =
size * text_scale();`), but several places that *measure* text to decide
layout — where to wrap a line, how far down to place a second line — were
measuring against the unscaled size returned by `Fonts::resolve()`. At
100% scale `text_scale()` is `1.0` so this was invisible; at 150% the
measured layout under-predicted the actually-rendered (scaled) text by a
third, so text overflowed the space that was reserved for it.

Two independent call sites hit this:

1. `draw_gallery_semantic_text`'s `wrap_text` call and its `line_height`
   (from `physical_line_height`) — the gallery's `SemanticText` demo string
   wrapped correctly on screen at 100%, but at 150% its second wrapped line
   rendered wider than the wrap point assumed, clipping off the right edge
   of the screen instead of wrapping where it visually needed to.
2. `draw_status_bar`'s battery/Wi-Fi right-aligned layout — `text_width`
   measured `battery_label`/`wifi_label` at the unscaled `40.0`/`36.0`, so
   at 150% the actually-rendered "Wi-Fi" glyphs were wider than the gap
   `wifi_left` reserved for them, and "Wi-Fi" visually overlapped "99%".

`StatusIndicator`'s reason-line offset and `DataRow`'s secondary-line
offset use the same `physical_line_height(TextRole::Body)` pattern as (1)
for stacking a second line under a first; not yet visibly broken in this
specific gallery fixture's text (the demo strings happen to be short
enough), but the same latent bug, fixed alongside (1) rather than left for
a later screenshot to find by chance.

## Decision

Added `scaled_line_height(role: TextRole) -> u32`, deliberately kept
separate from the existing `physical_line_height(role) -> u32` rather than
folding `text_scale()` into that function directly:

- `physical_line_height` backs a pinned host test
  (`physical_line_height_matches_the_pixel_7_scale`, asserting `Body` -> 72,
  `Caption` -> 48) that must stay deterministic regardless of the
  process-global, mutable `TEXT_SCALE_BITS` (`render::set_text_scale`).
- `gallery_row_positions`'s own between-row budget is a fixed fraction of
  screen height by design, independent of text scale, so it has no reason
  to call the scaled variant.

```rust
fn scaled_line_height(role: TextRole) -> u32 {
    (physical_line_height(role) as f32 * text_scale()).round() as u32
}
```

Wired in at the three affected line-stacking call sites:

- `draw_gallery_semantic_text`: `wrap_text` now measures against
  `size * text_scale()` instead of the raw `size`; line stacking uses
  `scaled_line_height(text.role)` instead of `physical_line_height(text.role)`.
- `draw_gallery_status_indicator`'s reason-line offset: same substitution.
- `draw_gallery_data_row`'s secondary-line offset: same substitution.

`draw_status_bar` fixed separately (no shared line-stacking helper
involved, just two `text_width` measurements): both `battery_width` and
`wifi_width` now measure at `40.0 * text_scale()` / `36.0 * text_scale()`
instead of the raw literal, matching what `draw_text` will actually render.

## Verification

- `cargo test -p saai-shell`: 91/91, unchanged (the pinned
  `physical_line_height_matches_the_pixel_7_scale` test still passes
  unmodified, confirming `scaled_line_height` staying a separate function
  achieved its purpose).
- `cargo clippy -p saai-shell --all-targets -- -D warnings`: clean.
- Real device screenshots at `text_scale_pct: 150`, before and after:
  "before" showed `SemanticText`'s second wrapped line clipped off the
  right edge, and the status bar's "Wi-Fi" glyphs overlapping "99%".
  "after" (two separate hot-swaps, one per fix, each independently
  screenshot-verified) shows both wrapped correctly within the screen and
  the status bar with a normal gap between "Wi-Fi" and "99%" — no
  regression at 100% scale either (device restored to
  `text_scale_pct: 100` and re-verified after testing).

## Consequences

- Text-scale-aware layout is now consistent wherever it stacks or wraps
  lines in the gallery and the persistent status bar.
- **Known, not-fully-solved edge case, flagged rather than silently
  claimed correct**: `gallery_row_positions`'s between-row budget is still
  a fixed, scale-independent screen-height fraction. At a large enough
  text scale (this project's clamp allows up to 200%) with a long enough
  wrapped `SemanticText` string, the *number of lines* a row needs could
  still exceed what the fixed row budget assumed, causing a row to collide
  with the row below it -- a different failure mode than the one fixed
  here (which was single-line/two-line text overflowing sideways or
  mis-stacking within its own row, not a whole extra row's worth of
  height). Not exercised by the current gallery demo strings at up to
  150%. Left open rather than addressed here, since making row budgets
  scale-aware is a larger change than this fix's scope.
- The same unscaled-measurement bug class could exist anywhere else in the
  codebase that measures text without going through `wrap_text`/
  `scaled_line_height`; this ADR fixes the three call sites a real
  screenshot found, not an exhaustive audit of every `text_width` call.

## Rollback

`saai-shell.pre-scalefix` and `saai-shell.pre-statusbarfix` both remain
on-device (one per hot-swap). Restoring either reverts that specific fix;
no data or settings migration involved, since only rendering math changed.

## Links

- ADR-107 -- introduced `wrap_text` and the gallery's wrapping demo that
  this bug was found through.
- ADR-105/106 -- `physical_line_height`'s own introduction and the pinned
  test this ADR deliberately avoided disturbing.
- ADR-099 -- the prior real-screenshot-found rendering bug
  (glyph-baseline), same discovery method (`screencap`), different bug
  class.
