# ADR-440: VUI-09 — pair fingerprint wrap reads the compiled slot

## Статус

Принято, 2026-10-11. `pair.fingerprint` already docks on
`stacked_row_rect` (ADR-432). Wrap still applied a second
`width/22` inside that padded card. MonoBody now starts from the
slot with the same inset as a Status ActionCard. Status invents no
action. PIN stays null. Do not tap Сопряжь. Leave Сейчас. Not
Visual v1 sign-off.

## Нумерация

После ADR-439 следующий свободный номер — **440**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-432 kept wrap as paint so `SHA256:` is not truncated. The wrap
origin used `fingerprint_rect.width / 22` after the compiler had
already inset the row. That was a second tree, the same class of
leftover ADR-436 closed for Object View.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. Fingerprint wrap was not on that list.

## Decision

1. **Slot.** Wrap uses `pair.fingerprint` x/y/width/height.
2. **Inset.** 34 px, same as `draw_action_card` text without an
   action, so the hash lines up with the client row. Not `width/22`.
3. **Not this slice.** Do not flatten the hash into an ActionCard
   label. Keyboard keys, gallery page, and lock idle/wake stay
   formulas.

## Consequences

- Fingerprint wrap and the compiled slot share one origin. Rollback:
  restore `fingerprint_rect.width / 22`. Still not Visual v1
  sign-off.

## Verification

Host: wrap box for `(49, 650, 982, 190)` is `(83, 650, 914, 190)`,
not `x=93`. Panther: do not flash; leave Сейчас.
