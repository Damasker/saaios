# ADR-376: packed gtk4-demo search_entry OSK forwards v3 `commit_string`

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-375 следующий свободный номер — **376**. Не S33.

## Контекст

ADR-375: OSK after `--run=search_entry` enable leaves shm
`233e0ee2…`. v3 `commit_string` had no compositor log (v2 did).
Silent drop vs GTK ignoring the event was unproven.

Host `saai-displayd` now prints `text-input-v3 commit_string` when
the IME text is forwarded, or `dropped no-active` / `no-focus` /
`no-match` when it is not. Same search_entry OSK, keyboard-less
seat, no click:

1. `text-input-v3 commit_string` (forwarded, not dropped).
2. Main shm stays `233e0ee2…`.

The compositor sent `zwp_text_input_v3::commit_string` to the
demo client. GTK did not attach a new shm. Packed `gtk414-entry`
still types. Remaining is GTK applying that commit in the demo
window, not a missing IME forward. Do not rustfmt overlay
`text_ime.rs`. Next displayd flash also carries this log.

## Decision

1. **Do not treat search_entry as an IME drop.** Forward succeeded.
   Do not claim typed. Do not add a fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352 and this v3 commit log. AUTH-10
   stays a later flash-week device pass.

## Consequences

- APP-04 gtk4-demo remaining is GTK insert/paint after a real v3
  commit, not compositor drop.
- Rollback: drop the v3 `commit_string` println and the
  forwarded assert on
  `packed_gtk4_demo_search_entry_osk_does_not_change_shm`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_search_entry_osk_does_not_change_shm`. dest-no-lock
kept. Do not flash. Leave Сейчас.
