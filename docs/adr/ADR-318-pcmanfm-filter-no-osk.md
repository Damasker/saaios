# ADR-318: PCManFM-Qt Filter is on panther; it does not Activate OSK

## Статус

Принято, 2026-09-21. APP-04 panther native widget. Not typed.
Not Visual v1 sign-off. PIN stays null. Do not flash displayd.

## Нумерация

После ADR-317 следующий свободный номер — **318**. Не S33.

## Контекст

ADR-317: Falkon `file://` `<input>` never mapped `saai-shell-osk`.
APP-04 needs a native Qt/GTK widget. Kirigami was not installed.
PCManFM-Qt is APP-05 (already Done) and has a `QLineEdit` filter.
Panther toplevels are fullscreen; the URL bar of Falkon is not
visible. Displayd is not flashed this week.

## Decision

1. **Install PCManFM-Qt again** with fontconfig → `/saaios/fonts`,
   no ibus input-context, no Wayland EGL (same shm rule as Falkon).
   `QT_IM_MODULE` is empty so Qt does not pick ibus.
2. **Show the Filter `QLineEdit`.** `ShowFilter=true`,
   `PathBarButtons=false`. The filter row is on screen under the
   file list.
3. **Touch inject works.** A protocol-B write to
   `/dev/input/touchscreen` opened `bin/` (`launch`, `pcmanfm-qt`).
   Taps on the Filter row still leave `grep -c saai-shell-osk` at 0.
   IME global is bound (no `foreign OSK stays off`). Activate never
   fired. Same class as ADR-317: the toolkit did not
   `zwp_text_input_v3::enable`.

## Consequences

- Native Qt widgets paint on panther without a displayd flash.
- APP-04 is not closed. A third-party field that actually enables
  text-input-v3 on a touch-only seat is still missing. Host OSK
  tests used a dedicated client, not Qt.
- Rollback: `appd` stop/remove `org.saaios.demo.pcmanfm`.

## Verification

Panther screencap of PCManFM with Filter row. `bin/` after inject.
`saai-shell-osk` count 0. dest-no-lock kept. Stop Дом · Сейчас.
Leave Сейчас.
