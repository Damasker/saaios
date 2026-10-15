# ADR-758: Wi-Fi status reuses adapter presence

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `e90fefa1…` (ADR-757). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-757 следующий свободный номер — **758**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-744 already fills `wlan0` exists on the adapters turn.
The net turn still called `wifi_adapter_present()` before
`wpa_cli status`. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not type PSK.
Section «Связь» stays. Device «Экран» stays.

## Решение

1. `apply_me_net_probes` passes `facts.wifi_present` into
   `wifi_status_from_cli`. It does not call
   `wifi_adapter_present`.
2. A Wi-Fi connect still drops `me_net_hold` only. Adapter
   presence stays on `me_adapters_hold`. Recapture skips
   `wpa_cli` when Wi-Fi status is held.
3. Host test: net probes and `wifi_status_from_cli` have no
   `wifi_adapter_present()`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `wifi_adapter_present` from `wifi_status_from_cli`. Live
`e90fefa1…` still stats `wlan0` during the net turn.

## Ссылки

- ADR-744, ADR-757, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
