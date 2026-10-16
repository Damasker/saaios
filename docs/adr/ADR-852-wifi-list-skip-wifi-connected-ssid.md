# ADR-852: wifi-list skip wifi_connected_ssid

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `e26cfeed…` (ADR-851). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`. Do not leftover-hunt
`wifi_status_line`. Do not tail the camp owner log.

## Нумерация

После ADR-851 следующий свободный номер — **852**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-850 skips `wifi_connected_ssid` while Me fill is pending.
The Wi-Fi list paint still called `wifi_connected_ssid` every
frame after Sistema had already held the SSID. dest-no-lock
stays off. Do not start the shell from COM13. Do not open
modem devices. Do not type PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. Wi-Fi list paint reads the SSID from `me_net_hold` and
   skips `wifi_connected_ssid`. The net turn still runs
   `wpa_cli` once. A Wi-Fi connect drops `me_net_hold` only,
   so the next list paint shells `wpa_cli` again.
2. Recapture skips JSON when runtime is held.
3. Host test: the net-hold arm has no `wifi_connected_ssid`.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Call `wifi_connected_ssid` from Wi-Fi list paint even when
`me_net_hold` is set. Live `e26cfeed…` still shells `wpa_cli`
on every Wi-Fi list frame after Sistema has held Wi-Fi status.

## Ссылки

- ADR-850, ADR-851, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
