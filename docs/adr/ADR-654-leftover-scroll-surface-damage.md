# ADR-654: leftover scroll_surface_damage is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-653 следующий свободный номер — **654**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Content-only scroll damage already is `root_content_rect`.
The alias was leftover. Leave Сейчас. Font compare stays a
device task. ADR-213 thin tuning is not this slice.

## Решение

1. Drop `scroll_surface_damage`.
2. Live draw and host tests call `root_content_rect`.

## Откат

Restore the leftover alias.

## Ссылки

- ADR-653
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
