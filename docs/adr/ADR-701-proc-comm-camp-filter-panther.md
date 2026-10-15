# ADR-701: `/proc` comm camp filter on panther

## Статус

Принято, 2026-10-15. `/data/saaios/system/saai-shell` is
`64d39dde…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 1292. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=shm` `idle_ok=1` `seq=9`. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-700 следующий свободный номер — **701**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-700 reads `comm` first and skips `cmdline` unless the
name is a camp binary. dest-no-lock stays off. Do not start
the shell from COM13. Do not open modem devices. Do not type
PSK.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr699` (`f9e2d372…`).
3. Kill 32725 only. displayd 2694 respawned 1292.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr699 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-699, ADR-700, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
