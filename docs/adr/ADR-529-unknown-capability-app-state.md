# ADR-529: unknown capability and app state are not schema tokens

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-528 следующий свободный номер — **529**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`capability_label` and `app_state_label` mapped the live vocabulary,
then printed the raw English token for anything else. A spoofed
`net.bluetooth` or `hacking` became a board word.

## Решение

1. Known capabilities keep their Russian copy.
2. Any other capability name is «недоступно».
3. Known appd states keep their Russian copy.
4. Any other app state is «Нет статуса».
5. No new grant or app state.

## Откат

Restore `other => other`.

## Ссылки

- ADR-521, ADR-522
