# ADR-309: panther displayd advertises fractional-scale and IME v2

## Статус

Принято, 2026-09-21. APP-02/03 compositor half on the phone-gate.
Not a GTK4 frame. Not Visual v1 sign-off. PIN stays null.

## Нумерация

После ADR-308 следующий свободный номер — **309**. Не S33.

## Контекст

ADR-266/267 grew `wp_fractional_scale_v1`, `wp_viewporter`, and
`zwp_input_method_manager_v2` on host. Panther still ran
`a54ef57c…` without those globals. Shell `4dc19018…` logged IME
unavailable. APP-02 still needed a GTK4 frame on Alpine 4.14.4
musl, the toolkit that SIGSEGV'd in ADR-025.

## Decision

1. **Flash displayd only.** Replace
   `/data/saaios/system/saai-displayd` with `02c78f9f…`. Kill pid
   3240. native-init respawns displayd, which respawns shell.
   dest-no-lock is touched first. No reboot.
2. **IME is optional on the old binary and required on this one.**
   The new shell binds the global when present.
3. **GTK4 4.14.4 is not a frame.** Packed Alpine `gtk4-demo
   --run=dialog` (GSK cairo) connects (`client connected pid=7026`)
   then SIGSEGV before `xdg_toplevel`. APP-02 toolkit half stays
   open. Do not call this Visual v1.

## Consequences

- Foreign OSK can map once a third-party field Activates. That
  typing pass is not this slice.
- Rollback: `saai-displayd.pre-ime`.
- Host GTK 4.18 glibc (ADR-305) is not this musl 4.14.4 crash.

## Verification

PUT `saai-displayd`
`02c78f9f96771abfd4b889004921b695c6e2998171293f853cc32357b44e9c51`.
Respawn pid 6886, shell 6892. No
`zwp_input_method_manager_v2 unavailable` line. `wayland-1` under
`/run/wayland`. gtk4-demo 4.14.4 SIGSEGV; no new `xdg_toplevel`.
Screencap `n.bmp` 1080×2400 still Дом · Сейчас. dest-no-lock kept.
Leave Сейчас.
