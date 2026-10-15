# ADR-742: defer bootconfig off the chrome turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `f4ae808e…` (ADR-741). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-741 следующий свободный номер — **742**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-741 fills trusted keys after paired Bluetooth. The chrome
turn still read `/proc/bootconfig` and `/proc/cmdline` together
with kernel / uptime / adapters. dest-no-lock stays off. Do
not start the shell from COM13. Do not open modem devices. Do
not type PSK. Section «Связь» stays. Device «Экран» stays.

## Решение

1. `apply_me_chrome_probes` does not call `boot_slot` or
   `boot_attempts`. Kernel / uptime / adapters fill without
   bootconfig.
2. The next event-loop turn reads slot / attempts onto held
   facts and stores `me_boot_hold`. Camp `/proc` still waits.
   A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   bootconfig when the slot is held.
3. Host test: chrome probes have no `boot_slot`; boot turn
   stores the hold; idle blocks on `boot_pending`. Do not
   flash this ADR. dest-no-lock stays off.

## Откат

Call `boot_slot` from `apply_me_chrome_probes`. Live
`f4ae808e…` still reads bootconfig during chrome.

## Ссылки

- ADR-726, ADR-741, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
