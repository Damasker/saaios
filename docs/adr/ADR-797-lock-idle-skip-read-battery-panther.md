# ADR-797: lock idle skip read_battery on panther

## Статус

Принято, 2026-10-16. `/data/saaios/system/saai-shell` is
`9b6735e0…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 16753. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` `seq=27`. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Section
«Связь» stays. Device «Экран» stays. Node identity stays
panther · aarch64. Do not split runtime into two socket
calls. Do not tap Orb. Do not un-group Связь.

## Нумерация

После ADR-796 следующий свободный номер — **797**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-796 stops `refresh_lock_idle_if_due` from calling
`read_battery`. dest-no-lock stays off. Do not start the shell
from COM13. Do not open modem devices. Do not type PSK.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr795` (`cb4aa243…`).
3. Kill 15857 only. displayd 2694 respawned 16753.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr795 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-795, ADR-796, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
