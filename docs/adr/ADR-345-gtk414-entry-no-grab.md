# ADR-345: packed GTK 4.14 Entry types without `grab_focus` on a keyboard seat

## Статус

Принято, 2026-09-21. APP-04 GTK 4.14 toolkit on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-344 следующий свободный номер — **345**. Не S33.

## Контекст

ADR-343: `gtk_widget_grab_focus` on a packed musl 4.14.4 Entry types
OSK `hi!`. ADR-344: `gtk4-demo --run=entry` binds v3; a center click
does not enable. The missing link was whether GTK needs
`grab_focus`, or only compositor keyboard focus (host seat has
`wl_keyboard`; panther is touch-only, ADR-012).

Host qemu, same `gtk414-entry`, `GTK4_NO_GRAB=1` (no
`grab_focus`):

1. `inject-click` before a focused surface logs
   `inject-click requested but no surface is focused yet`.
2. displayd then `keyboard focus set` on the toplevel.
3. `text-input-v3 enable` follows **without** a successful click.
4. Same OSK sequence prints `GTK_ENTRY_TEXT=hi!`.

A single Entry as the window child becomes the focus widget when
the seat gives the surface keyboard focus. gtk4-demo chrome still
does not enable (ADR-344): the focused widget there is not that
Entry.

## Decision

1. **APP-04 packed GTK 4.14 types without `grab_focus` on a host
   keyboard seat.** Pointer click is not the enable cause.
2. **Do not claim panther touch focuses an Entry.** No `wl_keyboard`
   there. AUTH-10 / typed chrome wait the next flash-week.
3. **Do not sweep Y.** gtk4-demo stays ADR-344.

## Consequences

- Host GTK 4.14 insert needs a focused widget, which a keyboard seat
  can supply. Qt chrome (PathEdit/LocationBar) still has
  `focusObject() == null` (ADR-340/341).
- Rollback: drop `GTK4_NO_GRAB` and the second gtk4_ime test.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime`. dest-no-lock kept.
Do not flash. Leave Сейчас.
