# ADR-320: Alpine GTK 4.14.4 frames when `configure_bounds` is the window

## Статус

Принято, 2026-09-21. APP-02 host qemu. Same 4.14.4 musl `gtk4-demo`
that crashed on panther (ADR-310). Not a panther flash. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-319 следующий свободный номер — **320**. Не S33.

## Контекст

ADR-310: panther displayd `02c78f9f…` sent `preferred_scale(120)` then
`configure_bounds(0, 0)`. GDK 4.14.4 `gdk_toplevel_size_init(0, 0)`
asked `create_buffer(508, 2337935)` and SIGSEGV'd. ADR-311 set host
displayd bounds to the window (1280×800). Host GTK 4.18 already framed
(ADR-305) because 4.18 ignores 0×0. 4.14.4 against non-zero bounds was
unproven. This week's compositor flash is spent (ADR-309).

## Decision

1. **Run the panther `gtk4-demo` 4.14.4 under `qemu-aarch64-static`
   against host displayd.** `GSK_RENDERER=cairo`, packed musl loader
   and `XKB_CONFIG_ROOT`. `--run=dialog`.
2. **It maps `1280x800 fullscreen=false` and commits a hashed
   `wl_shm` frame.** No `2337935` / `1776831` in the compositor log.
   Timeout-kill of the demo is not a crash: the dialog stayed up.
3. **Do not flash panther.** Next displayd experiment (not this week)
   must carry ADR-311 bounds **and** ADR-319 text-input-v2. APP-02 on
   the phone-gate stays open until that flash.

## Consequences

- ADR-311 is the remaining compositor cause of the ADR-025/310 crash,
  not an unproven hypothesis. GTK 4.14.4 aarch64 musl can frame when
  bounds are the window.
- Host GTK 4.18 (ADR-305) is a different toolkit; this slice is the
  panther binary.
- Rollback: drop the qemu test; bounds stay.

## Verification

Host `cargo test -p saai-displayd --test gtk4_alpine`. Probe at
`/tmp/gtk4-probe` (`GTK4_ALPINE_PROBE`). dest-no-lock kept. Do not
flash. Leave Сейчас.
