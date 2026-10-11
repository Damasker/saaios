# ADR-484: VUI-09 — Search results as live DataRow Buttons

## Статус

Принято, 2026-10-11. Handoff composer/search names a Search
destination with chevron DataRows. `search_rows()` already lists
space objects. Tappable rows are `search.item.{i}` Buttons. Empty
is «Нет объектов». Store down is «Нет связи», not zero. Public
`search-public.sui` stays the single leftover `search.item`
(ADR-457). Experimental. PIN stays null. Leave Сейчас. Not Visual
v1 sign-off.

## Нумерация

После ADR-483 следующий свободный номер — **484**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-240 painted Search as ActionCards on UUID locs. The board
wants the same stacked DataRow grammar as Space members. Inbox
stays an Orb surface, not a primary tab.

## Decision

1. **Host-lock** Search with N `search.item.{i}` Buttons on
   `stacked_row_rect`.
2. **Live.** One Button per `search_rows()`. Tap opens that
   entity. Secondary is the sourced kind label.
3. **Empty.** Store connected and no objects: Status «Нет
   объектов». Store down: Status «Нет связи».
4. **Not this slice.** Do not change `search-public.sui`. Do not
   convert leftover keyboard / gallery / lock formulas.

## Consequences

- Search matches the board row grammar without replacing ADR-240
  membership. Rollback: UUID ActionCards. Still not Visual v1
  sign-off.

## Verification

Host: live Task hits `search.item.0`; empty/offline have no
action. Panther: do not flash; leave Сейчас.
