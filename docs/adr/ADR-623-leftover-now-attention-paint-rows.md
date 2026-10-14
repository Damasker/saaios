# ADR-623: leftover now_attention_paint_rows is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-622 следующий свободный номер — **623**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live NOW attention paint already reads
`now_attention_paint_rows_scrolled`. The zero-scroll wrapper
was leftover. Leave Сейчас. Font compare stays a device task.
ADR-213 thin tuning is not this slice.

## Решение

1. Drop `now_attention_paint_rows`.
2. Host tests call `now_attention_paint_rows_scrolled(..., 0, None)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-622
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
