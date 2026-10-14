# ADR-671: Sistema works on the windowed flash

## Статус

Принято, 2026-10-14. Live `/data/saaios/system/saai-shell` is
`7c3b9bac…` (ADR-670). dest-no-lock is gone. displayd pid
2694 stayed. Shell 10333. The user confirmed Sistema works.
Search FramePace `p95_scroll=12` `p95_ok=1` `idle_ok=1`
seq=180. Warm Sistema switch: `surface=me` `p95_scroll=11`
`p95_ok=1` seq=185, process `S`. Unlock stays the proven
path (ADR-564). Experimental. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off. Not VUI-09 Acceptance. Wrap
Acceptance stays unchecked. VUI-06 Acceptance stays
unchecked.

## Нумерация

После ADR-670 следующий свободный номер — **671**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-670 painted only the visible Sistema window. The user
said it now works and asked to continue. dest-no-lock stays
off. Do not start the shell from COM13. Do not tap Orb.

## Решение

1. Record the user confirmation. No binary change.
2. Search already scrolled with `p95_ok=1`. Warm Me switch
   from Search committed and slept.
3. dest-no-lock stays off. First-open fact flatten is still
   heavy; that is the next slice, not this ADR.

## Откат

No binary change.

## Ссылки

- ADR-670, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
