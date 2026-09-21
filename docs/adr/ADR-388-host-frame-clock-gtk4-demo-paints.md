# ADR-388: host frame clock; gtk4-demo OSK paints

## Статус

Принято, 2026-09-21. APP-04 compositor on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-387 следующий свободный номер — **388**. Не S33.

## Контекст

ADR-375–383: packed gtk4-demo `--run=search_entry` / `password_entry`
applied OSK (`SetSurroundingText` grew) but never committed a new
toplevel shm. ADR-384–386: Falkon URL same class. Host displayd only
called `send_frames_surface_tree` from the panther DRM VBlank path.
Host has no VBlank. gtk4-demo waits on `wl_surface.frame` after IME
before committing. Ack-on-commit spun saai-shell (see `commit()`).

A host ~60 Hz timer acks pending frame callbacks on mapped toplevels
and layers. Not on every `wl_surface.commit`.

## Decision

1. **Host `calloop` 16 ms timer calls `send_host_frames`.** Log
   `host frame clock` once. Do not ack on commit.
2. **Panther stays VBlank-after-present.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385 **and** idle
   frame acks for pending callbacks when IME applied without a new
   buffer. Do not add a fake `wl_keyboard` (ADR-012).
3. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Packed gtk4-demo search_entry OSK surrounding `0 → 3` and a new
  toplevel shm. password_entry surrounding `0 → 9` and a new shm.
  Packed GTK 4.14 Entry still types.
- Packed PCManFM FolderView surrounding stays 0; shm now changes
  (redraw without a text field). Filter/PathEdit remain untyped.
- Packed Falkon URL click still disables v2 before OSK in this
  timing; remaining is not gtk4-demo apply-without-paint.
- Rollback: drop the host timer and restore the no-shm gtk4-demo
  asserts.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_search_entry_osk_commits_shm` (and
`click_then_osk`, `password_entry_osk_commits_shm`). Packed GTK
4.14 Entry without seat keyboard still types. dest-no-lock kept.
Do not flash. Leave Сейчас.
