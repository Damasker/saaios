# ADR-361: host Qt5 steal-back after tap does not type OSK

## Статус

Принято, 2026-09-21. APP-04 Qt5 toolkit on host glibc with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-360 следующий свободный номер — **361**. Не S33.

## Контекст

ADR-360: competing non-IM pane + click `160 20` types OSK `hi!`
without `wl_keyboard`. Falkon URL enables then disables within
2 s (ADR-358). Hypothesis: WebEngine/FolderView steals focus
back after the tap.

Same frameless competing probe, `QT_LINEEDIT_STEAL_MS=80`: when
the `QLineEdit` gains focus, a timer returns `setFocus` to the
pane. Keyboard-less seat, one click, OSK only after 200 ms quiet:

1. Pre-click: pane holds focus, no v2 enable.
2. Click `160 20` → `text-input-v2 enable`.
3. Within 200 ms: `text-input-v2 disable`.
4. OSK does not print `QT_LINEEDIT_TEXT=hi!`.

80 ms steal-back is the Falkon URL class. Tap-to-focus still
works (ADR-360). Insert is lost when a non-IM widget takes focus
again. PCManFM durable enable (ADR-357) is not this class.

## Decision

1. **Do not claim chrome typed when a pane steals focus after
   the tap.** Compositor tap-to-focus is not enough. Do not add
   a fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 Falkon remaining insert is steal-back (and `focusObject`
  null in the enable window, ADR-340). Probe without steal still
  types (ADR-360).
- Rollback: drop `QT_LINEEDIT_STEAL_MS` and
  `osk_ime_does_not_type_into_qt5_lineedit_after_steal_back`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
osk_ime_does_not_type_into_qt5_lineedit_after_steal_back`.
dest-no-lock kept. Do not flash. Leave Сейчас.
