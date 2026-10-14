# ADR-651: leftover now_node_rect is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-650 следующий свободный номер — **651**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live NOW chrome already reads `v2_named_rect`. The
`now.sui` why-string alias was leftover. Leave Сейчас. Font
compare stays a device task. ADR-213 thin tuning is not this
slice.

## Решение

1. Drop `now_node_rect`.
2. Footer and chrome call `v2_named_rect(..., "now.sui")`.

## Откат

Restore the leftover alias.

## Ссылки

- ADR-650
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
