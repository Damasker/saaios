# ADR-447: VUI-09 — drop leftover `draw_root`

## Статус

Принято, 2026-10-11. Live draw no longer constructs `Frame::Root`
(ADR-446). The leftover diagnostic Surface strip painter had no live
caller. Tab tests now call `draw_tab_bar`. Status invents no action.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-446 следующий свободный номер — **447**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-165 kept `draw_root` as a test fallback. ADR-446 removed the
live `Frame::Root` variant. The painter still filled a Surface bar
at `y=150` and placeholder rows — a second tree no screen uses.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover root painter was not on that list.

## Decision

1. **Delete** `draw_root`.
2. **Keep** `draw_tab_bar` / `draw_action_card` / `draw_apps_grid`.
   Host tab tests call `draw_tab_bar`.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas.

## Consequences

- Live chrome has no leftover Surface-strip painter. Rollback:
  restore `draw_root`. Still not Visual v1 sign-off.

## Verification

Host: `draw_root` is gone; selected-tab indicator still moves.
Panther: do not flash; leave Сейчас.
