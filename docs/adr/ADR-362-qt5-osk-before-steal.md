# ADR-362: host Qt5 OSK in the enable window types before steal-back

## Статус

Принято, 2026-09-21. APP-04 Qt5 toolkit on host glibc with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-361 следующий свободный номер — **362**. Не S33.

## Контекст

ADR-361: 80 ms steal-back after the tap disables v2; OSK after
200 ms quiet does not type. ADR-337/358: Falkon URL disables in
that same quiet. ADR-333 forwarded `commit_string` in the Falkon
enable window; LocationBar still did not paint (`focusObject`
null, ADR-340).

Same competing probe, `QT_LINEEDIT_STEAL_MS=80`, one click
`160 20`, OSK **immediately** on enable (no 200 ms wait):

1. Click → `text-input-v2 enable` + IME Activate.
2. OSK `hi!` in that window.
3. `QT_LINEEDIT_TEXT=hi!`.

A focused `QLineEdit` wins the 80 ms race. Falkon did not: v2
enable there is not a focused LocationBar `QObject`. PCManFM
durable enable (ADR-357) is the same silent-null class, not this
race.

## Decision

1. **Do not treat Falkon URL as an OSK-vs-steal timing gap on a
   real field.** Probe types in the enable window. Chrome enable
   without `focusObject` stays ADR-340/341.
2. **Do not add a fake `wl_keyboard` (ADR-012). Do not flash
   panther this week.** Next displayd flash must carry
   ADR-311+319+328+339+352. AUTH-10 stays a later flash-week
   device pass.

## Consequences

- APP-04 remaining chrome insert is a focused LocationBar/Filter
  `QObject` at commit time, not compositor tap-to-focus and not
  the 80 ms quiet.
- Rollback: drop
  `osk_ime_types_hi_bang_into_qt5_lineedit_before_steal_back`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
osk_ime_types_hi_bang_into_qt5_lineedit_before_steal_back`.
dest-no-lock kept. Do not flash. Leave Сейчас.
