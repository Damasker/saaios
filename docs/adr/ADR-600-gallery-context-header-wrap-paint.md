# ADR-600: gallery ContextHeader paint wraps the Space name

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-599 следующий свободный номер — **600**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-574 wraps ContextHeader Space names in `saai-ui-core`.
Composite gallery still drew `header.heading()` as one Title
`SemanticText`. Primitive gallery paint is wired (ADR-594–599).
Section title is not the Space name. Live overlay headers stay
`heading()`. Font compare stays a device task. ADR-213 thin
tuning is not this slice.

## Решение

1. `gallery_context_header_paint` uses `context_name_lines`. Height
   is Title × lines.
2. `draw_gallery_context_header` measures with `text_width` and
   paints wrapped Space-name lines. The section title stays one
   Label line. `heading_text` and accessibility keep the full
   `"{name} · {section}"` string. Do not shrink `TextRole::Title`.

## Откат

Restore the `draw_semantic_text` of `header.heading()`.

## Ссылки

- ADR-574, ADR-599
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-03
