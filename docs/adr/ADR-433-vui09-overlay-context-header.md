# ADR-433: VUI-09 — overlay `ContextHeader` paint reads `layout_v2()`

## Статус

Принято, 2026-10-10. Consent / pair Status cards and the pair
fingerprint slot already dock (ADR-431/432). Heading paint still
used `content_rect` plus a 150/2100 formula. The generated
`ContextHeader` slot is now the paint box. Inset matches NOW
(150/2400 of the panel). Status invents no action. PIN stays null.
Do not tap Разрешить or Сопряжь. Leave Сейчас. Not Visual v1
sign-off.

## Нумерация

После ADR-432 следующий свободный номер — **433**. Не S33.

## Контекст

NOW heading already sits inside the compiled `ContextHeader`
(ADR-225). Overlay cards shared that tree, but the title still
walked the clip rect. That was a second tree for the same overlay
family.

## Decision

1. **Slot.** Consent and remote-pair look up `ContextHeader` on the
   same `status_overlay_paint` tree as the cards. Height stays
   `stacked_row_rect` top (430 on 2400). No action.
2. **Inset.** Paint uses 150/2400 of the panel inside that slot,
   same clearance as `draw_now`. Margin is the compiled slot, not
   `width/20`.
3. **Not this slice.** List / apps / compose headers stay
   `paint_context_header(content)`. Keyboard keys, gallery page,
   and lock idle/wake stay formulas (ADR-232).

## Consequences

- Overlay heading, cards, fingerprint, and Buttons share
  `layout_v2`. Object View identity stays `ObjectSummary`. Rollback:
  restore `paint_context_header(content)`. Still not Visual v1
  sign-off.

## Verification

Host: public `ContextHeader` is `(x, 0, …, 430)` with no action;
cards stay `stacked_row_rect`. Panther: do not flash; leave Сейчас.
