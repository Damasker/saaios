# ADR-744: defer adapter sysfs off the chrome turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `7e2b6bbd…` (ADR-743). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-743 следующий свободный номер — **744**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-743 fills bootconfig after kernel / uptime. The chrome
turn still checked wlan0 / hci0 together with kernel /
playback. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK. Section
«Связь» stays. Device «Экран» stays.

## Решение

1. `apply_me_chrome_probes` does not call
   `wifi_adapter_present` or `bluetooth_adapter_present`.
   Kernel / uptime / playback fill without adapter sysfs.
2. The next event-loop turn checks wlan0 / hci0 onto held
   facts and stores `me_adapters_hold`. Bootconfig still
   waits. A Wi-Fi connect drops `me_net_hold` only. Recapture
   skips adapter sysfs when presence is held.
3. Host test: chrome probes have no adapter exists();
   adapters turn stores the hold; idle blocks on
   `adapters_pending`. Do not flash this ADR. dest-no-lock
   stays off.

## Откат

Call `wifi_adapter_present` from `apply_me_chrome_probes`.
Live `7e2b6bbd…` still checks wlan0 during chrome.

## Ссылки

- ADR-718, ADR-726, ADR-743, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
