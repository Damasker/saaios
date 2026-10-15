# ADR-784: Me orb skip runtime live facts

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `a0faf865…` (ADR-783). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not tap Orb.

## Нумерация

После ADR-783 следующий свободный номер — **784**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-782 already connects the runtime unix socket without
`path.exists()`. First-open Sistema still called
`read_runtime_live_facts` from `build_orb_frame` on every Me
paint. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK. One status
JSON read stays on the runtime turn.

## Решение

1. `build_orb_frame` passes `None` health. It does not call
   `read_runtime_live_facts`.
2. `apply_me_runtime_probes` still reads status JSON once. A
   Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the orb painter has no `read_runtime_live_facts()`.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Call `read_runtime_live_facts` from `build_orb_frame`. Live
`a0faf865…` still connects the runtime sock on every Me paint.

## Ссылки

- ADR-782, ADR-783, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
