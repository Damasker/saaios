# ADR-399: gtk4-demo combobox has no xdg_popup without a click

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-398 следующий свободный номер — **399**. Не S33.

## Контекст

ADR-394: compositor configures `xdg_popup`. gtk4-demo `--list` has
`combobox`. A dropdown is the remaining Wayland-popup candidate
after entry_completion typed in-window (ADR-397).

`--run=combobox` on a keyboard-less seat, no click, 2 s quiet after
map: toplevel Activated, no `xdg popup`. Do not more Y. Do not
`--run=entry`. Do not more Falkon clicks.

## Decision

1. **Do not treat combobox auto-map as an xdg_popup proof.** The
   dropdown waits for a tap. ADR-394 configure stays unexercised.
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+390+394.
   AUTH-10 stays a later flash-week device pass.

## Consequences

- Opening the combobox still needs a click, not this slice.
- Falkon URL remaining is still chrome (ADR-398), not a missing
  popup configure.
- Rollback: drop the combobox no-click asserts.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_combobox_without_click_has_no_popup`. dest-no-lock
kept. Do not flash. Leave Сейчас.
