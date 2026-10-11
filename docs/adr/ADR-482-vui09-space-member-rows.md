# ADR-482: VUI-09 — Space members as live DataRow Buttons

## Статус

Принято, 2026-10-11. Handoff Пространство names SOM members as
chevron DataRows. `space_member_rows()` already lists those
entities. Tappable rows are `spaces.member.{i}` Buttons. Empty is
«Нет объектов». Store down is «Нет связи», not zero. Focus Task
stays `spaces.focus` (ADR-480) and is not repeated. Trailing
Назад stays the leftover Button. Experimental. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-481 следующий свободный номер — **482**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-243 painted members as ActionCards on UUID locs. The board
wants the same stacked DataRow grammar as Object related and NOW
work. Inventing people or a map would skip SOM.

## Decision

1. **Host-lock** Space detail with N `spaces.member.{i}` Buttons
   after an optional focus row. Dock them on `stacked_row_rect`.
2. **Live.** One Button per remaining SOM member. Tap opens that
   entity. Secondary is the sourced kind label (`Намерение` /
   `Задача` / `Человек`).
3. **Empty.** Store connected and no members (and no focus):
   Status «Нет объектов». Store down: Status «Нет связи». Do not
   claim zero when the store is down.
4. **Not this slice.** Do not change `spaces-public.sui`. Do not
   invent a map. Do not convert leftover keyboard / gallery /
   lock formulas.

## Consequences

- Space members match the board row grammar without replacing
  ADR-243 membership. Rollback: UUID ActionCards. Still not
  Visual v1 sign-off.

## Verification

Host: live member hits `spaces.member.0`; empty/offline have no
action; focus still occupies slot 0 when running. Panther: do
not flash; leave Сейчас.
