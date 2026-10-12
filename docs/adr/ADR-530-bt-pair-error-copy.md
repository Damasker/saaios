# ADR-530: Bluetooth pair error does not print the log token

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-529 следующий свободный номер — **530**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`bt-pair` writes `PAIR-ERROR\tTIMEOUT` and similar tokens. Chrome
appended the raw token: «Ошибка сопряжения: timeout». That is a
log word, not a board label. WORK-06 already names timeout
«Таймаут».

## Решение

1. `timeout` / `TIMEOUT` is «Ошибка сопряжения: Таймаут».
2. Any other pair-error token is «Ошибка сопряжения» without the
   English suffix. No new BT taxonomy.
3. The log parser still returns the raw token.

## Откат

Restore `Ошибка сопряжения: {reason}`.

## Ссылки

- ADR-296, ADR-522
