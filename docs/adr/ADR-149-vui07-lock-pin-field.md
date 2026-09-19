# ADR-149: VUI-07 lock/PIN keys — ADR-029 keyboard, not a second painter

## Статус

Принято, 2026-09-19. PIN unlock and PIN setup keys come from the
same `Node`/`layout()`/`hit_test()` keyboard as Intent and Wi-Fi
password (ADR-029/076). Progress is a Password `Field` of occupancy,
never `pin_code`. Do not save a PIN. Do not type digits. Space
detail is not this slice.

## Нумерация

После ADR-148 следующий свободный номер — **149**. Не S33.

## Контекст

ADR-012/022 closed generic Wayland IME/`libxkbcommon` on panther.
ADR-029's bespoke touch-hit-test keyboard is the only input method
this source is allowed to use. Intent and Wi-Fi password already
share `intent_view()`. PIN setup and lock unlock still invent a
second geometry (`pin_keypad_rect`) and a second key painter
(`fill_rect` + magic 32/36 px labels). VUI-07 does not grow another
рисовалка.

Device currently has no PIN. This slice must not set one.

## Decision

1. **`pin_keypad_node` is the ADR-029 keyboard** with dialer rows
   `123` / `456` / `789` / ` 0⌫` (space is the blank cell, no
   action). Setup adds the existing Отмена/Готово/Убрать PIN row.
   Layout fills the remaining rect under the header/`Field`, the
   same Fill-on-both-axes rule as `intent_view()`.
2. **Hit-test is `hit_test()`**, not index math. Unlock and setup
   share one tree; only the bounds and the control row differ.
3. **`draw_lock_pin_entry` paints `lock_pin_entry_field`** in the
   keyboard header slot, then the shared key paint used by Intent
   and Wi-Fi. No Surface header bar. Revealed stays false. Dummy
   occupancy, never the secret.

## Consequences

- One keyboard implementation in this process.
- PIN keys reflow with panel size like Intent keys.
- Rollback: restore `PIN_KEYPAD_DIGIT_LABELS` / `pin_keypad_rect`.

## Verification

Host: digit/backspace hit, blank cell misses, forget only when a
PIN exists; Field occupancy is bullets. Panther: do not save a
PIN; no-PIN lock idle stays the clock and hint.
