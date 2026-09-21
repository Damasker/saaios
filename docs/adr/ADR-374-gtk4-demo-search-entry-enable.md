# ADR-374: packed gtk4-demo `--run=search_entry` enables v3 without a tap

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-373 следующий свободный номер — **374**. Не S33.

## Контекст

ADR-373: `--run=entry` is not a `--list` name; ADR-344/356 clicked
the demo browser. `--run=search_entry` is a real name. Demo source
(`search_entry.c`): a `GtkWindow` titled "Search Entry",
`gtk_entry_new()` (not `GtkSearchEntry`), no `grab_focus`, Find
button beside it, `resizable=FALSE`. `--run` hides the browser
and shows that transient.

Host qemu, `SAAIOS_SEAT_NO_KEYBOARD=1`, **no click**:

1. `new xdg_toplevel`, hashed shm, `xdg activated`, `focus set to`.
2. `text-input-v3 enable`. No `keyboard focus set`.

A real gtk4-demo Entry window auto-enables like packed
`gtk414-entry` (ADR-346/372). The browser `--run=entry` path did
not. Do not claim the demo field was typed: no `GTK_ENTRY_TEXT`,
no OSK this slice.

## Decision

1. **Do not treat gtk4-demo as unable to enable v3.** The named
   Search Entry demo enables without a tap and without
   `wl_keyboard`. Do not more `--run=entry` clicks. Do not add a
   fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 gtk4-demo remaining is insert/paint into that demo
  Entry, not bind-without-enable on the browser.
- Rollback: drop
  `packed_gtk4_demo_search_entry_enables_v3_without_click`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_search_entry_enables_v3_without_click`.
dest-no-lock kept. Do not flash. Leave Сейчас.
