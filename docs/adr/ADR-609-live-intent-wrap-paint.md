# ADR-609: live Sistema IntentSummary paint wraps the title

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-608 следующий свободный номер — **609**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-602 wired `title_lines` in the composite gallery. Sistema
`SystemSectionRow::Intent` still painted `intent.heading()` as
one Body `SemanticText` and nested Task through `status()`.
Missing work stays «Нет задачи». Sistema Task already wraps
(ADR-608). Font compare stays a device task. ADR-213 thin
tuning is not this slice. Do not type Intent.

## Решение

1. Sistema Intent rows paint through `draw_gallery_intent`
   (`title_lines` + missing caption or nested
   `draw_gallery_task`) and advance by that height.
2. Accessibility keeps the full title. «Нет задачи» stays.
   Leave Сейчас.

## Откат

Restore the `draw_semantic_text` of `intent.heading()` and the
nested `status()` StatusIndicator.

## Ссылки

- ADR-602, ADR-608
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
