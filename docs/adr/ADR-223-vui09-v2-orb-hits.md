# ADR-223: VUI-09 — OrbHost hits come from `layout_v2()`

## Статус

Принято, 2026-09-20. Privileged `OrbHost` is vocabulary. Live Orb
dot and menu hits go through generated `compile_v2()` / `layout_v2()`,
matching the closed and open `orb_zone_rect`. Paint may stay
`orb_view`. Gallery page taps stay a whole-surface formula: they are
not named `Button`/`OrbHost` locs. PIN stays null. Do not tap Orb
Изменить. Not Visual v1 sign-off.

## Нумерация

После ADR-222 следующий свободный номер — **223**. Не S33.

## Контекст

ADR-221 parked OrbHost as a zone formula because it is not a
Field/Button screen. The v2 grammar already names `OrbHost`. Linear
`layout_v2` cannot pin a top-right overlay, the same reason the apps
grid needed `Node::stack` spacers (ADR-220). USB Keyboard (ADR-222)
does not own this chrome.

## Decision

1. **Generated document.** Closed Orb is `component OrbHost` with
   `loc = "orb:toggle"` on the existing `now` surface. Open menu adds `Button` locs `orb-menu:*`.
   `compile_v2_public()` still rejects `OrbHost`.
2. **Zone overlay.** `layout_v2` places that tree on
   `v2_orb_zone_rect`. Closed is the 90 px dot. Open grows upward
   into the header dead space and stops short of stacked cards at
   y=430/2400. Menu rows fill the zone; the dot stays
   `Px(v2_orb_dot_size)` at the bottom.
3. **Paint.** `draw_orb` still reads `orb_view`. Hit-test does not.

## Consequences

- Live `orb_action_at` reads the compiler. Gallery 7-tap page flips
  stay `next_gallery_page`. Rollback: restore `orb_view.hit_test`.
  Still not Visual v1 sign-off.

## Verification

Host: closed-dot center toggles; stacked card y=525 misses; open
menu first row is inbox; bottom of the zone closes. Panther: flash;
Сейчас; do not open the Orb menu.
