# ADR-250: windowed displayd on the x86 surface

## Статус

Принято, 2026-09-20. Host displayd. Not Visual v1 sign-off.
PIN stays null. Panther chrome unchanged this slice.

## Нумерация

После ADR-249 следующий свободный номер — **250**. Не S33.

## Контекст

PCE-25 identity (ADR-249) made the laptop a distinct node. The
compositor still advertised `model=panther` and sized every
xdg_toplevel to the output, i.e. a phone panel. That substitutes
panther with the host. The laptop surface is a window, not a second
Pixel 7.

Pointer/HID and logical shell layout stay later D slices. This ADR
does not add winit, Resource Scheduler, or PCE-01..24.

## Decision

1. **Phone-gate path unchanged.** `panther-hardware` still configures
   toplevels to the DRM panel size.
2. **x86 path is windowed.** Without that feature, configure is
   1280×800 and `fullscreen=false`, smaller than the 1920×1080 host
   output.
3. **Output model** is `panther` only on the phone-gate build, else
   `x86`. Same Space/Entity/Intent/Observation; this is a surface,
   not a new world model.

## Consequences

S02 demo clients still commit 800×480 test patterns; hashes stay
client-buffer based. Next D slice is pointer + USB HID on this
windowed compositor, not a panther flash. Rollback: revert
`saai-displayd`.

## Verification

Host: `cargo test -p saai-displayd --offline --
x86_toplevel_is_windowed_not_fullscreen
panther_toplevel_stays_panel_fullscreen`.
Panther: no flash. Marker on. Leave Сейчас.
