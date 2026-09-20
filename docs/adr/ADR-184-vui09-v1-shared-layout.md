# ADR-184: VUI-09 — shared layout/hit-test from compiled `.sui` v1

## Статус

Принято, 2026-09-20. Tab and leftover NOW content-action rectangles
come from one `layout_v1_root()` tree built from `compile()` /
`compile_v1_rollback()`. `build.rs` still calls `compile()`, not
`compile_v2()`. Live NOW paint stays ObjectSummary + footer; leftover
`inspect_selected_entity` still has no tap. No new daemon. Do not tap
Inbox rows. Leave Сейчас.

## Нумерация

После ADR-183 следующий свободный номер — **184**. Не S33.

## Контекст

Tabs already used a `Node` tree. Content actions still used a second
scaled-rect table (`content_action_rect`). VUI-09 asked for layout and
hit-test from compiled output without pointing `root.sui` at
`compile_v2()`. `SuiV2Screen` still has no rectangles.

## Decision

1. **`layout_v1_root(&ScreenSpec, width, height)`** in
   `saai-ui-compiler` builds the same vertical chrome the shell used:
   content column, then a tab strip whose height is
   `tab_height * panel / 2400`, never below `MIN_TOUCH_TARGET`.
2. **Content actions are children of that tree.** Same `top`/`height`
   scale and `width/22` margin as `content_action_rect`. Hit-test is
   `LayoutNode::hit_test`. Page filter stays on the caller so Inbox
   does not inherit leftover NOW cards.
3. **Shell `root_view` calls that function** with
   `compile_v1_rollback()`. `tab_at` and `content_action_at` share the
   tree. `compile_v2()` stays off `build.rs`.

## Consequences

- A later v2 emitter can replace this only after it produces the same
  tab hits at 1080×2400 (135/405/675/945, y=2250). Rollback: keep
  `compile()` on `root.sui`.

## Verification

Host: `layout_v1_root(compile_v1_rollback())` maps the four tabs and
the two leftover NOW actions; Inbox at the same points is none;
`build.rs` still contains `saai_ui_compiler::compile(`. Panther: four
tabs still switch; leave Сейчас.
