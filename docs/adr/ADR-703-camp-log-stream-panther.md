# ADR-703: streamed camp owner log on panther

## Статус

Принято, 2026-10-15. `/data/saaios/system/saai-shell` is
`f15c170f…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 2321. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=shm` `idle_ok=1` `seq=9`. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not log the raw camp file.

## Нумерация

После ADR-702 следующий свободный номер — **703**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-702 streams the owner log and keeps filtered lines on
Sistema facts. dest-no-lock stays off. Do not start the shell
from COM13. Do not open modem devices. Do not type PSK.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr701` (`64d39dde…`).
3. Kill 1292 only. displayd 2694 respawned 2321.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr701 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-701, ADR-702, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
