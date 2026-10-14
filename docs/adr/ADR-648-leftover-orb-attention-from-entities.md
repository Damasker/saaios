# ADR-648: leftover orb_attention_from_entities is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-647 следующий свободный номер — **648**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live lock already reads `orb_attention`. The no-health
convenience wrapper was leftover. Leave Сейчас. Font compare
stays a device task. ADR-213 thin tuning is not this slice.
Do not tap Orb.

## Решение

1. Drop `orb_attention_from_entities`.
2. Host tests call `orb_attention(..., None)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-647
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
