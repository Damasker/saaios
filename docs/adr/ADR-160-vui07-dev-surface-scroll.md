# ADR-160: VUI-07 DevSurface scrolls above docked Назад

## Статус

Принято, 2026-09-19. Data rows on DevSurface that do not fit above
the docked «Назад» (ADR-159) scroll with the same
`scrolled_row_rect` / drag gesture as Система. Назад stays docked.
Flashed panther `bbde1ab2…`: 7-tap, swipe showed app capabilities
with header and Назад still on-panel; left with Назад. Do not change
diagnostic values.

## Нумерация

После ADR-159 следующий свободный номер — **160**. Не S33.

## Контекст

ADR-159 put «Назад» on-panel by omitting rows that would cross it.
On panther that hid the last facts (installed-app capabilities). Me
already scrolls. Intent QWERTY already avoids the field. This slice
is only the omitted DevSurface facts.

## Decision

1. **Content clip** is the panel above `stacked_control_rect`.
2. **Data rows** use `scrolled_row_rect` with that clip.
3. **Drag** while DevSurface is open updates the same offset Me uses,
   clamped by `me_max_scroll_offset`. A drag is not Назад.
4. **Назад** stays `stacked_control_rect` and does not scroll.

## Consequences

- Keyboard avoidance and focus order stay later.
- Rollback: ADR-159 omit-overlapping-rows.

## Verification

Host: nine data rows need a positive max offset; back stays on-screen.
Panther: 7-tap, swipe, last facts appear, Назад still on-panel, leave
with Назад.
