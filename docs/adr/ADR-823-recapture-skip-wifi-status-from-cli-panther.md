# ADR-823: recapture skip wifi_status_from_cli on panther

## Статус

Принято, 2026-10-16. `/data/saaios/system/saai-shell` is
`b0b2023d…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 27128. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` `seq=12`. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Section
«Связь» stays. Device «Экран» stays. Node identity stays
panther · aarch64. Do not split runtime into two socket
calls. Do not tap Orb. Do not un-group Связь. Do not merge
DNS/IPv4 back into Сотовая сеть.

## Нумерация

После ADR-822 следующий свободный номер — **823**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-822 stops recapture from calling `apply_me_net_probes`
when `me_net_hold` is set. dest-no-lock stays off. Do not
start the shell from COM13. Do not open modem devices. Do not
type PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr821` (`08cef1c9…`).
3. Kill 26638 only. displayd 2694 respawned 27128.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr821 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-821, ADR-822, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
