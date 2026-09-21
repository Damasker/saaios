# ADR-369: packed Qt 5.15.10 competing pane tap types OSK without `wl_keyboard`

## Статус

Принято, 2026-09-21. APP-04 Qt5 toolkit (PCManFM package) on host
qemu with a panther-like seat. Not a panther typed field. Not
Visual v1 sign-off. PIN stays null. Do not flash displayd this
week.

## Нумерация

После ADR-368 следующий свободный номер — **369**. Не S33.

## Контекст

ADR-360: host glibc Qt 5.15 competing pane + click `160 20` types
OSK. ADR-366: packed Qt 6.6.3 does the same. PCManFM chrome is
packed musl Qt 5.15.10, not those two. Same
`tests/qt5_lineedit.cpp` with `QT_LINEEDIT_COMPETE=1`, linked
against the PCManFM package, `SAAIOS_SEAT_NO_KEYBOARD=1`:

1. Pre-click: `xdg activated`, `focus set to`, no v2 enable.
2. `injected click 160 20` then `text-input-v2 enable`.
3. OSK types `QT_LINEEDIT_TEXT=hi!`.

Qt 5.15.10 under qemu takes IM from a tap among competing
widgets. PCManFM Filter/PathEdit remaining is FolderView IM
(ADR-363–365), not “packed Qt5 cannot type from a click.” Do not
sweep Y. Do not claim PathEdit typed.

## Decision

1. **Do not treat packed Qt 5.15.10 as the reason PathEdit/Filter
   stay untyped.** Competing pane + tap types on the PCManFM
   toolkit. Remaining chrome is FolderViewListView keeping IM
   (ADR-363–365). Do not add a fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 PCManFM remaining is FolderView swallowing IM, not the
  packed Qt5 QLineEdit tap path.
- Rollback: drop
  `osk_ime_types_hi_bang_into_packed_qt5_lineedit_competing_pane`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
osk_ime_types_hi_bang_into_packed_qt5_lineedit_competing_pane`.
dest-no-lock kept. Do not flash. Leave Сейчас.
