# ADR-578: AgentSummary execution titles wrap

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-577 следующий свободный номер — **578**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-05 Intent / Object wrap is in (ADR-576/577). Section 7.9
execution titles are the same long-Russian input. Unassigned
stays «Нет исполнения». No worker count. Gallery paint is not
this slice. Font compare stays a device task. ADR-213 thin
tuning is not this slice. Do not check VUI-02 wrap Acceptance.

## Решение

1. Execution `title_lines` wraps at two lines through
   `wrap_words`.
2. Unassigned / unavailable stay one caption. Accessibility
   keeps the full title.
3. Gallery paint is not wired. Do not invent `Воркеры (3)`.

## Откат

Remove `title_lines`.

## Ссылки

- ADR-577, ADR-576
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
