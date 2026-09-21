# ADR-373: packed gtk4-demo `--run=entry` is not an example name

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-372 следующий свободный номер — **373**. Не S33.

## Контекст

ADR-344/356: `gtk4-demo --run=entry` binds v3 after center click
and does not enable. Packed Entry types, including with a
competing pane (ADR-372). gtk4-demo `main.c` matches `--run`
against `DemoData.name`. `--list` prints those names.

Host qemu, `SAAIOS_SEAT_NO_KEYBOARD=1`, `gtk4-demo --list` (no
click, not a Y sweep):

1. Names include `search_entry` and `password_entry`.
2. No line is `entry`.

`--run=entry` never selects a demo function. The mapped window is
the demo browser (`create_window`), which has a header
`GtkSearchEntry` used to filter the list. v3 **get** without
**enable** is that browser, not a focused Entry demo. Do not
claim gtk4-demo Entry typed. Do not more center clicks on
`--run=entry`.

## Decision

1. **Do not treat ADR-344/356 as a typed-Entry failure.** The
   `--run` name was wrong. A later slice may use `--run=search_entry`
   or `--run=password_entry`. Do not add a fake `wl_keyboard`
   (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 gtk4-demo remaining is a real `--run=` name, not another
  click on the browser.
- Rollback: drop
  `packed_gtk4_demo_run_entry_is_not_an_example_name`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_run_entry_is_not_an_example_name`. dest-no-lock
kept. Do not flash. Leave Сейчас.
