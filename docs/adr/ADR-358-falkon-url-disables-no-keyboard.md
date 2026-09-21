# ADR-358: packed Falkon URL v2 disables without `wl_keyboard`

## Статус

Принято, 2026-09-21. APP-04 Qt6 chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-357 следующий свободный номер — **358**. Не S33.

## Контекст

ADR-355: one URL click `640 20` on `SAAIOS_SEAT_NO_KEYBOARD=1`
enables v2. ADR-337: the same URL enable on a keyboard seat
disables inside 80 ms quiet. ADR-357: packed PCManFM v2 stays
enabled through 2 s on that keyboard-less seat.

Same Falkon package, `hideTabsWithOneTab`, `about:blank`, one
click `640 20` after Activated, then 2 s quiet. Not a second
click. Not OSK. Not a Y sweep:

1. Pre-click: framed, Activated, no enable.
2. Click → `text-input-v2 enable`. No `keyboard focus set`.
3. `text-input-v2 disable` inside those 2 s.

URL disable is Qt chrome, not `wl_keyboard`. PCManFM stay
(ADR-357) does not transfer. LocationBar is still not typed
(`focusObject` null, ADR-340).

## Decision

1. **Falkon URL v2 on a panther-class seat is transient after
   the tap.** OSK must race disable. Do not claim LocationBar
   typed.
2. **Do not add a fake `wl_keyboard` (ADR-012). Do not flash
   panther this week.** Next displayd flash must carry
   ADR-311+319+328+339+352. AUTH-10 stays a later flash-week
   device pass.

## Consequences

- APP-04 Falkon chrome still needs a focused LocationBar
  `QObject` that outlives the 2 s quiet. Probe QLineEdit types
  (ADR-352). PCManFM enable stays (ADR-357) and still does not
  insert (ADR-341).
- Rollback: drop
  `packed_falkon_url_v2_disables_without_seat_keyboard`.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame
packed_falkon_url_v2_disables_without_seat_keyboard`. dest-no-lock
kept. Do not flash. Leave Сейчас.
