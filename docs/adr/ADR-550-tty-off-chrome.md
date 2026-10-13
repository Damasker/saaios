# ADR-550: tty stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512). Live shell is still
ADR-548 (`425fbafc…`).

## Нумерация

После ADR-549 следующий свободный номер — **550**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`tty` printed `tty` / `TTY`. That is a helper channel name, not a
board word. «Трасса» stays. Unknown tokens stay omitted.

## Решение

1. Collapsed cellular row does not print `tty`.
2. Observation `cellular.tty` has no row.
3. The fact is still read. Wire token stays. Do not send RadioState.

## Откат

Restore the `tty` / `TTY` phrases.

## Ссылки

- ADR-549
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
