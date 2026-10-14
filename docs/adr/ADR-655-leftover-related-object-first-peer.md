# ADR-655: leftover related_object_first_peer is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-654 следующий свободный номер — **655**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live related peers already read `related_object_peer_ids`.
The first-peer helper was leftover. Leave Сейчас. Font
compare stays a device task. ADR-213 thin tuning is not this
slice.

## Решение

1. Drop `related_object_first_peer`.
2. Host tests take the first `related_object_peer_ids` id.

## Откат

Restore the leftover helper.

## Ссылки

- ADR-654
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
