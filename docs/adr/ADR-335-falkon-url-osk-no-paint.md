# ADR-335: packed Falkon URL OSK does not paint a new shm

## Статус

Принято, 2026-09-21. APP-04 Qt6 LocationBar on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-334 следующий свободный номер — **335**. Не S33.

## Контекст

ADR-334: OSK `h i toggle backspace i !` in the URL enable window
forwards `commit_string` ×4 and `delete_surrounding` ×1. ADR-329/331:
the same sequence reaches packed PCManFM v2; main shm stays
`484823fc…`.

Host qemu, same `640 20` click, hash of the first toplevel `wl_surface`
taken at enable (not the cursor surface) vs 700 ms after OSK: **equal**.
A typed LocationBar would redraw the navigation toolbar.

## Decision

1. **Do not claim the LocationBar shows `hi!`.** OSK is compositor-true
   and still protocol-without-paint, same class as packed PCManFM.
2. **Probe `QLineEdit` (ADR-330) remains the only typed Qt field.**
3. **Do not flash panther.** URL chrome stays off screen (ADR-317).

## Consequences

- APP-04 host Falkon chrome is enable + OSK forward, not a painted
  URL. Next typed-field path is not more Y clicks.
- Rollback: drop the shm equality assert; keep ADR-334 protocol.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
