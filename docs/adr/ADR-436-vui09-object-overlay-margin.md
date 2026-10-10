# ADR-436: VUI-09 — Object View identity / facts share the `width/22` margin

## Статус

Принято, 2026-10-10. Object View already docks `ObjectSummary`, facts,
and permission on `layout_v2` (ADR-428–430). Those slots were still
full-bleed (`x=0`). Paint then applied `header.width/22` by hand.
The compiled cards now use the same margin as consent rows and the
compose Field. Status invents no action. PIN stays null. Do not open
Object View. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-435 следующий свободный номер — **436**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Consent / pair cards and compose `ContextHeader` already inset
`width/22` (ADR-431/434). Object View identity used a second tree:
full-width compiler slots plus a paint-side margin. That drifted
from the compiled x.

## Decision

1. **Slot.** `v2_decision_node` wraps `ObjectSummary`, fact
   `DataRow`s, and `SurfacePattern` in a padded column (`width/22`).
   The 140 px `ContextHeader` spacer stays full-bleed for the status
   layer. Buttons stay on the bottom row.
2. **Paint.** `draw_object_view` reads `summary_rect` / fact /
   permission x and width. No `header.x + margin` formula.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas (ADR-232).

## Consequences

- Object View identity and facts share `layout_v2` with the same
  margin as other overlays. Hits are unchanged (Status invents no
  action). Rollback: restore the unpadded object column and
  `header.x + margin`. Still not Visual v1 sign-off.

## Verification

Host: public `object.summary` is `(49, 140, 982, 144)` with no
action; related / permission share x=49; two-action buttons still
hit `y=2250`. Panther: do not flash; leave Сейчас.
