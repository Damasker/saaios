# ADR-613: live Sistema DataRow advances by wrap height

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-612 следующий свободный номер — **613**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Gallery DataRow wrap (ADR-596) paints wrapped primary above
`min_hit_height` when there is no secondary. Sistema still
advanced the cursor by `physical(min_hit_height())`, so a
two-line primary overlapped the next row. Task / Intent /
Metric already return paint height. Leave Сейчас. Font
compare stays a device task. ADR-213 thin tuning is not this
slice. NOW attention/task/next rows stay on `layout_v2`
rects.

## Решение

1. `draw_data_row` returns the painted height.
2. Live Sistema Data rows advance `cursor_y` by that height.

## Откат

Restore `cursor_y += physical(data_row.min_hit_height())`.

## Ссылки

- ADR-570, ADR-596, ADR-610
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-06
