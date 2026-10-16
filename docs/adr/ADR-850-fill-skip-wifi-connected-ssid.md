# ADR-850: fill skip wifi_connected_ssid

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `c751962f…` (ADR-849). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`. Do not leftover-hunt
`wifi_status_line`. Do not tail the camp owner log.

## Нумерация

После ADR-849 следующий свободный номер — **850**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-848 skips `wifi_connected_ssid` when `me_net_hold` is set.
The 5s context-signal tick still shelled `wpa_cli` during the
first-open staircase, before the net turn held Wi-Fi status.
dest-no-lock stays off. Do not start the shell from COM13. Do
not open modem devices. Do not type PSK. Leave Сейчас. Do not
tap Orb.

## Решение

1. `refresh_context_signals_if_due` returns while
   `me_fill_blocks_idle` is true, without bumping the timer
   and without `wifi_connected_ssid`. The net turn still runs
   `wpa_cli` once. A Wi-Fi connect drops `me_net_hold` only,
   so the next due tick after fill shells `wpa_cli` again.
2. Recapture skips JSON when runtime is held.
3. Host test: the fill-pending arm has no `wifi_connected_ssid`.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Call `wifi_connected_ssid` from
`refresh_context_signals_if_due` even while
`me_fill_blocks_idle` is true. Live `c751962f…` still shells
`wpa_cli` during a long first-open staircase.

## Ссылки

- ADR-848, ADR-849, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
