# ADR-162: VUI-07 focus order — Field is a stop in the intent tree

## Статус

Принято, 2026-09-19. Intent / Wi-Fi QWERTY puts the compose Field in
the same `Node`/`layout()` tree as the keys and assigns
`focus_order`. First live consumer of ADR-101 metadata. Do not type
a PSK. Do not send. Leave Отмена. Interrupted workflows later.

## Нумерация

После ADR-161 следующий свободный номер — **162**. Не S33.

## Контекст

ADR-101 added `Node::focus_order`; nothing on a live screen set it.
ADR-161 parked the Field above the keys with a parallel
`intent_field_rect`, so hit-test and focus did not share one tree.
Component library: the same rectangle tree drives rendering, hit
testing, and focus order.

## Decision

1. **`intent_view` is chrome + Field + keyboard.** Field is a padded
   leaf (`intent-field`) with `focus_order = 0`. Keys then controls
   are `1…N`. Chrome (`intent-header`) is not a stop.
2. **`intent_field_rect` reads that leaf.** No second geometry.
3. **Walk `focus_order`** for tests. No focus ring this slice.

## Consequences

- Visual stays Field-on-keys (ADR-161). Metadata is now in the tree.
- Rollback: two-child view + computed `intent_field_rect`.

## Verification

Host: first stop is the Field, last is Отправить, Field bottom meets
keyboard. Panther: NOW → Новое намерение, screenshot, Отмена.
