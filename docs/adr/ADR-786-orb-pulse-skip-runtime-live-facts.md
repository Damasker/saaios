# ADR-786: orb pulse skip runtime live facts

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `5ea093f2…` (ADR-785). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not tap Orb.

## Нумерация

После ADR-785 следующий свободный номер — **786**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-784 already skips `read_runtime_live_facts` in
`build_orb_frame`. `orb_wants_activity_pulse` still connected
the runtime sock on the Me tick. dest-no-lock stays off. Do
not start the shell from COM13. Do not open modem devices.
Do not type PSK. One status JSON read stays on the runtime
turn.

## Решение

1. `orb_wants_activity_pulse` passes `None` health. It does
   not call `read_runtime_live_facts`.
2. `apply_me_runtime_probes` still reads status JSON once. A
   Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the pulse helper has no `read_runtime_live_facts()`.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Call `read_runtime_live_facts` from `orb_wants_activity_pulse`.
Live `5ea093f2…` still connects the runtime sock on the Me
tick.

## Ссылки

- ADR-784, ADR-785, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
