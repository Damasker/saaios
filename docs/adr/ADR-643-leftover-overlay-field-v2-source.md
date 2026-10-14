# ADR-643: leftover overlay_field_v2_source is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-642 следующий свободный номер — **643**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live compose/lock/wifi Field paint already has
`with_keyboard` and can call `overlay_field_v2_source_with`
directly. The keyboard-true convenience wrapper was leftover.
Leave Сейчас. Font compare stays a device task. ADR-213 thin
tuning is not this slice.

## Решение

1. Drop `overlay_field_v2_source`.
2. `overlay_field_paint` calls
   `overlay_field_v2_source_with(screen_id, field_id, with_keyboard)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-642
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
