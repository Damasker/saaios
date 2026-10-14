# ADR-661: Me row cache on panther

## Статус

Принято, 2026-10-14. `/data/saaios/system/saai-shell` is
`4a03a41b…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 4652. Lock surface
`surface=lock` `backend=shm` `idle_ok=1` seq=3. Unlock stays
the proven path (ADR-564). Experimental. PIN stays null. Leave
Сейчас. Not Visual v1 sign-off. Not VUI-09 Acceptance. Wrap
Acceptance stays unchecked.

## Нумерация

После ADR-660 следующий свободный номер — **661**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-660 cached Sistema rows across Selection / `tick_motion`.
Live panther still had `eb3197c9…` (ADR-656). The user hung
that binary on a Sistema tap. dest-no-lock stays off. Do not
start the shell from COM13. Do not mix modem WIP.

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename
   over `/data/saaios/system/saai-shell`. Previous binary kept
   as `saai-shell.prev-adr656` (`eb3197c9…`).
3. Kill 4278 only. displayd pid 2694 stayed and respawned
   shell 4652. dest-no-lock stayed absent. FramePace:
   `surface=lock` `backend=shm` `idle_ok=1`.
4. dest-no-lock stays off. Do not restore the marker. Do not
   invent weather. Font compare stays a device task.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr656 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-656, ADR-660, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
