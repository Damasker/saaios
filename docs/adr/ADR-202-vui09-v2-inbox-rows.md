# ADR-202: VUI-09 — dock `EventRow` as Inbox stacked hits

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. `layout_v2()`
places `EventRow` on `stacked_row_rect` (430 + n×220, height 190
at 2400). `a11y = Button` emits `open_object`. `a11y = Status` is
the honest empty/offline row and is not actionable. A screen
without `EventRow` invents no Inbox hits. `compile()` stays v1.
No new daemon. Open the Inbox tab, do not tap a row, leave Сейчас.

## Нумерация

После ADR-201 следующий свободный номер — **202**. Не S33.

## Контекст

ADR-199/200 named NOW footer and object hits. Live Inbox cards
still come from `inbox_row_at` over `stacked_row_rect`. `EventRow`
is already a public v2 name. Empty and offline rows are not
actionable (`EventRow::empty` / `offline`). Tapping a live Inbox
row would open Object View; this slice does not tap rows.

## Decision

1. **Grammar.** `component EventRow` on `screen inbox`. Button
   rows get `open_object`. Status rows occupy the stacked rect
   without an action. Ids come from `loc` when present.
2. **Layout.** Rows use the live stacked formula. The header slot
   fills 0…first row so 540,250 misses. Duplicate rows stack.
3. **Example.** `docs/os/ui/examples/inbox-public.sui` names one
   Button row at `inbox.item`.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   `inbox_row_at` + `layout_v1_root()`.

## Consequences

- v2 layout owns the public Inbox row destination. Spaces/Me lists
  stay procedural. Rollback: treat `EventRow` as a Fill leaf.
  Next: Experimental→Stable after Visual v1, or operator-approved
  lock/display/cold-boot. Still not Visual v1 sign-off.

## Verification

Host: public Inbox 540,525 hits `open_object`; Status row misses;
NOW 540,525 still none. Panther: Inbox tab on HEAD, no row tap;
leave Сейчас.
