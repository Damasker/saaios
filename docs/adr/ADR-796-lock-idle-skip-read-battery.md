# ADR-796: lock idle skip read_battery

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `cb4aa243…` (ADR-795). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not tap Orb.

## Нумерация

После ADR-795 следующий свободный номер — **796**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-794 already skips `read_battery` in `check_low_battery`.
Lock idle refresh still stated `maxfg` on the same status
tick while locked. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not type PSK.

## Решение

1. `refresh_lock_idle_if_due` does not call `read_battery`.
   PIN-entry paint still reads the gauge.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the lock idle refresher has no `read_battery()`.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Call `read_battery` from `refresh_lock_idle_if_due`. Live
`cb4aa243…` still stats `maxfg` on lock idle refresh.

## Ссылки

- ADR-794, ADR-795, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
