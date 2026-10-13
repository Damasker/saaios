# ADR-558: diagnostic space row is the space name

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512). Live shell is still
ADR-557 (`126b72c8…`).

## Нумерация

После ADR-557 следующий свободный номер — **558**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Диагностика printed `Дом (home)`. The name is already the space.
The parenthetical id is a schema token. `space_display_name`
already names known ids. ContextFrame still prints `space_id`
(ADR-538).

## Решение

1. The Пространство row is `space_display_name`.
2. ContextFrame `space_id` and confidence stay.
3. Do not invent a space taxonomy.

## Откат

Print `name (id)` again.

## Ссылки

- ADR-538, ADR-556, ADR-557
- `docs/os/sprints/HIA-ROADMAP.md` HIA-20
