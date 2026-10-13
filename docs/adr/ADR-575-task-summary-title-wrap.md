# ADR-575: TaskSummary long titles wrap

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-574 следующий свободный номер — **575**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-03 ContextHeader wrap is in (ADR-574). Section 7.6 still
required long Russian Task titles to wrap rather than truncate.
Related Intent caption stays a separate string. Gallery paint is
not this slice. Font compare stays a device task. ADR-213 thin
tuning is not this slice. Do not check VUI-02 wrap Acceptance.

## Решение

1. `title_lines` wraps the Task title at two lines through
   `wrap_words`.
2. Accessibility keeps the full title. Related caption is not
   flattened into the name.
3. Nested `StatusIndicator` wrap stays ADR-573. Gallery paint is
   not wired. Do not shrink `TextRole`.

## Откат

Remove `title_lines`.

## Ссылки

- ADR-574, ADR-573
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
