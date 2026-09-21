# ADR-359: host Qt5 QLineEdit without `setFocus` still types OSK

## Статус

Принято, 2026-09-21. APP-04 Qt5 toolkit on host glibc with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-358 следующий свободный номер — **359**. Не S33.

## Контекст

ADR-352: host Qt 5.15 `QLineEdit` with `setFocus` types OSK `hi!`
on `SAAIOS_SEAT_NO_KEYBOARD=1` after xdg Activated. Packed
PCManFM/Falkon chrome still does not insert (`focusObject` null,
ADR-340/341). Hypothesis: chrome never calls `setFocus` at
startup, so the probe was too strong.

Same host probe, `QT_LINEEDIT_NO_SETFOCUS=1`, no click, no
`wl_keyboard`:

1. Window maps, `xdg activated`, `focus set to`.
2. Qt logs `QWaylandInputContext::setFocusObject` twice.
3. `text-input-v2 enable` then OSK types `QT_LINEEDIT_TEXT=hi!`.

A lone `QLineEdit` auto-focuses when the window becomes active.
`setFocus` in the probe is not the chrome gap. LocationBar and
Filter compete with WebView / FolderView and do not get that
auto-focus.

## Decision

1. **Do not treat probe `setFocus` as the remaining Qt insert
   gap.** A single-widget window types without it. Chrome needs a
   focused `QObject` among competing widgets.
2. **Do not add a fake `wl_keyboard` (ADR-012). Do not flash
   panther this week.** Next displayd flash must carry
   ADR-311+319+328+339+352. AUTH-10 stays a later flash-week
   device pass.

## Consequences

- APP-04 probe path is `setFocus` (ADR-352) and auto-focus
  (this ADR). Falkon URL still needs a tap then loses v2
  (ADR-355/358). PCManFM enable stays and still does not insert
  (ADR-357/341).
- Rollback: drop `QT_LINEEDIT_NO_SETFOCUS` and
  `osk_ime_types_hi_bang_into_qt5_lineedit_without_setfocus`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
osk_ime_types_hi_bang_into_qt5_lineedit_without_setfocus`.
dest-no-lock kept. Do not flash. Leave Сейчас.
