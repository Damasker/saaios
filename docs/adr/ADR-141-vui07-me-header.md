# ADR-141: VUI-07 Система — ContextHeader, not draw_root

## Статус

Принято, 2026-09-19. «Система» paints through `ContextHeader`
instead of `draw_root`'s concatenated `context · title` bar.
Host-green; panther hash follows flash. Consent, remote pairing,
PIN keypad chrome, and Space detail are not this slice.

## Нумерация

После ADR-140 следующий свободный номер — **141**. Не S33.

## Контекст

VUI-07 last root-tab fallback after Spaces. `Система` already groups
live `SettingRow` / `CapabilityRow` facts (ADR-126) but still goes
through `Frame::Root`: a Surface strip at y=150 and a centered
`format!("{context} · {tab}")` at y=210. Drag-to-scroll
(`scroll_content_only`) must not repaint the tab bar. Do not tap
build-id seven times, Wi-Fi, Bluetooth, PIN, or trusted clients for
the screenshot. Consent / pairing / PIN keypad chrome stay later.

## Decision

1. **`Frame::Me`**. Header is `ContextHeader` with section `Система`.
   Offline `entityd` sets lifecycle `Нет связи`. Archived selected
   space still uses `Архив`. Rows stay `me_content_cards` /
   scrolled hit-test.
2. **`draw_context_row_list` gains `paint_navigation`**. Full frames
   paint the tab bar. Scroll frames clip to content and leave the
   strip untouched — same contract `draw_root` already had for Me.
3. **`Frame::Root` remains** only for any leftover non-tab page that
   still has `ROOT_CONTENT_ACTIONS`. No root tab uses it after this
   slice.

## Consequences

- All four root tabs use `ContextHeader`.
- `draw_root` stays until those leftover actions and gallery paths
  are gone.
- Rollback: restore `Frame::Root` + `me_content_cards` in that branch.

## Verification

Host: header section is `Система`; a scroll frame does not paint the
tab strip. Panther: tab `Система` opens the live list below the
clock. No privileged row is tapped.
