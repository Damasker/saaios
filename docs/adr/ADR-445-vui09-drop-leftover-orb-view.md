# ADR-445: VUI-09 — drop leftover `orb_view`

## Статус

Принято, 2026-10-11. Orb paint and hits already read `layout_v2` over
`orb_v2_source` (ADR-223/230). The leftover v1 zone tree was only a
test oracle. Status invents no action. PIN stays null. Do not tap
Orb. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-444 следующий свободный номер — **445**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-223 switched Orb hits to a generated `OrbHost` document.
ADR-230 switched paint to the same tree. ADR-444 left `orb_view` as
the Orb geometry oracle. That helper still stacked `orb-dot` /
`orb-menu-{i}` leaves inside `orb_zone_rect` — a second tree.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover v1 Orb tree was not on that list.

## Decision

1. **Delete** `orb_view`, `orb_dot_size`, and `ORB_DOT_ID`.
2. **Keep** `orb_action_at` / `orb_v2_source` / `orb_zone_rect`.
3. **Not this slice.** `Frame::Root` / `draw_root` stay the test
   fallback (ADR-165). Keyboard keys, gallery page, and lock
   idle/wake stay formulas.

## Consequences

- Orb paint and hits share one generated tree. Rollback: restore
  `orb_view`. Still not Visual v1 sign-off.

## Verification

Host: `orb_view` is gone; closed toggle still matches
`orb_zone_rect`; menu rows stay inside the zone. Panther: do not
flash; leave Сейчас.
