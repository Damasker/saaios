# ADR-627: leftover now_next_paint_rows is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-626 следующий свободный номер — **627**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live NOW next paint already reads
`now_next_paint_rows_scrolled`. The zero-scroll wrapper was
leftover. Leave Сейчас. Font compare stays a device task.
ADR-213 thin tuning is not this slice.

## Решение

1. Drop `now_next_paint_rows`.
2. Host tests call `now_next_paint_rows_scrolled(..., 0)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-626
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
