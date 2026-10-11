# ADR-474: VUI-09 — NOW attention rows from the live projection

## Статус

Принято, 2026-10-11. Handoff `now-empty` / `now-one-item` / `now-default`
names 0/1/3 attention rows. `now_items()` already lists those
events. Tappable rows are `now.attention.{i}` DataRow Buttons.
Confirmed empty among other NOW work is «Ничего не требует
внимания». Whole-screen empty stays «Ничего срочного» /
offline (ADR-112/114). Mic stays later. Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-473 следующий свободный номер — **474**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-241 omitted empty attention. The board wants a truthful empty
caption when other work is visible, and a tap that opens the
source. Status-only rows cannot hit. Inventing weather or three
fake rows would skip the projection.

## Decision

1. **Host-lock** a NOW document with N `now.attention.{i}` Buttons
   docks them on `stacked_row_rect` below the NOW object.
2. **Live.** Emit one Button per `now_items()` whose source is a
   Task or notification. Health stays Status. Tap follows that
   entity. Show every live item; do not cap at 3.
3. **Empty.** Store connected and no `now_items`, but another NOW
   section exists: Status «Ничего не требует внимания». Alone
   on an empty NOW: omit, keep the whole-screen calm/offline
   pattern. Store down: do not claim empty.
4. **Not this slice.** Do not change `now-public.sui` leftover
   `now.empty`. Do not enable the composer mic. Do not invent
   weather.

## Consequences

- Attention matches the board row counts without replacing ADR-112.
  Rollback: omit empty, Status-only rows. Still not Visual v1
  sign-off.

## Verification

Host: 1/3 Buttons hit `now.attention.{i}`; empty caption has no
action; public `now.empty` stays Fill leftover. Panther: do not
flash; leave Сейчас.
