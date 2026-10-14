# ADR-642: leftover object_view_content is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-641 следующий свободный номер — **642**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live Object View already reads
`object_view_content_with`. The connected-store convenience
wrapper was leftover. Leave Сейчас. Font compare stays a
device task. ADR-213 thin tuning is not this slice.

## Решение

1. Drop `object_view_content`.
2. Host tests call `object_view_content_with(..., true)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-641
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
