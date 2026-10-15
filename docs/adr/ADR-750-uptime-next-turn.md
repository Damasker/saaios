# ADR-750: defer `/proc/uptime` off the chrome turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `738b3d91…` (ADR-749). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-749 следующий свободный номер — **750**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-749 fills tinyplay / tone after kernel / uptime. The
chrome turn still read `/proc/uptime` together with kernel /
model. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK. Section
«Связь» stays. Device «Экран» stays.

## Решение

1. `apply_me_chrome_probes` does not call `uptime_string`.
   Kernel / model fill without `/proc/uptime`.
2. The next event-loop turn reads uptime onto held facts and
   stores `me_uptime_hold`. Playback still waits. A Wi-Fi
   connect drops `me_net_hold` only. Recapture skips uptime
   when the string is held.
3. Host test: chrome probes have no `uptime_string()`;
   uptime turn stores the hold; idle blocks on
   `uptime_pending`. Do not flash this ADR. dest-no-lock
   stays off.

## Откат

Call `uptime_string` from `apply_me_chrome_probes`. Live
`738b3d91…` still reads `/proc/uptime` during chrome.

## Ссылки

- ADR-726, ADR-748, ADR-749, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
