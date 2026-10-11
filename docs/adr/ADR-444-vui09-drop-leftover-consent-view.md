# ADR-444: VUI-09 — drop leftover `consent_view`

## Статус

Принято, 2026-10-11. Consent decision Buttons already dock on
`layout_v2` (ADR-221/229/431). The leftover v1 header-plus-buttons
tree was only a test oracle. Hits stay `overlay_buttons_v2_source`.
Status invents no action. PIN stays null. Do not tap Разрешить.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-443 следующий свободный номер — **444**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-229 moved consent paint onto the generated tree. ADR-442 dropped
`task_confirm_view` and left `consent_view` as the overlay geometry
oracle. Live consent is `status_overlay_paint("consent", …)`. The
oracle compared that tree to a second v1 layout.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover v1 consent tree was not on that
list.

## Decision

1. **Delete** `consent_view` and its unused leaf ids / height.
2. **Keep** `consent_action_at` and the `consent:accept` /
   `consent:decline` locs.
3. **Not this slice.** `Frame::Root` / `draw_root` stay the test
   fallback (ADR-165). `orb_view` stays the Orb geometry oracle.
   Keyboard keys, gallery page, and lock idle/wake stay formulas.

## Consequences

- Consent hits share one generated tree. Rollback: restore
  `consent_view`. Still not Visual v1 sign-off.

## Verification

Host: `consent_view` is gone; accept/decline hits still resolve.
Panther: do not flash; leave Сейчас.
