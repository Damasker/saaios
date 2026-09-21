# ADR-353: packed PCManFM enables v2 without `wl_keyboard`

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-352 следующий свободный номер — **353**. Не S33.

## Контекст

ADR-352: xdg Activated without `wl_keyboard` lets a focused
`QLineEdit` probe type OSK. Packed PCManFM-Qt with `ShowFilter=true`
enables v2 on a keyboard seat (ADR-323). ADR-341: OSK into that
enable is silent `focusObject() == null`, not typed PathEdit.

Same package, `SAAIOS_SEAT_NO_KEYBOARD=1`, no click, no Ctrl+L:

1. Window maps 1280×800, hashed shm, `xdg activated`, `focus set to`.
2. No `keyboard focus set`.
3. `text-input-v2 get` then `text-input-v2 enable`.

Activated is enough for this chrome window to enable v2. That is
not a typed Filter/PathEdit and not a new shm (ADR-329/341). Do not
OSK here.

## Decision

1. **PCManFM v2 enable on a panther-class seat is Activated, not
   `wl_keyboard`.** Do not claim PathEdit/Filter typed.
2. **Do not click. Do not Ctrl+L. Do not add a fake `wl_keyboard`
   (ADR-012).**
3. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later flash-week
   device pass.

## Consequences

- APP-04 Qt chrome can enable v2 without a keyboard once Activated
  is set. Insert still needs a focused `QObject`.
- Rollback: drop `packed_pcmanfm_enables_v2_without_seat_keyboard`.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
