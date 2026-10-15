# ADR-760: runtime reuses the held device-tree model

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `2cd80952…` (ADR-759). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-759 следующий свободный номер — **760**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-752 already fills `/proc/device-tree/model` on the model
turn. The runtime turn still called `hardware_model()` as the
fallback for `node_model_resolved`. dest-no-lock stays off. Do
not start the shell from COM13. Do not open modem devices. Do
not type PSK. Do not split runtime identity and observations
into two socket calls. Section «Связь» stays. Device «Экран»
stays.

## Решение

1. `apply_me_runtime_probes` passes `facts.model` into
   `node_model_resolved`. It does not call `hardware_model`.
2. A Wi-Fi connect drops `me_net_hold` only. Model and runtime
   holds stay. Recapture still reads status JSON once when
   observations are empty.
3. Host test: runtime probes have no `hardware_model()`; one
   `read_runtime_live_facts`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `hardware_model` from `apply_me_runtime_probes`. Live
`2cd80952…` still reads the device-tree during runtime.

## Ссылки

- ADR-752, ADR-759, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
