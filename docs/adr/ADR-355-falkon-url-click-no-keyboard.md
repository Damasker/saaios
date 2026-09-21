# ADR-355: packed Falkon URL click enables v2 without `wl_keyboard`

## Статус

Принято, 2026-09-21. APP-04 Qt6 chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-354 следующий свободный номер — **355**. Не S33.

## Контекст

ADR-354: packed Falkon on `SAAIOS_SEAT_NO_KEYBOARD=1` binds v2
after xdg Activated and does **not** enable. LocationBar is not
auto-focused. ADR-333: the same URL click `640 20` enables v2 on a
keyboard seat. Probe `QLineEdit` types without `wl_keyboard`
(ADR-352). PCManFM chrome enables from Activated alone (ADR-353).

Same Falkon package, `hideTabsWithOneTab`, `about:blank`,
`SAAIOS_SEAT_NO_KEYBOARD=1`, **one** click `640 20` after
Activated (not a Y sweep, not a second click, not OSK):

1. Pre-click: framed, `xdg activated`, `focus set to`, no enable.
2. `injected click`. No `keyboard focus set`.
3. `text-input-v2 enable`.

Pointer tap is enough. `wl_keyboard` is not required for URL
protocol enable. Insert is still silent `focusObject() == null`
(ADR-340). Do not claim LocationBar typed. Do not more Y clicks.

## Decision

1. **Falkon URL v2 enable on a panther-class seat is a LocationBar
   tap, not a seat keyboard.** Bind+Activated ≠ enable (ADR-354).
   One click `640 20` = enable.
2. **Do not add a fake `wl_keyboard` (ADR-012). Do not flash
   panther this week.** Next displayd flash must carry
   ADR-311+319+328+339+352. AUTH-10 stays a later flash-week
   device pass.

## Consequences

- APP-04 Falkon chrome on a panther-class seat still needs a
  focused LocationBar `QObject` to insert. Protocol enable is not
  paint.
- Rollback: drop
  `packed_falkon_url_click_without_seat_keyboard_enables_v2`.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame
packed_falkon_url_click_without_seat_keyboard_enables_v2`.
dest-no-lock kept. Do not flash. Leave Сейчас.
