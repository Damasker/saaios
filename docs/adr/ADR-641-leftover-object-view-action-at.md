# ADR-641: leftover object_view_action_at is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-640 следующий свободный номер — **641**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live Object View hits already read
`object_view_hit_scrolled`. The Action-only wrapper was
leftover. Leave Сейчас. Font compare stays a device task.
ADR-213 thin tuning is not this slice.

## Решение

1. Drop `object_view_action_at`.
2. Host tests call `object_view_hit_scrolled` and match
   `ObjectViewHit::Action`.

## Откат

Restore the leftover helper.

## Ссылки

- ADR-640
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
