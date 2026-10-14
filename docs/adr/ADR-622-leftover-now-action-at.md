# ADR-622: leftover now_action_at is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-621 следующий свободный номер — **622**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live apps-grid hits already read `now_action_at_scrolled`
(ADR-220/498). The zero-scroll wrapper was leftover. Do not
launch apps. Leave Сейчас. Font compare stays a device task.
ADR-213 thin tuning is not this slice.

## Решение

1. Drop `now_action_at`.
2. Host tests call `now_action_at_scrolled(..., 0)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-220, ADR-498, ADR-621
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
