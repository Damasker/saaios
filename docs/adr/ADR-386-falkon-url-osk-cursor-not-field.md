# ADR-386: packed Falkon URL OSK extra shm is cursor, not LocationBar

## Статус

Принято, 2026-09-21. APP-04 Falkon chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-385 следующий свободный номер — **386**. Не S33.

## Контекст

ADR-385: URL OSK grows v2 surrounding; activated toplevel shm
stays `6cd11128…`. A LocationBar subsurface commit was
unproven because the test only hashed the Activated surface.

Same keyboard-less seat, one click `640 20`, OSK on first v2
enable:

1. Surrounding `0 → 3` (`hi!`). Cursor rectangle moves
   (`10x15+22+47` then `+165+14` … `+173+14`).
2. Zero commits on Activated `wl_surface@16`.
3. Extra commits after OSK are cursor `wl_surface@28`
   hashes `849af246…` / `8c6de10e…` (known cursor, not a
   field). Not a LocationBar subsurface.

Qt applied the commit. The mapped window did not attach new
pixels. Packed musl Qt 6 QLineEdit still types. Do not claim
typed LocationBar. Do not more Y.

## Decision

1. **Do not treat extra shm after OSK as LocationBar paint.**
   Those buffers are the pointer cursor. Do not add a fake
   `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385. AUTH-10
   stays a later flash-week device pass.

## Consequences

- APP-04 Falkon URL remaining is a toplevel/LocationBar redraw
  after Qt applied `hi!`. Rollback: drop the extra-surface /
  cursor-hash asserts on
  `packed_falkon_url_osk_immediately_after_enable_without_seat_keyboard`.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame
packed_falkon_url_osk_immediately_after_enable_without_seat_keyboard`.
dest-no-lock kept. Do not flash. Leave Сейчас.
