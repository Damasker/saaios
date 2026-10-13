# ADR-538: DevSurface ContextFrame source uses board words

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-537 следующий свободный номер — **538**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

HIA-20 names the diagnostic row `ContextFrame`. The value still
printed `manual` / `wifi` from `ContextSource`. Those are enum
tokens. «вручную» already exists on cellular selection. «Wi-Fi»
is already a board word.

## Решение

1. Keep the row label `ContextFrame`.
2. Manual is «вручную». Wifi is «Wi-Fi».
3. `space_id` and confidence stay. No new context taxonomy.

## Откат

Restore `manual` / `wifi` in `dev_surface_rows`.

## Ссылки

- ADR-087, ADR-537
- `docs/os/sprints/HIA-ROADMAP.md` HIA-20
