# ADR-425: policy-controlled system widgets on the lock screen

## Status

Accepted, 2026-09-22. Documentation first; implementation and physical
acceptance are separate. Based on ADR-424 / SOM, not the old native branch.

## Decision

The owner's request extends ADR-153: compact system widgets are allowed,
not a dashboard of application cards. Clock and unlock affordance remain
primary. Reuse existing Caption and StatusIndicator, typography and layout.

The initial shell-owned disclosure policy has three stable values:

| `lock_widgets` | Device battery | Generic attention/offline indicator |
|---|---|---|
| `hidden` | No | No |
| `device` | Yes | No |
| `summary` | Yes | Yes |

Missing key preserves existing `summary` behavior. An explicit unknown,
null or incorrectly typed value means `hidden`. No arbitrary strings,
entity titles, message bodies, counts, sender names or actions enter the
widget projection. This is presentation policy, not an authorization grant
and not a replacement for PolicyEngine. Applications cannot supply widgets.

Apply policy before rendering AND before calculating refresh keys. Both
dma-buf and shm consume the same projection. PIN and sleep remain widget-free;
sleep retains only its existing clock. Tap behavior and authentication do
not change. Hidden-source updates must not create extra lock frames.

The existing shell settings file persists this policy; this first slice
loads it at shell startup. No live file watcher or settings UI is claimed.
No new daemon, background polling loop, protocol or dependency is introduced.

## Security and limitations

Generic attention still discloses that something needs attention; `device`
and `hidden` remove that signal. A no-PIN lock is not authentication. This
does not add secure PIN storage or third-party widget isolation. Missing
battery readings are omitted, never shown as zero. No user content is logged.

## Verification and rollback

Test the complete mode matrix, malformed values, missing readings, hidden
refresh keys, PIN and sleeping suppression, and existing shell regressions
on R620 before the implementation commit. Device acceptance remains pending;
do not flash during the current APP-04 experiment. Revert the implementation
commit to restore ADR-153 rendering; older shells ignore the new settings key.

See [LOCK-WIDGETS roadmap](../os/sprints/LOCK-WIDGETS.md).
