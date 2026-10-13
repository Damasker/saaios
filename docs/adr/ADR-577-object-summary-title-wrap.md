# ADR-577: ObjectSummary long titles wrap

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-576 следующий свободный номер — **577**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-03 ContextHeader wrap is in (ADR-574). Section 7.3 Object
title is the same long-Russian input. Meta stays one identity
line (`{type} · версия {revision}`). Fact-key dump stays.
Gallery paint is not this slice. Font compare stays a device
task. ADR-213 thin tuning is not this slice. Do not check
VUI-02 wrap Acceptance.

## Решение

1. `title_lines` wraps the object title at two lines through
   `wrap_words`.
2. Meta and trailing status do not wrap here. Accessibility
   keeps the full title; value stays the meta line.
3. Gallery paint is not wired. Do not shrink `TextRole`.

## Откат

Remove `title_lines`.

## Ссылки

- ADR-576, ADR-574
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-03
