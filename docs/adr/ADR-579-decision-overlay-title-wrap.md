# ADR-579: DecisionOverlay object names wrap

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-578 следующий свободный номер — **579**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-05 AgentSummary wrap is in (ADR-578). Section 7.8 object
names are the same long-Russian input. Missing facts stay
omitted. The overlay does not invent undo. Gallery paint is
not this slice. Font compare stays a device task. ADR-213
thin tuning is not this slice. Do not check VUI-02 wrap
Acceptance.

## Решение

1. `title_lines` wraps the object name at two lines through
   `wrap_words`.
2. Fact lines stay separate. Accessibility keeps the full
   object name.
3. Gallery paint is not wired. Do not shrink `TextRole`.

## Откат

Remove `title_lines`.

## Ссылки

- ADR-578, ADR-107
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
