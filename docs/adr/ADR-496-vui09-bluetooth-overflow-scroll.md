# ADR-496: VUI-09 — Bluetooth overflow scrolls above pinned Искать

## Статус

Принято, 2026-10-11. Overflow Bluetooth devices honor
`layout_v2_scrolled`. Header, Искать, Обновить, and Назад stay at
offset 0. Experimental. PIN stays null. Leave Сейчас. Not Visual v1
sign-off. No flash.

## Нумерация

После ADR-495 следующий свободный номер — **496**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-130/206/214/226 host-locked the Bluetooth list and docked
trailing controls. Twelve devices already cross pinned Искать.
ADR-495 taught the compiler to honor `scroll_offset` when trailing
cards exist. The shell never passed an offset for Bluetooth, so
overflow names stayed unreachable.

## Decision

1. **Host-lock** `bluetooth_scroll_offset`. Paint and device hits
   read `layout_live_v2_scrolled`. Искать / Обновить / Назад stay on
   the offset-0 tree. Offset resets when the list opens or closes.
2. **Gesture.** The open Bluetooth list arms the same drag
   recognizer as Wi-Fi, only inside the stacked content pane. A drag
   is not a tap.
3. **Not this slice.** Do not scroll trusted clients. Do not convert
   leftover keyboard / gallery / lock formulas.

## Consequences

- Overflow devices stay reachable. Trailing controls stay pinned.
  Rollback: offset-0 paint/hit. Still not Visual v1 sign-off.

## Verification

Host: 12 devices produce `max_offset > 0`; last device is missing at
offset 0 and hittable after scroll; header / Искать / Обновить /
Назад stay put. Panther: do not flash; leave Сейчас.
