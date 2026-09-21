# ADR-380: packed gtk4-demo search_entry click then OSK still no toplevel shm

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-379 следующий свободный номер — **380**. Не S33.

## Контекст

ADR-379: OSK after `--run=search_entry` grows surrounding and
does not `commit on surface`. Missing pointer focus on the
visible Entry was unproven.

Same keyboard-less seat, v3 enable, one `inject-click 640 40`
(top band, not `--run=entry` center), then OSK `hi!`:

1. `injected click 640 40`.
2. Cursor `wl_surface@14` commits `849af246…` (not the
   toplevel).
3. Surrounding grows `0 → 3` after OSK (`hi!`).
4. Zero toplevel `commit on surface` after OSK. Main shm stays
   `233e0ee2…` on `wl_surface@19`.

A tap does not make gtk4-demo queue a mapped buffer. Packed
`gtk414-entry` still types. Do not more `--run=entry` center
clicks. Do not more Y.

## Decision

1. **Do not treat a top-band click as gtk4-demo typed.**
   Surrounding is `hi!`. Toplevel shm does not change. Do not
   add a fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378. AUTH-10 stays a
   later flash-week device pass.

## Consequences

- APP-04 gtk4-demo remaining is a mapped redraw after IM apply,
  not missing pointer focus. Rollback: drop
  `packed_gtk4_demo_search_entry_click_then_osk`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_search_entry_click_then_osk`. dest-no-lock
kept. Do not flash. Leave Сейчас.
