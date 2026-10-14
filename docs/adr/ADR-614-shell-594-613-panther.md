# ADR-614: host 594–613 wrap paint on panther

## Статус

Принято, 2026-10-14. `/data/saaios/system/saai-shell` is
`f09b83e0…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 24004. Lock surface
`surface=lock` `backend=shm`. Unlock stays the proven path
(ADR-564). Experimental. PIN stays null. Leave Сейчас. Not
Visual v1 sign-off. Not VUI-09 Acceptance. Wrap Acceptance
stays unchecked.

## Нумерация

После ADR-613 следующий свободный номер — **614**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live panther still had `74a69097…` (ADR-593). Host gallery and
live wrap paint (ADR-594–613) lived only in the tree. Leftover
wrap-paint class emptied. dest-no-lock stays off. Do not start
the shell from COM13. Do not mix modem WIP.

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr593` (`74a69097…`).
3. Kill the old shell only. displayd pid 2694 stayed and respawned
   shell 24004. dest-no-lock stayed absent. FramePace:
   `surface=lock` `backend=shm`.
4. dest-no-lock stays off. Do not restore the marker. Do not
   invent weather. Font compare stays a device task.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr593 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-593, ADR-594–613
- `docs/os/sprints/PIXEL-PATH.md`
