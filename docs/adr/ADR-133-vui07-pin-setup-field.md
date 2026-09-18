# ADR-133: VUI-07 PIN setup — Field

## Статус

Принято, 2026-09-18. Host slice: «Новый PIN-код» previews digits
through `Field` (`FieldKind::Password`, not revealed). Keypad
geometry, save/clear handlers, lock-surface PIN entry, intent input,
and lock restyle are not this slice. Pixel flash with the rest of
VUI-07.

## Нумерация

После ADR-132 следующий свободный номер — **133**. Не S33.

## Контекст

VUI-07 next remaining surface after the Wi-Fi password Field.
«Изменить PIN» still uses `draw_pin_setup`: title at `header.y + 40`
(hidden under the 120px PIXEL_7 status layer) and a hand-rolled `•`
mask of `buffer`. ADR-132 moved Wi-Fi password onto `Field`; PIN
setup is the same secret-preview bug, not the lock surface. Lock
unlock still paints progress dots (`draw_lock_pin_entry`) and must
not start drawing digits. Do not log the PIN. Do not restyle intent
input or the lock fill in the same slice.

Keep setup behavior: digits accumulate; Готово saves at length ≥ 4;
Отмена discards; Убрать PIN clears an existing code. Keypad stays
`pin_keypad_rect`.

## Decision

1. **`pin_setup_field(buffer) -> Field`**. Password, not revealed.
   Label is `Новый PIN-код`. Placeholder
   `Введите новый PIN (минимум 4 цифры)` is distinct from an empty
   value. Preview paint and accessibility both use
   `Field::accessible_value()`.
2. **`draw_pin_setup` takes that Field.** Title and preview sit at
   the same `+140` / `+200` inset as `draw_wifi_password`. Keys
   unchanged. `draw_lock_pin_entry` stays dots, no Field.
3. **No reveal toggle, no lock restyle.** `revealed` stays false.

## Consequences

- Setup masking lives in `Field`, same as the Wi-Fi password.
- Unlock still never paints the entered digits.
- Rollback: restore `draw_pin_setup(&buffer)` with a hand-masked
  string.

## Verification

Host: field is Password and not revealed; accessible value is bullets
not the digits; empty is distinct from the placeholder. Physical
Pixel with the rest of VUI-07: title visible below the clock,
placeholder when empty, keypad still typeable. Do not save a PIN
just to take the screenshot.
