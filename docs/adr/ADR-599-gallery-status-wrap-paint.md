# ADR-599: gallery StatusIndicator paint wraps reason below the label

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-598 следующий свободный номер — **599**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-573 wraps StatusIndicator label and reason in `saai-ui-core`.
Gallery and Sistema paint still drew `indicator.label` and
`visible_reason()` as one line each and advanced `cursor_y` by a
fixed Body + optional Caption. Disclosure paint already wraps
(ADR-598). Font compare stays a device task. ADR-213 thin tuning
is not this slice.

## Решение

1. `gallery_status_paint` uses `label_lines` and `reason_lines`.
   Height is Body × labels plus Caption × reason, at least the
   Medium mark.
2. `draw_status_indicator` takes `max_width`, measures with
   `text_width`, paints wrapped lines, and returns that height.
   Compact still hides the reason. Accessibility keeps the full
   label. Sistema, lock attention, object summary, ContextHeader,
   and both galleries pass a width and use the returned height.

## Откат

Restore the single-line `draw_text` of `indicator.label` and
`visible_reason()`.

## Ссылки

- ADR-573, ADR-598
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
