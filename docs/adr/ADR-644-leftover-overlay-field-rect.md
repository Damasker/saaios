# ADR-644: leftover overlay_field_rect is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-643 следующий свободный номер — **644**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Compose / PIN / lock field helpers already sit on
`overlay_field_rect_with`. The keyboard-true convenience
wrapper was leftover. Leave Сейчас. Font compare stays a
device task. ADR-213 thin tuning is not this slice.

## Решение

1. Drop `overlay_field_rect`.
2. `intent_field_rect`, `pin_setup_field_rect`, and
   `lock_pin_field_rect` call
   `overlay_field_rect_with(..., true)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-643
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
