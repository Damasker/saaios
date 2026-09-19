# ADR-152: VUI-07 DevSurface — ContextHeader, not Surface bar

## Статус

Принято, 2026-09-19. Hidden HIA-20 «Диагностика» sits under a real
`ContextHeader` instead of `draw_action_row_list`'s Surface header
fill. Live Static `DataRow` cards and the 7-tap gesture stay. Leave
with **Назад**. Space detail is not this slice. This is the last
`draw_action_row_list` consumer.

## Нумерация

После ADR-151 следующий свободный номер — **152**. Не S33.

## Контекст

Trusted / Wi-Fi / Bluetooth already use `draw_context_row_list`.
DevSurface is the remaining Surface strip: title «Диагностика» at
`header.y+140` and `N показателей` at `+200` under a 260px fill.
ADR-136 already split facts into Static `DataRow`. Do not invent a
lifecycle. Do not change values. 7-tap remains silent until threshold.

## Decision

1. **`Frame::DevSurface` carries `ContextHeader`**. Section title is
   `Диагностика`. Space name is the live selected space.
2. **`draw_context_row_list` paints it** with `paint_navigation =
   false` and a full-frame `content_rect`. No Surface header bar.
3. **Rows stay** the live diagnostic cards plus Назад.
   `dev_surface_back_tapped` is unchanged. The old Surface subtitle
   (`N показателей`) leaves the header; the count test stays.

## Consequences

- Diagnostics is a named section, not a leftover overlay chrome.
- `draw_action_row_list` has no remaining caller.
- Rollback: restore the Surface header fill and status line.

## Verification

Host: heading is `{space} · Диагностика`; no Surface bar at y=210.
Panther: 7-tap build-id on Система, screenshot, only **Назад**.
