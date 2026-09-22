# Lock-screen system widgets

## Goal

Show useful device state on the lock screen only within an explicit
disclosure policy, without exposing personal content or bypassing unlock.

## Current state

SOM already renders clock, battery Caption and generic attention. ADR-425
adds a shared policy contract, not a new toolkit or an application dashboard.
Base: ADR-424. Keep current known-working phone binaries; no flash here.

## Sprint LW-01 — bounded policy projection

Status: Ready. Dependencies: ADR-153, ADR-154, ADR-425.

1. Commit this contract and roadmap independently of code.
2. Persist `lock_widgets` (`hidden`, `device`, `summary`); unknown values hide.
3. Use one typed, policy-filtered projection for render and refresh keys.
4. Reuse existing widgets on both rendering backends; do not alter PIN/tap.
5. Run host tests, formatting and shell lint on R620; commit implementation.

Acceptance: all modes and malformed settings tested; denied content cannot
reach render or change refresh keys; no widgets in PIN/sleep; missing battery
omits widget. Host success moves to Verify, not Done.

## Sprint LW-02 — owner controls and physical acceptance

Status: Backlog. Add an unlocked System settings selector, immediate policy
re-evaluation and persistence tests. Test actual Pixel typography at 100/150%,
wake/unlock, settings restart, cold reboot, both render paths where available,
and rollback during a coordinated device-testing window. No PIN enrollment
is required or authorized by this plan. Record binary hash and owner feedback.

## Sprint LW-03 — extensibility only after policy review

Status: Backlog. Define provider identity, per-widget classifications,
freshness, bounded slots and capabilities before accepting app providers.
Require explicit unlock for details/actions; test revocation and stale data.
Do not ship arbitrary provider text using the initial trusted-system path.

## Threat / privacy impact

Generic attention reveals existence, not content. `hidden` suppresses even
that signal. This is disclosure control, not a second authorization engine.

## Telemetry without user content

Use deterministic host fixtures and test outcomes; do not log entity payloads.

## Rollback

Revert only the widget implementation commit; keep the pre-change phone
artifact untouched. No partitions or device data are modified in LW-01.

## Evidence / known limitations

Physical acceptance pending. No third-party widgets, drag layout, live settings
reload or full notification content. Host verification recorded with code.
