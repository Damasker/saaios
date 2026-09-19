# ADR-159: VUI-07 overflowing Назад docks on screen

## Статус

Принято, 2026-09-19. Trailing «Назад» on a stacked list that would
paint below the fold docks to the last on-screen row. First consumer:
DevSurface. Flashed panther `b06c0bb1…`: 7-tap opened Диагностика with
Назад on-panel; left with Назад. Keyboard avoidance, Me scroll, and
focus order are not this slice.

## Нумерация

После ADR-158 следующий свободный номер — **159**. Не S33.

## Контекст

`stacked_row_rect` is `430 + n*220` on 2400px. Nine diagnostic rows
put «Назад» at y=2410 (ADR-136/152). The card exists in hit-test
space but not on the panel, so the only exit was an unlocked kill.
Intent QWERTY already keeps the keyboard below its header (ADR-150).
Me already scrolls (`scrolled_row_rect`). Those stay.

## Decision

1. **`stacked_control_rect`**. Same x/height as `stacked_row_rect`.
   If the desired bottom exceeds the panel, `y` becomes
   `height - row_height`.
2. **DevSurface** uses that rect for «Назад». Data rows whose bottom
   would cross the back card are omitted this slice (no new scroller).
3. **7-tap is allowed.** Screenshot Назад on-panel, then leave with
   Назад. Do not change diagnostic values.

## Consequences

- Overflowing diagnostics still hide the last facts until a later
  scroll slice. Exit is possible.
- Rollback: `stacked_row_rect(row_count)` for back again.

## Verification

Host: nine data rows put back on-screen; a data row is not back.
Panther: 7-tap, screenshot Назад, tap Назад.
