# ADR-705: camp owner log next turn on panther

## Статус

Принято, 2026-10-15. `/data/saaios/system/saai-shell` is
`d7afc85b…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 3090. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` `seq=10`. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not log the raw camp file.

## Нумерация

После ADR-704 следующий свободный номер — **705**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-704 fills `/proc` presence on the camp turn and streams
the owner log on the next turn. dest-no-lock stays off. Do
not start the shell from COM13. Do not open modem devices.
Do not type PSK.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr703` (`f15c170f…`).
3. Kill 2321 only. displayd 2694 respawned 3090.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr703 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-703, ADR-704, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
