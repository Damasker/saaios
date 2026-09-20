# ADR-221: VUI-09 — overlay Field and decision hits come from `layout_v2()`

## Статус

Принято, 2026-09-20. Overlay chrome the v2 vocabulary already names
(`Field`, `Button`, `DecisionOverlay` screens) hit-tests through
`compile_v2()` / `layout_v2()`. Keyboard keys are not vocabulary;
they stay the existing `intent_view` / PIN keypad formula. Paint
may stay procedural. PIN stays null. Do not tap Разрешить, Сопряжь,
or type intent. Leave PIN setup with Отмена. Not Visual v1 sign-off.

## Нумерация

После ADR-220 следующий свободный номер — **221**. Не S33.

## Контекст

ADR-220 closed the apps grid. Remaining procedural chrome was
overlays. Linear `layout_v2` can dock a bottom decision row and a
Field above a keyboard reserve. It cannot name QWERTY/PIN keys.

## Decision

1. **Decision row.** `Button` locs that are not `manage_app:` overlay
   as a full-width horizontal row of height `ROOT_TAB_HEIGHT` (300),
   matching live consent / task-confirm / object-view / remote-pair.
2. **Compose Field.** `Field` on `intent` / `wifi-password` /
   `pin-setup` docks immediately above a keyboard-height reserve
   (4 rows + Small pad, or 5 rows + XSmall on `pin-setup`). Keys
   inside the reserve stay `intent_view` / `pin_setup_view`.
3. **Lock Field.** `Field` on `lock` overlays at y=24 inside the
   260 header band. PIN keys stay the unlock keypad formula.
4. **ObjectSummary** without tabs invents no `open_object` hit.

## Consequences

- Overlay hits that have vocabulary read the compiler. Keyboard
  keys remain a second formula until a later ADR names them.
  OrbHost and gallery page taps stay their zone formulas; they are
  not sui 2 Field/Button screens. Rollback: restore `consent_view` /
  `intent_view` field finds. Still not Visual v1 sign-off.

## Verification

Host: consent 270/810 at y=2250; Field sits on the keyboard
reserve; lock Field stays in the header band. Panther: flash;
Сейчас; do not open PIN or tap Разрешить.
