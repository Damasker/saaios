# ADR-109: dev-only toggle to skip the session lock during active development

## Status

Accepted, 2026-09-17. Physically verified on Pixel 7. Currently enabled on
the development device (marker present).

## Context

Explicit user request: a switch to disable the screen lock so it stops
interfering with development. This session's own history is the concrete
motivation -- every shell binary hot-swap this session (ADR-096 through
ADR-108) restarts the shell, which always re-locks (`Shell::new()`'s
`locked: true`, boot-time `session_lock_state.lock(&qh)` call), requiring
the device owner to unlock by hand before the next screenshot or test could
proceed. This happened well over a dozen times across the session.

## Decision

Added the same volatile, developer-only gate `UI_CALIBRATION_MARKER`/
`UI_GALLERY_MARKER` already use: `SAAIOS_DEV_NO_LOCK` environment variable
or a `/run/saaios/dev-no-lock` marker file, checked once at startup via the
same `calibration_requested()` helper. `/run` is tmpfs -- the marker cannot
survive a reboot and cannot become a persistent setting, matching
`UI_CALIBRATION_MARKER`'s own established "can never become a persistent
user setting" property.

When set:
- The boot-time `session_lock_state.lock(&qh)` call is skipped entirely;
  `Shell.locked` starts `false` instead of `true`.
- `check_idle_timeout` returns immediately, before checking
  `self.settings.idle_timeout_secs` -- so an unlocked dev session does not
  silently re-lock itself later after real inactivity either.

Deliberately does not touch `ShellSettings` (PIN code, idle timeout
duration) at all -- a device with this marker present still has its real
PIN configured exactly as before; the marker only stops the shell from
*acting* on locking during this run, it does not weaken or remove the PIN
itself. Removing the marker and restarting the shell (or a real reboot,
which clears `/run` unconditionally) restores normal locking immediately.

## Verification

- `cargo test -p saai-shell`: 91/91, unchanged.
- `cargo clippy -p saai-shell --all-targets -- -D warnings`: clean.
- Hot-swapped onto the live device (backup preserved as
  `saai-shell.pre-nolock`; a transient SSH connection drop to the build
  server occurred mid-deploy -- verified via `sha256sum` before proceeding
  that no half-applied state existed, then completed the swap cleanly).
  Marker set, shell restarted: real device screenshot confirms the shell
  now boots directly to the normal unlocked `Сейчас` home screen (app grid,
  navigation tabs, status bar all visible) with no lock surface and no
  manual unlock step -- the first time this session a shell restart did not
  require asking the device owner to unlock by hand.

## Consequences

- Every subsequent shell hot-swap this session can be screenshot-verified
  immediately after restart, without an unlock round-trip.
- This is left enabled (marker present) on the development device as of
  this ADR, per the request that motivated it. It must be removed (`rm
  /run/saaios/dev-no-lock`, or simply reboot) before the device is treated
  as representing normal locked behavior again -- any future physical
  review of lock/unlock flows themselves must first confirm this marker is
  absent.
- Not a new security boundary or a weakening of one: the real PIN and idle
  timeout configuration are untouched, and the marker cannot outlive a
  reboot. The risk this ADR accepts is narrower and explicit: a shell
  restart during a deliberate development session no longer re-arms the
  lock, which is exactly what was asked for.

## Rollback

`saai-shell.pre-nolock` remains on-device. Removing `/run/saaios/dev-no-lock`
and restarting the shell (or a reboot) restores normal locked-boot and
idle-timeout behavior immediately -- no data or settings to migrate back,
since none were changed.

## Links

- ADR-094/VUI-01 -- the calibration fixture whose exact gating pattern this
  reuses.
- ADR-105 -- the gallery fixture, same pattern, same reason this session
  needed a third instance of it.
