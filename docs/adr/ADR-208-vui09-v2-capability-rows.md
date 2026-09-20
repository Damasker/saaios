# ADR-208: VUI-09 — dock `CapabilityRow` as Me app stacked hits

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. `layout_v2()`
places privileged `CapabilityRow` on `stacked_row_rect` (same 430 +
n×220 as Me settings). The row occupies the stacked rect and is
never actionable: live `CapabilityRow` is Static with no revoke
protocol. `a11y = Button` does not invent a dispatch.
`compile_v2_public()` rejects the name. `compile_v2()` marks the
screen privileged. A screen without `CapabilityRow` invents no app
hits. `compile()` stays v1. No new daemon. Do not open Me apps this
slice. Leave Сейчас.

## Нумерация

После ADR-207 следующий свободный номер — **208**. Не S33.

## Контекст

ADR-207 named privileged trusted-client hits. Live Система app
cards still come from `me_system_sections` `CapabilityRow` flattened
through `me_action_at`. `intern_me_action` has no app-revoke key.
This slice does not invent one and does not launch an installed app.
Trailing list controls stay procedural.

## Decision

1. **Grammar.** `component CapabilityRow` on `screen me`. Status is
   the honest a11y. Button still occupies the stacked rect without
   an action. Ids come from `loc` when present.
2. **Layout.** Same stacked formula as Me `SettingRow`. Header slot
   fills 0…first row so 540,250 misses. 540,525 misses because the
   row is not actionable.
3. **Example.** `docs/os/ui/examples/capability-privileged.sui`
   names one Status row at `me.app`. `compile_v2_public()` fails.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   `me_action_at` + `layout_v1_root()`.

## Consequences

- v2 layout owns the privileged Me app row destination. No revoke
  protocol. Rollback: treat `CapabilityRow` as a Fill leaf. Next:
  Experimental→Stable after Visual v1, or operator-approved
  lock/display/cold-boot. Still not Visual v1 sign-off.

## Verification

Host: privileged sample docks `me.app` at stacked_row 0 with no
action; Button still inert; `compile_v2_public` rejects. Panther:
chrome unchanged on HEAD; do not tap Me apps; leave Сейчас.
