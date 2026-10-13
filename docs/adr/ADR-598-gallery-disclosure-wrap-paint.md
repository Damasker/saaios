# ADR-598: gallery Disclosure paint wraps long labels

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-597 следующий свободный номер — **598**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-572 wraps Disclosure labels in `saai-ui-core`. Gallery paint
still drew `disclosure.label` as one line. Metric paint already
wraps (ADR-597). Chevron stays 16–20. StatusIndicator paint stays
open. Font compare stays a device task. ADR-213 thin tuning is
not this slice.

## Решение

1. `gallery_disclosure_paint` uses `label_lines` and
   `min_hit_height`. Two lines grow to `TWO_LINE_ROW_HEIGHT`.
2. `draw_gallery_disclosure` measures with `text_width` and paints
   each line. Chevron mapping is unchanged. Accessibility keeps
   the full label.

## Откат

Restore the single-line `draw_text` of `disclosure.label`.

## Ссылки

- ADR-572, ADR-597
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
