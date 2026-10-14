# ADR-646: leftover now_workflow_sections is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-645 следующий свободный номер — **646**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live NOW already reads `now_workflow_sections_with`. The
connected-store convenience wrapper was leftover. Leave
Сейчас. Font compare stays a device task. ADR-213 thin tuning
is not this slice.

## Решение

1. Drop `now_workflow_sections`.
2. Host tests call
   `now_workflow_sections_with(..., None, true, None)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-645
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
