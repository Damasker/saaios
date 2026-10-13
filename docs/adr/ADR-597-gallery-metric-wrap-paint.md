# ADR-597: gallery Metric paint wraps the label

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-596 следующий свободный номер — **597**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-571 wraps Metric labels in `saai-ui-core`. Gallery paint still
drew `metric.label` as one line. DataRow paint already reflows
(ADR-596). Known value plus unit stay one Title line. Disclosure
/ StatusIndicator paint stay open. Font compare stays a device
task. ADR-213 thin tuning is not this slice.

## Решение

1. `gallery_metric_paint` uses `label_lines` and `verified_lines`.
2. `draw_gallery_metric` paints wrapped Caption lines, then the
   value, then verified. Accessibility keeps the full label.

## Откат

Restore the single-line `draw_text` of `metric.label`.

## Ссылки

- ADR-571, ADR-596
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
