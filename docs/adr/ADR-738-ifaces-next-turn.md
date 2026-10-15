# ADR-738: defer rmnet ifaces off the local-sysfs turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `57db0be8…` (ADR-737). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-737 следующий свободный номер — **738**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-737 fills v4l after ifaces / paired / trusted. The
local-sysfs turn still listed `/sys/class/net` and shelled
`/saaios/ip` together with paired / trusted. dest-no-lock
stays off. Do not start the shell from COM13. Do not open
modem devices. Do not type PSK. Section «Связь» stays.
Device «Экран» stays.

## Решение

1. `apply_me_local_probes` does not call `cellular_ifaces`.
   Paired / trusted fill without walking net class or `ip`.
2. The next event-loop turn lists live rmnet ifaces onto held
   facts and stores `me_ifaces_hold`. Camera still waits.
   A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   net class when ifaces are held.
3. Host test: local probes have no `cellular_ifaces`; ifaces
   turn stores the hold; idle blocks on `ifaces_pending`. Do
   not flash this ADR. dest-no-lock stays off.

## Откат

Call `cellular_ifaces` from `apply_me_local_probes`. Live
`57db0be8…` still walks net class during paired / trusted.

## Ссылки

- ADR-730, ADR-736, ADR-737, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
