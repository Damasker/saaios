# ADR-591: smscb stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-590 следующий свободный номер — **591**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`smscb` printed `рассылка` / «Рассылка». That is a helper dump,
not a board word. `cellular.linkcrit` is already off (ADR-590).
Sistema «Связь» stays. «Трасса» stays. SMSC / VoNR / PS stay.
Do not send RadioState. Do not mix modem WIP.

## Решение

1. Collapsed cellular row does not print `smscb`.
2. Observation `cellular.smscb` has no row.
3. The fact is still read. Wire token stays.

## Откат

Restore the `рассылка` / «Рассылка» phrases.

## Ссылки

- ADR-590, ADR-550
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
