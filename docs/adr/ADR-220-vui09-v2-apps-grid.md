# ADR-220: VUI-09 — apps grid hits come from `layout_v2()`

## Статус

Принято, 2026-09-20. Linear `layout_v2` cannot place a 3-column tile
beside another. Grid `Button`s overlay through `Node::stack` (y-spacer
then a horizontal x-spacer + cell). Live `now_action_at` compiles the
installed-app list and hit-tests `layout_v2`. Paint still uses
`now_grid_rect`. PIN stays null. Do not launch an app. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-219 следующий свободный номер — **220**. Не S33.

## Контекст

ADR-219 closed Me scroll. The apps grid was the next procedural
chrome the v2 vocabulary already names (`apps` surface, `Button`).

## Decision

1. **Overlay.** Each tile is a stacked slot: vertical spacer to the
   design-canvas top, then a horizontal spacer to the column origin
   inside the content margin, then a `Button` sized to the cell.
2. **Hits.** Shell generates one `Button` per live installed app
   (`loc = "manage_app:<id>"`) and reads `layout_v2`. Missing apps
   and Status tiles invent no `manage_app`.
3. **Paint.** `draw_apps_grid` / `now_grid_rect` stay procedural
   while the hit rects match.

## Consequences

- Apps grid hits the compiler. Overlays with vocabulary stay next.
  Keyboard keys are not vocabulary; those overlays keep a formula
  until a later ADR names them. Rollback: restore `now_grid_rect`
  hits. Still not Visual v1 sign-off.

## Verification

Host: one installed app hits cell 0 and misses cell 1. Panther:
flash; open Приложения without launching; leave Сейчас.
