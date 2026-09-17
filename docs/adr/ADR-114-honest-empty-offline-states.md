# ADR-114: honest offline state on the composed `Сейчас`, plus a real audit of quiet/empty/stale/blocked/failed

## Status

Accepted, 2026-09-17. Physically verified on Pixel 7 with a real
`saai-entityd` disconnect, not simulated.

## Context

VUI-03's sixth task: "Implement honest quiet, empty, stale, offline,
blocked, and failed states." Audited each state against the composed
`Сейчас` (ADR-112/113) rather than assuming; found one real, shipped gap
and confirmed the rest were already honestly handled.

## Decision

**Real gap found and fixed**: `now_context_header()`/`now_sections()`/
`now_object_summary()` (ADR-112) all read `self.selected_entities`
without ever checking `self.entityd.is_connected()`. When `saai-entityd`
is unreachable, `self.selected_entities` is simply whatever was last
fetched -- often empty on a fresh connection -- so the composed screen's
own whole-screen empty state ("Ничего срочного", ADR-112) fired
identically whether the space genuinely had nothing pending or the shell
simply could not tell. Fixed two ways:

1. `now_context_header()` now checks `self.entityd.is_connected()` first,
   before the existing archived-space lifecycle check -- offline takes
   priority, since without a live connection this shell cannot honestly
   claim to know the space's *current* lifecycle either. Sets
   `ContextHeader.lifecycle` to `StatusIndicator::new(UniversalState::
   Offline, "Нет связи")`.
2. `draw_now`'s whole-screen empty message now reads `header.lifecycle`'s
   state: `"Нет связи с пространствами"` instead of `"Ничего срочного"`
   when offline.

**Second, smaller gap found while fixing the first**: `ContextHeader.
lifecycle` (added in ADR-111, used for the archived-space case since
ADR-112) was being *computed* but never actually *drawn* -- `draw_now`
only ever rendered `header.heading()`, never the nested `StatusIndicator`
mark itself. It only mattered indirectly, through the empty-state
message. This meant the archived-space badge has been silently invisible
on the composed screen since ADR-112, and the offline case would have
shipped the same way had this not been caught here: if `sections`/
`object` still had real (possibly stale) content to show, the offline
signal would have been completely absent from the screen. Fixed by
drawing `header.lifecycle` as a real, always-visible compact
`StatusIndicator` row directly under the heading, independent of whether
the empty-state branch fires at all.

**Audited, no code needed**:

- **Failed** (`saaios.task`/`saaios.action` reaching `status: "failed"`):
  already honest per ADR-089 -- `notify_task_failed()`/`fail_confirmed_
  action()` already create a real `saaios.notification` (`kind:
  "task_failed"`), which `now_sections()`'s "Требует внимания" already
  surfaces by reusing `inbox_rows()` verbatim (ADR-112). No gap.
- **Empty/quiet**: already honest per ADR-112's own whole-screen "Ничего
  срочного" fallback, matching HIA-13's second mockup and `SystemSection`'s
  own "never invents a placeholder row" rule.
- **Blocked**: covered for this surface via the same `ContextHeader.
  lifecycle` slot (the pre-existing archived-space case, `UniversalState::
  Blocked`) -- now actually visible after the drawing fix above, not a
  new code path.

**Deliberately not built, flagged rather than silently skipped**:

- **Stale**: no code anywhere in this project tracks "time since last
  successful entityd sync" as its own concept -- entities carry their own
  `updated_at`, but nothing computes or surfaces "this view might be out
  of date" when the connection is up but data hasn't changed in a while.
  The offline signal covers the one case this session could verify
  physically (no connection at all); a genuine staleness indicator for a
  live-but-quiet connection is new plumbing, not something this pass's
  scope covers. Left in `docs/os/ideas.md`-style as real follow-up, not
  claimed done.
- **`saaios-runtime` (AI) availability on the intent-input screen**:
  ADR-089's own "Последствия" section already named this exact gap
  ("нет статус-индикатора вроде 'AI недоступен' на экране ввода") and
  left it open. Out of scope here too -- it's the intent-input screen,
  not `Сейчас`.

## Verification

- `cargo test -p saai-shell`: 96/96, unchanged.
- `cargo clippy -p saai-shell --all-targets -- -D warnings`: clean.
- Two hot-swap deploys (the fix itself, then the header-drawing follow-up
  fix found while verifying it), each hash-verified.
- Real physical test, not simulated: held `saai-entityd` down with a
  repeated-kill loop (`kill -9 $(pgrep saai-entityd)` every 300ms for
  ~1.2s, `pkill -f <full path>` was tried first and silently matched
  nothing -- `argv[0]` is the short name, not the full path, a real
  methodology gotcha worth recording) long enough to catch a real
  screenshot mid-disconnect, then let it recover. First attempt (before
  the header-drawing fix) showed the empty-state message correctly but
  no visible header badge -- confirming the second gap by direct
  observation, not by code review alone. Second attempt (after the fix)
  showed both signals together: the header's own "Нет связи" compact
  mark and the whole-screen "Нет связи с пространствами" message.
  `saai-entityd` and `saai-shell` both confirmed alive and reconnected
  normally afterward.

## Consequences

- The composed `Сейчас` no longer has a state where "cannot tell" and
  "confirmed nothing pending" look identical.
- `ContextHeader.lifecycle` is now a real, working visual element for the
  first time since ADR-111 defined it -- the archived-space case
  (untested by a real archived space this pass, but using the exact same
  drawing path just proven for offline) benefits from the same fix.
- The `pkill -f <full-path>` gotcha is worth remembering for any future
  physical test that needs to interrupt a `native-init`-managed process
  by pattern rather than a freshly-`pgrep`'d pid.

## Rollback

Both fixes are additive drawing/branching logic with no data or protocol
changes. `saai-shell.pre-offlinestate`/`saai-shell.pre-headerlifecycle`
remain on-device for a full binary rollback if ever needed.

## Links

- ADR-112 -- `now_context_header`/`draw_now`'s whole-screen empty state,
  both touched here.
- ADR-111 -- `ContextHeader.lifecycle`'s original definition, drawn for
  the first time by this ADR.
- ADR-089 -- the failed-task notification path this ADR confirmed already
  covers "Failed" honestly, and the AI-availability gap this ADR
  deliberately leaves open, matching that ADR's own scope boundary.
