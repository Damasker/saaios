# ADR-547: gpsnfw stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512). Live shell is still
ADR-545 (`abd38350…`).

## Нумерация

После ADR-546 следующий свободный номер — **547**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`gpsnfw` printed `gpsnfw` / `NFW`. That is a helper token, not a
board word. GPS lock stays. Unknown tokens stay omitted.

## Решение

1. Collapsed cellular row does not print `gpsnfw`.
2. Observation `cellular.gpsnfw` has no row.
3. The fact is still read. Wire token stays. Do not send RadioState.

## Откат

Restore the `gpsnfw` / `NFW` phrases.

## Ссылки

- ADR-546
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
