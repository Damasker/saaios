# ADR-323: empty `QT_IM_MODULE` blocks Qt text-input-v2

## Статус

Принято, 2026-09-21. APP-04 Qt toolkit on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-322 следующий свободный номер — **323**. Не S33.

## Контекст

ADR-318: panther PCManFM Filter did not Activate OSK. Compositor was
v3-only; Qt 5.15 only speaks v2 (ADR-319). `launch.c` also set
`QT_IM_MODULE="" ` so Qt would skip ibus. ibus was later deleted from
the package.

Host displayd now advertises v2. Packed aarch64 `pcmanfm-qt` under
`qemu-aarch64-static` + `dbus-run-session`:

- With `QT_IM_MODULE=""`: shm frame, **no** `get_text_input`.
- With `QT_IM_MODULE` unset: bind `zwp_text_input_manager_v2` (not
  v3), `get_text_input`, `enter`, **`enable(surface)`**.

Empty string is not "use Wayland IM". It disables the module.

## Decision

1. **`unsetenv("QT_IM_MODULE")` in `apps/pcmanfm-demo/launch.c`.**
   Keep `QT_QPA_PLATFORMTHEME=""`. ibus plugin stays deleted.
2. **Host test `pcmanfm_qt_binds_text_input_v2_when_im_module_is_unset`.**
   Requires v2 get + enable. `PCMANFM_PACKAGE_DIR` or the packed
   dist tree.
3. **Do not flash panther.** Next displayd experiment still carries
   ADR-311 bounds + ADR-319 v2. This env fix must ship in the
   package before that flash, or Qt will still not `enable`.

## Consequences

- Qt 5.15 on a v2 compositor does the APP-04 protocol. Panther typed
  Filter still waits that flash (touch-only seat unproven).
- Rollback: restore empty `QT_IM_MODULE`.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
