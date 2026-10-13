# ADR-584: cellular.screen stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-583 следующий свободный номер — **584**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`cellular.screen` printed `экран` / «Экран». That is a modem
helper, not the device display section. Sistema «Экран»
(brightness / lock / blank) stays. `unsol` is already off
(ADR-583). «Трасса» stays. Do not send RadioState. Do not mix
modem WIP.

## Решение

1. Collapsed cellular row does not print `screen`.
2. Observation `cellular.screen` has no row.
3. The fact is still read. Wire token stays. Device «Экран»
   stays.

## Откат

Restore the `экран` / «Экран» observation phrases.

## Ссылки

- ADR-583, ADR-550
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
