# ADR-546: prefmodem stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512). Live shell is still
ADR-545 (`abd38350…`).

## Нумерация

После ADR-545 следующий свободный номер — **546**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`prefmodem` printed `pref` / `Pref`. That is a helper truncation,
not a board word. Unknown tokens stay omitted.

## Решение

1. Collapsed cellular row does not print `pref`.
2. Observation `cellular.prefmodem` has no row.
3. The fact is still read. Wire token stays. Do not send RadioState.

## Откат

Restore the `pref` / `Pref` phrases.

## Ссылки

- ADR-543, ADR-544, ADR-545
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
