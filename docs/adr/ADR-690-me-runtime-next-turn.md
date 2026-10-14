# ADR-690: defer runtime JSON off the Sistema camp turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `0e6ea3d9…` (ADR-689). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-689 следующий свободный номер — **690**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-689 classifies `/proc` inline. The camp turn still blocked
on runtime `status` JSON (250 ms timeout) for identity,
observations, and memory records. dest-no-lock stays off. Do
not start the shell from COM13. Do not type PSK.

## Решение

1. `me_facts()` uses phone-gate identity and empty
   observations / memory. Camp, `/proc`, and supervisor still
   run on the full-facts turn.
2. After Wi-Fi/`df`, `me_runtime_pending` reads runtime once
   and fills identity, «Наблюдения», and memory. Idle stays
   blocked while that turn is pending.
3. Host test: stripping runtime keeps `cellular_cp`; applying
   values restores observations. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Revert `me_runtime_pending` / `apply_me_runtime_probes`. Live
`0e6ea3d9…` still waits on runtime JSON during the camp turn.

## Ссылки

- ADR-672, ADR-686, ADR-689, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
