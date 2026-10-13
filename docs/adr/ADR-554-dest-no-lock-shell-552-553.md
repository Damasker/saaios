# ADR-554: dest-no-lock shell carries 552–553

## Статус

Принято, 2026-10-13. `/data/saaios/system/saai-shell` is
`181cffbf…` (pixel7 musl, panic messages on). dest-no-lock is
set. displayd pid stayed. Unlock is not proven. Experimental.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off. Not
VUI-09 Acceptance.

## Нумерация

После ADR-553 следующий свободный номер — **554**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Host 552–553 hid `current` and reqwest Display from the locality
row. Those live in `saai-weather`, linked into the shell. Live
panther still had `e1205a3e…` (ADR-551). dest-no-lock must stay;
this is not an unlock proof (ADR-512).

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr554` (`e1205a3e…`).
3. Kill the old shell only. displayd pid 26159 stayed and respawned
   the shell. New shell 10132: dest-no-lock stayed, first configure
   1080×2400, entityd connected, `surface=now` `backend=shm`.
4. dest-no-lock stays. Do not try lock. Do not invent weather.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr554 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-512, ADR-551, ADR-552, ADR-553
- `docs/os/sprints/PIXEL-PATH.md`
