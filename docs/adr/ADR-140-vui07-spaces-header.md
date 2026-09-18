# ADR-140: VUI-07 Spaces list — ContextHeader, not draw_root

## Статус

Принято, 2026-09-19. «Пространства» paints through `ContextHeader`
instead of `draw_root`'s concatenated `context · title` bar.
Host-green; panther `464c0e92…`. Me chrome, consent, remote
pairing, PIN keypad, and Space detail are not this slice.

## Нумерация

После ADR-139 следующий свободный номер — **140**. Не S33.

## Контекст

VUI-07 remaining root fallback after Inbox. The Пространства tab
already lists live `SpaceRow`s (ADR-128) but still goes through
`Frame::Root`: a Surface strip at y=150 and a centered
`format!("{context} · {tab}")` at y=210. Object counts, empty
«Нет пространств», and store-offline «Нет связи» stay. Do not tap a
row for the screenshot (select / retap still cycles lifecycle). Do
not restyle Me / consent / pairing / PIN keypad chrome. Space detail
stays deferred.

## Decision

1. **`Frame::Spaces`**. Header is `ContextHeader` with section
   `Пространства`. Offline `entityd` sets lifecycle `Нет связи`.
   Archived selected space still uses `Архив`. Rows stay
   `spaces_content_cards` / `space_row_at`.
2. **`draw_context_row_list`**. Shared with Inbox (ADR-139): same
   status-layer inset, no concatenated Surface bar, no skeleton
   tiles. Tab bar stays. Orb still draws on this page.
3. **No hit-test change.** Tap still selects; retap still cycles
   lifecycle. Empty and offline rows stay non-actionable.

## Consequences

- Spaces is a real section, not a `draw_root` fallback.
- Me still uses `draw_root` until its own header slice.
- Rollback: restore `Frame::Root` + `spaces_content_cards` in that
  branch.

## Verification

Host: header section is `Пространства`; offline names `Нет связи`.
Panther: tab `Пространства` opens the live list below the clock. No
row is tapped.
