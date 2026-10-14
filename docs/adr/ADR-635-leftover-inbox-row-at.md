# ADR-635: leftover inbox_row_at is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-634 следующий свободный номер — **635**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live Inbox hits already read `inbox_row_at_scrolled`. The
zero-scroll wrapper was leftover. Leave Сейчас. Font compare
stays a device task. ADR-213 thin tuning is not this slice.

## Решение

1. Drop `inbox_row_at`.
2. Host tests call `inbox_row_at_scrolled(..., 0)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-634
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
