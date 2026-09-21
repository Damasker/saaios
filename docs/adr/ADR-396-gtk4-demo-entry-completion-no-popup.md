# ADR-396: gtk4-demo entry_completion enables v3 without xdg_popup

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-395 следующий свободный номер — **396**. Не S33.

## Контекст

ADR-394: compositor configures `xdg_popup`. QCompleter did not hit
that path (second toplevel). gtk4-demo `--list` has no `popover` /
`menu`. It has `entry_completion` and `combobox`.

`--run=entry_completion` on a keyboard-less seat, no click:

1. Maps, Activated, v3 enable, surrounding 0, cursor `0x32+41+98`.
2. No `xdg popup` in the enable window.

Completion dropdown is not mapped until type/click. Do not
`--run=entry`. Do not more Y.

## Decision

1. **Do not treat entry_completion auto-enable as an xdg_popup
   proof.** ADR-394 configure stays unexercised by this demo.
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+390+394.
   AUTH-10 stays a later flash-week device pass.

## Consequences

- Packed gtk4-demo search_entry still paints (ADR-388).
- Combobox/Falkon dropdown remaining is still a click or type, not
  this auto-enable.
- Rollback: drop the `--list`/`entry_completion` asserts.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_list_has_entry_completion`. Host `cargo test -p
saai-displayd --test gtk4_ime
packed_gtk4_demo_entry_completion_enables_v3_without_popup`.
dest-no-lock kept. Do not flash. Leave Сейчас.
