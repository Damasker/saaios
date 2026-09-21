# ADR-360: host Qt5 competing pane — tap types OSK without `wl_keyboard`

## Статус

Принято, 2026-09-21. APP-04 Qt5 toolkit on host glibc with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-359 следующий свободный номер — **360**. Не S33.

## Контекст

ADR-359: a lone `QLineEdit` auto-focuses after xdg Activated and
types OSK without `setFocus`. Chrome LocationBar/Filter compete
with WebView/FolderView.

Host probe `QT_LINEEDIT_COMPETE=1`: frameless 320×200 window,
`QLineEdit` fixed 40 px on top, non-IM `QWidget` pane with
`StrongFocus` below (the pane `setFocus` at startup).
`SAAIOS_SEAT_NO_KEYBOARD=1`, one click `160 20`, not a Falkon
Y sweep:

1. Pre-click: `xdg activated`, `focus set to`, no v2 enable.
   The pane holds focus. Qt `setFocusObject` runs; IM stays off.
2. First click with CSD still on the title bar: no enable.
   Frameless: `injected click 160 20` then `text-input-v2 enable`.
3. OSK types `QT_LINEEDIT_TEXT=hi!`.

Pointer tap can focus a `QLineEdit` among competing widgets
without `wl_keyboard`. Falkon URL still enables then drops
(ADR-355/358) and PCManFM enable stays without insert
(ADR-357/341): those chrome widgets steal focus back. That is
not a compositor tap-to-focus gap.

## Decision

1. **Do not treat missing `wl_keyboard` as the reason a tap
   cannot focus a Qt field.** Competing non-IM pane + tap types.
   Do not claim LocationBar/Filter typed.
2. **Do not add a fake `wl_keyboard` (ADR-012). Do not flash
   panther this week.** Next displayd flash must carry
   ADR-311+319+328+339+352. AUTH-10 stays a later flash-week
   device pass.

## Consequences

- APP-04 remaining chrome insert is focus steal-back (WebEngine /
  FolderView), not Activated and not a lone `QLineEdit`.
- Rollback: drop `QT_LINEEDIT_COMPETE` and
  `osk_ime_types_hi_bang_into_qt5_lineedit_competing_pane`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
osk_ime_types_hi_bang_into_qt5_lineedit_competing_pane`. dest-no-lock
kept. Do not flash. Leave Сейчас.
