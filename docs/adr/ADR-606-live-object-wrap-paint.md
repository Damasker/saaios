# ADR-606: live ObjectSummary paint wraps the title

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-605 следующий свободный номер — **606**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-603 wired `title_lines` in the composite gallery.
`draw_object_summary` still painted `title_text()` and advanced
one Body line, so a wrapped title sat on the meta line. Meta
stays one identity line. Fact-key dump stays. Live overlay
headers stay `heading()`. Font compare stays a device task.
ADR-213 thin tuning is not this slice.

## Решение

1. `draw_object_summary` paints through `draw_gallery_object`
   (`title_lines` + one Caption meta) and returns that height
   before trailing status or value.
2. Accessibility keeps the full title; value stays the meta
   line. NOW / Object View keep their compiled slots. Leave
   Сейчас.

## Откат

Restore the `draw_semantic_text` of `title_text()` and the
fixed Body advance.

## Ссылки

- ADR-603, ADR-577
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-03
