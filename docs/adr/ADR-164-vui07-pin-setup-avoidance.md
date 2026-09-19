# ADR-164: VUI-07 PIN setup — Field on the docked dialer

## Статус

Принято, 2026-09-19. PIN setup parks the Password Field on a
bottom-docked `MIN_TOUCH_TARGET` dialer, same avoidance as Intent
(ADR-161). Field is a stop in the tree (ADR-162). Do not type
digits. Do not tap Готово. Leave Отмена. Lock unlock keypad is
unchanged.

## Нумерация

После ADR-163 следующий свободный номер — **164**. Не S33.

## Контекст

Intent/Wi-Fi already dock QWERTY and sit the Field on the keys.
PIN setup still put the Field in stacked slot 0 (`y ≈ 430`) and
filled the rest of the panel with huge digit cells. Remaining
VUI-07 checkbox: keyboard avoidance on remaining frame variants.
Lock unlock keeps the occupancy Field in the header slot
(`lock_pin_field_rect`) — that frame is not compose.

## Decision

1. **`pin_setup_view`** is chrome + Field + dialer. Dialer height is
   five `MIN_TOUCH_TARGET` rows (4 digit + Отмена/Готово), docked at
   the bottom. Field is the padded `pin-field` leaf, `focus_order = 0`.
2. **`pin_setup_field_rect` / `pin_setup_action_at` / keys** read that
   tree. Unlock still layouts `pin_keypad_node` under
   `INTENT_HEADER_HEIGHT`.
3. **Digits keep `focus_order` 1…N.** Last stop is Готово, or
   Убрать PIN when a PIN already exists.

## Consequences

- Visual matches Intent: header, empty canvas, Field on keys.
- Rollback: `stacked_row_rect(0)` + Fill keypad under it.

## Verification

Host: Field bottom meets dialer; keys ≥ min-touch; Отмена still
hits. Panther: Система → PIN-код, screenshot, Отмена. Do not type.
Do not tap Готово.
