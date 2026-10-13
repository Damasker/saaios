# ADR-581: RAT stays RAT, not Радио

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-580 следующий свободный номер — **581**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`cellular.rat` and `cellular.radio` both painted «Радио». RAT is
a protocol acronym and must stay an acronym. Radio is the radio
on/off row. Do not case-normalize collapsed `vonr` / `gps` /
`sa`. Do not invent weather. Do not mix modem WIP.

## Решение

1. `observation_row_label("cellular.rat")` is `RAT`.
2. `cellular.radio` stays «Радио». Value tokens stay
   `lte` / `umts` / `gsm` / `nr`.
3. No shell flash this slice.

## Откат

Restore the «Радио» label on `cellular.rat`.

## Ссылки

- ADR-561, ADR-580
- `docs/os/sprints/VISUAL-ROADMAP.md`
