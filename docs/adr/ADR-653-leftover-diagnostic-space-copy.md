# ADR-653: leftover diagnostic_space_copy is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-652 следующий свободный номер — **653**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Sistema space copy already is `space_display_name`. The alias
was leftover. Leave Сейчас. Font compare stays a device task.
ADR-213 thin tuning is not this slice. Do not convert space
keys.

## Решение

1. Drop `diagnostic_space_copy`.
2. Live rows and host tests call `space_display_name`.

## Откат

Restore the leftover alias.

## Ссылки

- ADR-652
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
