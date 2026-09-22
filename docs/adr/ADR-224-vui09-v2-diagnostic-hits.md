# ADR-224: VUI-09 — diagnostic hits come from `layout_v2()`

## Статус

Принято, 2026-09-20. Privileged surface `diagnostic` is vocabulary.
Live DevSurface Назад goes through generated `compile_v2()` /
`layout_v2()` `DataRow`s plus trailing `row back`, matching
`stacked_control_rect`. Data rows stay read-only. Paint may stay
`scrolled_row_rect`. Gallery page taps stay a whole-surface formula.
Keyboard keys stay inside the IME object. PIN stays null. Do not
7-tap gallery this slice. Not Visual v1 sign-off.

## Нумерация

После ADR-223 следующий свободный номер — **224**. Не S33.

## Контекст

ADR-223 closed OrbHost. Remaining named chrome was the hidden
diagnostic list: `DataRow` plus the same trailing Назад lists already
dock (ADR-214). Linear `layout_v2` already places that shape.

## Decision

1. **Generated document.** `screen diagnostic` with one `DataRow`
   (`a11y = Status`) per live row and `row back`. No per-row action.
2. **Back hit.** `dev_surface_back_tapped` reads `list_back` from
   `layout_v2`. Overflow docks like `stacked_control_rect`.
3. **Paint.** Diagnostic cards still use `scrolled_row_rect`. Hit-test
   of Назад does not.

## Consequences

- Live diagnostic Назад reads the compiler. Gallery page flips stay
  `next_gallery_page`. Keyboard keys stay `intent_view` /
  `pin_keypad_node` inside the IME. Lock idle/wake stay whole-surface
  taps, not named Buttons. Rollback: restore `stacked_control_rect`.
  Still not Visual v1 sign-off.

## Verification

Host: three-row data miss; back at stacked index 3; nine-row overflow
docks. `compile_v2_public()` rejects `diagnostic`. Panther: flash;
Сейчас; do not 7-tap gallery.
