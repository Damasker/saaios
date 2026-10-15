# ADR-730: defer local sysfs off the wpa_cli / df turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `4499003c…` (ADR-729). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather.

## Нумерация

После ADR-729 следующий свободный номер — **730**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-729 fills «Местность» after camp `/proc`. The net turn
still listed v4l / ifaces / paired / trusted during
`wpa_cli` / `df`. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not type
PSK. Section «Связь» stays. Device «Экран» stays. Camera
names stay `video*`.

## Решение

1. `apply_me_net_probes` does not call `apply_me_local_probes`.
   Wi-Fi / storage / supervisor fill without v4l.
2. The next event-loop turn reads local sysfs onto held facts
   and stores `me_local_hold`. Runtime JSON still waits. A
   Wi-Fi connect drops `me_net_hold` only. Recapture skips
   the listings when camera / ifaces are held.
3. Host test: net probes have no `apply_me_local_probes`;
   local-sysfs turn stores the hold; idle blocks on
   `local_sysfs_pending`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_local_probes` from `apply_me_net_probes`. Live
`4499003c…` still lists v4l during `wpa_cli`.

## Ссылки

- ADR-692, ADR-710, ADR-728, ADR-729, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
