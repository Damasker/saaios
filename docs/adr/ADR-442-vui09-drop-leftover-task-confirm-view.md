# ADR-442: VUI-09 — drop leftover `task_confirm_view`

## Статус

Принято, 2026-10-11. Pair / consent decision Buttons already dock on
`layout_v2` (ADR-221/229/431). The leftover v1 header-plus-buttons
tree had no caller. Hits stay `overlay_buttons_v2_source`. Status
invents no action. PIN stays null. Do not tap Сопряжь. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-441 следующий свободный номер — **442**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-088 kept `task_confirm_view` because remote-pair reused that
geometry. ADR-229 moved pair/consent paint onto the generated tree.
ADR-441 left the unused function as a leftover path. The live pair
frame is `status_overlay_paint("remote-pair", …)`.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover v1 confirm tree was not on that
list.

## Decision

1. **Delete** `task_confirm_view` and its unused leaf ids / height.
2. **Keep** `task_confirm_action_at` and the
   `task_confirm:accept` / `task_confirm:decline` locs. Pair Buttons
   still bind those strings on `layout_v2`.
3. **Not this slice.** `consent_view` stays the overlay geometry
   oracle. `Frame::Root` / `draw_root` stay the test fallback
   (ADR-165). Keyboard keys, gallery page, and lock idle/wake stay
   formulas.

## Consequences

- Confirm / pair hits share one generated tree. Rollback: restore
  `task_confirm_view`. Still not Visual v1 sign-off.

## Verification

Host: `task_confirm_view` is gone; accept/decline hits still resolve.
Panther: do not flash; leave Сейчас.
