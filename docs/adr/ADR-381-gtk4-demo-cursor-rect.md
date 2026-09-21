# ADR-381: packed gtk4-demo search_entry OSK sends a v3 cursor rectangle

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-380 следующий свободный номер — **381**. Не S33.

## Контекст

ADR-378…380: search_entry OSK applies surrounding (`hi!` after a
tap) and does not commit toplevel shm. An unmapped IM widget
was unproven. v3 `SetCursorRectangle` had no compositor log.

Host `saai-displayd` now prints `text-input-v3 cursor` with size
and offset (not surrounding text). Same `--run=search_entry`
OSK, keyboard-less seat, no click:

1. At least one `text-input-v3 cursor` (GTK reports a caret
   rectangle on the IM widget).
2. Surrounding still grows. Zero toplevel shm commits. Main
   shm stays `233e0ee2…`.

The IM widget is mapped enough to publish a cursor rectangle.
gtk4-demo still does not `wl_surface.commit` new pixels.
Packed `gtk414-entry` still types. Do not rustfmt overlay
`text_ime.rs`. Next displayd flash also carries this log.

## Decision

1. **Do not treat search_entry as an unmapped IM widget.** Cursor
   rectangle is present. Do not claim typed. Do not add a fake
   `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381. AUTH-10 stays
   a later flash-week device pass.

## Consequences

- APP-04 gtk4-demo remaining is a surface redraw after a mapped
  IM apply. Rollback: drop the cursor println and the
  `n_cursor >= 1` assert on `search_entry_osk_case`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_search_entry_osk_does_not_change_shm`. dest-no-lock
kept. Do not flash. Leave Сейчас.
