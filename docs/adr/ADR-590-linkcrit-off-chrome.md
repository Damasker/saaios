# ADR-590: linkcrit stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-589 следующий свободный номер — **590**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`linkcrit` printed `связь` / «Связь». That is a modem helper
dump, not the Sistema connectivity section. Sistema «Связь»
(Wi-Fi / Bluetooth) stays. `smsact` is already off (ADR-589).
«Трасса» stays. SMSC / VoNR / PS stay. Do not send RadioState.
Do not mix modem WIP.

## Решение

1. Collapsed cellular row does not print `linkcrit`.
2. Observation `cellular.linkcrit` has no row.
3. The fact is still read. Wire token stays. Device «Связь»
   stays.

## Откат

Restore the `связь` / «Связь» observation phrases.

## Ссылки

- ADR-589, ADR-584, ADR-550
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
