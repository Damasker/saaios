# ADR-397: gtk4-demo entry_completion OSK types without xdg_popup

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-396 следующий свободный номер — **397**. Не S33.

## Контекст

ADR-396: `--run=entry_completion` enables v3 without mapping
`xdg_popup`. Hypothesis: OSK `hi!` would open GtkEntryCompletion as
xdg_popup and exercise ADR-394 configure.

Same demo, OSK after enable:

1. v3 surrounding `0 → 3` (`hi!`).
2. Toplevel shm changes (`919b56c6…` → `37b16d33…`).
3. No `xdg popup`.

Completion chrome is in-window / not a Wayland popup for this
demo. Do not `--run=entry`. Do not more Y.

## Decision

1. **Do not treat entry_completion OSK as an xdg_popup proof.**
   ADR-394 configure stays unexercised. The field is typed on host.
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+390+394.
   AUTH-10 stays a later flash-week device pass.

## Consequences

- Packed gtk4-demo search_entry/password_entry/entry_completion OSK
  all paint on host. Combobox dropdown still needs a click.
- Rollback: drop the entry_completion OSK popup-negative asserts.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_entry_completion_osk_types_without_popup`. dest-no-lock
kept. Do not flash. Leave Сейчас.
