# ADR-574: ContextHeader Space name wraps

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-573 следующий свободный номер — **574**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-02 host wrap contracts are in (ADR-568–573). Section 7.1 still
required a long Space name to wrap to a second line rather than
truncate. The section title is not the Space name. Gallery paint is
not this slice. Font compare stays a device task. ADR-213 thin
tuning is not this slice. Do not check VUI-02 wrap Acceptance.

## Решение

1. `context_name_lines` wraps the Space name at two lines through
   `wrap_words`.
2. `heading_text` and accessibility keep the full
   `"{name} · {section}"` string.
3. Gallery paint is not wired. Do not shrink `TextRole::Title`.

## Откат

Remove `context_name_lines`.

## Ссылки

- ADR-573, ADR-107
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-03
