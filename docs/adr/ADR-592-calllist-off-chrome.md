# ADR-592: calllist stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-591 следующий свободный номер — **592**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`calllist` printed `список` / «Список». That is a helper dump,
not a board word. `cellular.smscb` is already off (ADR-591).
«Трасса» stays. SMSC / VoNR / PS / GPS / SA stay. Do not send
RadioState. Do not mix modem WIP.

## Решение

1. Collapsed cellular row does not print `calllist`.
2. Observation `cellular.calllist` has no row.
3. The fact is still read. Wire token stays.

## Откат

Restore the `список` / «Список» phrases.

## Ссылки

- ADR-591, ADR-550
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
