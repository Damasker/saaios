# ADR-629: leftover wifi_list_action_at is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-628 следующий свободный номер — **629**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live Wi-Fi list hits already read
`wifi_list_action_at_scrolled`. The zero-scroll wrapper was
leftover. Leave Сейчас. Font compare stays a device task.
ADR-213 thin tuning is not this slice.

## Решение

1. Drop `wifi_list_action_at`.
2. Host tests call `wifi_list_action_at_scrolled(..., 0)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-628
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
