# ADR-593: host 566–592 chrome on panther

## Статус

Принято, 2026-10-13. `/data/saaios/system/saai-shell` is
`74a69097…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 15341. Lock surface
1080×2400, `surface=lock` `backend=shm`. Unlock stays the
proven path (ADR-564). Experimental. PIN stays null. Leave
Сейчас. Not Visual v1 sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-592 следующий свободный номер — **593**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live panther still had `dce4a158…` (ADR-564). Host wrap
(ADR-566–580) and leftover chrome (ADR-581–592) lived only in
the tree. The user asked to flash. dest-no-lock stays off.
Do not start the shell from COM13. Do not mix modem WIP.

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr564` (`dce4a158…`).
3. Kill the old shell only. displayd pid 2694 stayed and respawned
   shell 15341. dest-no-lock stayed absent. FramePace:
   `surface=lock` `backend=shm`.
4. dest-no-lock stays off. Do not restore the marker. Do not
   invent weather.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr564 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-564, ADR-566–592
- `docs/os/sprints/PIXEL-PATH.md`
