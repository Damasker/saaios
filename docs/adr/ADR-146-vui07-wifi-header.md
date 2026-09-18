# ADR-146: VUI-07 Wi-Fi list — ContextHeader, not Surface bar

## Статус

Принято, 2026-09-19. «Wi-Fi сети» sits under a real `ContextHeader`
instead of `draw_action_row_list`'s Surface header fill. Host-green;
panther `d562249b…`. Live `WifiRow` cards and refresh/back
hit-test stay. Do not tap a network. Do not type a PSK. Leave with
**Назад**. Trusted clients, DevSurface, lock unlock dots, and Space
detail are not this slice.

## Нумерация

После ADR-145 следующий свободный номер — **146**. Не S33.

## Контекст

VUI-07 remaining `draw_action_row_list` consumers after Bluetooth.
ADR-129 already lists live scan rows; `draw_action_row_list` still
fills a Surface strip and paints «Wi-Fi сети» at `header.y+140`.
Bluetooth already uses `draw_context_row_list`. This list is a modal
overlay, not a tab — no tab bar. Do not invent a lifecycle for scan
text. Do not open the password keyboard.

## Decision

1. **`Frame::WifiList` carries `ContextHeader`**. Section title is
   `Wi-Fi`. Space name is the live selected space. No invented
   lifecycle — connected/empty/scan facts stay on the live rows.
2. **`draw_context_row_list` paints it** with `paint_navigation =
   false` and a full-frame `content_rect` so the heading uses the
   same status-layer inset as Bluetooth. No Surface header bar.
3. **Rows stay** the live `WifiRow` cards plus Обновить / Назад.
   `wifi_list_action_at` is unchanged.

## Consequences

- Wi-Fi is a named section, not a diagnostic overlay.
- The old Surface subtitle (`wifi_status_line`) leaves the header;
  empty «Нет сетей» and control cards still name the facts.
- Rollback: restore `draw_action_row_list` and the Surface header
  fill.

## Verification

Host: heading is `{space} · Wi-Fi`; no Surface bar at y=210.
Panther: Система → Wi-Fi Сети opens the live list below the clock.
Only **Назад** is tapped.
