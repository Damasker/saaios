# ADR-375: packed gtk4-demo `--run=search_entry` OSK does not change shm

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-374 следующий свободный номер — **375**. Не S33.

## Контекст

ADR-374: `--run=search_entry` enables v3 without a tap. Packed
`gtk414-entry` types OSK `hi!` (ADR-346). Same keyboard-less seat,
IME bound first, OSK `hi!` immediately after enable, no click:

1. Activated surface `wl_surface@19`, frame `233e0ee2…`, then
   `text-input-v3 enable`.
2. OSK `commit_string` / `delete_surrounding`.
3. Main shm stays `233e0ee2…`. No second toplevel frame.

Protocol enable is not insert. Same class as PCManFM/Falkon
chrome (ADR-335/363): commit reaches v3, paint does not. Do not
claim the demo Entry typed. Do not more `--run=entry` clicks.

## Decision

1. **Do not treat v3 enable on search_entry as a typed field.**
   OSK does not attach a new shm. Packed Entry still types. Do
   not add a fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 gtk4-demo remaining is insert into that demo Entry, not
  enable. Rollback: drop
  `packed_gtk4_demo_search_entry_osk_does_not_change_shm`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_search_entry_osk_does_not_change_shm`. dest-no-lock
kept. Do not flash. Leave Сейчас.
