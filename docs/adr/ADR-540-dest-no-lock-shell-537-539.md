# ADR-540: dest-no-lock shell carries 537–539

## Статус

Принято, 2026-10-13. `/data/saaios/system/saai-shell` is
`39453be9…` (pixel7 musl, panic messages on). dest-no-lock is
set. displayd pid stayed. Unlock is not proven. Experimental.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off. Not
VUI-09 Acceptance.

## Нумерация

После ADR-539 следующий свободный номер — **540**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Host 537–539 hid Bluetooth CLASSIC, ContextSource tokens, and
dotted `saaios.*` ObjectSummary ids. Live panther still had
`0bc48f1a…` (ADR-536). dest-no-lock must stay; this is not an
unlock proof (ADR-512).

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr540` (`0bc48f1a…`).
3. Kill the old shell only. displayd pid 26159 stayed. New shell
   1169: dest-no-lock stayed, `surface=now` `backend=shm`.
4. dest-no-lock stays. Do not try lock. Do not convert leftover
   keyboard / gallery / lock formulas.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr540 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-512, ADR-536, ADR-537, ADR-538, ADR-539
- `docs/os/sprints/PIXEL-PATH.md`
