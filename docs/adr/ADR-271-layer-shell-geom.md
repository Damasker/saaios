# ADR-271: layer-shell size and touch hit the client's surface

## Статус

Принято, 2026-09-21. Host compositor geometry. Not a panther flash.
Not Visual v1 sign-off. Status bar stays a 120px top strip because
that is what it asks for.

## Нумерация

После ADR-270 следующий свободный номер — **271**. Не S33.

## Контекст

APP-04's OSK has to sit over a third-party field. Shell's
xdg_toplevel is not that field. The panel has to be a layer-shell
surface (bottom, keyboard height). `saai-displayd` still forced every
new layer to `output_width × 120` at the top and routed touch only to
`focused_surface` / lock. A keyboard layer would paint as a second
status bar and never receive a tap. ADR-015 already noted layer touch
as a known limitation.

## Decision

1. **Honor client size.** Width/height 0 means the output size on that
   axis. An 800px bottom request stays 800px, not 120.
2. **Hit the topmost layer** whose destination contains the contact,
   then `focused_surface`. Locked still goes only to the lock surface.
3. **Host-only this slice.** Do not flash panther `saai-displayd`.
   Blit offset for a bottom strip is still the full-frame hardware
   blit; panther paint of a bottom OSK waits with the flash.

## Consequences

- Status bar `set_size(0, 120)` still configures 120px.
- Shell can map an OSK layer at the real keyboard height once this
  binary is on device.
- Rollback: force `output_width × 120` again and route touch only to
  `focused_surface`.

## Verification

Host: `status_bar_stays_a_top_strip`, `osk_docks_at_the_bottom`,
`later_layer_wins_when_rects_overlap`, and
`bottom_layer_keeps_requested_height` against a spawned
`saai-displayd`. No panther flash. Leave Сейчас.
