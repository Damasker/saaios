# ADR-230: VUI-09 — OrbHost paint reads `layout_v2()`

## Статус

Принято, 2026-09-20. Overlay paint already reads generated
`layout_v2()` (ADR-229). Live Orb closed-dot and open-menu rects now
come from the same `orb_v2_source` / `layout_v2()` tree hit-test
already uses (ADR-223). Gallery page taps stay a whole-surface
formula. PIN stays null. Do not tap Изменить. Leave Сейчас. Not
Visual v1 sign-off.

## Нумерация

После ADR-229 следующий свободный номер — **230**. Не S33.

## Контекст

Hits for OrbHost already generate `orb:toggle` and `orb-menu:`
Buttons. Paint still read `orb_view` children, a second tree.

## Decision

1. **One tree.** `build_orb_frame` builds `layout_v2` from
   `orb_v2_source`.
2. **Locs.** Closed and open dots look up `orb:toggle`. Menu rows
   look up `action.wire()`.
3. **No invented Buttons.** Closed omits menu locs so layout docks
   only the dot.

## Consequences

- Orb paint and hit-test share the generated tree. Diagnostic paint
  stays the next slice. Rollback: restore `orb_view` in
  `build_orb_frame`. Still not Visual v1 sign-off.

## Verification

Host: closed `orb:toggle` matches `orb_zone_rect(..., 0)`; open
`orb-menu:inbox` matches `orb_view` child 0. Panther: flash; Сейчас;
do not tap Изменить; leave Сейчас.
