# ADR-478: VUI-09 — NOW current task rows from live running work

## Статус

Принято, 2026-10-11. Handoff `now-empty` / `now-one-item` / `now-default`
names 0/3 current tasks. `in_progress_work()` already lists running
and verifying Tasks. Tappable rows are `now.task.{i}` DataRow
Buttons. Confirmed empty among other NOW work is «Нет текущих
задач». Whole-screen empty stays «Ничего срочного» / offline
(ADR-112/114). Weather stays chrome, not work. No invented
percent. Experimental. PIN stays null. Leave Сейчас. Not Visual
v1 sign-off.

## Нумерация

После ADR-477 следующий свободный номер — **478**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-474 locked attention Buttons. Current work still painted as
procedural Task rows under «Продолжается». The board wants the
same stacked DataRow grammar and an honest empty caption when
other NOW work is visible.

## Decision

1. **Host-lock** a NOW document with N `now.task.{i}` Buttons after
   weather and attention. Dock them on `stacked_row_rect`.
2. **Live.** One Button per `in_progress_work()` Task. Tap opens
   that Task. Secondary is the sourced status phrase, never a
   guessed percent. Show every live running/verifying item.
3. **Empty.** Store connected and no current work, but another NOW
   section or attention item exists: Status «Нет текущих задач».
   Alone on an empty NOW: omit, keep the whole-screen calm/offline
   pattern.
4. **Not this slice.** Do not change `now-public.sui`. Do not
   convert leftover keyboard / gallery / lock formulas.

## Consequences

- Current work matches the board row counts without replacing
  ADR-112. Rollback: omit empty, paint «Продолжается» as the old
  Task section. Still not Visual v1 sign-off.

## Verification

Host: running Task hits `now.task.0` after weather; empty caption
has no action; attention hits stay on later slots. Panther: do
not flash; leave Сейчас.
