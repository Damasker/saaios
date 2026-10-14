# ADR-672: defer Sistema observations to the next turn

## Статус

Принято, 2026-10-14. Host. Live `/data/saaios/system/saai-shell`
is `7c3b9bac…` (ADR-670). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked.

## Нумерация

После ADR-671 следующий свободный номер — **672**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-671 recorded that Sistema works. Warm revisit sleeps.
The first fill is still ~30s because `me_system_sections`
inserts «Наблюдения» at section index 1 whenever
`facts.observations` is non-empty. That list sits on the
first screen, so the first flatten and the first window
still compile every observation row on the same turn as
`me_facts()`.

## Решение

1. `me_facts_for_first_paint` clears `observations` until
   `me_obs_ready`. The first flatten is device chrome only.
2. That first capture sets `me_obs_pending`. The next
   event-loop turn marks ready, drops the light cache, and
   recaptures with live observations.
3. Facts-pending and obs-pending are not processed on the
   same turn. dest-no-lock stays off. Do not flash this ADR.

## Откат

Revert `me_obs_ready` / `me_obs_pending` /
`me_facts_for_first_paint`. Live `7c3b9bac…` still works
after a long first fill.

## Ссылки

- ADR-664, ADR-669, ADR-671, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
