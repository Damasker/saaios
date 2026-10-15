# ADR-794: low-battery skip read_battery

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `609702f3…` (ADR-793). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not tap Orb.

## Нумерация

После ADR-793 следующий свободный номер — **794**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-792 already skips `read_battery` in `present_status_bar`.
The same status-refresh tick still stated `maxfg` from
`check_low_battery`. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not type PSK.

## Решение

1. `check_low_battery` does not call `read_battery`. Lock still
   reads the gauge.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the low-battery checker has no `read_battery()`.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Call `read_battery` from `check_low_battery`. Live `609702f3…`
still stats `maxfg` on every status refresh.

## Ссылки

- ADR-792, ADR-793, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
