# ADR-462: VUI-09 — public subset rejects remaining privileged names

## Статус

Принято, 2026-10-11. `compile_v2_public` already rejects `OrbHost`,
`lock`, `TrustedClientRow`, `CapabilityRow`, `Keyboard`, and
`diagnostic`. `DecisionOverlay`, `SystemStatus`, and `gallery`
were privileged in the vocabulary without a named public reject.
Public subset stays Experimental. Leftover named chrome stays
closed. Keyboard keys, gallery page, and lock idle/wake stay
formulas. PIN stays null. Do not 7-tap. Leave Сейчас. Not Visual
v1 sign-off.

## Нумерация

После ADR-461 следующий свободный номер — **462**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-185/186 made `compile_v2_public` the third-party gate.
Privileged gallery fixtures may appear on-device for review;
copying those names into an app must fail. Object View public
example is Status cards, not `DecisionOverlay`.

Converting gallery page taps into Buttons, or promoting any of
these names, is not this gate.

## Decision

1. **Host-lock** `compile_v2_public` rejects `DecisionOverlay`,
   `SystemStatus`, and `gallery`. `compile_v2` still accepts
   `DecisionOverlay` as privileged.
2. **`object-public.sui` omits `DecisionOverlay`.** Status invents
   no overlay Buttons.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not 7-tap gallery. Do not start thin tuning.

## Consequences

- Remaining privileged names fail the public gate. Rollback: drop
  the reject test. Still not Visual v1 sign-off.

## Verification

Host: public compile rejects those three names. Panther: do not
flash; do not 7-tap; leave Сейчас.
