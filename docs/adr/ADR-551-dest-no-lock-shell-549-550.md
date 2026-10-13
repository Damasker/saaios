# ADR-551: dest-no-lock shell carries 549–550

## Статус

Принято, 2026-10-13. `/data/saaios/system/saai-shell` is
`e1205a3e…` (pixel7 musl, panic messages on). dest-no-lock is
set. displayd pid stayed. Unlock is not proven. Experimental.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off. Not
VUI-09 Acceptance.

## Нумерация

После ADR-550 следующий свободный номер — **551**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Host 549–550 dropped `sgc` / `SGC` and `tty` / `TTY` from
Sistema. Live panther still had `425fbafc…` (ADR-548).
dest-no-lock must stay; this is not an unlock proof (ADR-512).

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr551` (`425fbafc…`).
3. Kill the old shell only. displayd pid 26159 stayed and respawned
   the shell. New shell 8910: dest-no-lock stayed, first configure
   1080×2400, entityd connected, `surface=now` `backend=shm`.
4. dest-no-lock stays. Do not try lock. Do not convert leftover
   keyboard / gallery / lock formulas.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr551 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-512, ADR-548, ADR-549, ADR-550
- `docs/os/sprints/PIXEL-PATH.md`
