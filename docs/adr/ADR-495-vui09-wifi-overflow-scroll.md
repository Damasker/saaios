# ADR-495: VUI-09 — Wi-Fi overflow scrolls above pinned Обновить

## Статус

Принято, 2026-10-11. Overflow Wi-Fi networks honor `layout_v2_scrolled`.
Header, Обновить, and Назад stay at offset 0. Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. No flash.

## Нумерация

После ADR-494 следующий свободный номер — **495**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-129/205/214/226 host-locked the Wi-Fi list and docked trailing
controls. Twelve networks already cross pinned Обновить. The compiler
clipped stacked rows against trailing cards, but `scroll_offset` was
honored only for tabbed Me-style lists and DevSurface. Wi-Fi has no
tabs, so overflow SSIDs stayed unreachable.

## Decision

1. **Compiler.** When a trailing control exists, `layout_v2_scrolled`
   honors `scroll_offset`. Header and trailing cards stay at offset 0.
   Stacked rows still clip under the first trailing top and do not
   slide above the first stacked slot.
2. **Host-lock** `wifi_scroll_offset`. Paint and network hits read
   `layout_live_v2_scrolled`. Обновить / Назад stay on the offset-0
   tree. Offset resets when the list opens or closes.
3. **Gesture.** The open Wi-Fi list arms the same drag recognizer as
   Space detail, only inside the stacked content pane. A drag is not
   a tap.
4. **Not this slice.** Do not scroll Bluetooth or trusted clients.
   Do not convert leftover keyboard / gallery / lock formulas.

## Consequences

- Overflow SSIDs stay reachable. Обновить and Назад stay pinned.
  Rollback: offset-0 paint/hit. Still not Visual v1 sign-off.

## Verification

Host: 12 networks produce `max_offset > 0`; last SSID is missing at
offset 0 and hittable after scroll; header / Обновить / Назад stay
put. Panther: do not flash; leave Сейчас.
