# ADR-203: VUI-09 — dock `SpaceRow` as Spaces stacked hits

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. `layout_v2()`
places `SpaceRow` on `stacked_row_rect` (same 430 + n×220 as Inbox).
`a11y = Button` emits `select_space:<loc>`. `a11y = Status` is the
honest empty/offline row and is not actionable. A screen without
`SpaceRow` invents no Spaces hits. `compile()` stays v1. No new
daemon. Open the Spaces tab, do not tap a row, leave Сейчас.

## Нумерация

После ADR-202 следующий свободный номер — **203**. Не S33.

## Контекст

ADR-202 named Inbox `EventRow` hits. Live Пространства cards still
come from `space_row_at` over `stacked_row_rect`. `SpaceRow` is
already a public v2 name. Live actions are `select_space:{id}`.
This slice does not invent a live space UUID; the public sample
uses loc `spaces.item`. Tapping a live Spaces row would switch
context; this slice does not tap rows. Me lists stay procedural.

## Decision

1. **Grammar.** `component SpaceRow` on `screen spaces`. Button
   rows get `select_space:<loc>`. Status rows occupy the stacked
   rect without an action. Ids come from `loc` when present.
2. **Layout.** Same stacked formula as Inbox. Header slot fills
   0…first row so 540,250 misses.
3. **Example.** `docs/os/ui/examples/spaces-public.sui` names one
   Button row at `spaces.item`.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   `space_row_at` + `layout_v1_root()`.

## Consequences

- v2 layout owns the public Spaces row destination. Me/System
  lists stay procedural. Rollback: treat `SpaceRow` as a Fill leaf.
  Next: Experimental→Stable after Visual v1, or operator-approved
  lock/display/cold-boot. Still not Visual v1 sign-off.

## Verification

Host: public Spaces 540,525 hits `select_space:spaces.item`; Status
row misses. Panther: Spaces tab on HEAD, no row tap; leave Сейчас.
