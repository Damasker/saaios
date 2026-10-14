# ADR-608: live Sistema TaskSummary paint wraps the title

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-607 следующий свободный номер — **608**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-601 wired `title_lines` in the composite gallery. Sistema
`SystemSectionRow::Task` still painted `task.status()` as a bare
StatusIndicator and advanced related as a second Caption.
Related stays a separate string. Nested Intent Task rows stay
`status()`. Live ContextHeader already wraps (ADR-607). Font
compare stays a device task. ADR-213 thin tuning is not this
slice.

## Решение

1. Sistema Task rows paint through `draw_gallery_task`
   (`title_lines` + optional reason + related) and advance by
   that height.
2. Accessibility keeps the full title. Related is not flattened
   into the name. Leave Сейчас.

## Откат

Restore the `draw_status_indicator` of `task.status()` and the
separate related Caption advance.

## Ссылки

- ADR-601, ADR-575
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
