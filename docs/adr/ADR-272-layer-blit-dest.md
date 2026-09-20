# ADR-272: layer blit uses the client's destination, not (0,0)

## Статус

Принято, 2026-09-21. Host clip math + panther-hardware blit path.
Not a panther flash. Not Visual v1 sign-off.

## Нумерация

После ADR-271 следующий свободный номер — **272**. Не S33.

## Контекст

ADR-271 sized and hit-tested a bottom OSK layer. Hardware still
blitted every surface at `(0, 0)`. A keyboard strip would cover the
status bar. The GPU helper has no destination offset.

## Decision

1. **CPU blit takes `(dst_x, dst_y)`**, clipped to the panel
   (`layer_geom::blit_window`).
2. **GPU blit stays origin-only.** A non-zero dest skips the helper
   and uses the CPU path. Status bar `(0, 0)` still uses GPU.
3. **Host-only this slice.** Do not flash panther `saai-displayd`.

## Consequences

- A bottom OSK composites at `output_height - keyboard_height` once
  this binary is on device.
- Rollback: `blit` at `(0, 0)` again.

## Verification

Host: `osk_blit_starts_at_the_bottom`, `status_bar_blit_stays_at_the_origin`,
`offscreen_blit_is_skipped`. No panther flash. Leave Сейчас.
