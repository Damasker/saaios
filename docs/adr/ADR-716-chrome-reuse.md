# ADR-716: reuse Sistema chrome sysfs; skip recapture reads

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `b77c623c…` (ADR-715). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-715 следующий свободный номер — **716**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-715 reuses runtime JSON on panther. After the staircase
dropped the hold, recapture still read kernel, uptime, boot
slot, boot attempts, playback, and device-tree model.
dest-no-lock stays off. Do not start the shell from COM13.
Do not open modem devices. Do not type PSK. Section «Связь»
stays. Device «Экран» stays.

## Решение

1. `apply_me_chrome_probes` returns when kernel is already
   held. It does not call `kernel_release` / `uptime_string` /
   `boot_slot`.
2. The first chrome fill stores those fields on
   `me_chrome_hold`. Recapture seeds facts from that cache
   before apply.
3. Host test: held kernel / uptime / boot survive apply.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Always call `kernel_release` from chrome probes. Live
`b77c623c…` still reads sysfs on recapture.

## Ссылки

- ADR-672, ADR-714, ADR-715, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
