# ADR-326: Ctrl+I into packed PCManFM disables text-input-v2

## Статус

Принято, 2026-09-21. APP-04 Qt Filter on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-325 следующий свободный номер — **326**. Не S33.

## Контекст

ADR-324: compositor forwards IME `commit_string` after PCManFM `enable`,
but Qt does not insert without a focused `QLineEdit`. ADR-325: a
focused GTK 4.18 Entry does insert `hi!`.

Host attempts to focus packed PCManFM-Qt Filter (`ShowFilter=true`):

1. `inject-click 300 70` (toolbar/path) and `200 755` (status/filter)
   created a new `wl_surface` popup (`sha256=64156441…`), not a
   Filter `enable`.
2. `inject-ctrl-i` (Show/Focus Filter Bar, `KEY_LEFTCTRL`+`KEY_I`)
   reached the focused toplevel. Qt then sent `update_state` twice
   and **`text-input-v2 disable`**. No second `enable`. IME
   `commit_string` after that is dropped (`active` is None).

## Decision

1. **Host displayd stdin may inject `ctrl-i` and pointer clicks**
   (`not(panther-hardware)` only). Same debug seat as `inject-key`.
2. **Do not claim Filter was typed.** Ctrl+I currently ends the v2
   session. Pointer click is not a QLineEdit hit.
3. **Do not flash panther.** Next displayd experiment is still
   ADR-311 bounds + ADR-319 v2, plus this stdin inject (host-only).

## Consequences

- APP-04 Qt field on host is still enable + forwarded commit, not a
  painted Filter. Panther tap remains the real focus path.
- Rollback: drop `inject-ctrl-i` / `inject-click`.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
