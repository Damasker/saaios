# ADR-310: panther `preferred_scale=120` does not fix GTK 4.14.4 shm height

## Статус

Принято, 2026-09-21. APP-02 compositor hypothesis from ADR-025 is
falsified on the phone-gate. Not a GTK4 frame. Not a product refuse of
GTK4. Visual v1 stays unsigned. PIN stays null.

## Нумерация

После ADR-309 следующий свободный номер — **310**. Не S33.

## Контекст

ADR-025 blamed missing `wp_fractional_scale_v1` for
`gdk_wayland_display_create_shm_surface(..., height=1776831, scale=…)`.
ADR-266/309 put the global on panther displayd `02c78f9f…`. Alpine
4.14.4 `gtk4-demo --run=dialog` still SIGSEGV'd. This slice asked
whether GDK actually received `preferred_scale` before the crash.

## Decision

1. **The compositor delivered the event.** `WAYLAND_DEBUG=1` on the
   live socket: bind `wp_fractional_scale_manager_v1` + `wp_viewporter`,
   `get_fractional_scale` / `get_viewport`, then
   `preferred_scale(120)` twice, then `xdg_toplevel.configure(1080, 2400)`.
2. **GDK 4.14.4 still asks for a garbage buffer.** Immediately after
   `ack_configure`: `set_min_size(508, 2337935)`, `set_max_size` the
   same, `wl_shm_pool` 455716624 bytes, `create_buffer(508, 2337935,
   stride=2032)`. Cairo: `gdkdisplay-wayland.c:1494` invalid size.
   SIGSEGV (wait 139). No hashed gtk4 frame. Displayd logged
   `new xdg_toplevel 1080x2400` for pid 7274, then the client died.
3. **Do not flash displayd again for this crash.** More globals will
   not change 4.14.4 aarch64 height. Host GTK 4.18 glibc (ADR-305)
   remains a different toolkit. Next APP-02 choice is patch/newer GTK
   or a later refuse ADR — not compositor protocol.
4. **Do not treat this as Visual v1.** Qt stays the panther app path
   (ADR-026/306). Epiphany waits.

## Consequences

- ADR-025's "uninitialized scale because no preferred_scale" is not
  the remaining mechanism. Scale 1.0 arrived; height is still ~2.3e6.
- `configure_bounds(0, 0)` is in the same trace; it is not proven
  causal. A compositor bounds fix would be another displayd flash.
- Throwaway: `incoming/gtk4-probe` + Alpine xkb. Not a package.

## Verification

unshare-bind `/lib` onto `gtk4-probe/lib`, `XKB_CONFIG_ROOT` from
Alpine xkb, `GSK_RENDERER=cairo`. dest-no-lock kept. displayd 6886,
shell 6892, hashes `02c78f9f…` / `4dc19018…`. Screencap still
Дом · Сейчас. Leave Сейчас.
