# ADR-666: Sistema hang fix on panther

## Статус

Принято, 2026-10-14. `/data/saaios/system/saai-shell` is
`ed2bc0d1…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 6662. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` seq=8. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked.

## Нумерация

После ADR-665 следующий свободный номер — **666**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Sistema on `eb3197c9…` hung (`R`, `reason=motion`). Host
fixes ADR-660–665: row cache, paint cache, skip motion
repaint, defer `me_facts` one loop, empty header dock.
ADR-664 (`81e7d8f9…`) SIGABRT'd on missing `ContextHeader`.
dest-no-lock stays off. Do not start the shell from COM13.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr664` (`81e7d8f9…`).
3. Kill 6243 only. displayd 2694 respawned 6662.
4. Unlock (540,1200) then Sistema (945,2250). Chrome
   committed, facts filled on the next turn, process went
   `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr664 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-660–665, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
