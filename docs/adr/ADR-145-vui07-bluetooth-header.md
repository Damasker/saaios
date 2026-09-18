# ADR-145: VUI-07 Bluetooth list — ContextHeader, not Surface bar

## Статус

Принято, 2026-09-19. «Bluetooth устройства» sits under a real
`ContextHeader` instead of `draw_action_row_list`'s Surface header
fill. Host-green; panther `efbab13a…`. Live `BluetoothRow`
cards and scan/refresh/back hit-test stay. Do not tap **Сопрячь**.
Leave with **Назад**. Wi-Fi list, trusted clients, DevSurface, lock
unlock dots, and Space detail are not this slice.

## Нумерация

После ADR-144 следующий свободный номер — **145**. Не S33.

## Контекст

VUI-07 remaining `draw_action_row_list` consumers after pairing.
ADR-130 already lists live scan rows; `draw_action_row_list` still
fills a Surface strip and paints «Bluetooth устройства» at
`header.y+140`. Inbox/Spaces/Me already use `draw_context_row_list`.
This list is a modal overlay, not a tab — no tab bar. Do not invent
a lifecycle for scan text. Do not pair a device.

## Decision

1. **`Frame::BluetoothList` carries `ContextHeader`**. Section title
   is `Bluetooth`. Space name is the live selected space. No invented
   lifecycle — scan/empty/paired facts stay on the live rows.
2. **`draw_context_row_list` paints it** with `paint_navigation =
   false` and a full-frame `content_rect` so the heading uses the
   same status-layer inset as Me. No Surface header bar.
3. **Rows stay** the live `BluetoothRow` cards plus Искать /
   Обновить / Назад. `bluetooth_list_action_at` is unchanged.

## Consequences

- Bluetooth is a named section, not a diagnostic overlay.
- The old Surface subtitle (`bluetooth_status_summary`) leaves the
  header; empty «Нет устройств» and control cards still name the
  facts.
- Rollback: restore `draw_action_row_list` and the Surface header
  fill.

## Verification

Host: heading is `{space} · Bluetooth`; no Surface bar at y=210.
Panther: Система → Bluetooth opens the live list below the clock.
Only **Назад** is tapped.
