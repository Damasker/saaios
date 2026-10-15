# ADR-802: lock pin skip read_runtime_live_facts

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `8c29b96e…` (ADR-801). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not tap Orb.

## Нумерация

После ADR-801 следующий свободный номер — **802**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-800 already skips `read_runtime_live_facts` in
`refresh_lock_idle_if_due`. PIN-entry paint still read runtime
JSON. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK. Leave
Сейчас.

## Решение

1. `present_lock_pin_entry` does not call
   `read_runtime_live_facts`.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the lock PIN painter has no
   `read_runtime_live_facts()`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `read_runtime_live_facts` from `present_lock_pin_entry`.
Live `8c29b96e…` still reads runtime JSON on lock PIN paint.

## Ссылки

- ADR-800, ADR-801, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
