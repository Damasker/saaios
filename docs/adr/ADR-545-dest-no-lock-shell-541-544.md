# ADR-545: dest-no-lock shell carries 541–544

## Статус

Принято, 2026-10-13. `/data/saaios/system/saai-shell` is
`abd38350…` (pixel7 musl, panic messages on). dest-no-lock is
set. displayd pid stayed. Unlock is not proven. Experimental.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off. Not
VUI-09 Acceptance.

## Нумерация

После ADR-544 следующий свободный номер — **545**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Host 541–544 hid tinyplay, named notice/schedule, replaced
`camp` with «процесс», and replaced `CP ушёл` with
«модем ушёл». Live panther still had `39453be9…` (ADR-540).
dest-no-lock must stay; this is not an unlock proof (ADR-512).

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr545` (`39453be9…`).
3. Kill the old shell only. displayd pid 26159 stayed and respawned
   the shell. New shell 5233: dest-no-lock stayed, first configure
   1080×2400, entityd connected, `surface=now` `backend=shm`.
4. dest-no-lock stays. Do not try lock. Do not convert leftover
   keyboard / gallery / lock formulas.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr545 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-512, ADR-540, ADR-541, ADR-542, ADR-543, ADR-544
- `docs/os/sprints/PIXEL-PATH.md`
