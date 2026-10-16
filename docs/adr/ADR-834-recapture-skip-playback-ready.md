# ADR-834: recapture skip playback_ready

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `c6f2d1b5…` (ADR-833). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`. Do not invent a playback split turn.

## Нумерация

После ADR-833 следующий свободный номер — **834**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-832 skips `/proc/uptime` on recapture through `me_facts()`.
Recapture still applied `me_playback_hold` and then called
`apply_me_playback_probes`, so `playback_ready == false` still
`exists()` tinyplay and the test tone. dest-no-lock stays off.
Do not start the shell from COM13. Do not open modem devices.
Do not type PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. `me_facts()` applies `me_playback_hold` and skips
   `apply_me_playback_probes`. The playback turn still
   `exists()` tinyplay and the tone together.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the playback hold arm has no
   `apply_me_playback_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_playback_probes` after applying
`me_playback_hold` in `me_facts()`. Live `c6f2d1b5…` still
`exists()` tinyplay on recapture when playback is not ready.

## Ссылки

- ADR-832, ADR-833, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
