# ADR-225: VUI-09 — NOW paint reads `layout_v2()`

## Статус

Принято, 2026-09-20. Live Сейчас header, ObjectSummary, empty
`SurfacePattern`, footer rows, and tabs already hit through
`layout_v2(now.sui)` (ADR-217). Paint now draws those chrome slots
from the same tree. Live `SystemSection` rows stay runtime content,
not invented Buttons. Keyboard keys, gallery page taps, and lock
idle/wake stay formulas (ADR-222–224). PIN stays null. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-224 следующий свободный номер — **225**. Не S33.

## Контекст

Hit-test for NOW chrome already reads `now_view()`. `draw_now` still
placed the heading from a 150/2400 cursor and the object from that
cursor, so paint could drift from `ObjectSummary` / `ContextHeader`
rects. Tabs came from `root_view` even though `now.sui` names the
same `BottomNavigation`.

## Decision

1. **One tree.** `Frame::Now` takes chrome rects and tab rects from
   `layout_v2(now.sui)`.
2. **Header / object / empty.** Paint `ContextHeader` into its node,
   a live `ObjectSummary` into its node, and the empty pattern into
   `SurfacePattern`. Status-layer clearance stays the 150/2400 inset
   inside the header slot so copy is not drawn under the status
   surface.
3. **Footer / tabs.** Footer rows already used compiled rects.
   Tab strip rects come from `now.sui` `BottomNavigation` children.
4. **Live sections.** `Сегодня` / `Продолжается` / `Далее` are not in
   `now.sui`. They paint below the object (or header) slot. No new
   `loc`.

## Consequences

- NOW chrome paint and hit-test share `now_view()`. Inbox/Spaces/list
  paint stays procedural until their slices. Rollback: restore the
  cursor `draw_now`. Still not Visual v1 sign-off.

## Verification

Host: chrome rects equal `layout_v1_find` on `now_view`; object y=263;
tabs match `root_view` BottomNavigation. Panther: flash; Сейчас; do
not tap Приложения or the object; leave Сейчас.
