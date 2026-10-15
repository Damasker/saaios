# ADR-792: status bar skip read_battery

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `05f63594…` (ADR-791). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not tap Orb.

## Нумерация

После ADR-791 следующий свободный номер — **792**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-788 already skips `read_battery` in `build_orb_frame`. The
status bar still stated `maxfg` on every refresh during
first-open Sistema. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not type PSK.

## Решение

1. `present_status_bar` does not call `read_battery`. Lock and
   low-battery notify still read the gauge.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the status painter has no `read_battery()`. Do
   not flash this ADR. dest-no-lock stays off.

## Откат

Call `read_battery` from `present_status_bar`. Live `05f63594…`
still stats `maxfg` on every status refresh.

## Ссылки

- ADR-788, ADR-791, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
