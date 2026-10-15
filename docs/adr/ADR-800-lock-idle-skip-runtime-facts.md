# ADR-800: lock idle skip read_runtime_live_facts

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `5961f994…` (ADR-799). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not tap Orb.

## Нумерация

После ADR-799 следующий свободный номер — **800**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-796 already skips `read_battery` in `refresh_lock_idle_if_due`.
Lock idle refresh still called `read_runtime_live_facts` on the
same status tick while locked. dest-no-lock stays off. Do not
start the shell from COM13. Do not open modem devices. Do not
type PSK. Leave Сейчас.

## Решение

1. `refresh_lock_idle_if_due` does not call
   `read_runtime_live_facts`. PIN-entry paint still reads live
   facts.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the lock idle refresher has no
   `read_runtime_live_facts()`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `read_runtime_live_facts` from `refresh_lock_idle_if_due`.
Live `5961f994…` still reads runtime JSON on lock idle refresh.

## Ссылки

- ADR-784, ADR-796, ADR-799, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
