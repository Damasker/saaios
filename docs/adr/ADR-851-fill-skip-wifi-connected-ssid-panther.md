# ADR-851: fill skip wifi_connected_ssid on panther

## Статус

Принято, 2026-10-16. `/data/saaios/system/saai-shell` is
`e26cfeed…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 1699. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` `seq=18` `input_ok=0`. Process
settled `S`. Unlock stays the proven path (ADR-564).
Experimental. PIN stays null. Leave Сейчас. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked. Do not type PSK. Do not invent weather. Camera
names stay `video*`. Section «Связь» stays. Device «Экран»
stays. Node identity stays panther · aarch64. Do not split
runtime into two socket calls. Do not tap Orb. Do not
un-group Связь. Do not merge DNS/IPv4 back into Сотовая сеть.
Do not leftover-hunt `apply_me_chrome_probes`. Do not
leftover-hunt `wifi_status_line`. Do not tail the camp owner
log.

## Нумерация

После ADR-850 следующий свободный номер — **851**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-850 stops `refresh_context_signals_if_due` from calling
`wifi_connected_ssid` while `me_fill_blocks_idle` is true.
dest-no-lock stays off. Do not start the shell from COM13. Do
not open modem devices. Do not type PSK. Leave Сейчас. Do not
tap Orb.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr849` (`c751962f…`).
3. Kill 1301 only. displayd 2694 respawned 1699.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent. FramePace
   `input_ok=0` recorded honestly.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr849 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-849, ADR-850, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
