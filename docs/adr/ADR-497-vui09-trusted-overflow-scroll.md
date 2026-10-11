# ADR-497: VUI-09 — trusted clients overflow scrolls above pinned Назад

## Статус

Принято, 2026-10-11. Overflow trusted-client rows honor
`layout_v2_scrolled`. Header and Назад stay at offset 0.
Experimental. PIN stays null. Leave Сейчас. Not Visual v1
sign-off. No flash.

## Нумерация

После ADR-496 следующий свободный номер — **497**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-131/207/214/226 host-locked the trusted-client list and docked
Назад. Twelve fingerprints already cross the pinned back card.
ADR-495 taught the compiler to honor `scroll_offset` when trailing
cards exist. The shell never passed an offset here, so overflow
clients stayed unreachable.

## Decision

1. **Host-lock** `trusted_scroll_offset`. Paint and revoke hits read
   `layout_live_v2_scrolled`. Назад stays on the offset-0 tree.
   Offset resets when the list opens or closes.
2. **Gesture.** The open list arms the same drag recognizer as
   Wi-Fi / Bluetooth, only inside the stacked content pane. A drag
   is not a tap. Revoke still requires the existing two-tap confirm.
3. **Not this slice.** Do not convert leftover keyboard / gallery /
   lock formulas. Do not flash.

## Consequences

- Overflow clients stay reachable. Назад stays pinned. Rollback:
  offset-0 paint/hit. Still not Visual v1 sign-off.

## Verification

Host: 12 clients produce `max_offset > 0`; last row is missing at
offset 0 and hittable after scroll; header and Назад stay put.
Panther: do not flash; leave Сейчас.
