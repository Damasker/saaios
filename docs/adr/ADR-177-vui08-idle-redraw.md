# ADR-177: VUI-08 — idle main surface does not commit

## Статус

Принято, 2026-09-20. `FramePace` counts every main-surface commit as
`seq`. After clocks expire, `idle_ok=1` means the last commit did not
request `wl_surface.frame`. A quiet Сейчас hold must keep `seq`
unchanged. Status-bar ticks stay on the layer surface. No new daemon.
Do not tap Inbox rows. Leave Сейчас.

## Нумерация

После ADR-176 следующий свободный номер — **177**. Не S33.

## Контекст

S04 forbids idle `wl_surface.frame`. ADR-167 already requests a
callback only while a clock needs a frame. Appd's 1 Hz `list()` no
longer forces a redraw when the registry is unchanged. VUI-08
acceptance still needs a live count so a quiet hold is distinguishable
from a hidden commit loop. SHM/Vulkan paths are unchanged.

## Decision

1. **`seq`** saturates on each `record`. It survives ring wrap. The
   last line appends `seq=` and `idle_ok` (`1` when the last sample
   did not request a frame, `0` while a clock still does, `-` if empty).
2. **Idle hold.** After Selection/MicroFeedback expire, two reads of
   `/run/saaios/shell-frame.last` two seconds apart must show the same
   `seq` and `dropped`. Entity/app cache refreshes that compare equal
   do not `draw()`.
3. **Status is not the main surface.** `present_status_bar` may still
   commit the layer once a second. That is not a VUI-08 idle failure.

## Consequences

- A stuck Orb loop or a 1 Hz List redraw shows up as a climbing `seq`.
  Rollback: drop `seq` / `idle_ok`.

## Verification

Host: `seq` increments per record; `idle_ok` follows
`requested_frame`. Panther: Сейчас, wait for `idle_ok=1`, sleep 2 s,
`seq` unchanged, leave Сейчас.
