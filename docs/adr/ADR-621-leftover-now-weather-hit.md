# ADR-621: leftover now_weather_hit is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-620 следующий свободный номер — **621**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live NOW weather hits already read `now_weather_hit_scrolled`
(ADR-476). The zero-scroll wrapper was leftover. Do not invent
weather. Leave Сейчас. Font compare stays a device task.
ADR-213 thin tuning is not this slice.

## Решение

1. Drop `now_weather_hit`.
2. Host tests call `now_weather_hit_scrolled(..., 0)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-476, ADR-620
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
