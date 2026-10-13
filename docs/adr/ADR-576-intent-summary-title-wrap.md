# ADR-576: IntentSummary long titles wrap

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-575 следующий свободный номер — **576**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-05 TaskSummary wrap is in (ADR-575). Section 7.7 Intent
titles are the same long-Russian input. Missing work stays
«Нет задачи». No worker list. Gallery paint is not this slice.
Font compare stays a device task. ADR-213 thin tuning is not
this slice. Do not check VUI-02 wrap Acceptance.

## Решение

1. `title_lines` wraps the Intent title at two lines through
   `wrap_words`.
2. Accessibility keeps the full title. Nested Task wrap stays
   ADR-575. Missing-task caption is unchanged.
3. Gallery paint is not wired. Do not shrink `TextRole`.

## Откат

Remove `title_lines`.

## Ссылки

- ADR-575, ADR-107
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
