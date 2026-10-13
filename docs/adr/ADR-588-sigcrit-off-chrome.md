# ADR-588: sigcrit stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-587 следующий свободный номер — **588**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`sigcrit` printed `критерий` / «Критерий». That is a helper dump,
not a board word. `cellular.slot` is already off (ADR-587).
«Трасса» stays. SMSC / VoNR / PS stay. Do not send RadioState.
Do not mix modem WIP.

## Решение

1. Collapsed cellular row does not print `sigcrit`.
2. Observation `cellular.sigcrit` has no row.
3. The fact is still read. Wire token stays.

## Откат

Restore the `критерий` / «Критерий» phrases.

## Ссылки

- ADR-587, ADR-550
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
