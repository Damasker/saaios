# ADR-589: smsact stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-588 следующий свободный номер — **589**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`smsact` printed `активация` / «Активация». That is a helper dump,
not a board word. `cellular.sigcrit` is already off (ADR-588).
«Трасса» stays. SMSC / VoNR / PS stay. Do not send RadioState.
Do not mix modem WIP.

## Решение

1. Collapsed cellular row does not print `smsact`.
2. Observation `cellular.smsact` has no row.
3. The fact is still read. Wire token stays.

## Откат

Restore the `активация` / «Активация» phrases.

## Ссылки

- ADR-588, ADR-550
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
