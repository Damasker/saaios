# ADR-187: VUI-09 — close the color allowlist and leftover NOW chrome

## Статус

Принято, 2026-09-20. Production shell no longer keeps diagnostic
panel-color literals. Leftover `root.sui` NOW cards are gone; live
NOW stays ObjectSummary + footer. `draw_root` reuses `draw_action_card`
instead of a second copy. `compile()` still builds `sui 1`. No new
daemon. Do not tap Inbox rows. Leave Сейчас.

## Нумерация

После ADR-186 следующий свободный номер — **187**. Не S33.

## Контекст

ADR-094 allowed raw panel bytes only in the theme, the backend packer,
and named test sentinels. `LOCK_SCREEN_COLOR` / `SLEEP_INDICATOR_COLOR`
were leftover diagnostic fills. Live NOW no longer paints the two
`root.sui` cards; they were a second, unused chrome next to
ObjectSummary. `draw_root` still inlined the same ActionCardView paint
`draw_action_card` already owns.

## Decision

1. **No production panel-color literals** in `main.rs`. Lock idle and
   sleep already fill `ColorRole::Canvas`. Test sentinels in
   `render.rs` stay. `panel_pixel` remains the only packer.
2. **`root.sui` content is empty.** Tabs stay. Leftover
   `inspect_selected_entity` / `open_intent_input` actions are not a
   rollback golden anymore; `compile_v1_rollback()` still compiles the
   four tabs.
3. **`draw_root` calls `draw_action_card`.** One paint path. Literal
   text sizes in that helper are not rewritten in this slice.

## Consequences

- Ghost NOW hits at the old card rects go away. Rollback: restore the
  two `root.sui` actions and the two color constants.

## Verification

Host: `ROOT_CONTENT_ACTIONS` is empty; tab hits unchanged; no
`LOCK_SCREEN_COLOR` in production `main.rs`. Panther: four tabs on
Сейчас, leave Сейчас.
