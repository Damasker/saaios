# ADR-764: runtime skips `/data/saaios` exists

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `09032944…` (ADR-763). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64.

## Нумерация

После ADR-763 следующий свободный номер — **764**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-760 already passes `facts.model` into `node_model_resolved`.
The runtime turn still called `phone_gate_surface()`, which
stats `/data/saaios` and may read the device-tree. dest-no-lock
stays off. Do not start the shell from COM13. Do not open
modem devices. Do not type PSK. Do not split runtime identity
and observations into two socket calls. Section «Связь» stays.
Device «Экран» stays.

## Решение

1. `apply_me_runtime_probes` derives the phone gate from the
   held model string. It does not call `phone_gate_surface`.
2. A Wi-Fi connect drops `me_net_hold` only. Model and runtime
   holds stay. Recapture still reads status JSON once when
   observations are empty.
3. Host test: runtime probes have no `phone_gate_surface()`
   and no `/data/saaios`. One `read_runtime_live_facts`. Do
   not flash this ADR. dest-no-lock stays off.

## Откат

Call `phone_gate_surface` from `apply_me_runtime_probes`. Live
`09032944…` still stats `/data/saaios` during runtime.

## Ссылки

- ADR-760, ADR-763, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
