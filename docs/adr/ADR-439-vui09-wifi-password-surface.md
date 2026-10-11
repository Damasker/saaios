# ADR-439: VUI-09 — wifi-password compose uses its own surface

## Статус

Принято, 2026-10-10. Intent / PIN / Wi-Fi password headings already
paint from `layout_v2` (ADR-434). Live Wi-Fi password still generated
an `intent` document and bound `intent-field`. The public surface is
`wifi-password`. That document is now its own screen. Keyboard keys
stay a formula. PIN stays null. Do not type a PSK. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-438 следующий свободный номер — **439**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`sui_v2_surfaces()` already lists `wifi-password` and `pin-setup`.
Intent has `intent-public.sui`. Wi-Fi password borrowed the Intent
tree so a third-party `wifi-password` document was not the live
compose path. PIN setup already used `pin-setup`.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The wifi-password surface was not on that list.

## Decision

1. **Surface.** Live Wi-Fi password builds `overlay_field_paint` on
   `wifi-password` / `wifi-password-field`. QWERTY reserve stays the
   Intent keyboard height, not PIN's five rows.
2. **Bind.** The Field-bound Keyboard loc is `wifi-password-field`.
   Keys stay the existing formula.
3. **Public examples.** `wifi-password-public.sui` and
   `pin-setup-public.sui` are header plus Status `Field`. No
   `Keyboard`. Still Experimental.
4. **Not this slice.** Do not send a PSK. Do not change PIN.

## Consequences

- Public `wifi-password` and live compose share one surface. Rollback:
  generate `intent` / `intent-field` again. Still not Visual v1
  sign-off.

## Verification

Host: public wifi-password Field x matches the header margin and the
Intent Field; `pin-setup-public.sui` compiles public. Panther: do not
flash; leave Сейчас.
