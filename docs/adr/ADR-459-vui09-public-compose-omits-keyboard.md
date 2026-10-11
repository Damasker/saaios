# ADR-459: VUI-09 — public compose omits `Keyboard`

## Статус

Принято, 2026-10-11. Intent / Wi-Fi password / PIN setup public
examples are header plus Status `Field` (ADR-434/439). Privileged
`Keyboard` stays off `compile_v2_public()`. Public subset stays
Experimental. Leftover named chrome stays closed. Keyboard keys,
gallery page, and lock idle/wake stay formulas. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-458 следующий свободный номер — **459**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-222 made `Keyboard` privileged. Public compose examples already
omit it. Host locks measured header/field margins (ADR-434/439)
and rejected `OrbHost` / `TrustedClientRow` / `CapabilityRow`.
They did not name the three compose sources as a Keyboard-free
set, or lock PIN setup to the same header/field geometry as
Intent.

Inventing a public Keyboard, or converting key paint into Buttons,
would break the IME object.

## Decision

1. **Host-lock** `intent-public.sui`, `wifi-password-public.sui`,
   and `pin-setup-public.sui`: no `Keyboard` in the source or
   compiled tree. Header x matches the Field (`width/22`). Field
   fills to the panel bottom. Status invents no action.
2. **`compile_v2_public` rejects `Keyboard`** on compose. Shell
   IME stays `compile_v2()` only.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not start thin tuning.

## Consequences

- Public compose stays header+Field. Rollback: drop the compose
  Keyboard-free test. Still not Visual v1 sign-off.

## Verification

Host: three public compose examples omit `Keyboard`; adding one
fails `compile_v2_public`. Panther: do not flash; leave Сейчас.
