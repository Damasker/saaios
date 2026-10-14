# ADR-640: leftover object_overlay_paint is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-639 следующий свободный номер — **640**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live Object View overlay paint already reads
`object_overlay_paint_scrolled`. The zero-scroll wrapper was
leftover. `object_overlay_scroll_max` now calls scrolled at
offset 0. Leave Сейчас. Font compare stays a device task.
ADR-213 thin tuning is not this slice.

## Решение

1. Drop `object_overlay_paint`.
2. Host tests and `object_overlay_scroll_max` call
   `object_overlay_paint_scrolled(..., 0)`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-639
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
