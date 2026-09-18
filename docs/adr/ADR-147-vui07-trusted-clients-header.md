# ADR-147: VUI-07 trusted clients — ContextHeader, not Surface bar

## Статус

Принято, 2026-09-19. «Доверенные клиенты» sits under a real
`ContextHeader` instead of `draw_action_row_list`'s Surface header
fill. Host-green; panther `b73f9433…`. Live `TrustedClientRow`
cards and back hit-test stay. Do not tap **Отозвать**. Leave with
**Назад**. DevSurface, lock unlock dots, and Space detail are not
this slice.

## Нумерация

После ADR-146 следующий свободный номер — **147**. Не S33.

## Контекст

VUI-07 remaining `draw_action_row_list` consumers after Wi-Fi.
ADR-131 already lists live key rows; `draw_action_row_list` still
fills a Surface strip and paints «Доверенные клиенты» at
`header.y+140`. Wi-Fi/Bluetooth already use `draw_context_row_list`.
This list is a modal overlay, not a tab — no tab bar. Do not invent
a lifecycle for the key count. Do not revoke a key.

## Decision

1. **`Frame::TrustedClients` carries `ContextHeader`**. Section title
   is `Ключи`. Space name is the live selected space. No invented
   lifecycle — name/fingerprint/empty facts stay on the live rows.
2. **`draw_context_row_list` paints it** with `paint_navigation =
   false` and a full-frame `content_rect` so the heading uses the
   same status-layer inset as Wi-Fi. No Surface header bar.
3. **Rows stay** the live `TrustedClientRow` cards plus Назад.
   `trusted_client_action_at` is unchanged.

## Consequences

- Trusted clients is a named section, not a diagnostic overlay.
- The old Surface subtitle (`N доверенных ключей`) leaves the
  header; empty «Нет клиентов» and live name rows still name the
  facts.
- Rollback: restore `draw_action_row_list` and the Surface header
  fill.

## Verification

Host: heading is `{space} · Ключи`; no Surface bar at y=210.
Panther: Система → Доверенные клиенты opens the live list below
the clock. Only **Назад** is tapped.
