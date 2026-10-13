# ADR-549: sgc stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512). Live shell is still
ADR-548 (`425fbafc…`).

## Нумерация

После ADR-548 следующий свободный номер — **549**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`sgc` printed `sgc` / `SGC`. That is a vendor helper name, not a
board word. Unknown tokens stay omitted.

## Решение

1. Collapsed cellular row does not print `sgc`.
2. Observation `cellular.sgc` has no row.
3. The fact is still read. Wire token stays. Do not send RadioState.

## Откат

Restore the `sgc` / `SGC` phrases.

## Ссылки

- ADR-546, ADR-547, ADR-548
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
