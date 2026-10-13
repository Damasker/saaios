# ADR-583: unsol stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-582 следующий свободный номер — **583**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`unsol` printed `отбор` / «Отбор». That is a helper token, not a
board word. `unsolff` is already off (ADR-582). «Трасса» stays.
Unknown tokens stay omitted. Do not send RadioState. Do not mix
modem WIP.

## Решение

1. Collapsed cellular row does not print `unsol`.
2. Observation `cellular.unsol` has no row.
3. The fact is still read. Wire token stays.

## Откат

Restore the `отбор` / «Отбор» phrases.

## Ссылки

- ADR-582, ADR-550
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
