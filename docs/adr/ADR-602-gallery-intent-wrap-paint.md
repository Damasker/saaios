# ADR-602: gallery IntentSummary paint wraps the title

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-601 следующий свободный номер — **602**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-576 wraps IntentSummary titles in `saai-ui-core`. Composite
gallery still drew `intent.heading()` as one Body `SemanticText`.
Missing work stays «Нет задачи». TaskSummary paint already wraps
(ADR-601). Sistema Intent rows stay `heading()`. Font compare
stays a device task. ADR-213 thin tuning is not this slice.

## Решение

1. `gallery_intent_paint` uses `title_lines`. Height is Body ×
   titles, plus Caption when work is missing.
2. `draw_gallery_intent` measures with `text_width` and paints
   wrapped title lines. «Нет задачи» stays one Caption line.
   Nested Task paint stays ADR-601. Accessibility keeps the
   full title. Do not shrink `TextRole::Body`.

## Откат

Restore the `draw_semantic_text` of `intent.heading()`.

## Ссылки

- ADR-576, ADR-601
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
