# ADR-317: WebEngine file:// field does not activate foreign OSK

## Статус

Принято, 2026-09-21. APP-04 panther negative. Not Visual v1
sign-off. PIN stays null. Do not flash displayd or shell.

## Нумерация

После ADR-316 следующий свободный номер — **317**. Не S33.

## Контекст

ADR-309 advertises `zwp_input_method_v2` on panther. ADR-273 maps
`saai-shell-osk` only on IME Activate. ADR-316 painted
`file://…/hello.html` with an `<input id="q">`. APP-04 still needed
that field to receive OSK `commit_string`.

## Decision

1. **This field is not a text-input-v3 client.** Launch + autofocus +
   an evdev protocol-B tap at the box (`cat` to
   `/dev/input/touchscreen`, rc=0) never mapped `saai-shell-osk`
   (zero hits in `/run/saai-displayd.log`). Shell did not log
   `foreign OSK stays off`, so the IME global is bound. Activate
   never fired.
2. **Not ibus.** Removing
   `libibusplatforminputcontextplugin.so` left only compose. Still
   no OSK layer. Ozone already stubs the keymap
   (`StubKeyboardLayoutEngine`).
3. **Do not treat the HTML box as APP-04.** APP-04 needs a native
   Qt/GTK widget that calls `zwp_text_input_v3::enable`. Kirigami
   and PCManFM are not installed. Panther xdg_toplevels are
   fullscreen, so Falkon chrome (URL `QLineEdit`) is not on screen.
   Displayd is not flashed this week.

## Consequences

- file:// hello.html stays a painted document, not a typed field.
- Rollback: none. No compositor change.
- Next APP-04 on panther is a native widget or a WebEngine path
  that actually enables text-input-v3, not another Mesa/font
  tweak.

## Verification

Panther screencap of hello.html with no bottom Keyboard.
`grep -c saai-shell-osk /run/saai-displayd.log` = 0.
dest-no-lock kept. Stop Дом · Сейчас. Leave Сейчас.
