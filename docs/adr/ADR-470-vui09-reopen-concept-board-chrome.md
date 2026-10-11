# ADR-470: VUI-09 — reopen concept-board chrome

## Статус

Принято, 2026-10-11. Operator unblocked ADR-213/232/265 as a park
for work the accepted boards already name. Concept images in
`docs/os/ui/references/` and `graphic-kit/mockups/` are the visual
source. Public subset stays Experimental until a real Stable pass.
VUI-09 Acceptance stays unchecked until Pixel evidence. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-469 следующий свободный номер — **470**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-213 parked thin tuning while layout/hit-test work existed.
ADR-232 kept Keyboard keys, gallery page, and lock idle/wake as
formulas because they had no vocabulary target. ADR-265 kept
Visual v1 unsigned and Experimental. Those parks stopped us from
shipping board chrome that already exists: Object related rows,
quick actions, Intent steps, NOW attention list.

The boards still forbid fake weather, voice-as-if-AoC, and painted
Android VM.

## Decision

1. **Boards are the source.** Remaining chrome is taken from
   `v1-surfaces.jpg`, `v1-orb.jpg`, `v1-visual-system.jpg`, and the
   graphic-kit mockups. Poster hexes still lose to calibrated tokens.
2. **Implement what the board names and the runtime already has.**
   Related peers, workflow follow actions, and OAM actions may be
   Buttons. Missing runtime stays empty.
3. **Leftover formulas stay until a board names them.** Keyboard
   keys, gallery page, and lock idle/wake are not on these boards.
4. **Do not fake.** No weather cluster, no listen state as live
   voice, no Android VM «Готово».
5. **Do not paper-sign.** Experimental stays. Acceptance stays
   `[ ]` until panther re-verify. Thin tuning may follow board
   chrome; it is no longer a reason to refuse a named region.

## Consequences

- Next Visual slices ship board regions, not more public-reject
  locks. Rollback: restore the park. Still not Visual v1 sign-off.

## Verification

Host: this ADR is cited from the roadmap. Panther: do not flash
this slice; leave Сейчас.
