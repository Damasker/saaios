# ADR-605: gallery DecisionOverlay paint wraps the object name

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-604 следующий свободный номер — **605**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-579 wraps DecisionOverlay object names in `saai-ui-core`.
Composite gallery still dumped `fact_lines` as one Caption and
never painted `title_lines`. Fact lines stay separate. Missing
facts stay omitted. AgentSummary paint already wraps (ADR-604).
Live overlay `heading()` stays unpainted. Font compare stays a
device task. ADR-213 thin tuning is not this slice.

## Решение

1. `gallery_decision_paint` uses `title_lines`. Height is Title ×
   lines plus Caption when facts exist.
2. `draw_gallery_decision` measures with `text_width` and paints
   wrapped object-name lines. Facts stay a separate Caption dump.
   Accessibility keeps the full object name. Accept / decline
   stay `draw_gallery_button`.

## Откат

Restore the joined `fact_lines` Caption `draw_semantic_text`.

## Ссылки

- ADR-579, ADR-604
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
