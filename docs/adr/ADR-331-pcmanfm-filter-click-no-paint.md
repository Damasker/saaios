# ADR-331: Filter-band click re-enables v2; packed PCManFM still does not paint

## Статус

Принято, 2026-09-21. APP-04 Qt toolkit on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-330 следующий свободный номер — **331**. Не S33.

## Контекст

ADR-330: packed musl Qt 5.15.10 `QLineEdit` with `setFocus` receives
OSK `hi!`. ADR-327/329: toolbar click `400 40` (PathEdit band)
disable+enable v2, OSK `commit_string`+`delete_surrounding`, main shm
stays `484823fc…`.

Permanent Filter is at the bottom of TabPage (`ShowFilter=true`).
Host qemu `inject-click 400 760` (1280×800 window):

1. Cursor surface maps (`64156441…` / `92cea996…`) — same as any
   pointer inject, not a Filter popup.
2. Qt `disable` then `enable`.
3. OSK hi! sequence reaches v2 (`commit_string` ×4,
   `delete_surrounding` ×1, then `update_state` ×12).
4. Main shm stays `484823fc…`. A typed Filter would change the
   folder listing.

`v2 enable` after a synthetic click is not `QLineEdit::setFocus`.
Packed QLineEdit types; packed PCManFM after pointer inject does not.

## Decision

1. **Filter-band click is compositor-true** the same way as PathEdit
   click. Do not keep sweeping y coordinates.
2. **Do not claim Filter shows `hi!`.** Next packed PCManFM step is a
   real Qt focus on that QLineEdit (not more inject-click). Panther
   tap remains the hardware focus path and waits for the next
   displayd flash (ADR-311+319+328 delete).
3. **Do not flash panther.**

## Consequences

- APP-04 host Qt insert is proven only for a probe QLineEdit, not
  for PCManFM chrome.
- Rollback: restore click `400 40` (ADR-329).

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
