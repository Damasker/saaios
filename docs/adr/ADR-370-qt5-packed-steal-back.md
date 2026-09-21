# ADR-370: packed Qt 5.15.10 steal-back after tap does not type OSK

## Статус

Принято, 2026-09-21. APP-04 Qt5 toolkit (PCManFM package) on host
qemu with a panther-like seat. Not a panther typed field. Not
Visual v1 sign-off. PIN stays null. Do not flash displayd this
week.

## Нумерация

После ADR-369 следующий свободный номер — **370**. Не S33.

## Контекст

ADR-361: host Qt 5.15 competing pane + 80 ms steal-back disables
v2. ADR-367: packed Qt 6.6.3 does the same. ADR-369: packed Qt
5.15.10 competing pane without steal types. PCManFM chrome is
this toolkit; FolderView keeps IM after PathEdit/Filter taps
(ADR-363–365).

Same probe, `QT_LINEEDIT_COMPETE=1` and `QT_LINEEDIT_STEAL_MS=80`,
keyboard-less seat, one click `160 20`. Not a Y sweep:

1. Pre-click: pane holds focus, no v2 enable.
2. Click enables v2.
3. 80 ms later the pane `setFocus`; `text-input-v2 disable`.
4. OSK after 200 ms quiet does not set `QT_LINEEDIT_TEXT=hi!`.

Packed Qt 5.15.10 reproduces steal-back. Remaining PathEdit is
FolderView keeping IM (durable enable, ADR-357/363), not this
80 ms disable — unless a tap never moved focus off FolderView.

## Decision

1. **Do not treat packed Qt5 as immune to 80 ms steal-back.** Same
   class as host Qt5 ADR-361. Do not claim PathEdit typed. Do not
   add a fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 PCManFM remaining is FolderView IM ownership, not the
  packed QLineEdit tap path.
- Rollback: drop
  `osk_ime_does_not_type_into_packed_qt5_lineedit_after_steal_back`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
osk_ime_does_not_type_into_packed_qt5_lineedit_after_steal_back`.
dest-no-lock kept. Do not flash. Leave Сейчас.
