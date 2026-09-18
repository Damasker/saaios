# ADR-132: VUI-07 Wi-Fi password — Field

## Статус

Принято, 2026-09-18. «Пароль для «SSID»» previews the PSK through
`Field` (`FieldKind::Password`, not revealed). Flashed panther
`0eeb36d3…`: title below the status layer, empty placeholder
`Введите пароль…` for Wallbox, keyboard unchanged, no PSK on
screen. PIN setup, intent input, and lock restyle are not this
slice.

## Нумерация

После ADR-131 следующий свободный номер — **132**. Не S33.

## Контекст

VUI-07 next remaining surface after the trusted-client list. ADR-129
left the password keyboard on `draw_intent_input`: title at
`header.y + 40` (hidden under the 120px PIXEL_7 status layer, the
same bug ADR-129's list title had) and a hand-rolled `•` mask of
`buffer`. `Field` already owns Password masking and accessibility
(section 6.7 / ADR-103). There is no reveal control. Do not log the
PSK. Do not restyle PIN setup, intent input, or the lock surface in
the same slice.

Keep connect behavior: Cancel closes; Send still calls
`wifi_connect_psk`. Keyboard geometry stays the shared intent tree
(`WifiPasswordState`'s own doc comment).

## Decision

1. **`wifi_password_field(ssid, buffer) -> Field`**. Password, not
   revealed. Label is `Пароль для «{ssid}»`. Placeholder
   `Введите пароль…` is distinct from an empty value. Preview paint
   and accessibility both use `Field::accessible_value()`.
2. **`draw_wifi_password`** paints that Field. Title and preview sit
   at the same `+140` / `+200` inset as `draw_action_row_list`. Keys
   unchanged. Intent input keeps `draw_intent_input`.
3. **No reveal toggle, no PIN, no lock.** `revealed` stays false.

## Consequences

- Masking lives in one place (`Field`), not a second `•` loop next
  to the PIN setup's own loop.
- Rollback: restore `draw_intent_input` with a hand-masked buffer.

## Verification

Host: field is Password and not revealed; accessible value is bullets
not the PSK; empty is distinct from the placeholder; SSID stays in
the label. Panther `0eeb36d3…`: `Пароль для «Wallbox»` below the
clock, placeholder `Введите пароль…`, keyboard typeable, no PSK
painted.
