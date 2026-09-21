# ADR-377: packed gtk4-demo search_entry has one v3 object

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-376 следующий свободный номер — **377**. Не S33.

## Контекст

ADR-376: OSK after `--run=search_entry` forwards
`zwp_text_input_v3::commit_string`; shm stays `233e0ee2…`.
Hypothesis: gtk4-demo keeps two v3 objects (hidden browser
`GtkSearchEntry` + visible demo `GtkEntry`) and the compositor
commits into the hidden one.

Host `saai-displayd` now prints object ids on v3 `get`, `enable`,
and forwarded `commit_string`. Same search_entry OSK,
keyboard-less seat, no click:

1. One `text-input-v3 get` (`zwp_text_input_v3@26`).
2. `enable` and four `commit_string` lines use that same id.
3. Main shm stays `233e0ee2…`.

GTK binds one `zwp_text_input_v3` per seat. The remaining gap is
GTK applying that commit in the demo window, not a second protocol
object. Packed `gtk414-entry` still types. Do not rustfmt overlay
`text_ime.rs`. Next displayd flash also carries these ids.

## Decision

1. **Do not treat search_entry as two v3 objects.** Commit target
   equals enable. Do not claim typed. Do not add a fake
   `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376 and these v3 object-id logs.
   AUTH-10 stays a later flash-week device pass.

## Consequences

- APP-04 gtk4-demo remaining is GTK insert/paint after a real v3
  commit to the one seat object. Rollback: drop object ids from
  the v3 printlns and the get-count / id-match asserts on
  `packed_gtk4_demo_search_entry_osk_does_not_change_shm`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_search_entry_osk_does_not_change_shm`. dest-no-lock
kept. Do not flash. Leave Сейчас.
