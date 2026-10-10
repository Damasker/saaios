# ADR-428: VUI-09 — Object View `ObjectSummary` reads `layout_v2()`

## Статус

Принято, 2026-10-10. Empty Object View already docks `ContextHeader`
through `v2_decision_node` (ADR-427). Identity still painted at
`header.y + 140` (ADR-137). That slot is now a named `ObjectSummary`
on the same generated overlay tree as the decision Buttons. Status
invents no `open_object`. Facts stay runtime content below the
summary. PIN stays null. Do not open Object View this slice. Leave
Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-427 следующий свободный номер — **428**. Не S33.

## Контекст

Public `object-public.sui` already named `ObjectSummary`, but
`layout_v2` dropped it on a no-tab overlay. Live paint used the
ADR-137 formula inside the header rect, a second tree from hits.

## Decision

1. **Overlay identity.** When an overlay document has `ObjectSummary`,
   `v2_decision_node` docks `ContextHeader` at 140 px (status layer)
   and `ObjectSummary` at `v2_now_object_height()`. No action unless
   a later slice names one. Status invents none.
2. **Buttons stay at the bottom.** A Fill body sits between the
   summary and the 300 px decision row so accept/decline hits stay
   on `y=2250`.
3. **Live paint.** `object_overlay_v2_source` emits header + Status
   `ObjectSummary` + Button locs. `draw_object_view` paints identity
   at `summary_rect.y`. Related/details stay below, clipped to the
   body above the buttons.
4. **Consent unchanged.** Overlays without `ObjectSummary` keep the
   header-Fill + button row from ADR-229.

## Consequences

- Object View identity and hit-test share `layout_v2`.
- `object-public.sui` now lays out a real Status summary.
- Leftover formulas stay Keyboard keys, gallery page, lock
  idle/wake (ADR-232). Thin tuning stays last (ADR-213). Rollback:
  paint identity at `header.y + 140` again. Still not Visual v1
  sign-off.

## Verification

Host: public object summary is `(0, 140, 1080, 144)` with no action;
two-action buttons still hit `y=2250`; consent overlay unchanged.
Panther: do not flash; leave Сейчас.
