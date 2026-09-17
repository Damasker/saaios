# ADR-112: real composed `Сейчас` (`Frame::Now`/`draw_now`), dev-gated

## Status

Accepted, 2026-09-17. Physically verified on Pixel 7 with real device data
(not fixture data). Behind a dev-only marker, off by default -- `RootPage::Now`
still renders through `draw_root`'s app-grid scaffold in production. Enabling
the marker in production is VUI-03's next task, not this one.

## Context

VUI-03's third Task List item: "Compose `Сейчас` so its first viewport
answers the five product questions." ADR-111 built the `ContextHeader`/
`SystemSection`/`ObjectSummary` contracts; this ADR is the first real
consumer of them -- an actual screen, not just a tested data type.

`human-interface-architecture-v2.md` section 13's own `Now` mockup groups
content under "Сегодня" / "Продолжается" / "Требует внимания", falling back
to a calm "Ничего срочного" when there is nothing to show. Section 14's five
things (NOW/SPACE/OBJECT/INTENT/ACTION) and this task's own acceptance line
("states context, current work/system activity, attention, and next action
when the data exists") ask for a fourth grouping beyond the three the mockup
names -- "next action" -- which the sprint's own inventory task (first
VUI-03 commit) found `saai-shell` had zero existing code for.

## Decision

**Real data sources, no invented ones.** Traced each section to something
already flowing through `self.selected_entities` (every entity in the
currently selected space, already fetched -- see the inventory commit):

- **Сегодня**: enabled `saaios.schedule` entities (ADR-036). This project's
  schedules are recurring intervals (`every_secs`), not times of day -- the
  row shows the schedule's own `text` property, never an invented clock
  time no real field backs.
- **Продолжается**: `saaios.task` entities with `status == "running"` (the
  status `handle_object_view_action` already writes once a user confirms
  one) -- new `in_progress_work()` filter. A task still `waiting_
  confirmation` stays out of this section; it needs the user, not the
  system, so it surfaces under "Требует внимания" instead.
- **Требует внимания**: reuses `inbox_rows()` verbatim -- the exact same
  tasks-needing-confirmation-plus-undismissed-notifications list "Входящие"
  already shows, as a preview on `Сейчас` rather than a second
  implementation of the same query.
- **Далее** ("next action"): `saaios.action` entities with
  `status == "pending"` -- new `next_pending_action()`, the first code in
  this file that reads `saaios.action` at all, closing the gap the
  inventory task found.
- **Object summary**: `self.selected_entities.first()` -- the same
  "most recently updated entity in the space" the old ad hoc
  `"inspect_selected_entity"` card already used, now returning a real
  `ObjectSummary` instead of a hand-formatted `ActionCardView`.

A `SystemSection` with zero real rows is left out of the returned list
entirely -- never handed to the renderer empty, matching the composite's own
"never invents a placeholder row" rule (ADR-111). When every section and the
object summary are both empty, `draw_now` shows one centered "Ничего
срочного" for the whole screen instead of empty section chrome, matching
HIA-13's own second mockup exactly.

**Rendering**: added `Frame::Now` (new `enum Frame` variant) and
`render::draw_now`, built from the same primitive draw functions the
gallery already proved (`draw_semantic_text`, `draw_divider`,
`draw_data_row`, `draw_status_indicator` -- renamed from their `draw_
gallery_*` names in this same commit, since they are no longer
gallery-only). `draw_tab_bar` was extracted from `draw_root` first so both
frames share the exact same bottom-navigation drawing code instead of a
second copy.

**Real bug found and fixed mid-implementation**: the first physical
screenshot showed the `ContextHeader` text completely missing. Root cause:
`content` (the Rect `draw_now`/`draw_root` receive) spans the full canvas
from `y=0` -- the status bar is a *separate*, always-on-top compositor
surface (`layer.set_size(0, 120)` in `main.rs`), not a reserved inset
inside this one. `draw_root` already knew this and avoids it with
hardcoded 150/430 (2400-scale) starting offsets; `draw_now` used
`content.y + margin` directly and drew its header underneath the status
bar surface, invisible. Fixed by scaling the same 150 proportionally
(`top_inset`) instead of repeating the literal.

