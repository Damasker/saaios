# ADR-455: VUI-09 — drop leftover `stacked_trailing_rect`

## Статус

Принято, 2026-10-11. Live overflow Back already reads compiler
`v2_trailing_top` through `layout_v2` (ADR-214/218/226). The leftover
shell dock had no live caller. `stacked_row_rect` and
`stacked_control_rect` stay. Status invents no action. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-454 следующий свободный номер — **455**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-165 named the trailing cluster so Назад stays on-screen.
ADR-214 copied that dock into compiler `v2_trailing_top`. Live
Wi-Fi / Bluetooth / trusted / space-detail hits and list paint
already read `layout_v2`. The shell helper stayed as a second
tree: `stacked_control_rect` minus `stacked_row_pitch` steps.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover trailing dock was not on that
list.

## Decision

1. **Delete** `stacked_trailing_rect` and `stacked_row_pitch`.
2. **Keep** overflow Back / refresh / scan on `layout_v2`. Keep
   `stacked_row_rect` / `stacked_control_rect`.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas.

## Consequences

- Overflow Back shares one compiled tree. Rollback: restore
  `stacked_trailing_rect`. Still not Visual v1 sign-off.

## Verification

Host: `stacked_trailing_rect` is gone; overflow Back still docks
on-screen. Panther: do not flash; leave Сейчас.
