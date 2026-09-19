# ADR-165: VUI-07 closeout — dock list Назад, drop dead painters

## Статус

Принято, 2026-09-19. Wi-Fi / Bluetooth / trusted Назад docks on-screen
the same way DevSurface does. Dead Surface-list painters go. Remaining
exceptions are recorded. Do not tap Сопрячь, Отозвать, or a network.
Leave Назад.

## Нумерация

После ADR-164 следующий свободный номер — **165**. Не S33.

## Контекст

Intent / Wi-Fi password / PIN setup already park Fields, set
`focus_order`, and commit only same-target keys. DevSurface docks
Назад. The three live lists still placed Назад with
`stacked_row_rect`, so a long scan put it below the fold.
`draw_row_list` / `draw_action_row_list` / `diagnostic_status_line`
had no live callers.

## Decision

1. **`stacked_trailing_rect`** docks the trailing control cluster so
   the last row (Назад) stays on-screen. Hit-test those controls
   before data rows.
2. **Delete** `draw_row_list`, `draw_action_row_list`,
   `diagnostic_status_line`.
3. **Exceptions** (not this slice): lock PIN Field under
   `INTENT_HEADER_HEIGHT`; Space detail deferred; MEM-08 omitted;
   `Frame::Root`/`draw_root` kept as test fallback; list/modal frames
   have no `focus_order`; consent/object/pair fire on up without
   `committed_action`; no live `waiting_confirmation` on panther.

## Consequences

- Long Wi-Fi/BT/trusted lists keep Назад reachable. Data rows may sit
  under the docked cluster until a later scroll (DevSurface already
  scrolls).
- Rollback: `stacked_row_rect` for trailing controls; restore the
  three deleted painters from this commit.

## Verification

Host: 20 Wi-Fi rows, Back bottom ≤ height and hit-tests Back.
Panther: open Wi-Fi or Bluetooth, screenshot Назад on-screen, leave
Назад. Do not tap Сопрячь or a network.
