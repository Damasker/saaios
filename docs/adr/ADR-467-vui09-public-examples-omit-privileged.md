# ADR-467: VUI-09 — public examples omit privileged chrome

## Статус

Принято, 2026-10-11. `compile_v2_public` already rejects privileged
names (ADR-185/462). The fourteen `*-public.sui` samples omitted
them in source without a set lock. Public subset stays
Experimental. Leftover named chrome stays closed. Keyboard keys,
gallery page, and lock idle/wake stay formulas. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-466 следующий свободный номер — **467**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-462 locked the compile reject. ADR-459 locked compose without
`Keyboard`. ADR-463 locked three examples against deferred names.
A later edit could name `OrbHost` in a public sample and still
compile if the token were commented out, or teach privileged
chrome by copying `screen lock`. The public examples are the
third-party contract.

Converting OrbHost / lock / gallery into public names is not this
gate.

## Decision

1. **Host-lock** every `*-public.sui` compiles through
   `compile_v2_public`, is not privileged, and omits the
   privileged vocabulary: `OrbHost`, `SystemStatus`,
   `DecisionOverlay`, `CapabilityRow`, `TrustedClientRow`,
   `Keyboard`, `lock`, `diagnostic`, `gallery`.
2. **Privileged samples stay labelled.** `trusted-privileged.sui`
   and `capability-privileged.sui` remain `compile_v2` only.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not start thin tuning.

## Consequences

- Public examples cannot teach privileged chrome. Rollback: drop
  the omit-set test. Still not Visual v1 sign-off.

## Verification

Host: fourteen public examples omit the privileged set. Panther:
do not flash; leave Сейчас.
