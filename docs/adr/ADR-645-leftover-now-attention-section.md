# ADR-645: leftover now_attention_section is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-644 следующий свободный номер — **645**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live NOW already reads `now_attention_section_with`. The
no-health/no-context convenience wrapper was leftover. Leave
Сейчас. Font compare stays a device task. ADR-213 thin tuning
is not this slice.

## Решение

1. Drop `now_attention_section`.
2. Host tests call `now_attention_section_with(..., None, None)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-644
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
