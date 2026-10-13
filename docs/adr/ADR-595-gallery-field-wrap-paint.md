# ADR-595: gallery Field paint wraps label and support

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-594 следующий свободный номер — **595**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-569 wraps Field labels in `saai-ui-core`. Gallery paint still
drew `field.label` as one line. Button gallery paint already wraps
(ADR-594). The editor stays one line. DataRow / Metric /
Disclosure / StatusIndicator paint stay open. Font compare stays a
device task. ADR-213 thin tuning is not this slice.

## Решение

1. `gallery_field_paint` uses `label_lines` and `support_lines`.
2. `draw_gallery_field` paints wrapped Caption lines above the
   editor. The value stays one line. Hit height stays
   `MIN_TOUCH_TARGET`.
3. Error support uses Critical; help uses TextSecondary.
   Accessibility keeps the full label.

## Откат

Restore the single-line `draw_text` of `field.label`.

## Ссылки

- ADR-569, ADR-594
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
