# ADR-560: dest-no-lock shell carries 558–559

## Статус

Принято, 2026-10-13. `/data/saaios/system/saai-shell` is
`72cba91c…` (pixel7 musl, panic messages on). dest-no-lock is
set. displayd pid stayed. Unlock is not proven. Experimental.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off. Not
VUI-09 Acceptance.

## Нумерация

После ADR-559 следующий свободный номер — **560**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Host 558–559 hid the raw space id on Диагностика and stopped
repeating Open-Meteo on attribution. Those live in `saai-shell`.
Live panther still had `126b72c8…` (ADR-557). dest-no-lock must stay;
this is not an unlock proof (ADR-512).

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr560` (`126b72c8…`).
3. Kill the old shell only. displayd pid 26159 stayed and respawned
   the shell. New shell 14170: dest-no-lock stayed, first configure
   1080×2400, entityd connected, `surface=now` `backend=shm`.
4. dest-no-lock stays. Do not try lock. Do not invent weather.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr560 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-512, ADR-557, ADR-558, ADR-559
- `docs/os/sprints/PIXEL-PATH.md`
