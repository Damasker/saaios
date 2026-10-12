# ADR-533: dest-no-lock shell carries 515–532

## Статус

Принято, 2026-10-12. `/data/saaios/system/saai-shell` is
`efca9bd3…` (pixel7 musl, panic messages on). dest-no-lock is
set. displayd pid stayed. Unlock is not proven. Experimental.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off. Not
VUI-09 Acceptance.

## Нумерация

После ADR-532 следующий свободный номер — **533**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Host 515–532 (chrome honesty + two-slot lock SHM) lived only on
the host. Live panther still had `0fb762d1…` (ADR-511). dest-no-lock
must stay; this is not an unlock proof (ADR-512).

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr533` (`0fb762d1…`).
3. Kill the old shell only. displayd pid 26159 stayed. New shell
   2320: `dev_no_lock enabled`, first configure 1080×2400, entityd
   connected, `surface=now`.
4. dest-no-lock stays. Do not try lock. Do not convert leftover
   keyboard / gallery / lock formulas.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr533 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-503, ADR-510, ADR-511, ADR-512, ADR-532
- `docs/os/sprints/PIXEL-PATH.md`
