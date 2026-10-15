# ADR-718: reuse Sistema adapter presence; skip recapture sysfs

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `95b17773…` (ADR-717). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-717 следующий свободный номер — **718**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-717 reuses kernel / uptime / boot on panther. Recapture
still called `wifi_adapter_present` and
`bluetooth_adapter_present` from `me_facts` / chrome.
dest-no-lock stays off. Do not start the shell from COM13.
Do not open modem devices. Do not type PSK. Section «Связь»
stays. Device «Экран» stays.

## Решение

1. Chrome probes fill Wi-Fi and Bluetooth adapter presence.
   When chrome values are held, they do not call those sysfs
   `exists` checks.
2. `me_chrome_hold` stores the booleans. Recapture seeds them
   from that cache. `me_facts` / chrome no longer read the
   paths inline.
3. Host test: held adapter flags survive apply. Do not flash
   this ADR. dest-no-lock stays off.

## Откат

Call `wifi_adapter_present` from `me_facts` again. Live
`95b17773…` still checks `wlan0` / `hci0` on recapture.

## Ссылки

- ADR-716, ADR-717, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
