# ADR-432: VUI-09 — remote-pair fingerprint reads `layout_v2()`

## Статус

Принято, 2026-10-10. Consent / pair Status cards already dock
(ADR-431). The SHA256 fingerprint still sat `client.y + height + 24`
by hand. `pair.fingerprint` is now the next stacked Status
`DataRow`. Wrap stays MonoBody so the full string stays readable.
Status invents no action. PIN stays null. Do not tap Сопряжь.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-431 следующий свободный номер — **432**. Не S33.

## Контекст

ADR-144 kept the fingerprint as wrapped mono text so ActionCard
would not truncate `SHA256:`. ADR-431 docked `pair.client` and left
that wrap on a y-cursor under the client card, a second tree from
the stacked slot.

## Decision

1. **Slot.** `pair.fingerprint` docks on `stacked_row_rect` after
   `pair.client`. Status invents no action.
2. **Wrap stays paint.** MonoBody wrap lives inside that rect. Do
   not flatten the fingerprint into an ActionCard label.
3. **Buttons stay at the bottom.** Accept/decline remain the 300 px
   row. Hits at `y=2250` are unchanged.
4. **Public example.** `remote-pair-public.sui` is the labelled
   sample. Still Experimental.

## Consequences

- Pair chrome except leftover formulas shares `layout_v2`. Consent
  cards and Object View are unchanged. Keyboard keys, gallery page,
  and lock idle/wake stay formulas (ADR-232). Rollback: restore the
  `client.y + height + 24` cursor. Still not Visual v1 sign-off.

## Verification

Host: public `pair.fingerprint` is `(x, 650, …, 190)` with no
action; accept still hits `y=2250`. Panther: do not flash; leave
Сейчас.
