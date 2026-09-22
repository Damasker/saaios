# ADR-215: VUI-09 — Me flatten/scroll in `layout_v2()`

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. `layout_v2()`
treats `SystemSection` headers and following `SettingRow`s as the
live `flatten_me_rows` list. At scroll offset 0 it clips with the
`scrolled_row_rect` rule: a row that does not fit wholly in the
content pane above tabs is not hittable. The public sample names
the live «Устройство» prefix so `cycle_timezone` sits at flatten
index 4. Inbox/Spaces/list screens stay unclipped stacked hits.
`compile()` stays v1. No new daemon. Do not tap Me rows. Leave
Сейчас.

## Нумерация

После ADR-214 следующий свободный номер — **215**. Не S33.

## Контекст

ADR-204 docked a single `SettingRow` at stacked 0. Live Система
inserts a section header plus readout/silent rows before
`cycle_timezone`, and `me_action_at` uses `scrolled_row_rect`
(offset `stacked_row_rect`) so rows that overflow the tab-clipped
content pane disappear whole. `layout_v2()` still emitted every
stacked leaf. Trailing list rows are host (ADR-214). Thin tuning
stays last (ADR-213).

## Decision

1. **Grammar.** Document order is flatten order. `SystemSection` is
   inert. `SettingRow` Button emits interned `loc`. Status rows
   occupy a slot with no action.
2. **Layout.** When the screen has `BottomNavigation` and at least
   one Me-like name (`SystemSection` / `SettingRow` / `DataRow` /
   `CapabilityRow`), stacked rows that fail the offset-0
   `scrolled_row_rect` test are omitted. Runtime drag offset stays
   in the shell until `compile_v2()` is production.
3. **Example.** `me-public.sui` lists the «Устройство» prefix:
   header, device readout, `tap_build_info`, updates readout,
   `cycle_timezone`.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   `me_action_at` + `layout_v1_root()`.

## Consequences

- v2 layout matches rest-state Me hits for a flatten-shaped
  document. Scroll-while-dragging stays procedural. Rollback:
  emit every stacked leaf. Next: `compile_v2()` only after this
  host proof. Still not Visual v1 sign-off.

## Verification

Host: public Me 540,1405 hits `cycle_timezone`; 540,525 misses
(header); eight Button rows clip index 7 at 1080×2400. Panther:
unchanged HEAD; no Me row tap; leave Сейчас.
