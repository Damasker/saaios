# ADR-443: VUI-09 — drop leftover `now_object_summary_rect`

## Статус

Принято, 2026-10-11. NOW object paint and hits already read
`layout_v2` (ADR-217/225). The leftover `width/20` cursor math had
no live caller. `open_object` stays on the compiled `ObjectSummary`.
Status invents no action. PIN stays null. Leave Сейчас. Not Visual
v1 sign-off.

## Нумерация

После ADR-442 следующий свободный номер — **443**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-200 copied the live `now_object_summary_rect` vertical formula
into `layout_v2`. ADR-217 switched hits to that tree. ADR-225
switched paint to `NowPaintChrome.object`. The helper stayed as a
second tree: `width/20` on an optional 2160-tall content box, not
the compiled `width/22` slot.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover NOW object formula was not on that
list.

## Decision

1. **Delete** `now_object_summary_rect`.
2. **Keep** `now_object_tapped` / `chrome.object` on `layout_v2`.
3. **Not this slice.** `consent_view` stays the overlay geometry
   oracle. `Frame::Root` / `draw_root` stay the test fallback
   (ADR-165). Keyboard keys, gallery page, and lock idle/wake stay
   formulas.

## Consequences

- NOW object paint and hit-test share one compiled slot. Rollback:
  restore `now_object_summary_rect`. Still not Visual v1 sign-off.

## Verification

Host: `now_object_summary_rect` is gone; 540,335 still hits
`open_object` when a summary exists. Panther: do not flash; leave
Сейчас.
