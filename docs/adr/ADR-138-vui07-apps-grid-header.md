# ADR-138: VUI-07 apps grid — ContextHeader, live apps only

## Статус

Принято, 2026-09-18. «Приложения» paints through `ContextHeader`
instead of `draw_root`'s concatenated `context · title` bar.
Host-green; panther `a57da14a…`. Consent, remote pairing,
PIN keypad chrome, and Space detail are not this slice.

## Нумерация

После ADR-137 следующий свободный номер — **138**. Не S33.

## Контекст

VUI-07 next remaining surface after Object View. Composed `Сейчас`
already uses `ContextHeader`. The app grid is still `Frame::Root`:
a Surface strip at y=150, a centered `format!("{context} · {tab}")`
at y=210, and `now_content_cards` still appends the two old
`root.sui` NOW cards (`inspect_selected_entity`, `open_intent_input`)
into the icon grid even though those actions live on the footer
(ADR-113). Skeleton placeholder rows stay in `draw_root` for other
pages; the grid should not invent tiles. Do not launch an app for
the screenshot (that would open consent). Do not restyle consent,
remote pairing, or PIN keypad chrome.

## Decision

1. **`Frame::AppsGrid`**. Header is `ContextHeader` with section
   `Приложения`. Offline `appd` sets lifecycle `Нет связи`. Archived
   space still uses `Архив`. Apps are only `installed_apps` — no
   leftover inspect/intent cells.
2. **`draw_apps_grid`**. Same status-layer inset as `draw_now`
   (heading below the 120px PIXEL_7 clock). Empty connected is
   `Нет приложений`. Empty offline is `Нет связи`. No skeleton
   tiles. Letter-square icons stay: there is still no icon asset
   pipeline.
3. **`now_action_at` only hits installed apps.** Intent stays on the
   NOW footer. Re-tap `Сейчас` still closes the grid.

## Consequences

- The grid is a real section, not a `draw_root` fallback.
- Inbox / Spaces / Me still use `draw_root` until their own
  remaining chrome is migrated.
- Rollback: restore `Frame::Root` + `now_content_cards` appending
  `ROOT_CONTENT_ACTIONS`.

## Verification

Host: header section is `Приложения`; empty names connected vs
offline; `now_action_at` misses the old inspect/intent cells.
Panther: footer `Приложения` opens the grid below the clock with
live installed apps only. No app is launched.
