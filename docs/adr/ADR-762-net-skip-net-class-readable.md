# ADR-762: Wi-Fi status skips `/sys/class/net` is_dir

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `d4bd42c8…` (ADR-761). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-761 следующий свободный номер — **762**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-758 already passes `facts.wifi_present` into
`wifi_status_from_cli`. When the adapter is absent the net
turn still called `wifi_net_class_readable()` (`/sys/class/net`
is_dir). dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK. Section
«Связь» stays. Device «Экран» stays. Copy «Нет адаптера»
stays when `wlan0` is missing.

## Решение

1. `wifi_status_from_cli` does not call
   `wifi_net_class_readable`. No adapter means readable
   true so the Wi-Fi row stays «Нет адаптера».
2. A Wi-Fi connect drops `me_net_hold` only. Adapter
   presence stays on `me_adapters_hold`. Recapture skips
   `wpa_cli` when Wi-Fi status is held.
3. Host test: `wifi_status_from_cli` has no
   `wifi_net_class_readable()`; absent adapter sets
   readable without touching net class. Do not flash this
   ADR. dest-no-lock stays off.

## Откат

Call `wifi_net_class_readable` from `wifi_status_from_cli`.
Live `d4bd42c8…` still stats `/sys/class/net` when `wlan0`
is missing.

## Ссылки

- ADR-758, ADR-761, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
