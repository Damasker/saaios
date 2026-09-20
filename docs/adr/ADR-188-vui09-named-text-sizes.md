# ADR-188: VUI-09 — name leftover draw_text sizes

## Статус

Принято, 2026-09-20. Draw sizes that already match a `TextRole` on
Pixel 7 go through `role_px()`. The rest are named leftovers, not raw
`NN.0` at the call. Paint stays the same. `compile()` stays `sui 1`.
No new daemon. Do not 7-tap. Leave Сейчас.

## Нумерация

После ADR-187 следующий свободный номер — **188**. Не S33.

## Контекст

ADR-187 closed production RGB. Remaining `draw_text` sizes were still
raw physical pixels (38/27/54/40/…). Mapping ActionCard 38 onto Title
(72) would change list paint. Pixel 7 scale 3 already matches some
literals: Section 54, Label 42, Caption 36.

## Decision

1. **`role_px(role)`** is `physical(role.style().size) as f32`. Lock
   clock, gallery Label heading, `draw_root` Section header, apps-grid
   initial, and status-bar Wi-Fi use it.
2. **Named leftovers** live next to that helper. New draw sizes add a
   name, not a new raw literal.
3. **Do not retoken ActionCard / tabs / status time** in this slice.

## Consequences

- Host test fails if a draw call grows a lone `NN.0` line. Rollback:
  restore the literals. Mapping leftovers onto Title/Body is a later
  paint-normalization ADR.

## Verification

Host: `role_px` goldens; no untracked size lines. Panther: Сейчас
ObjectSummary + footer; tabs still hit; leave Сейчас.
