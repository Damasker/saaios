# ADR-631: leftover trusted_client_action_at is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-630 следующий свободный номер — **631**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live trusted-client list hits already read
`trusted_client_action_at_scrolled`. The zero-scroll wrapper
was leftover. Leave Сейчас. Font compare stays a device task.
ADR-213 thin tuning is not this slice.

## Решение

1. Drop `trusted_client_action_at`.
2. Host tests call `trusted_client_action_at_scrolled(..., 0)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-630
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