**Second real finding, not a code bug**: after fixing the above, a
screenshot taken immediately after a shell restart still showed the
whole-screen empty state despite the selected space (`personal`, 19 real
entities including one genuinely `running` task) having real content to
show. Verified via `saai-shell`'s own log (its stdout/stderr are
redirected to `/run/saai-displayd.log`, confirmed via `/proc/<pid>/fd/1`)
that `saai-entityd` connects successfully but the entities response
arrives *after* the first paint. A forced redraw (switching root tabs and
back) immediately showed the correct populated content -- `ObjectSummary`
("Маджонг: победа!" / "saaios.notification · версия 2") and "Продолжается"
with the real running task, both exactly matching the entity store's own
JSON. This is a pre-existing "first frame can predate the entityd
round-trip" characteristic every `Frame::Root`-driven page already has
(nothing here is `Frame::Now`-specific), not a new regression -- it just
made a fast `screencap` immediately after restart a misleading test
method, worth recording so a future session does not mistake it for a
data bug again.

**Deliberately gated, not switched on**: a new `NOW_COMPOSED_MARKER`
(`/run/saaios/ui-now-composed`, `SAAIOS_UI_NOW_COMPOSED` env var), same
volatile `/run`-only pattern as `UI_CALIBRATION_MARKER`/`UI_GALLERY_MARKER`/
`DEV_NO_LOCK_MARKER`. `RootPage::Now` still builds `Frame::Root` with the
existing app grid by default; the marker is the only way to reach
`Frame::Now`. This is not because the composed screen is unfinished
display logic -- it is physically verified -- but because making it the
*only* way to reach `Сейчас` would silently remove app-grid access, which
is explicitly VUI-03's next, separate task ("Move the application grid
behind an explicit secondary `Приложения` entry ... without removing
application access"). Shipping that relocation and this composition in the
same change would make either one harder to verify or roll back
independently.

## Verification

- `cargo test -p saai-shell`: 94/94, unchanged.
- `cargo clippy -p saai-shell --all-targets -- -D warnings`: clean.
- Two hot-swap deploys (one for the composition itself, one for the
  top-inset fix), each backed up (`saai-shell.pre-nowcomposed`,
  `saai-shell.pre-topinset`) and hash-verified before and after.
- Real device screenshots, marker off: app grid unchanged from before this
  ADR -- no regression to the production path.
- Real device screenshots, marker on: `ContextHeader` ("Личное · Сейчас"),
  the whole-screen empty state, and (after the redraw finding above) real
  populated content -- `ObjectSummary` and a `Продолжается` row -- all
  confirmed against the entity store's actual on-disk JSON, not assumed.
- **Not exercised with real data**: no `saaios.schedule` or pending
  `saaios.action` entity exists anywhere in this environment, so
  "Сегодня"/"Далее"'s `DataRow` rendering path was not seen populated on
  a real screen this pass -- `draw_data_row` itself is already proven
  through VUI-02's gallery (same function, ADR-105/108), but this
  specific call site (real schedule/action titles, this section's exact
  layout position) was not. Flagged rather than silently claimed
  complete, matching this session's own precedent (ADR-106, ADR-110).

## Consequences

- `Сейчас` finally has a real, data-truthful home-screen composition ready
  to become the default -- gated behind one boolean flip
  (`self.now_composed`) plus the app-grid relocation task, not a rewrite.
- `now_content_cards`/`now_grid_rect` (the app-grid renderer) are
  untouched and still the live default -- their own relocation is a
  separate, deliberately deferred task.
- The `draw_gallery_*` rename (four functions) is a pure rename with no
  behavior change; `draw_gallery` itself is unaffected since it now calls
  the renamed functions directly.
- The "first frame can predate entityd" characteristic is now documented
  here for whoever next needs to reason about it -- it was already true
  of `Frame::Root`, just newly *observed* while testing `Frame::Now`.

## Rollback

Remove `/run/saaios/ui-now-composed` (or never set it) -- `RootPage::Now`
already defaults to the unaffected `Frame::Root`/app-grid path with no
code change needed. `saai-shell.pre-nowcomposed`/`saai-shell.pre-topinset`
remain on-device for a full binary rollback if ever needed.

## Links

- ADR-111 -- the `ContextHeader`/`SystemSection`/`ObjectSummary` contracts
  this ADR is the first real consumer of.
- ADR-105/108 -- `draw_data_row`/`draw_status_indicator`'s own original
  introduction and touch-target verification, reused here unchanged.
- ADR-036 -- `saaios.schedule`, the real data source behind "Сегодня".
- ADR-030/031 -- `saaios.task`/`saaios.action`/`saai-taskd`, the real data
  sources behind "Продолжается" and "Далее".
- The VUI-03 data-source inventory commit -- found the "next action" gap
  this ADR closes.
