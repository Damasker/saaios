# ADR-150: VUI-07 QWERTY — compact bottom keyboard, not a full-bleed fill

## Статус

Принято, 2026-09-19. Intent and Wi-Fi password keep ADR-029's
`Node`/`layout()`/`hit_test()` QWERTY. It docks at the bottom at
`MIN_TOUCH_TARGET` row height, with iPhone-style staggered letter
rows and a wide space. PIN dialer is unchanged. Do not type a PSK.
Do not send an intent. Leave with Отмена. Space detail is not this
slice.

## Нумерация

После ADR-149 следующий свободный номер — **150**. Не S33.

## Контекст

ADR-012/022 closed generic Wayland IME. ADR-029's bespoke QWERTY is
the only letter keyboard this source may use. It currently `Fill`s
the panel under a 260px header, so letter keys are several hundred
pixels tall — the opposite of a practical thumb keyboard. First
iPhone QWERTY was compact, bottom-docked, staggered (`asdf`/`zxcv`
inset so keys stay one size), with a wide space. That geometry, not
a new OSK.

## Decision

1. **Keyboard height is four `MIN_TOUCH_TARGET` rows** plus small
   padding, clamped so a field still fits above. Header `Fill`s the
   rest. Same tree as today, not a second painter.
2. **Shorter letter rows inset** so a 7-key `zxcvbnm` matches the
   10-key `qwertyuiop` cell width. Symbols rows with ≥10 keys stay
   flush.
3. **Space `Fill`s**; Отмена/Готово-sized side keys and 123/⌫ stay
   at least `MIN_TOUCH_TARGET` wide. Actions unchanged.

## Consequences

- Compose/Wi-Fi keep a visible field above the keys.
- Landscape still hits `MIN_TOUCH_TARGET`.
- Rollback: restore `INTENT_HEADER_HEIGHT` + Fill keyboard.

## Verification

Host: Q/A/Z hit their own keys; space wider than 123; keyboard in
the bottom half. Panther: open «Новое намерение», do not send, leave
Отмена.
