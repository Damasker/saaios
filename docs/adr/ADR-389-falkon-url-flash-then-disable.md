# ADR-389: packed Falkon URL click flashes then disables before OSK

## Статус

Принято, 2026-09-21. APP-04 Falkon chrome on host qemu with a
panther-like seat and host frame clock (ADR-388). Not a panther
typed LocationBar. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-388 следующий свободный номер — **389**. Не S33.

## Контекст

ADR-385: OSK on first v2 enable grew surrounding `0 → 3` without a
toplevel shm commit. ADR-388 acks `wl_surface.frame` at ~60 Hz.
gtk4-demo then painted. Same clock on packed Falkon URL click
`640 20`:

1. v2 enable, then toplevel shm `6cd11128… → f0e21a69…`.
2. v2 disable. Toplevel returns to `6cd11128…`.
3. OSK on that first enable still misses v2 (`commit_string` only
   on v3, dropped no-active). Surrounding stays empty.

Qt painted a focus flash, then stole IM off the URL field before
any OSK. That is ADR-358 on a refresh clock, not gtk4-demo
apply-without-paint. Packed Qt 6 QLineEdit still types. Do not more
Y. Do not more URL click sequences as the next default.

## Decision

1. **Do not treat the `f0e21a69…` flash as a typed LocationBar.**
   Disable still wins. Do not add a fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+388 idle
   frame acks. AUTH-10 stays a later flash-week device pass.

## Consequences

- APP-04 Falkon remaining is durable URL `QLineEdit` IM, not a
  missing frame callback. Rollback: restore the ADR-385
  surrounding-grew asserts if the clock is dropped.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame
packed_falkon_url_osk_immediately_after_enable_without_seat_keyboard`.
dest-no-lock kept. Do not flash. Leave Сейчас.
