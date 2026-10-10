# ADR-429: VUI-09 — Object View facts read `layout_v2()`

## Статус

Принято, 2026-10-10. Object View identity already docks
`ObjectSummary` (ADR-428). Related and detail lines still painted
as a y-cursor under the summary. Those lines are now Status
`DataRow`s on the same generated overlay tree. They invent no
actions. DecisionOverlay facts and OAM permission stay runtime
content. PIN stays null. Do not open Object View this slice. Leave
Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-428 следующий свободный номер — **429**. Не S33.

## Контекст

`object_view_details` and the related caption already exist. Paint
walked them with `header.y` increments. Public `object-public.sui`
had no fact row, so a third-party overlay could not name a Status
line without shell internals.

## Decision

1. **Fact rows.** `v2_decision_node` docks each overlay `DataRow`
   below `ObjectSummary` at Body line height. Status invents no
   action. Buttons still sit on a Fill spacer above the 300 px row.
2. **Live source.** `object_overlay_v2_source` emits
   `object.related` when a related line exists and
   `object.detail.{i}` for each detail. Paint reads those rects.
3. **Public example.** `object-public.sui` names one Status
   `DataRow` at `object.related`. Still Experimental.
4. **Not this slice.** DecisionOverlay fact lines and permission
   `SurfacePattern` stay the existing cursor below the last fact.
   Keyboard keys, gallery page, and lock idle/wake stay formulas
   (ADR-232).

## Consequences

- Related/detail geometry and hit-test share `layout_v2`.
- Consent overlays without `ObjectSummary` are unchanged.
- Rollback: paint related/details with the y-cursor again. Still
  not Visual v1 sign-off.

## Verification

Host: public `object.related` sits at `y=284` with no action;
two-action buttons still hit `y=2250`. Panther: do not flash;
leave Сейчас.
