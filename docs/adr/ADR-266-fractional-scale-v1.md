# ADR-266: `saai-displayd` advertises `wp-fractional-scale-v1` and `wp-viewporter`

## Статус

Принято, 2026-09-20. Host compositor protocol. Not a panther flash. Not
GTK4-on-device acceptance. Visual v1 stays unsigned.

## Нумерация

После ADR-265 следующий свободный номер — **266**. Не S33.

## Контекст

ADR-025 localized GTK4's shm crash to
`gdk_wayland_display_create_shm_surface` with `height=1776831` and an
uninitialized `double *scale`. The plausible mechanism is that GDK only
writes that scale when it receives `wp_fractional_scale_v1.preferred_scale`.
`saai-displayd` did not advertise that global (or the pair `wp_viewporter`).
APP-02 asked to implement the protocol or refuse GTK4. This week's
hardware-changing displayd flash is not spent on panther; host can still
grow the globals.

## Decision

1. **Advertise both globals.** `FractionalScaleManagerState` and
   `ViewporterState` from smithay 0.7. `new_fractional_scale` sets
   preferred scale **1.0** (protocol units 120), matching the existing
   `wl_output` `Scale::Integer(1)`.
2. **Host-only this slice.** Do not flash panther `saai-displayd`. Touch
   recovery is a process restart, not a new binary.
3. **APP-02 stays open** until a real GTK4 frame is verified on panther
   (same method as ADR-026 for Qt) or a later ADR refuses GTK4. This ADR
   is the compositor half, not the toolkit half.

## Consequences

- Clients that bind `wp_fractional_scale_v1` receive `preferred_scale=120`
  instead of hanging on an absent event.
- Viewport dest size is not applied to the DRM blit path yet.
- Rollback: drop the two smithay states and the test.

## Verification

Host: `advertises_fractional_scale_and_viewporter` and
`preferred_scale_is_one_after_bind` against a spawned `saai-displayd`.
No panther flash. Leave Сейчас.
