# ADR-536: dest-no-lock shell carries 534–535

## Статус

Принято, 2026-10-13. `/data/saaios/system/saai-shell` is
`0bc48f1a…` (pixel7 musl, panic messages on). dest-no-lock is
set. displayd pid stayed. Unlock is not proven. Experimental.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off. Not
VUI-09 Acceptance.

## Нумерация

После ADR-535 следующий свободный номер — **536**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-534/535 sized OSK, status-bar, and main-window SHM for two
frames. Live panther still had `efca9bd3…` (ADR-533). dest-no-lock
must stay; this is not an unlock proof (ADR-512).

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr536` (`efca9bd3…`).
3. Kill the old shell only. displayd pid 26159 stayed. New shell
   28902: `dev_no_lock enabled`, first configure 1080×2400, entityd
   connected, `surface=now`, `backend=shm`.
4. dest-no-lock stays. Do not try lock. Do not convert leftover
   keyboard / gallery / lock formulas.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr536 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-512, ADR-532, ADR-533, ADR-534, ADR-535
- `docs/os/sprints/PIXEL-PATH.md`
