# ADR-717: reused Sistema chrome sysfs on panther

## Статус

Принято, 2026-10-15. `/data/saaios/system/saai-shell` is
`95b17773…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 8023. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` `seq=11`. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Section «Связь» stays. Device «Экран» stays.

## Нумерация

После ADR-716 следующий свободный номер — **717**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-716 reuses kernel / uptime / boot on recapture.
dest-no-lock stays off. Do not start the shell from COM13.
Do not open modem devices. Do not type PSK.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr715` (`b77c623c…`).
3. Kill 7325 only. displayd 2694 respawned 8023.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr715 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-715, ADR-716, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
