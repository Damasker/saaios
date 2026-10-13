# ADR-586: aptime stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-585 следующий свободный номер — **586**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`aptime` printed `время` / «Время». That is a helper dump, not a
board word. `cellular.cellinfo` is already off (ADR-585). «Трасса»
stays. SMSC / VoNR stay. Do not send RadioState. Do not mix
modem WIP.

## Решение

1. Collapsed cellular row does not print `aptime`.
2. Observation `cellular.aptime` has no row.
3. The fact is still read. Wire token stays.

## Откат

Restore the `время` / «Время» phrases.

## Ссылки

- ADR-585, ADR-550
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
