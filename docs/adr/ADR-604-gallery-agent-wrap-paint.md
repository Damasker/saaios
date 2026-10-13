# ADR-604: gallery AgentSummary paint wraps the execution title

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-603 следующий свободный номер — **604**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-578 wraps AgentSummary execution titles in `saai-ui-core`.
Composite gallery still drew `agent.caption()` as one
`SemanticText`. Unassigned stays «Нет исполнения». No worker
count. ObjectSummary paint already wraps (ADR-603). Font compare
stays a device task. ADR-213 thin tuning is not this slice.

## Решение

1. `gallery_agent_paint` uses `title_lines`. Execution height is
   Body × lines. Unassigned / unavailable stay one Caption.
2. `draw_gallery_agent` measures with `text_width` and paints
   wrapped execution lines. Accessibility keeps the full title.
   Do not invent `Воркеры (3)`.

## Откат

Restore the `draw_semantic_text` of `agent.caption()`.

## Ссылки

- ADR-578, ADR-603
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
