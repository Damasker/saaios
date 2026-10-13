# ADR-596: gallery DataRow paint reflows long content

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-595 следующий свободный номер — **596**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-570 wraps DataRow primary/secondary in `saai-ui-core`. Paint
still drew each as one line. Field gallery paint already wraps
(ADR-595). Value stays one trailing line. Metric / Disclosure /
StatusIndicator paint stay open. Font compare stays a device
task. ADR-213 thin tuning is not this slice.

## Решение

1. `gallery_data_row_paint` uses `primary_lines` and
   `secondary_lines`. Height follows `visual_height`.
2. `draw_data_row` measures with `text_width` and paints wrapped
   lines. Value stays one line. Accessibility keeps the full
   primary.

## Откат

Restore the single-line `draw_text` of `row.primary` /
`row.secondary`.

## Ссылки

- ADR-570, ADR-595
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
