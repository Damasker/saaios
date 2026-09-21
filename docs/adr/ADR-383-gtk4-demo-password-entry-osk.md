# ADR-383: packed gtk4-demo password_entry OSK does not commit shm

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-382 следующий свободный номер — **383**. Не S33.

## Контекст

ADR-382: `--run=password_entry` enables v3 without a tap.
Password bullets would change shm if gtk4-demo redraws. Same
keyboard-less seat, OSK `hi!`, no click:

1. Surrounding grows `0 → 9` (3 UTF-8 bullets, not 3 ASCII).
2. Cursor rectangle x advances (`+41` → `+80`).
3. Zero toplevel `commit on surface`. Main shm stays
   `1eddcfe1…` on `wl_surface@19`.

Same class as search_entry (ADR-379): mapped IM apply, caret
moves, gtk4-demo does not attach new pixels. Packed
`gtk414-entry` still types. Do not claim typed. Do not more
`--run=entry` center clicks.

## Decision

1. **Do not treat password_entry as a gtk4-demo paint success.**
   Surrounding and cursor move. Toplevel shm does not. Do not
   add a fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381. AUTH-10 stays
   a later flash-week device pass.

## Consequences

- APP-04 gtk4-demo remaining is a surface redraw after mapped
  IM apply (search_entry and password_entry). Rollback: drop
  `packed_gtk4_demo_password_entry_osk_does_not_change_shm`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_password_entry_osk_does_not_change_shm`.
dest-no-lock kept. Do not flash. Leave Сейчас.
