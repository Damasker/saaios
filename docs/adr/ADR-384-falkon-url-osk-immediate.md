# ADR-384: packed Falkon URL OSK immediately after enable still no shm

## Статус

Принято, 2026-09-21. APP-04 Falkon chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-383 следующий свободный номер — **384**. Не S33.

## Контекст

ADR-355: URL click `640 20` on a keyboard-less seat enables v2.
ADR-358: that enable disables within 2 s. ADR-362/368: OSK in
the enable window types `hi!` on a real QLineEdit. Immediate
OSK into LocationBar on that seat was unproven.

Same keyboard-less seat, IME bound first, one `inject-click
640 20` (not a Y sweep), OSK `hi!` immediately after enable:

1. `text-input-v2 enable`, then `text-input-v2 commit_string`
   (forwarded; v3 `no-match` because Falkon is v2).
2. Cursor surfaces `@28` (`849af246…` / `8c6de10e…`) are not
   the toplevel.
3. Zero toplevel `commit on surface` after OSK. Main shm stays
   `6cd11128…` on `wl_surface@16`. Then `update_state` /
   `disable`.

OSK-before-steal on a focused QLineEdit (ADR-362) is not
LocationBar. Packed musl Qt 6 QLineEdit still types. Do not
claim typed. Do not more Y.

## Decision

1. **Do not treat immediate OSK as a typed Falkon URL.**
   `commit_string` reaches v2. Toplevel shm does not change.
   Do not add a fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381. AUTH-10 stays
   a later flash-week device pass.

## Consequences

- APP-04 Falkon URL remaining is a focused LocationBar
  `QLineEdit` (silent `focusObject` null, ADR-340), not missing
  the enable window. Rollback: drop
  `packed_falkon_url_osk_immediately_after_enable_without_seat_keyboard`.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame
packed_falkon_url_osk_immediately_after_enable_without_seat_keyboard`.
dest-no-lock kept. Do not flash. Leave Сейчас.
