# ADR-127: VUI-07 EventRow — Inbox as decision vs notice

## Статус

Принято, 2026-09-18. Host + panther (`c1547c02…`): typed `EventRow`
and Inbox mapping. Paint stays `ActionCardView`. Inbox tab stays.

## Нумерация

После ADR-126 следующий свободный номер — **127**. Не S33.

## Контекст

VUI-07 starts with remaining system surfaces. Visual Language 9.2 says
`Входящие` is not a primary v1 tab — decisions live on `Сейчас` and the
Object they concern. PIXEL-PATH still keeps the tab; ripping it would
drop ATTN-03 from the product path. The first bounded slice is the row
contract, not Spaces, lock, or a DataRow paint rewrite.

Inbox today already lists the attention projection (`inbox_rows`):
waiting tasks and undismissed notifications. Empty copy is «Нет новых
задач и уведомлений». Store-down currently looks the same as empty
because `selected_entities` is just empty. No timestamps are invented.
There is no `EventRow` type; the gallery still omits it.

## Decision

1. **`EventRow` wraps `DataRow`**, same as `SettingRow`. Two live kinds
   from the existing projection, plus two honest absences:
   - decision: Navigation, value «Ждёт подтверждения», action
     `open_object` (waiting task);
   - notice: Navigation, value = notification `body`, same action;
   - empty (store up, projection empty): Static «Нет новых задач и
     уведомлений», no action;
   - offline (entityd down): Static «Нет связи», no action. Offline
     wins over cached rows. No fabricated time, actor, or object.
     Actor/action/scope stay on `DecisionOverlay` in Object View.
2. **Keep the Inbox tab.** Internal id `inbox` / `RootPage::Inbox`.
   Hit-test still uses `inbox_rows` entity ids; empty and offline rows
   are not tappable. Flatten to `ActionCardView` so `stacked_row_rect`
   and Object View open stay the accepted path. `draw_data_row` remains
   later VUI-07.
3. **Gallery fixtures include both live kinds.** The composite gallery
   page does not grow a new layout slot in this slice.

## Consequences

- Empty and offline are distinguishable. Live rows keep the same
  Russian copy users already see.
- Rollback: revert this commit; Inbox cards return to the ad hoc
  `inbox_content_cards` match.

## Verification

Host: `saai-ui-core` EventRow constructors; `saai-shell` mapping
(decision/notice/empty/offline) and inert tap when the store is down.
Physical Pixel review with the rest of VUI-07.
