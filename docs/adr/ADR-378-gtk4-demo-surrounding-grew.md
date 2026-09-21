# ADR-378: packed gtk4-demo search_entry OSK grows surrounding, not shm

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-377 следующий свободный номер — **378**. Не S33.

## Контекст

ADR-377: search_entry binds one `zwp_text_input_v3`; OSK
`commit_string` targets that enable; shm stays `233e0ee2…`.
Silent GTK drop vs applied-without-paint was unproven.
v3 `done` after IME Commit and `SetSurroundingText` had no
compositor log.

Host `saai-displayd` now prints v3 `done`, client `commit`,
`disable`, and `surrounding` byte length (not the text). Same
search_entry OSK, keyboard-less seat, no click:

1. IME `done` fires (not dropped).
2. `SetSurroundingText` byte length after OSK is greater than
   at enable. GTK applied the commit into the IM widget.
3. Main shm stays `233e0ee2…`. Pixel hash is buffer contents
   (ADR-052), not a buffer id.

The compositor finished the v3 round-trip. GTK reports more
surrounding text. gtk4-demo did not attach new pixels.
Packed `gtk414-entry` still types. Remaining is demo paint,
not IME apply. Do not rustfmt overlay `text_ime.rs`. Next
displayd flash also carries these logs. Do not log surrounding
text content (no Intent text).

## Decision

1. **Do not treat search_entry as an IME apply miss.** Surrounding
   grew. Do not claim typed (shm unchanged). Do not add a fake
   `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377 and these v3
   done/surrounding logs. AUTH-10 stays a later flash-week
   device pass.

## Consequences

- APP-04 gtk4-demo remaining is GSK/shm paint after GTK applied
  the commit. Rollback: drop the done/surrounding printlns and
  the grow assert on
  `packed_gtk4_demo_search_entry_osk_does_not_change_shm`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_search_entry_osk_does_not_change_shm`. dest-no-lock
kept. Do not flash. Leave Сейчас.
