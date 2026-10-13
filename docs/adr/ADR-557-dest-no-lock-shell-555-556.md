# ADR-557: dest-no-lock shell carries 555–556

## Статус

Принято, 2026-10-13. `/data/saaios/system/saai-shell` is
`126b72c8…` (pixel7 musl, panic messages on). dest-no-lock is
set. displayd pid stayed. Unlock is not proven. Experimental.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off. Not
VUI-09 Acceptance.

## Нумерация

После ADR-556 следующий свободный номер — **557**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Host 555–556 mapped locality source to Open-Meteo and Записи
space ids to space names. Those live in `saai-shell`. Live
panther still had `181cffbf…` (ADR-554). dest-no-lock must stay;
this is not an unlock proof (ADR-512).

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr557` (`181cffbf…`).
3. Kill the old shell only. displayd pid 26159 stayed and respawned
   the shell. New shell 12687: dest-no-lock stayed, first configure
   1080×2400, entityd connected, `surface=now` `backend=shm`.
4. dest-no-lock stays. Do not try lock. Do not invent weather.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr557 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-512, ADR-554, ADR-555, ADR-556
- `docs/os/sprints/PIXEL-PATH.md`
