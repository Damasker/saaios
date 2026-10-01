# ADR-429: Orb quantity is a circular arc

## Статус

Принято, 2026-10-01. Host rendering only; no flash, not looked at on the
Pixel 7 panel yet (that review is the end-of-queue hardware pass). Не S33.

## Контекст

Context Light grammar (VUI-04, ADR-116) maps quantity to arc/fill. The boards
([product-visual-target-v1](../os/ui/product-visual-target-v1.md)) show the
Orb with a progress ring. The shell had no circle primitive, so quantity was
a thin bottom line, recorded as "the honest square analogue".

## Decision

1. `Canvas::arc(center, outer, thickness, fraction, color)` draws a stroke
   clockwise from 12 o'clock. Radial edges are anti-aliased through the
   existing `blend`; the angular ends are cut square. `fraction` is clamped
   to `0..=1`, so a bad reading never wraps.
2. `draw_orb` paints quantity with it, inscribed in the Orb dot rect. The
   `Orb` input, hit-test geometry, and `OrbHost::quantity` are unchanged.
3. Colour stays the Border token, never severity. Missing reading and `0%`
   draw nothing. Nothing is invented: only the existing determinate battery
   `Progress` feeds it.

## Consequences

Rollback: restore `draw_quantity_fill` (bottom track). Continuous glow,
blur, and a round Orb body remain out of scope (opaque-first). The arc
colour contrast on the real panel is unverified.

## Verification

Host: `cargo test -p saai-shell -- --test-threads=1`, including
`quantity_arc_sweeps_clockwise_from_twelve_in_border_not_severity`,
`quantity_arc_grows_with_the_reading_and_missing_stays_absent`,
`quantity_arc_stays_inside_the_dot_rect`. A throwaway render at 25/50/87/100 %
was inspected on the host to confirm direction and start angle.
