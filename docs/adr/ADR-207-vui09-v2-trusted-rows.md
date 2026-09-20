# ADR-207: VUI-09 — dock `TrustedClientRow` as trusted-client stacked hits

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. `layout_v2()`
places privileged `TrustedClientRow` on `stacked_row_rect` (same
430 + n×220 as Wi-Fi). `a11y = Button` emits the live
`revoke_trusted_client` action from `TrustedClientRow::open`.
`a11y = Status` is `TrustedClientRow::empty` «Нет клиентов» and is
not actionable. `compile_v2_public()` rejects the name.
`compile_v2()` marks the screen privileged. A screen without
`TrustedClientRow` invents no trusted-client hits. No root tabs:
live «Доверенные клиенты» has trailing Назад, not
`BottomNavigation`. `compile()` stays v1. No new daemon. Do not
open the live list this slice (Отозвать is forbidden). Leave Сейчас.

## Нумерация

После ADR-206 следующий свободный номер — **207**. Не S33.

## Контекст

ADR-206 named Bluetooth `BluetoothRow` hits. Live trusted-client
cards still come from `trusted_client_action_at` over
`stacked_row_rect`. `TrustedClientRow` is a privileged v2 name. Live
row taps are `TrustedClientTap::Revoke(index)` with action
`revoke_trusted_client`. This slice does not invent a fingerprint
and does not model trailing Назад. Tapping a live row would revoke a
key; this slice does not open the list.

## Decision

1. **Grammar.** `component TrustedClientRow` on `screen trusted`.
   Button rows get `revoke_trusted_client`. Status rows occupy the
   stacked rect without an action. Ids come from `loc` when present.
2. **Layout.** Same stacked formula as Wi-Fi. Header slot fills
   0…first row so 540,250 misses. No `BottomNavigation` in the
   privileged sample, so 135,2250 misses.
3. **Example.** `docs/os/ui/examples/trusted-privileged.sui` names
   one Button row at `trusted.item`. `compile_v2_public()` fails.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   `trusted_client_action_at` + `layout_v1_root()`. Trailing Назад
   stays procedural.

## Consequences

- v2 layout owns the privileged trusted-client row destination.
  Назад and revoke stay off the public subset. Rollback: treat
  `TrustedClientRow` as a Fill leaf. Next: Experimental→Stable after
  Visual v1, or operator-approved lock/display/cold-boot. Still not
  Visual v1 sign-off.

## Verification

Host: privileged sample 540,525 hits `revoke_trusted_client`; Status
row misses; `compile_v2_public` rejects. Panther: chrome unchanged
on HEAD; do not open the list; leave Сейчас.
