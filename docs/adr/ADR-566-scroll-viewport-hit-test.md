# ADR-566: scroll viewport clips hit_test

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-565 следующий свободный номер — **566**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-101 added inset, separator, and focus-order. Scroll region
stayed open: `hit_test` only knew the node's own `rect`, so a row
shifted by a scroll offset could still be hit where its unclipped
content rect sits. `Rect.y` is `u32`, so a scrolled-away row cannot
move to a negative coordinate.

Leftover chrome after ADR-561 is empty. Lock lift is ADR-564.
Displayd DPMS is ADR-565. This is the next VUI-02 leftover, not
ADR-213 thin tuning and not an ADR-232 formula.

## Решение

1. `NodeKind::Scroll { offset }` and `Node::scroll`. The node's
   laid-out `rect` is the viewport.
2. Children layout in content coordinates (heights may exceed the
   viewport). `LayoutNode.scroll_offset` is `Some(offset)`.
3. `hit_test` inside a scroll viewport adds `offset` to `y` before
   walking children. A touch outside the viewport hits nothing.
4. Do not migrate Me / Sistema / list screens this slice.

## Откат

Remove `NodeKind::Scroll` and the `scroll_offset` field.

## Ссылки

- ADR-017, ADR-101
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
