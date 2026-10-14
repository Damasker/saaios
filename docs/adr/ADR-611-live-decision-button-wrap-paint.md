# ADR-611: live Object View DecisionOverlay buttons wrap

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-610 следующий свободный номер — **611**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-594 wired `label_lines` for gallery buttons. Object View
still painted overlay accept/decline through
`draw_text_centered` of the raw label. `heading()` stays
unpainted (ADR-157). Metric paint already wraps (ADR-610).
Font compare stays a device task. ADR-213 thin tuning is not
this slice. Do not tap Подтвердить or Отклонить.

## Решение

1. When a `DecisionOverlay` is present, Object View paints
   `accept` / `decline` through `draw_gallery_button`.
2. Non-overlay actions stay the centered Label. Accessibility
   keeps the full button name. Leave Сейчас.

## Откат

Restore the `overlay_labels` `draw_text_centered` path.

## Ссылки

- ADR-594, ADR-157
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
