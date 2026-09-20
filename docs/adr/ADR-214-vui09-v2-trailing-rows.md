# ADR-214: VUI-09 — dock list trailing rows in the sui 2 grammar

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. Nested `row
refresh` / `row scan` / `row back` belong on a `sui 2` list screen.
`layout_v2()` docks them with the live `stacked_trailing_rect`
formula and emits `list_refresh` / `list_scan` / `list_back`. Empty
screens invent no trailing hits. Footer `apps`/`intent` stay NOW.
Me flatten/scroll stays a later slice. `compile()` stays v1. No new
daemon. Leave Сейчас.

## Нумерация

После ADR-213 следующий свободный номер — **214**. Не S33.

## Контекст

ADR-205/206/207 named stacked list hits. Live Wi-Fi still has
Обновить/Назад, Bluetooth Искать/Обновить/Назад, trusted Назад —
procedural `stacked_trailing_rect`. `layout_v2()` treated every
`row` as a NOW footer, so those controls had no v2 destination.
Thin tuning stays last (ADR-213).

## Decision

1. **Grammar.** Closed trailing ids: `refresh` → `list_refresh`,
   `scan` → `list_scan`, `back` → `list_back`. Unknown or duplicate
   ids still fail. `apps`/`intent` stay footer.
2. **Layout.** Trailing rows dock after stacked data with the live
   control/trailing formula. Stacked data that would overlap the
   first trailing control is not hittable. Footer height is not used.
3. **Examples.** `wifi-public.sui` lists refresh then back;
   `bluetooth-public.sui` scan/refresh/back;
   `trusted-privileged.sui` back. `compile_v2_public` still rejects
   trusted.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   `wifi_list_action_at` / `bluetooth_list_action_at` /
   `trusted_client_action_at`.

## Consequences

- v2 layout owns list trailing hits. Me flatten/scroll stays next.
  Rollback: drop trailing ids. Still not Visual v1 sign-off.

## Verification

Host: public Wi-Fi 540,745 `list_refresh` and 540,965 `list_back`;
overflow docks back on-screen; a screen without trailing invents
none. Panther: chrome unchanged; leave Сейчас.
