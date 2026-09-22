# ADR-227: VUI-09 — Me paint reads `layout_v2_scrolled()`

## Статус

Принято, 2026-09-20. List paint already reads generated `layout_v2()`
(ADR-226). Live «Я» row cards now take rects from the same
`compile_v2()` / `layout_v2_scrolled()` tree hit-test already uses
(ADR-219). Clipped rows stay omitted, not invented Buttons. Apps
grid, overlays, OrbHost, and diagnostic paint stay later slices. PIN
stays null. Do not open «Я». Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-226 следующий свободный номер — **227**. Не S33.

## Контекст

Hits for Me already generate `me_v2_source` and shift stacked
`SettingRow`s with `layout_v2_scrolled`. Paint still placed cards on
`scrolled_row_rect`, a second formula. Tabs came from `root_view`
even though the live document names `BottomNavigation`.

## Decision

1. **One scrolled tree.** Paint builds `layout_v2_scrolled` from
   `me_v2_source` at the same clamped offset hit-test uses.
2. **Row ids.** Dispatch rows look up their loc (`cycle_timezone`,
   `open_wifi_list`, …). Quiet section headers use `me.quiet.{index}`.
   A loc missing after clip is off-screen and is not painted.
3. **Tabs.** Tab rects come from that document's `BottomNavigation`,
   same helper NOW and lists use.
4. **No invented Buttons.** Section titles stay Status rows.

## Consequences

- Me paint and hit-test share the generated scrolled tree. Apps /
  overlay paint stay procedural until their slices. Rollback: restore
  `scrolled_row_rect` in the card builder. Still not Visual v1
  sign-off.

## Verification

Host: fixture `cycle_timezone` rect matches `layout_v1_find` at offset
0 and after a drag; a clipped last row is absent. Panther: flash;
Сейчас; do not open «Я»; leave Сейчас.
