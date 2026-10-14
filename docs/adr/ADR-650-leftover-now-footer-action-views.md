# ADR-650: leftover now_footer_action_views is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-649 следующий свободный номер — **650**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live NOW footer paint already reads
`now_footer_action_views_from`. The zero-tree convenience
wrapper was leftover. Leave Сейчас. Font compare stays a
device task. ADR-213 thin tuning is not this slice. Do not
launch apps.

## Решение

1. Drop `now_footer_action_views`.
2. Host tests call
   `now_footer_action_views_from(&now_view(...))`.

## Откат

Restore the leftover wrapper.

## Ссылки

- ADR-649
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
