# ADR-587: slot stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-586 следующий свободный номер — **587**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`slot` printed `слот` / «Слот». That is a helper dump, not a
board word. `cellular.aptime` is already off (ADR-586). «Трасса»
stays. SMSC / VoNR / PS stay. Do not send RadioState. Do not mix
modem WIP.

## Решение

1. Collapsed cellular row does not print `slot`.
2. Observation `cellular.slot` has no row.
3. The fact is still read. Wire token stays.

## Откат

Restore the `слот` / «Слот» phrases.

## Ссылки

- ADR-586, ADR-550
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
