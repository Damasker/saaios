# ADR-311: `xdg_toplevel.configure_bounds` is the window, not `(0, 0)`

## Статус

Принято, 2026-09-21. APP-02 compositor remaining after ADR-310.
Host protocol. Not a panther flash. Not a GTK4 frame. Visual v1
stays unsigned. PIN stays null.

## Нумерация

После ADR-310 следующий свободный номер — **311**. Не S33.

## Контекст

ADR-310's `WAYLAND_DEBUG` on panther showed
`preferred_scale(120)` then `configure_bounds(0, 0)` then
`set_min_size(508, 2337935)` / `create_buffer(508, 2337935)`.
Smithay 0.7 `send_toplevel_configure` always emits bounds when
xdg-shell ≥ 4: `bounds.unwrap_or_default()` is `(0, 0)` if
`ToplevelState.bounds` is `None`. GTK 4.14
`xdg_toplevel_configure_bounds` sets `has_bounds = TRUE` even for
zeros, then `gdk_wayland_toplevel_compute_size` calls
`gdk_toplevel_size_init(0, 0)` instead of monitor geometry.
Host GTK 4.18 still frames (ADR-305) because that series does not
treat 0×0 as a real max size.

## Decision

1. **Set bounds to the same size as configure.** `new_toplevel`
   writes `state.bounds = Some(geo.size)` next to `state.size`.
   Host window is 1280×800 (ADR-250). Panther panel is the output.
2. **Host-only this slice.** This week's displayd flash is spent
   (ADR-309). Do not replace panther `02c78f9f…`.
3. **APP-02 stays open** until a GTK 4.14.4 frame on panther, or a
   later refuse ADR. This is the compositor half of ADR-310, not the
   toolkit half.

## Consequences

- GTK 4.14 clients that honor bounds get a finite max size.
- A zero-bounds compositor was a protocol-legal smithay default, not
  a GDK uninitialized `*scale`.
- Rollback: drop `state.bounds`.

## Verification

Host: `configure_bounds_is_windowed_geometry_not_zero` against a
spawned `saai-displayd`. `bounds_are_the_toplevel_geometry_never_zero`.
No panther flash. dest-no-lock kept. Leave Сейчас.
