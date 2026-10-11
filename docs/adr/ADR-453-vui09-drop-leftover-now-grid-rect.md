# ADR-453: VUI-09 — drop leftover `now_grid_rect`

## Статус

Принято, 2026-10-11. Apps paint and hits already read `layout_v2`
(ADR-220/228). The leftover 3-column formula had no live caller.
Tiles stay generated `Button`s. Status invents no action. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-452 следующий свободный номер — **453**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-067 introduced `now_grid_rect`. ADR-220 named apps tiles in
`.sui` v2 so `layout_v2` matches that formula. ADR-228 switched
paint to those nodes. The helper stayed as a second tree.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover apps-grid formula was not on that
list.

## Decision

1. **Delete** `now_grid_rect` and `NOW_GRID_COLUMNS`.
2. **Keep** `now_action_at` / `apps_v2_source` / `draw_apps_grid`
   on `layout_v2`.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas.

## Consequences

- Apps paint and hit-test share one compiled grid. Rollback:
  restore `now_grid_rect`. Still not Visual v1 sign-off.

## Verification

Host: `now_grid_rect` is gone; one installed app still hits
`manage_app`. Panther: do not flash; leave Сейчас.
