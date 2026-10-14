# ADR-656: host 615–655 leftover helpers on panther

## Статус

Принято, 2026-10-14. `/data/saaios/system/saai-shell` is
`eb3197c9…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 2396. Lock surface
`surface=lock` `backend=shm`. Unlock stays the proven path
(ADR-564). Experimental. PIN stays null. Leave Сейчас. Not
Visual v1 sign-off. Not VUI-09 Acceptance. Wrap Acceptance
stays unchecked.

## Нумерация

После ADR-655 следующий свободный номер — **656**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live panther still had `f09b83e0…` (ADR-614). Host leftover
helpers (ADR-615–655) lived only in the tree. The leftover
unused-alias class emptied at ADR-655. The user asked to flash,
not hunt leftovers. dest-no-lock stays off. Do not start the
shell from COM13. Do not mix modem WIP.

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr614` (`f09b83e0…`).
3. Kill the old shell only. displayd pid 2694 stayed and respawned
   shell 2396. dest-no-lock stayed absent. FramePace:
   `surface=lock` `backend=shm`.
4. dest-no-lock stays off. Do not restore the marker. Do not
   invent weather. Font compare stays a device task.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr614 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-614, ADR-615–655
- `docs/os/sprints/PIXEL-PATH.md`
