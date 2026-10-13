# ADR-585: cellinfo stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-584 следующий свободный номер — **585**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`cellinfo` printed `соты` / «Соты». That is a helper dump, not a
board word. `cellular.screen` is already off (ADR-584). «Трасса»
stays. SMSC / VoNR stay. Do not send RadioState. Do not mix
modem WIP.

## Решение

1. Collapsed cellular row does not print `cellinfo`.
2. Observation `cellular.cellinfo` has no row.
3. The fact is still read. Wire token stays.

## Откат

Restore the `соты` / «Соты» phrases.

## Ссылки

- ADR-584, ADR-550
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
