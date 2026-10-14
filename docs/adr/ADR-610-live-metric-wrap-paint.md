# ADR-610: live Sistema Metric paint wraps the label

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-609 следующий свободный номер — **610**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-597 wired `label_lines` in the gallery. Sistema
`SystemSectionRow::Metric` still painted `metric.label` as one
Caption and advanced one Body for the value. Value plus unit
stay one line. Intent paint already wraps (ADR-609). Font
compare stays a device task. ADR-213 thin tuning is not this
slice.

## Решение

1. `draw_gallery_metric` returns the `gallery_metric_paint`
   height.
2. Sistema Metric rows paint through `draw_gallery_metric` and
   advance by that height. Accessibility keeps the full label.
   Leave Сейчас.

## Откат

Restore the single-line `draw_text` of `metric.label` and the
fixed Caption + Body advance.

## Ссылки

- ADR-597, ADR-571
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
