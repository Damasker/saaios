# ADR-652: leftover space_member_rows is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-651 следующий свободный номер — **652**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Space members already are `search_rows`. The alias was
leftover. Leave Сейчас. Font compare stays a device task.
ADR-213 thin tuning is not this slice. Do not invent people.

## Решение

1. Drop `space_member_rows`.
2. Focus and detail members call `search_rows`.

## Откат

Restore the leftover alias.

## Ссылки

- ADR-651
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
