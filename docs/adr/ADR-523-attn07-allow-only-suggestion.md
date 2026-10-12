# ADR-523: NOW suggestion requires Policy Allow

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-522 следующий свободный номер — **523**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ATTN-07 is one already-permitted OAM verb. `one_suggested_oam_with`
treated AskUser as permitted (`!Deny`). A High-risk tool would appear
on NOW even though execute still needs confirmation. Tap opens the
object and does not execute.

`display.inspect` stays Low / Allow. Object View still lists AskUser
actions; that screen is where confirmation lives.

## Решение

1. A suggestion candidate is permitted only on `PolicyVerdict::Allow`.
2. AskUser and Deny produce no NOW suggestion.
3. Empty tools still produce none.

## Откат

Restore `!Deny` as permitted.

## Ссылки

- ADR-508, ADR-511
- `docs/os/sprints/ATTN-ROADMAP.md` ATTN-07
