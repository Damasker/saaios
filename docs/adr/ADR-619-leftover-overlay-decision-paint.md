# ADR-619: leftover overlay_decision_paint is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-618 следующий свободный номер — **619**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live consent / pair paint already reads `status_overlay_paint`
(ADR-433). Hits stay `overlay_buttons_v2_source` (ADR-442/444).
`overlay_decision_paint` was a leftover buttons-only tree. Do
not tap Разрешить. Leave Сейчас. Font compare stays a device
task. ADR-213 thin tuning is not this slice.

## Решение

1. Drop `overlay_decision_paint`.
2. Host overlay button geometry tests use `status_overlay_paint`.

## Откат

Restore the leftover helper and its test calls.

## Ссылки

- ADR-229, ADR-433, ADR-442, ADR-444
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
