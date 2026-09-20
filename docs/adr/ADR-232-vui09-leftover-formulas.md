# ADR-232: VUI-09 — leftover paint stays formula

## Статус

Принято, 2026-09-20. Named chrome paint now reads the same generated
`layout_v2()` / `layout_v2_scrolled()` tree as hits (ADR-225–231).
Three leftover paints have no vocabulary target as named Buttons:
Keyboard keys inside the IME object (ADR-222), gallery page flips
(ADR-223/224), and lock idle/wake whole-surface taps (ADR-154/209).
They stay formulas. PIN stays null. Do not 7-tap gallery this slice.
Not Visual v1 sign-off.

## Нумерация

После ADR-231 следующий свободный номер — **232**. Не S33.

## Контекст

The paint goal closed NOW, lists, Me, apps, overlays, OrbHost, and
diagnostic onto one Node tree. Remaining draws are not missing
screens: they are keys, a whole-surface gallery page, and lock
idle/wake. Inventing Button locs for them would break the IME object,
7-tap gallery, and lock contract.

## Decision

1. **Keyboard keys.** Paint stays `intent_view` / `pin_keypad_node`
   inside privileged `Keyboard`. USB HID may replace the panel. Do
   not emit per-key Buttons in `.sui`.
2. **Gallery page.** Page flips stay `next_gallery_page` on the whole
   surface. Do not 7-tap this slice.
3. **Lock idle/wake.** Idle and wake stay whole-surface taps, not
   named Buttons. PIN stays null. Restore `/run/saaios/dev-no-lock`
   after lock proofs.

## Consequences

- Named chrome paint and hit-test share `layout_v2`. Leftover formulas
  are recorded, not next named-chrome work. Rollback: none; these
  paints were never compiler slots. Still not Visual v1 sign-off.

## Verification

Host: ledger and known-limitations name ADR-232 leftover formulas.
Panther already on ADR-231 `f00632f2…`; Сейчас live; do not 7-tap.
