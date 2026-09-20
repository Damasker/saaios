# ADR-252: logical layout — window on x86, panel on panther

## Статус

Принято, 2026-09-20. Host shell geometry. Not Visual v1 sign-off.
PIN stays null. Panther chrome unchanged: aarch64 + `/data/saaios`
still uses 1080×2400 fullscreen.

## Нумерация

После ADR-251 следующий свободный номер — **252**. Не S33.

## Контекст

displayd on x86 configures 1280×800 (ADR-250). saai-shell still
forced min-size 1080×2400 and `set_fullscreen`, i.e. a second phone
panel. Logical layout is the shell's canvas for the same
Space/Entity/Intent/Observation, not a new world model.

## Decision

1. **Phone-gate:** 1080×2400 + fullscreen. Detected as aarch64 and
   (`/data/saaios` or Pixel in device-tree). Host tests are not that.
2. **x86:** 1280×800, no fullscreen request. Matches displayd window.
3. **Layout math in tests** may still pass explicit 1080×2400; that is
   the panther reference canvas, not the laptop window.

## Consequences

Laptop proof is this geometry, not a panther flash. Next D remainder
is shared Space/Entity/Intent/Observation across nodes without
PCE-01..24. Rollback: revert the shell size helpers.

## Verification

Host: `cargo test -p saai-shell --offline --
logical_surface_is_windowed_on_x86_and_panel_on_panther`.
Panther: no flash. Marker on. Leave Сейчас.
