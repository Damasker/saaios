# ADR-539: ObjectSummary meta uses the Search kind label

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-538 следующий свободный номер — **539**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-137 keeps ObjectSummary meta as type/version, not workflow
status. Live NOW and Object View still painted `saaios.intent`.
Search already names that type «Намерение». The dotted id is a
schema token, not a board word.

## Решение

1. Meta stays `{kind} · версия {revision}`.
2. `kind` is `search_kind_label`. Unknown types are «Объект», not
   the dotted id.
3. Trailing status is unchanged. Do not convert leftover formulas.

## Откат

Restore `entity.entity_type` in `object_summary_meta`.

## Ссылки

- ADR-137, ADR-529
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-07 / VUI-09
