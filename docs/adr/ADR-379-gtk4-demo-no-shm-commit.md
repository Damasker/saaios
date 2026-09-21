# ADR-379: packed gtk4-demo search_entry OSK does not commit shm

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-378 следующий свободный номер — **379**. Не S33.

## Контекст

ADR-378: OSK after `--run=search_entry` sends v3 `done` and GTK
`SetSurroundingText` grows; shm stays `233e0ee2…`. Silent same-
pixel commit vs no Wayland buffer commit was unproven.

Same search_entry OSK, keyboard-less seat, no click, 4 s quiet
after OSK:

1. Surrounding still grows (ADR-378).
2. Zero `commit on surface` lines after OSK.
3. Main shm stays `233e0ee2…`.

GTK applied the commit into the IM widget and did not attach a
new shm. Packed `gtk414-entry` still types (it does commit).
Remaining is gtk4-demo not queueing a surface redraw, not the
compositor hashing identical pixels. Do not more `--run=entry`
center clicks.

## Decision

1. **Do not treat search_entry as a same-pixel shm commit.** There
   was no commit. Do not claim typed. Do not add a fake
   `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378. AUTH-10 stays a
   later flash-week device pass.

## Consequences

- APP-04 gtk4-demo remaining is a mapped-widget redraw after IM
  apply. Rollback: drop the post-OSK commit-count assert on
  `packed_gtk4_demo_search_entry_osk_does_not_change_shm`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_search_entry_osk_does_not_change_shm`. dest-no-lock
kept. Do not flash. Leave Сейчас.
