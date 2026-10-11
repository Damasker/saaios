# ADR-480: VUI-09 — Space focus task from live in-space work

## Статус

Принято, 2026-10-11. Handoff Пространство names a focus task only
when it is linked to the current space and live. `in_progress_work()`
already lists running and verifying Tasks. The row is `spaces.focus`
DataRow Button. No live member Task: omit, do not invent a map or
caption. Members stay SOM ActionCards (ADR-243). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-479 следующий свободный номер — **480**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-243 opened Space detail as SOM members. The board also wants
the current live Task of that space as a distinct focus row. A
synthetic focus or a Ready Android-style map would skip the store.

## Decision

1. **Host-lock** Space detail with `spaces.focus` Button on
   `stacked_row_rect` above members when a live Task is a SOM
   member.
2. **Live.** One Button: first `in_progress_work()` Task that
   `space_member_rows()` also lists. Tap opens that Task. Secondary
   is the sourced status phrase. That Task is not repeated as a
   member card.
3. **Empty.** No live in-space Task: omit the row. Store down:
   omit. Do not claim «Нет фокусной задачи». Whole-space empty
   stays «Нет объектов» / «Нет связи».
4. **Not this slice.** Do not change `spaces-public.sui`. Do not
   invent a map, people cluster, or Android VM. Do not convert
   leftover keyboard / gallery / lock formulas.

## Consequences

- Space focus matches the board without replacing ADR-243 members.
  Rollback: drop `spaces.focus`, restore the Task as a member card.
  Still not Visual v1 sign-off.

## Verification

Host: running member Task hits `spaces.focus` at stacked slot 0;
peer member stays on slot 1; offline omits focus. Panther: do not
flash; leave Сейчас.
