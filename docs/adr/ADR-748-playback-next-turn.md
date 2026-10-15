# ADR-748: defer playback exists() off the chrome turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `3692690a…` (ADR-747). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-747 следующий свободный номер — **748**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-747 fills CP after the `/proc` walk. The chrome turn still
checked tinyplay / tone together with kernel / uptime / model.
dest-no-lock stays off. Do not start the shell from COM13. Do
not open modem devices. Do not type PSK. Section «Связь»
stays. Device «Экран» stays.

## Решение

1. `apply_me_chrome_probes` does not call `playback_ready`.
   Kernel / uptime / model fill without those exists().
2. The next event-loop turn checks tinyplay / tone onto held
   facts and stores `me_playback_hold`. Adapters still wait.
   A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   playback when ready is held.
3. Host test: chrome probes have no `playback_ready()`;
   playback turn stores the hold; idle blocks on
   `playback_pending`. Do not flash this ADR. dest-no-lock
   stays off.

## Откат

Call `playback_ready` from `apply_me_chrome_probes`. Live
`3692690a…` still checks tinyplay during chrome.

## Ссылки

- ADR-726, ADR-744, ADR-747, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
