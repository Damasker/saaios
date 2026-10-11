# ADR-479: VUI-09 — NOW next-action rows from live next work

## Статус

Принято, 2026-10-11. Handoff `now-empty` / `now-one-item` / `now-default`
names the next direct action after current work. `next_work()` already
picks a pending Action else the first derived-ready Task. Tappable
rows are `now.next.{i}` DataRow Buttons. Confirmed empty among other
NOW work is «Нет следующего действия». Whole-screen empty stays
«Ничего срочного» / offline (ADR-112/114). Weather stays chrome, not
work. No invented percent. Experimental. PIN stays null. Leave
Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-478 следующий свободный номер — **479**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-478 locked current-work Buttons. «Далее» still painted as a
procedural Task/Data section. The board wants the same stacked
DataRow grammar and an honest empty caption when other NOW work is
visible.

## Decision

1. **Host-lock** a NOW document with N `now.next.{i}` Buttons after
   weather, attention, and current tasks. Dock them on
   `stacked_row_rect`.
2. **Live.** One Button from `next_work()` (pending Action else first
   derived-ready Task). Tap opens that entity. Secondary is the
   sourced status phrase, never a guessed percent.
3. **Empty.** Store connected and no next work, but another NOW
   section, attention, or current task exists: Status «Нет
   следующего действия». Alone on an empty NOW: omit, keep the
   whole-screen calm/offline pattern.
4. **Not this slice.** Do not change `now-public.sui`. Do not
   convert leftover keyboard / gallery / lock formulas.

## Consequences

- Next action matches the board row grammar without replacing
  ADR-112. Rollback: omit empty, paint «Далее» as the old
  Task/Data section. Still not Visual v1 sign-off.

## Verification

Host: pending Action hits `now.next.0` after weather; empty caption
has no action; current-task hits stay on earlier slots. Panther: do
not flash; leave Сейчас.
