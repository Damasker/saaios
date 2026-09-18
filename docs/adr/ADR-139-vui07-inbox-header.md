# ADR-139: VUI-07 Inbox — ContextHeader, not draw_root

## Статус

Принято, 2026-09-19. «Входящие» paints through `ContextHeader`
instead of `draw_root`'s concatenated `context · title` bar.
Host-green; panther `75f1f054…`. Spaces / Me chrome, consent,
remote pairing, PIN keypad, and Space detail are not this slice.

## Нумерация

После ADR-138 следующий свободный номер — **139**. Не S33.

## Контекст

VUI-07 remaining root fallback after the apps grid. Inbox already
lists live `EventRow`s (ADR-127) but still goes through `Frame::Root`:
a Surface strip at y=150 and a centered
`format!("{context} · {tab}")` at y=210. Event rows, empty
«Нет новых задач и уведомлений», and store-offline «Нет связи» stay.
Do not tap a row for the screenshot (that opens Object View). Do not
restyle Spaces / Me / consent / pairing / PIN keypad chrome.

## Decision

1. **`Frame::Inbox`**. Header is `ContextHeader` with section
   `Входящие`. Offline `entityd` sets lifecycle `Нет связи`. Archived
   space still uses `Архив`. Rows stay `inbox_content_cards` /
   `inbox_row_at`.
2. **`draw_inbox`**. Same status-layer inset as `draw_now` /
   `draw_apps_grid`. No concatenated Surface bar. No skeleton tiles.
   Tab bar stays. Orb still draws on this page.
3. **No hit-test change.** A tap on a live row still opens Object
   View. Empty and offline rows stay non-actionable.

## Consequences

- Inbox is a real section, not a `draw_root` fallback.
- Spaces / Me still use `draw_root` until their own header slice.
- Rollback: restore `Frame::Root` + `inbox_content_cards` in that
  branch.

## Verification

Host: header section is `Входящие`; offline names `Нет связи`.
Panther: tab `Входящие` opens the list below the clock. No row is
tapped.
