# ADR-691: deferred runtime JSON on panther

## Статус

Принято, 2026-10-15. `/data/saaios/system/saai-shell` is
`8f60e55c…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 29286. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `seq=8`. Process settled `S`. Unlock stays
the proven path (ADR-564). Experimental. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off. Not VUI-09 Acceptance.
Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-690 следующий свободный номер — **691**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-690 fills identity and «Наблюдения» after camp and
Wi-Fi/`df`. dest-no-lock stays off. Do not start the shell
from COM13. Do not type PSK.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr689` (`0e6ea3d9…`).
3. Kill 28202 only. displayd 2694 respawned 29286.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr689 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-689, ADR-690, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
