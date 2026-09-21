# ADR-329: OSK hi! reaches packed PCManFM v2; PathEdit still does not paint

## Статус

Принято, 2026-09-21. APP-04 Qt toolkit on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-328 следующий свободный номер — **329**. Не S33.

## Контекст

ADR-328: a focused host Qt 5.15.15 `QLineEdit` receives OSK `hi!`
(IME `commit_string` plus v2 `delete_surrounding`). ADR-327: packed
PCManFM toolbar click re-enables v2; a single `commit_string("hi!")`
does not attach a new toplevel shm.

Host qemu packed `pcmanfm-qt` (musl Qt 5.15.10), IME client first,
`inject-click 400 40`, then the same Keyboard actions as ADR-328:

1. `text-input-v2 enable` after click (same as ADR-327).
2. OSK forwards `commit_string` ×4 and `delete_surrounding` ×1.
3. Qt then `update_state` ×12.
4. Main shm stays `484823fc…`. Cursor surface hashes are not the
   toplevel. Qt stderr is empty.

Host `QLineEdit` with `setFocus` inserts. Packed PathEdit after a
pointer click does not. Do not claim the click focused a QLineEdit
that accepted the event.

## Decision

1. **OSK protocol into packed PCManFM is compositor-true** after the
   PathEdit click: v2 gets both `commit_string` and
   `delete_surrounding`.
2. **Do not claim PathEdit or Filter shows `hi!`.** No new toplevel
   shm. Next packed probe is a musl QLineEdit that prints
   `textChanged`, not more PCManFM shortcut races.
3. **Do not flash panther.**

## Consequences

- Host glibc Qt types; packed musl PCManFM still does not paint IME
  text. The remaining gap is the client field, not the compositor
  forwarder (on host).
- Rollback: drop the OSK half of `pcmanfm_frame`; keep ADR-323–328.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
