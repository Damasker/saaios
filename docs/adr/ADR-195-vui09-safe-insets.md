# ADR-195: VUI-09 — convert logical `SafeInsets` to physical `EdgeInsets`

## Статус

Принято, 2026-09-20. HEAD shell stays `e8865301…`.
`EdgeInsets::from_safe` is the only conversion from logical
`SafeInsets` into the physical `Node` tree. `layout_v2()` uses the
physical bottom inset as the 2400-canvas tab height. Top inset stays
the status layer (ADR-112), not tree padding. `compile()` stays v1.
No new daemon. Leave Сейчас.

## Нумерация

После ADR-194 следующий свободный номер — **195**. Не S33.

## Контекст

`Node` / `EdgeInsets` / `Rect` are physical. `SafeInsets` are logical.
ADR-193 left that mix unnamed as Visual v2 item 2. Applying the top
inset as `Node` padding would double-count the status overlay.
`inset = safe` in markup means the surface supplies the insets; it
does not invent cutout numbers (ADR-182).

## Decision

1. **`EdgeInsets::from_safe(SafeInsets, SurfaceScale)`** converts at
   the surface boundary. `layout()` still takes no scale.
2. **`layout_v2` tab height** scales that physical bottom inset the
   same way `v1_tab_strip_height` scales `root.sui` `height=300`. On
   Pixel 7 portrait those numbers match, so tab hits stay 135/405/
   675/945 y=2250.
3. **Do not pad the tree with the top inset.** Status is a layer.
4. **Do not switch `build.rs` or reflash.**

## Consequences

- Logical and physical insets have one named conversion. Rollback:
  drop `from_safe` and keep v2 tabs on `spec.tab_height`. Next:
  leftover ActionCard sizes, or v2-named tabs. Still not Visual v1
  sign-off.

## Verification

Host: Pixel 7 portrait → 120/300; public NOW `layout_v2` still matches
v1 tabs; ContextHeader stays at y=0. Panther: Сейчас on HEAD; leave
Сейчас.
