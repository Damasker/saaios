# ADR-542: ObjectSummary names notice and schedule

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-541 следующий свободный номер — **542**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-539 maps Search kinds onto ObjectSummary. Inbox still opens
`saaios.notification`; Сегодня still has `saaios.schedule`. Those
fell through to «Объект». «Уведомление» is ADR-088. «Расписание»
is ADR-036. Search membership stays unchanged.

## Решение

1. Notification meta is «Уведомление · версия N».
2. Schedule meta is «Расписание · версия N».
3. Unknown types stay «Объект». Search still omits both types.

## Откат

Drop the two match arms.

## Ссылки

- ADR-036, ADR-088, ADR-539
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-07 / VUI-09
