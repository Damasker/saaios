# ADR-371: packed Qt 5.15.10 OSK in the enable window types before steal-back

## Статус

Принято, 2026-09-21. APP-04 Qt5 toolkit (PCManFM package) on host
qemu with a panther-like seat. Not a panther typed field. Not
Visual v1 sign-off. PIN stays null. Do not flash displayd this
week.

## Нумерация

После ADR-370 следующий свободный номер — **371**. Не S33.

## Контекст

ADR-368: packed Qt 6.6.3 OSK immediately after enable types
`hi!` before 80 ms steal-back. ADR-370: packed Qt 5.15.10 after
200 ms quiet does not type. PCManFM PathEdit/Filter taps never
move IM off FolderView (ADR-363–365).

Same packed probe, `QT_LINEEDIT_STEAL_MS=80`, one click `160 20`,
OSK **immediately** on enable (no 200 ms wait):

1. Click → `text-input-v2 enable` + IME Activate.
2. OSK `hi!` in that window.
3. `QT_LINEEDIT_TEXT=hi!`.

A focused packed Qt 5.15.10 `QLineEdit` wins the 80 ms race.
PathEdit did not: enable there is FolderView, not a focused
PathEdit `QObject`. Do not sweep Y.

## Decision

1. **Do not treat PCManFM PathEdit as an OSK-vs-steal timing gap
   on packed Qt5.** Probe types in the enable window. Chrome IM
   staying on FolderViewListView stays ADR-363–365. Do not add a
   fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 remaining PCManFM insert is a focused PathEdit/Filter
  `QObject` at commit time, not compositor tap-to-focus and not
  the 80 ms quiet on this toolkit.
- Rollback: drop
  `osk_ime_types_hi_bang_into_packed_qt5_lineedit_before_steal_back`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
osk_ime_types_hi_bang_into_packed_qt5_lineedit_before_steal_back`.
dest-no-lock kept. Do not flash. Leave Сейчас.
