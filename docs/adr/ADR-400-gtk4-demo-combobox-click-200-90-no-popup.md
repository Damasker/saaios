# ADR-400: gtk4-demo combobox click 200 90 has no xdg_popup

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-399 следующий свободный номер — **400**. Не S33.

## Контекст

ADR-399: `--run=combobox` maps without `xdg_popup` until a tap.
`do_combobox` packs the first combo under CSD + 10 px margin + a
frame label. One click `200 90` (not a Y sweep):

1. `injected click 200 90`.
2. Cursor shm `849af246…`.
3. Toplevel shm unchanged (`71cca3c1…`).
4. No `xdg popup`.

The tap did not open the dropdown. Do not more combobox Y. Do not
`--run=entry`. Do not more Falkon clicks.

## Decision

1. **Do not treat click 200 90 as a mapped combobox.** ADR-394
   configure stays unexercised. Dropdown hit-target unproven.
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+390+394.
   AUTH-10 stays a later flash-week device pass.

## Consequences

- Opening gtk4-demo combobox still needs a proven tap, not this
  coordinate. Falkon URL remaining is still chrome (ADR-398).
- Rollback: drop the 200 90 no-popup asserts.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_combobox_click_200_90_has_no_popup`. dest-no-lock
kept. Do not flash. Leave Сейчас.
