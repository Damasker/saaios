# ADR-601: gallery TaskSummary paint wraps the title

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-600 следующий свободный номер — **601**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-575 wraps TaskSummary titles in `saai-ui-core`. Composite
gallery still painted `task.status()` as a bare StatusIndicator,
so the title only wrapped through ADR-599. Related caption stays
a separate string. ContextHeader paint already wraps (ADR-600).
Sistema Task rows stay `status()`. Font compare stays a device
task. ADR-213 thin tuning is not this slice.

## Решение

1. `gallery_task_paint` uses `title_lines`. Height is Body ×
   titles, plus Caption for a visible reason and for related.
2. `draw_gallery_task` measures with `text_width` and paints
   wrapped title lines. Related stays one Caption line.
   Accessibility keeps the full title. Nested StatusIndicator
   wrap stays ADR-573 / ADR-599.

## Откат

Restore the `draw_status_indicator` of `task.status()`.

## Ссылки

- ADR-575, ADR-600
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
