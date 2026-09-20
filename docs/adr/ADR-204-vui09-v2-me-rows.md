# ADR-204: VUI-09 — dock `SettingRow` as Me stacked hits

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. `layout_v2()`
places `SettingRow` / `DataRow` on `stacked_row_rect` (same 430 +
n×220 as Inbox/Spaces). `a11y = Button` emits the interned `loc`
(`cycle_timezone` in the public sample). `a11y = Status` and
`SystemSection` occupy the stacked rect and are not actionable. A
screen without `SettingRow` invents no Me hits. `compile()` stays
v1. No new daemon. Open the Система tab, do not tap a row, leave
Сейчас.

## Нумерация

После ADR-203 следующий свободный номер — **204**. Не S33.

## Контекст

ADR-203 named Spaces `SpaceRow` hits. Live Система cards still come
from `me_action_at` over `scrolled_row_rect` (offset `stacked_row_rect`).
`SettingRow` is already a public v2 name. Live dispatch keys are the
interned strings in `intern_me_action` (`cycle_timezone`,
`cycle_brightness`, …). This slice does not invent a new prefix and
does not attach scroll offset to `layout_v2()`. Tapping a live Me row
would cycle a setting, open Wi-Fi, or count toward 7-tap; this slice
does not tap rows. Full Me flatten/scroll stays procedural.

## Decision

1. **Grammar.** `component SettingRow` on `screen me`. Button rows
   get `loc` as the action. Status rows occupy the stacked rect
   without an action. `SystemSection` is always inert. Ids come from
   `loc` when present.
2. **Layout.** Same stacked formula as Inbox/Spaces. Header slot
   fills 0…first row so 540,250 misses. Live `scrolled_row_rect`
   offset is not modelled.
3. **Example.** `docs/os/ui/examples/me-public.sui` names one Button
   row at interned `cycle_timezone`.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   `me_action_at` + `layout_v1_root()`.

## Consequences

- v2 layout owns the public Me row destination. Live flatten, scroll,
  silent 7-tap, PIN, lock, and Orb stay procedural. Rollback: treat
  `SettingRow` as a Fill leaf. Next: Experimental→Stable after Visual
  v1, or operator-approved lock/display/cold-boot. Still not Visual
  v1 sign-off.

## Verification

Host: public Me 540,525 hits `cycle_timezone`; Status /
`SystemSection` miss. Panther: Система tab on HEAD, no row tap;
leave Сейчас.
