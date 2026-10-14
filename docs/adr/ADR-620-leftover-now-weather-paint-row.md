# ADR-620: leftover now_weather_paint_row is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-619 следующий свободный номер — **620**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live NOW weather paint already reads
`now_weather_paint_row_scrolled` (ADR-476). The zero-scroll
wrapper was leftover. Do not invent weather. Leave Сейчас.
Font compare stays a device task. ADR-213 thin tuning is not
this slice.

## Решение

1. Drop `now_weather_paint_row`.
2. Host tests call `now_weather_paint_row_scrolled(..., 0)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-476, ADR-619
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
