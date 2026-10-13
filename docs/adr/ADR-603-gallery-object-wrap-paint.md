# ADR-603: gallery ObjectSummary paint wraps the title

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-602 следующий свободный номер — **603**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-577 wraps ObjectSummary titles in `saai-ui-core`. Composite
gallery still drew `object.title_text()` as one Body
`SemanticText`. Meta stays one identity line. Fact-key dump
stays. IntentSummary paint already wraps (ADR-602). Live
`draw_object_summary` stays `title_text()`. Font compare stays a
device task. ADR-213 thin tuning is not this slice.

## Решение

1. `gallery_object_paint` uses `title_lines`. Height is Body ×
   titles plus one Caption for meta.
2. `draw_gallery_object` measures with `text_width` and paints
   wrapped title lines. Meta stays one Caption line.
   Accessibility keeps the full title; value stays the meta
   line. Do not shrink `TextRole::Body`.

## Откат

Restore the `draw_semantic_text` of `object.title_text()`.

## Ссылки

- ADR-577, ADR-602
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-03
