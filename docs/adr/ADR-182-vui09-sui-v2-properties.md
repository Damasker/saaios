# ADR-182: VUI-09 — `sui 2` component property grammar

## Статус

Принято, 2026-09-20. `component {}` may name proven tokens and
metadata: `text`, `color`, `spacing`, `inset`, `scroll`, `loc`,
`focus`, `a11y`. Values are `saai-ui-core` enum names, `safe` insets,
`region` scroll, a loc key, or a focus index. `compile()` still
builds only `sui 1` `root.sui`. No paint change, no new daemon. Do
not tap Inbox rows. Leave Сейчас.

## Нумерация

После ADR-181 следующий свободный номер — **182**. Не S33.

## Контекст

ADR-181 parsed empty `{}` so a later slice could add properties
without a silent skip. VUI-09 still needs semantic roles, token
references, safe insets, list/scroll, localization, focus, and
accessibility on that grammar. The values must be types already on
the device, not a second palette.

## Decision

1. **Keys.** `text` (`TextRole`), `color` (`ColorRole`), `spacing`
   (`SpacingToken`), `inset` (`none`/`safe`), `scroll`
   (`none`/`region`), `loc` (ident or string), `focus` (number),
   `a11y` (`AccessibilityRole`). Duplicate or unknown keys fail.
2. **`safe` inset** means the surface supplies `SafeInsets`; markup
   does not invent cutout numbers. `scroll = region` names a
   scrollable list region; it does not move hit-testing yet.
3. **Empty `{}` stays valid.** Shell chrome still compiles through
   `compile()`. This slice does not emit layout.

## Consequences

- A later renderer can read `SuiV2Props` without a syntax break.
  Rollback: require empty `{}` again.

## Verification

Host: a `now` screen with `text`/`a11y`/`loc`/`focus`/`inset`
compiles; unknown `role` and unknown `TextRole` fail; `compile()`
still rejects `sui 2`. Panther: four v1 tabs on Сейчас, leave
Сейчас. No flash — the running binary is unchanged.
