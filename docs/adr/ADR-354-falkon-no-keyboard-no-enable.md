# ADR-354: packed Falkon without `wl_keyboard` does not enable v2

## Статус

Принято, 2026-09-21. APP-04 Qt6 chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-353 следующий свободный номер — **354**. Не S33.

## Контекст

ADR-352: xdg Activated lets a focused `QLineEdit` probe type OSK
without `wl_keyboard`. ADR-353: packed PCManFM chrome enables v2
from that same Activated, still not a typed PathEdit. Packed Falkon
URL click `640 20` enables v2 on a keyboard seat (ADR-333); OSK is
silent `focusObject() == null` (ADR-340).

Same Falkon package, `hideTabsWithOneTab`, `about:blank`,
`SAAIOS_SEAT_NO_KEYBOARD=1`, **no click**:

1. Window maps 1280×800, shm `6cd11128…`, `text-input-v2 get`.
2. `xdg activated` and `focus set to`. No `keyboard focus set`.
3. No `text-input-v2 enable` in 700 ms after Activated.

LocationBar is not auto-focused. Activated is not a URL field.
Do not start a Y sweep. Do not click `640 20` as the next default.

## Decision

1. **Do not claim Falkon URL typed without a click and without
   `wl_keyboard`.** Bind ≠ enable. PCManFM enable (ADR-353) does
   not transfer to LocationBar.
2. **Do not add a fake `wl_keyboard` (ADR-012). Do not flash
   panther this week.** Next displayd flash must carry
   ADR-311+319+328+339+352. AUTH-10 stays a later flash-week
   device pass.

## Consequences

- APP-04 Falkon chrome on a panther-class seat still needs a
  focused LocationBar `QObject`. Probe QLineEdit types (ADR-352).
- Rollback: drop
  `packed_falkon_without_seat_keyboard_does_not_enable_v2`.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
