# ADR-647: leftover orb_visual_state is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-646 следующий свободный номер — **647**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live Orb already reads `orb_visual_state_with`. The
no-health/no-context convenience wrapper was leftover. Leave
Сейчас. Font compare stays a device task. ADR-213 thin tuning
is not this slice. Do not tap Orb.

## Решение

1. Drop `orb_visual_state`.
2. Host tests call `orb_visual_state_with(..., None, None)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-646
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
