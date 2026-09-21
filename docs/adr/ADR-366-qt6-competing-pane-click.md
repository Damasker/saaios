# ADR-366: packed Qt 6.6.3 competing pane tap types OSK without `wl_keyboard`

## Статус

Принято, 2026-09-21. APP-04 Qt6 toolkit (Falkon package) on host
qemu with a panther-like seat. Not a panther typed field. Not
Visual v1 sign-off. PIN stays null. Do not flash displayd this
week.

## Нумерация

После ADR-365 следующий свободный номер — **366**. Не S33.

## Контекст

ADR-360: host glibc Qt 5.15 competing non-IM pane + click `160 20`
types OSK. Falkon chrome is packed musl Qt 6.6.3, not that host
Qt5. Same `tests/qt5_lineedit.cpp` with `QT_LINEEDIT_COMPETE=1`,
linked against the Falkon package, `SAAIOS_SEAT_NO_KEYBOARD=1`:

1. Pre-click: `xdg activated`, `focus set to`, no v2 enable.
   The pane holds focus.
2. `injected click 160 20` then `text-input-v2 enable`.
3. OSK types `QT_LINEEDIT_TEXT=hi!`.

Qt 6.6.3 under qemu takes IM from a tap among competing widgets.
Falkon LocationBar remaining is not "packed Qt6 cannot type from
a click." Do not more Falkon Y clicks. Do not claim LocationBar
typed.

## Decision

1. **Do not treat Qt 6.6.3 as the reason Falkon URL stays untyped.**
   Competing pane + tap types on the Falkon toolkit. Remaining
   chrome is LocationBar/WebEngine `focusObject` null (ADR-340)
   and disable-within-2s (ADR-358). Do not add a fake
   `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 Falkon remaining is chrome focus steal / null
  `focusObject`, not the packed Qt6 QLineEdit class.
- Rollback: drop
  `osk_ime_types_hi_bang_into_packed_qt6_lineedit_competing_pane`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
osk_ime_types_hi_bang_into_packed_qt6_lineedit_competing_pane`.
dest-no-lock kept. Do not flash. Leave Сейчас.
