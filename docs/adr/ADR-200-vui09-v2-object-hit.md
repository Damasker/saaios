# ADR-200: VUI-09 — dock `ObjectSummary` as the NOW object hit

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. `layout_v2()`
stacks `ContextHeader` then `ObjectSummary` using the live
`now_object_summary_rect` vertical formula (150/2400 inset, Title,
Medium, Body+Caption, `MIN_TOUCH_TARGET`). Action is `open_object`.
A screen without `ObjectSummary` invents no object hit. Footer and
tab hits stay. `compile()` stays v1. No new daemon. Leave Сейчас.

## Нумерация

После ADR-199 следующий свободный номер — **200**. Не S33.

## Контекст

ADR-199 named the NOW footer. Live Сейчас still opens Object View
from a procedural `now_object_tapped` rect. `layout_v2()` treated
`ObjectSummary` as a Fill leaf, so 540,335 missed. Borrowing v1
`content_actions` would revive leftover inspect cards (ADR-187).
`inspect_selected_entity` is not a public name. Inbox already uses
`open_object`.

## Decision

1. **No new keyword.** Naming `component ObjectSummary` is the
   destination. Duplicate summaries after the first stay
   non-actionable Fill leftovers.
2. **Layout.** Header height is the live stack above the object
   (150 scaled to content height + Title + Medium). Object height
   is Body+Caption, never below `MIN_TOUCH_TARGET`. No lifecycle
   and no trailing — the public sample does not name those.
3. **Action.** `open_object`. Horizontal inset stays the shared
   content pad `width/22` (footer convention), not `draw_now`'s
   `width/20`.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   `now_object_tapped` + `layout_v1_root()`.

## Consequences

- v2 layout owns the public NOW object destination. Lifecycle and
  trailing still stay paint-side. Rollback: treat `ObjectSummary`
  as a Fill leaf again. Next: leftover badge/kicker/swatch/app-tile
  paint, or Experimental→Stable after Visual v1 acceptance. Still
  not Visual v1 sign-off.

## Verification

Host: public NOW 540,335 hits `open_object`; 540,250 misses (header);
a screen without `ObjectSummary` has no object hit; footer and tabs
still match. Panther: Сейчас on HEAD; leave Сейчас.
