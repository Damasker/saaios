# ADR-527: Sistema «Записи» does not print MemoryKind

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-526 следующий свободный номер — **527**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-285 labels kind for the model prompt. Sistema chrome printed
`explicit_fact` next to the value. That is a schema token, not a
board word. «Known facts» stays absent. Learning is not this path.

## Решение

1. «Записи» shows key + value only. Kind is not painted.
2. `learned_hypothesis` is omitted, same class as Restricted.
3. No new Memory taxonomy. No MEM-09.

## Откат

Restore `value · kind` and keep hypothesis rows.

## Ссылки

- ADR-268, ADR-285, ADR-304
- `docs/os/sprints/MEM-ROADMAP.md` MEM-08
