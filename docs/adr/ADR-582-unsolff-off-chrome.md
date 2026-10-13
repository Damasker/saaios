# ADR-582: unsolff stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-581 следующий свободный номер — **582**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`unsolff` printed `фильтр` / «Фильтр». That is a helper filter
name, not a board word. `unsol` / «Отбор» stays this slice.
«Трасса» stays. Unknown tokens stay omitted. Do not send
RadioState. Do not mix modem WIP.

## Решение

1. Collapsed cellular row does not print `unsolff`.
2. Observation `cellular.unsolff` has no row.
3. The fact is still read. Wire token stays.

## Откат

Restore the `фильтр` / «Фильтр» phrases.

## Ссылки

- ADR-550, ADR-581
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
