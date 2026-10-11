# ADR-502: VUI-09 — long-RU fixtures stay in tests

## Статус

Принято, 2026-10-11. Long Russian titles at 150% text scale are host
fixtures. They never ship as runtime data. Layout hits stay ≥48.
Experimental. PIN stays null. Leave Сейчас. Not Visual v1 sign-off.
No flash.

## Нумерация

После ADR-501 следующий свободный номер — **502**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Spec acceptance wants default / empty / one-item / overflow /
offline plus long Russian at a larger font scale on the five
surfaces. Board labels like «Связанный объект 1» are placeholders.
ADR-190 already cycled 150% on panther chrome. `role_px` stays
layout-stable; `draw_text` multiplies by `text_scale()`.

## Decision

1. **Fixtures live in tests.** A long-RU title exercises Object,
   Intent, and NOW task paint. Production `main.rs` and public
   `.sui` samples omit board placeholders.
2. **150% is paint-only.** `role_px` does not grow. Stacked rows
   stay ≥48 physical. Restore 100 after the scale assert.
3. **Not this slice.** Do not convert leftover keyboard / gallery /
   lock formulas. Do not flash.

## Consequences

- Long titles do not become product copy. Rollback: drop the
  fixture test. Still not Visual v1 sign-off.

## Verification

Host: production omits «Связанный объект 1»; long title paints;
150% grows line height; `role_px` unchanged. Panther: COM13 is
live; do not flash; leave Сейчас.
