# ADR-607: live ContextHeader paint wraps the Space name

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-606 следующий свободный номер — **607**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-600 wired `context_name_lines` in the composite gallery.
`paint_overlay_context_header` still painted `heading()` as one
Title `SemanticText` and advanced one Title line. The section
title is not the Space name. `heading_text` stays the full
`"{name} · {section}"` string. Live ObjectSummary already wraps
(ADR-606). Font compare stays a device task. ADR-213 thin
tuning is not this slice.

## Решение

1. `paint_overlay_context_header` paints through
   `draw_gallery_context_header` and advances by that height
   before the optional lifecycle mark.
2. Section title stays one Label line. Accessibility keeps the
   full heading. Leave Сейчас.

## Откат

Restore the `draw_semantic_text` of `header.heading()` and the
fixed Title advance.

## Ссылки

- ADR-600, ADR-574
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-03
