# ADR-594: gallery Button paint wraps long labels

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-593 следующий свободный номер — **594**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-568 wraps Button labels in `saai-ui-core`. Gallery paint
still drew `button.label` as one centered line. SemanticText
already wraps (ADR-107). Field / DataRow / Metric / Disclosure
/ StatusIndicator paint stay open. Font compare stays a device
task. ADR-213 thin tuning is not this slice.

## Решение

1. `gallery_button_paint` uses `Button::label_lines` and
   `min_hit_height`. Two lines grow to `TWO_LINE_ROW_HEIGHT`.
2. `draw_gallery_button` measures with `text_width` and paints
   each line. Type size stays `Label`.
3. The gallery demo label is the same wrap fixture as ADR-568.
   Accessibility keeps the full name.

## Откат

Restore the single-line `draw_text_centered` of `button.label`.

## Ссылки

- ADR-107, ADR-568, ADR-593
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
