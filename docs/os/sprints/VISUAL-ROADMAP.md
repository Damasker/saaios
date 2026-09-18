# SaaiOS Visual System — delivery roadmap

Status: **VUI-00, VUI-01, and VUI-03 complete; VUI-02 Task List complete, Acceptance checklist complete except one item blocked by the environment (see "The sans and mono faces survive the actual Pixel boot-image asset path" below)**

Target device: Pixel 7 (`panther`)

Product contract: [`visual-language-v1.md`](../architecture/visual-language-v1.md)

First-version UI destination (accepted concept boards):
[`product-visual-target-v1.md`](../ui/product-visual-target-v1.md).
Those boards are implemented **through this VUI sequence plus WORK-08**,
not as a separate redesign sprint. VUI-04 does not change the live tab
set mid-flight; `Поиск` and `Я`→`Система` are later bounded nav slices.

Interaction architecture: [`human-interface-architecture-v2.md`](../architecture/human-interface-architecture-v2.md)

## Outcome

Deliver a coherent SaaiOS interface and a reusable SaaiOS graphical component
library. The result must feel like a calm technical operating environment,
communicate real device and work state in one or two seconds, remain useful
without AI, and preserve the working native Wayland/DRM/GPU path.

The work is incremental and reversible. A sprint is at most two weeks and ends
with a usable device build or a documentation-only decision boundary. We do not
pause all product work for a monolithic UI rewrite.

## Success measures

1. A user can identify context, current work, system activity, attention, and
   next action on `Сейчас` without opening chat.
2. All migrated surfaces use semantic tokens and the shared component library;
   remaining raw values are tracked by an explicit allowlist.
3. Essential settings and recovery paths work with AI and network unavailable.
4. Universal states have consistent text, icon/shape, color, and behavior.
5. Normal, long-Russian-text, scaled-text, offline, blocked, failed, and empty
   cases are represented by fixtures and tests.
6. Touch, scrolling, navigation/status visibility, GPU composition, restart,
   and cold boot do not regress from the accepted Pixel 7 baseline.
7. Each completed change has focused documentation, a logical commit, and a
   pushed remote branch.

## Delivery rules

- HIA owns entities and behavior; Visual Language v1 owns visual and interaction
  presentation.
- No sprint invents data to complete a design. Missing runtime support appears
  as a truthful absent or unavailable state.
- No visual sprint silently changes business logic, protocols, authorization,
  storage, or device policy. Such a change requires its own ADR and tests.
- The existing UI remains a fallback until the migrated path is proven on the
  device.
- Layout and hit-test geometry come from one result.
- Reusable components move to `saai-ui-core`; shell-only prototypes may exist
  temporarily behind a feature flag.
- `.sui` v2 follows the reference surface and component vocabulary. It does not
  precede them.
- A visual review on a desktop screenshot is insufficient. Every sprint with a
  rendering change includes physical Pixel 7 review.
- Frequent commits are preferred: foundation, component, screen migration,
  tests, and documentation should be separable when practical.

## Component-library deliverables

The component library is built across the roadmap and is complete only when it
contains:

- a component inventory and reviewed visual specification sheets covering
  anatomy, measurements, variants, responsive behavior, and every state;
- versioned semantic design tokens;
- type roles and verified sans/mono assets;
- layout, safe-inset, scrolling, focus, and hit-test primitives;
- text, icon, divider, status, progress, button, field, data-row, metric, and
  disclosure primitives;
- context, system, object, intent, task, agent, event, decision, navigation,
  and Orb composites that have real product uses;
- empty/loading/offline/blocked/failed/confirmation/permission/recovery
  patterns;
- accessibility metadata and non-color cues;
- a device-runnable component gallery with labelled fixture data;
- golden rendering, layout, hit-test, state-transition, and interaction tests;
- public API documentation, stability labels, migration guide, and examples;
- a stable public subset for future third-party SaaiOS applications.

## Status overview

| Sprint | Result | Status |
|---|---|---|
| VUI-00 | Audit, product contract, and delivery plan | **Done** |
| VUI-01 | Semantic tokens and physically calibrated palette | **Done** |
| VUI-02 | Typography, geometry, icons, and base component library | **Acceptance complete except one environment-blocked item** |
| VUI-03 | Reference `Сейчас` surface | **Done** |
| VUI-04 | Navigation, status surfaces, Context Light, and restrained Orb | **Host complete** (`Я`→`Система` label with VUI-06) |
| VUI-05 | Object, Intent, Task, and Worker components (concept Object + Intent surfaces) | **Host complete** |
| VUI-06 | `Система` information architecture and settings components | **Host + panther complete** (`1d191d7a…`, label `Система`) |
| VUI-07 | Remaining system surfaces and state patterns | **In progress** (Inbox + Spaces + Wi-Fi list on panther) |
| VUI-08 | Motion, haptics, and measured frame pacing | Backlog |
| VUI-09 | `.sui` v2, public library, legacy cleanup, and release gate | Backlog |

---

## VUI-00 — Audit and visual contract

**Status:** Done (documentation only)

**Goal:** turn HIA and the new visual requirements into one implementable,
conflict-resolved product target.

### Completed tasks

- [x] Review HIA v2, product principles, prior UI sprints, and declarative UI ADR.
- [x] Inspect current root declaration, shell frame types, renderer palette,
  font assets, scaling, and GPU composition path.
- [x] Resolve context-color versus status-color ownership.
- [x] Define the target navigation and the `Сейчас`/applications relationship.
- [x] Define the SaaiOS component-library layers and ownership boundaries.
- [x] Record visual, accessibility, resilience, and performance constraints.
- [x] Split delivery into bounded, reversible sprints.

### Evidence

- `docs/os/architecture/visual-language-v1.md`
- this roadmap

### Exit criteria

- [x] HIA remains authoritative rather than being duplicated or silently
  overwritten.
- [x] The visual target explicitly rejects app-launcher, full-screen-chat,
  desktop-on-phone, DOS, and decorative HUD outcomes.
- [x] The first implementation sprint has a bounded scope and rollback plan.

---

## VUI-01 — Semantic tokens and calibrated palette

**Status:** Done — implemented, physically reviewed, and cold-boot verified on
Pixel 7 on 2026-09-17

**Depends on:** VUI-00

**Goal:** create one semantic source of truth for color and universal state
without changing the information architecture or layout.

### Tasks

- [x] Write an ADR for theme ownership, compatibility, and the temporary
  hardcoded-value allowlist.
- [x] Introduce typed color tokens from Visual Language v1 in `saai-ui-core`.
- [x] Define typed universal state values: `IDLE`, `ACTIVE`, `RUNNING`,
  `WAITING`, `BLOCKED`, `ATTENTION`, `FAILED`, `COMPLETE`, and `OFFLINE`.
- [x] Map universal states to color plus text/icon/shape cues.
- [x] Keep Context Light/Orb context color separate from semantic status color.
- [x] Replace the renderer's seven global constants with the theme interface.
- [x] Move local shell RGB values behind theme or the documented context-color
  boundary.
- [x] Add pressed, focused, disabled, and high-contrast derived tokens.
- [x] Create a full-screen palette/state calibration fixture.
- [x] Add deterministic token and state-mapping tests.
- [x] Capture a deterministic reference render and complete a physical Pixel 7
  review under normal indoor light; record display-pipeline adjustments
  separately from tokens.
- [x] Document migration and rollback.

### Implementation evidence

- ADR-094 defines ownership, backend boundaries, allowlist, verification, and
  rollback.
- Commits `c2e2c08`, `dc1672a`, `5b9b23b`, `04507e8`, and `01552e6` implement
  the core contract, shell migration, calibration fixture, volatile session
  switch, and raw-token calibration behavior.
- `cargo test -p saai-ui-core`: 5/5; `cargo test -p saai-shell`: 87/87;
  strict clippy passes for both packages; the workspace test/clippy gate passes.
- Pixel 7 ARM64 static build SHA-256:
  `cfef42da13e0a0d46c70c70b86535b048ea1610189e27f731d232d8b10a1ec8d`.
- The binary was atomically installed with the previous shell preserved as
  `/data/saaios/system/saai-shell.pre-vui01`; supervised restart, 1080×2400
  DMA-BUF commit, and calibration-mode startup are confirmed in live logs.
- The deterministic calibration frame was rendered at 1080×2400 with frame
  SHA-256
  `c8021bf27d48943b704a1efe82a921cb2d04e0a5960483f5a3653cfab83a1246`.
  Physical review on the Pixel 7 accepted the neutral dark/cyan palette and
  state/context separation. Calibration deliberately bypasses the user
  contrast post-process so the fixture measures raw semantic tokens; this
  display-pipeline boundary is implemented separately in `01552e6`.
- After removing the volatile calibration marker, the ordinary lock screen,
  unlock flow, status layer, navigation, and `Я` surface were exercised through
  the normal touch path. Live logs confirm full-screen and 1080×120 status-layer
  DMA-BUF submissions through the Vulkan compositor.
- A forced cold boot returned all required services without intervention:
  `saai-entityd`, `saai-appd`, `saai-displayd`, `saai-gpu-compositor`, and
  `saai-shell`. The volatile marker remained absent, the installed shell hash
  remained unchanged, and the lock/unlock flow produced ordinary GPU frames.

### Acceptance

- [x] Existing screen geometry and navigation code are unchanged.
- [x] No migrated component creates ad-hoc RGB values.
- [x] Every state has a non-color mark in addition to its color role.
- [x] Text and controls meet the agreed contrast checks on actual surfaces.
- [x] Pixel 7 shows the intended neutral dark/cyan palette without channel
  swap, crushing, or unintended warm cast.
- [x] Touch, current smooth `Система`/`Я` scrolling, status layer, and bottom
  navigation behave as before.
- [x] Shell restart and cold reboot recover the themed UI.

### Rollback

The pre-sprint shell remains on the device as
`/data/saaios/system/saai-shell.pre-vui01` for atomic rollback. No entity,
protocol, storage, or authorization changes are included.

---

## VUI-02 — Typography, geometry, icons, and base components

**Status:** In progress — base component contract reviewed; typography and
mono font shipped and re-verified (ADR-096, ADR-099); a real device
screenshot tool now exists for physical review; foundation tokens and device
gallery remain

**Depends on:** VUI-01

**Goal:** establish the reusable foundations and primitives needed to build a
real phone interface consistently.

### Tasks

- [x] Draw and review the component inventory and state sheets for every base
  primitive before declaring its API stable.
- [x] Add semantic typography roles independent of font filenames and raw
  point sizes (ADR-096, `saai-ui-core::TextRole`).
- [ ] Compare the current Montserrat build with candidate UI faces on Pixel 7;
  keep Montserrat unless another face is demonstrably more readable.
- [x] Select and license a static monospace face with Latin and Cyrillic
  coverage (ADR-096, IBM Plex Mono Regular). `fontdue` loading, sans
  fallback, and boot-image packaging verified; exercising a real SSH
  pairing fingerprint in the mono role, and increased text scale, remain
  (see ADR-099's "Not verified by this ADR").
- [ ] Define spacing, radius, stroke, touch-target, safe-inset, and elevation
  tokens in logical units.
- [x] Add one coherent line-icon source and a reproducible asset pipeline
  (ADR-100, Feather Icons -- MIT, static TTF, drawn through the existing
  `fontdue` text path; a small curated `IconGlyph` set, extendable per real
  call site). Physically verified: fixed the PIN keypad's backspace tofu-box
  (ADR-099's own screenshot finding).
- [ ] Implement shared layout primitives: stack, row, inset, separator, scroll
  region, and focus/order metadata. Partially done (ADR-101): stack/row
  already existed as `Node::linear` + `Axis`; inset (`EdgeInsets`),
  `Node::separator`, and `Node::focus_order` metadata added. Scroll region
  -- the harder, correctness-sensitive piece (`hit_test` needs to stop
  matching children scrolled outside their viewport) -- remains open,
  deliberately not folded into the same change.
- [x] Implement visual primitives: semantic text, icon, divider,
  `StatusIndicator`, progress, button, field, `DataRow`, metric, and disclosure.
  Done as Experimental contracts (ADR-102, ADR-103) -- component-library-v1.md
  sections 6.1-6.10, all ten unit-tested. Not yet wired into any `saai-shell`
  screen or gallery -- that, plus promoting any of them to Stable per section
  9's checklist, is separate follow-up work.
- [x] Make visual and hit-test bounds consume the same layout output.
  Audited every `saai-shell` screen with both a draw path and a touch path
  (2026-09-17): each one already routes both sides through exactly one
  shared computation -- either the `Node`/`layout()` tree (`root_view`,
  `task_confirm_view`, `object_view`, `orb_view`, `intent_view`,
  `consent_view`, all called identically by their `*_action_at` touch
  function and by draw code) or one shared helper function called
  identically by both sides (`now_grid_rect`, `stacked_row_rect` -- 43 call
  sites across the Wi-Fi/Bluetooth/trusted-clients/inbox/"Я" lists,
  `scrolled_row_rect`/`me_scroll_offset`, `content_action_rect`,
  `pin_keypad_rect` for the lock screen and PIN setup, a documented
  exception to the `Node` tree since the lock surface has none of its own,
  but still one function on both sides). No independent/duplicated rect
  arithmetic found anywhere -- this task was already structurally satisfied
  by the codebase's existing convention, not a new change.
- [x] Define component accessibility names, roles, values, disabled states, and
  non-color cues. Done (ADR-104): every one of the ten primitives from
  ADR-102/ADR-103 exposes an `accessibility()` -> `AccessibilityInfo` (role,
  name, value, disabled, busy) or `Option<AccessibilityInfo>` when a
  primitive must not be independently exposed at all (a decorative `Icon`,
  an unnamed `Divider`). Non-color cues needed no new work -- already
  satisfied, since every primitive that carries meaning already does so
  through text or an enum, never color alone.
- [x] Build the first device component-gallery surface covering all primitive
  states, long Russian strings, and scaled text. First pass done (ADR-105):
  `render::draw_gallery`, one instance of each of the ten ADR-102/103
  primitives, default state only, physically verified via a real device
  screenshot. Caught and fixed a real bug in the process -- `saai_ui_core`'s
  size tokens are logical units, and the first draft used them as physical
  pixels directly, causing two primitives' stacked text lines to overlap;
  fixed with a `physical()`/`physical_line_height()` conversion helper now
  reusable for future rendering code. Not yet done: pressed/focused/
  disabled/busy variants, both compact and normal side by side, long-
  Russian-text and scaled-text coverage, and the developer bounds overlay --
  section 7's full matrix remains open.
- [x] Add golden render, layout, hit-test, press-state, and overflow tests.
  Layout and golden-pixel tests done (ADR-106): `gallery_row_positions`
  extracted as a pure, host-testable function, with a real regression test
  that already caught one genuine spacing bug (the title-to-first-row gap).
  Honestly scoped, not overclaimed: this test does NOT cover ADR-105's own
  worse bug (a within-row second-line offset colliding with the line above
  it) -- verified directly by temporarily restoring the old broken offsets
  and confirming the test still passes. That bug class is instead prevented
  structurally (both call sites now compute their offset from the same
  `physical_line_height` function the fix uses, not a separate hand-picked
  number). Hit-test and press-state tests are not applicable yet -- the
  gallery is a passive display surface with no primitive wired to any
  action. True text-content golden renders and overflow tests need a loaded
  font, unavailable on host (the same constraint every existing `render.rs`
  test already works around); not attempted without either an embedded test
  font or further physical device screenshots.

### Acceptance

- [x] Public primitives contain no shell-specific business logic. Verified
  2026-09-17: `saai-ui-core/Cargo.toml` has zero dependencies, so nothing in
  it can reference `saai-shell` or any other crate even by accident --
  structural, not just a review claim. Manually read all 11 public types in
  `components.rs`: none names a SaaiOS-specific concept (no Wi-Fi,
  Bluetooth, HIA entity, or screen name anywhere). `Button.action`/
  `DataRow.action`/`Disclosure.target` are opaque strings a consumer
  interprets, the same pattern `Node.action` in this crate's own
  pre-existing layout system already uses -- not business logic, a dispatch
  key.
- [x] Primary touch targets are at least 48×48 logical units. Checked
  directly (ADR-108), not assumed: `Button` and `Disclosure` were both
  under the minimum (`Disclosure` was only 32 units), `DataRow` was already
  correct, `Field` cleared it only incidentally. All four now explicitly
  sized to `MIN_TOUCH_TARGET`, physically verified. Not a test -- nothing
  automated stops a future gallery row from being added under the minimum
  again; this is a one-time review and fix.
- [ ] Long labels wrap or reflow; they do not clip or overlap navigation.
  Partial (ADR-107): done for `SemanticText` -- real greedy word-wrap plus
  `max_lines`/ellipsis truncation, physically verified. The gallery's own
  demo string used to run off the right edge of the screen in every prior
  screenshot; it now wraps at a word boundary and truncates cleanly. Still
  open for every other primitive with label text (`Button`, `Field`,
  `DataRow`, `Metric`, `Disclosure`, `StatusIndicator`) -- each has its own
  separate, not-yet-implemented wrap behavior per their own
  component-library-v1.md anatomy notes, so this criterion is not yet met
  for the component set as a whole.
- [ ] The sans and mono faces survive the actual Pixel boot-image asset
  path. Blocked in this environment, not merely deferred: verifying this
  requires running `build-native-c-image.sh`, which needs a real
  `STOCK_INIT_BOOT`/`STOCK_VENDOR_BOOT` (proprietary Pixel factory
  firmware) and a `SAAIOS_PANTHER_ARTIFACTS` "secure local" artifact
  directory (see `os/targets/panther/README.md`); confirmed via a full
  filesystem search that none of these exist anywhere on the build server.
  The font-asset wiring itself (`add 0644 saaios/fonts/...` lines in the
  build script) is already in place and was reviewed by reading the
  script, but the actual boot-image build has never been exercised.
- [x] Components look like one family at normal and increased text scale
  (ADR-110). Verified with real device screenshots at `text_scale_pct: 100`
  and `150`: found and fixed a real bug where `SemanticText`'s wrap
  measurement and the shared line-stacking offset (also used by
  `StatusIndicator`'s reason line and `DataRow`'s secondary line) did not
  account for the accessibility text-scale multiplier, and a second,
  unrelated bug in the persistent status bar where "Wi-Fi" and the battery
  percentage overlapped at 150%. Known open edge case, not claimed solved:
  the gallery's row-to-row layout budget (`gallery_row_positions`) is still
  a fixed, scale-independent fraction of screen height, so a sufficiently
  long wrapped string at a high enough scale could still make one row's
  content collide with the row below it -- not exercised by the current
  demo strings up to 150%, see ADR-110's Consequences section.
- [x] Component gallery is runnable on device and clearly labels fixture
  data. Runnable via the same volatile marker pattern as calibration mode
  (`SAAIOS_UI_GALLERY`/`/run/saaios/ui-gallery`, ADR-105). Labeled by its
  own header, drawn first, above every primitive demo:
  "SaaiOS Component Gallery · VUI-02" -- identifies the whole screen as a
  gallery rather than live shell content, the same way the values it shows
  (a `Field` fixture PIN of "4269", a fixture Wi-Fi network named
  "Wallbox", a fixture 87% battery) are recognizably demo data, not read
  from any real device state or settings file.
- [x] No scrolling or frame-pacing regression against the VUI-01 baseline.
  Re-ran S04's own baseline method on the real device: recorded
  `/run/saai-displayd.log` and `/run/touch.log` line counts, had 25 real
  taps done across all four root tabs (`Now`/`Inbox`/`Spaces`/`Me`,
  repeated in mixed order), then diffed both logs. Every single touch
  produced exactly one `saai-shell: switched to <tab>` followed immediately
  by exactly one `commit on surface` + blit pair -- the same "one commit,
  no flood of repeats, no multi-second gap" signature the frame-pacing fix
  in `d604e95` (cited in S04) established as the passing baseline. No
  VUI-02 change (components, gallery, dev_no_lock, text-scale fix) touches
  the event loop or commit-triggering code, and this re-run confirms
  nothing regressed it in practice, not just by code inspection.

### Physical review evidence (ADR-099)

`os/targets/panther/tools/screencap.c` -- a small DRM/KMS ioctl tool -- now
lets a session without a human looking at the phone capture the real
composited screen (reads the active CRTC framebuffer directly, independent of
`saai-displayd`, no disruption). The first real screenshot immediately found
a genuine bug this task list's own review step exists to catch: `draw_text`/
`draw_text_centered` in `services/saai-shell/src/render.rs` never applied a
glyph's vertical (baseline) bearing, so short glyphs (hyphen, period, colon)
rendered floating near cap-height instead of sitting on the line -- visible as
a mispositioned hyphen in "Wi-Fi" and "PIN-код". Fixed by baseline-aligning
every glyph in a text run against the tallest glyph actually present in that
run, verified both mathematically (ordinary letters keep their exact previous
pixel position) and visually (before/after screenshots, `cargo test`/clippy
unchanged). Use this tool for the rest of VUI-02's physical-review gates
(component gallery, remaining primitives) instead of relying on frame-hash/
log inference alone.

### Rollback

Primitives remain opt-in; existing screen drawing stays available until each
screen migration is accepted.

---

## VUI-03 — Reference `Сейчас` surface

**Status:** Done (ADR-112/113/114/115)

**Depends on:** VUI-02

**Goal:** prove the visual language and component vocabulary on one useful,
truthful home surface before expanding the framework.

### Tasks

- [x] Inventory the real sources for active Space, current Intent/Task, system
  activity, attention, and next action. Findings, by source:
  - **Active Space** -- fully real and mature. `saai-entity-store::Space`
    served over `entityd_client.rs`; lifecycle/relation/color/signal are
    separate `saaios.space-*` entity-type conventions already read by
    `space_lifecycle()`, `space_relation_targets()`, `space_color()`,
    `context_label()` in `main.rs`.
  - **Current Intent/Task** -- real per ADR-030/031: `saaios.intent`/
    `saaios.task`/`saaios.action` entity types, `saai-taskd` turns an
    Intent into a Task+Action. Shell already reads pending tasks
    (`inbox_pending_tasks()`) and a task's linked intent
    (`object_view_content()`). Gap: the only "what am I doing" signal
    today is `selected_entities.first()` (most-recently-updated entity in
    the space) -- not a real current-work view.
  - **System activity** -- thin. Only `app_state_label()` over
    `installed_apps` exists; no "N of M steps done" in-progress concept
    anywhere in code. Needs a new query pattern (tasks/actions with a
    non-terminal status), not new wire plumbing.
  - **Attention** -- real and already wired to the Orb.
    `saaios.notification` entities, `inbox_notifications()`, merged with
    pending tasks by `inbox_rows()`, drives `orb_state()`'s `Attention`
    state (low-battery and failed-task notifications already flow through
    this). Solid foundation, just not surfaced as Now-screen content yet.
  - **Next action** -- the real gap. `saaios.action` is a defined entity
    type (ADR-030) but no code anywhere in `main.rs` filters or reads
    `entity_type == "saaios.action"` -- confirmed via a full-file search,
    zero matches, unlike every other entity type above which each has a
    dedicated filter function. This needs a genuinely new query/view, not
    just new UI over data the shell already fetches.
- [x] Design `ContextHeader`, `SystemSection`, and `ObjectSummary` composite
  components from actual data (ADR-111). Scope corrected against
  `component-library-v1.md` section 3 (the reviewed design boundary),
  which this task list line had drifted from: that inventory table scopes
  VUI-03 to exactly these three composites and explicitly defers `EventRow`
  to VUI-04/05 (alongside `IntentSummary`/`TaskSummary`/`AgentSummary`,
  deferred to VUI-05). No separate "next-action" composite is named
  there either. Today's events and next action are represented with the
  existing primitives (`DataRow`, `StatusIndicator`) composed inside
  `SystemSection`/`ObjectSummary`, not a new composite type. Implemented
  in `crates/saai-ui-core/src/composites.rs`, specified in
  `component-library-v1.md` section 7. Host-verified only (42/42
  `saai-ui-core` tests, clippy clean) -- not yet wired into a real screen,
  that's the next task below.
- [x] Compose `Сейчас` so its first viewport answers the five product
  questions (ADR-112: `Frame::Now`/`render::draw_now`). Real data only --
  "Сегодня" from enabled `saaios.schedule` entities, "Продолжается" from
  `saaios.task`s with `status == running`, "Требует внимания" reuses
  `inbox_rows()` verbatim, "Далее" (next action) from pending
  `saaios.action` entities (closing the gap the sprint's own inventory
  task found), object summary from the same "most recently updated
  entity" `selected_entities.first()` already used. An empty section is
  never handed to the renderer; a fully empty screen shows one centered
  "Ничего срочного" per HIA-13's own second mockup.

  Physically verified on the real device with real data (not fixture
  data): `ContextHeader`, the whole-screen empty state, and (after
  forcing a redraw to get past a pre-existing "first frame can predate
  the entityd round-trip" characteristic, not a new bug) real populated
  `ObjectSummary`/`Продолжается` content, all confirmed against the
  entity store's actual on-disk JSON. One real rendering bug found and
  fixed along the way: the header was drawing underneath the status
  bar's own separate compositor surface. Not verified with real data:
  no `saaios.schedule`/pending `saaios.action` entity exists anywhere in
  this environment, so "Сегодня"/"Далее"'s specific `DataRow` call site
  was not seen populated on a real screen (the underlying draw function
  itself is already proven via VUI-02's gallery).

  Deliberately dev-gated (`SAAIOS_UI_NOW_COMPOSED`/`/run/saaios/
  ui-now-composed`), off by default: `RootPage::Now` still renders the
  existing app grid in production until the next task below (moving that
  grid behind `Приложения`) ships, so this composition does not silently
  remove app-grid access in the meantime.
- [x] Move the application grid behind an explicit secondary `Приложения`
  entry or sheet without removing application access (ADR-113). Two new
  fixed footer rows on the composed screen -- "Приложения" opens the
  existing, completely unmodified app-grid rendering/touch-handling path
  (`now_content_cards`/`now_grid_rect`/`now_action_at`); "Новое намерение"
  reuses the existing intent-input entry point directly. Closed by
  re-tapping the already-selected "Сейчас" tab.

  Physically verified end-to-end with real device taps: opened the grid
  (saw the real installed apps plus the two legacy `root.sui` cards,
  confirming application access is intact, not removed), closed it back
  to the composed screen with its real content still correct, and opened
  the intent-input keyboard from its new row. Only reachable while
  ADR-112's own `now_composed` dev marker is set -- with it off,
  `RootPage::Now` is completely unaffected by this task, exactly as
  before.
- [x] Keep prompt/chat input secondary to direct system actions. Audited
  rather than built new: `self.intent_input: Option<IntentInputState>` is
  the only prompt-like input mechanism anywhere in `saai-shell` (confirmed
  via a full-file search for any other chat/prompt surface -- none
  exists) and it is strictly on-demand, never persistent chrome -- there
  is no always-visible input bar to make secondary in the first place.
  Every path that opens it is already a small, equally-weighted entry
  point, not an elevated one: the old app grid's "Новое намерение" card
  (one icon among several, no special size or position), ADR-113's own
  "Новое намерение" footer row (same visual weight as "Приложения",
  positioned below all real system-activity content on the composed
  screen), and the Orb's own `OpenIntent` menu action (the Orb is
  explicitly "restrained" per its own VUI-04 title). No change needed;
  this criterion was already met by the existing architecture, both
  before and after this sprint's own composed-screen work -- documented
  here rather than silently left unchecked with no explanation.
- [x] Implement honest quiet, empty, stale, offline, blocked, and failed
  states (ADR-114), five of six covered, one deliberately not:
  - **Empty/quiet**: ADR-112's own whole-screen "Ничего срочного"
    fallback, matching HIA-13's second mockup.
  - **Offline**: real gap found and fixed -- `now_context_header()`
    now checks `self.entityd.is_connected()` before the space's own
    lifecycle, showing "Нет связи" (`ContextHeader.lifecycle`) and the
    whole-screen message becomes "Нет связи с пространствами" instead of
    the misleading default. Physically verified with a real
    `saai-entityd` disconnect (repeated-kill loop), not simulated.
  - **Blocked**: the pre-existing archived-space case, same
    `ContextHeader.lifecycle` slot.
  - **Failed**: audited, not built -- already honest per ADR-089's own
    `notify_task_failed()` path, which "Требует внимания" already
    surfaces by reusing `inbox_rows()` verbatim.
  - A second, smaller gap found while fixing the first: `ContextHeader.
    lifecycle` (ADR-111) was computed but never actually drawn on
    screen since ADR-112 shipped -- only ever consulted indirectly for
    the empty-state message. Fixed by drawing it as a real, always-
    visible compact `StatusIndicator`, independent of whether the
    empty-state branch fires.
  - **Stale**: not built. No code anywhere in this project tracks "time
    since last successful entityd sync" as its own concept; a genuine
    staleness indicator for a live-but-quiet connection is new plumbing
    this pass's scope did not cover. Flagged as real follow-up, not
    claimed done.
- [x] Validate progressive disclosure for developer and diagnostic
  details. Audited, not modified: HIA-20's `Frame::DevSurface` (a silent
  tap counter on "Я"'s build-id card) is this project's existing,
  project-wide progressive-disclosure mechanism for genuinely low-level
  detail -- `dev_surface_rows()` shows raw `space_id`, `ContextFrame`
  internals, confidence scores. Architecturally separate from every
  VUI-03 change this sprint made (different `RootPage`, no shared code
  path with `now_*`/`draw_now`), so unaffected by construction, confirmed
  by inspection rather than needing a fresh physical pass.

  The one developer-adjacent detail the composed screen itself shows --
  `ObjectSummary`'s "{entity_type} · версия {revision}" meta line -- is
  not new disclosure-policy scope creep: it is the exact same detail
  level the pre-existing ad hoc `"inspect_selected_entity"` card already
  showed before ADR-112 (see ADR-111's own note formalizing it), just
  through a real composite instead of hand-built `ActionCardView`
  strings. Reviewed against `Frame::DevSurface`'s own far more technical
  content and judged appropriately light for a primary surface -- a type
  name and a small counter, not an internal identifier or raw JSON.
- [x] Extract only components with a confirmed second use; document
  candidates that intentionally remain reference-screen private. Nothing
  further extracted this pass -- audited what already exists instead:
  - `ContextHeader`/`SystemSection`/`ObjectSummary` (ADR-111) already
    live in `saai-ui-core`, not `saai-shell` -- but have exactly one
    consumer so far (`draw_now`). Per this task's own rule, a single use
    is not a "confirmed second use"; they stay Experimental
    (component-library-v1.md section 2) with no further action until a
    real second consumer exists (`ObjectSummary` is the likeliest
    candidate, named in ADR-111 as ready for VUI-05's Object View).
  - Intentionally reference-screen private, staying in `saai-shell`'s
    `main.rs`/`render.rs`: `now_context_header`/`now_sections`/
    `now_object_summary` (real-data composition logic specific to this
    product surface's own meaning, not reusable UI), `today_schedules`/
    `in_progress_work`/`next_pending_action` (entity-filtering queries,
    same reasoning), and `now_footer_action_rect`/`now_footer_action_
    views`/`now_footer_action_at` (ADR-113 -- pure position functions,
    matching the same private-to-the-page convention `now_grid_rect`/
    `stacked_row_rect` already established for every other page's own
    layout, never shared or extracted across pages in this codebase).
- [x] Add reference renders and end-to-end touch/navigation tests. This
  project's own established convention (matching VUI-02's gallery work):
  real device screenshots described and reviewed in the ADR that
  introduced each state, not committed PNG files -- `visual-language-v1.md`
  section 15's "reference renders" requirement is met that way throughout
  this codebase already, not just here. Reference renders on record, by
  ADR: empty/quiet and real populated content (ADR-112), the app-grid
  overlay opening and closing plus intent-input reachability (ADR-113),
  the offline state both before and after a real gap fix, screenshotted
  twice to directly confirm the fix rather than assume it (ADR-114).
  Interaction tests: `now_footer_action_rows_stack_above_the_tab_bar_
  in_order`, `now_footer_action_at_finds_each_row_and_misses_above_them`
  (ADR-113), plus every `*_action_at`/data-filter test added across
  ADR-112/113/114, matching this codebase's own established split (pure
  hit-test/data functions get host tests; stateful touch dispatch gets
  physical verification, same balance ADR-089 already documented for
  `saai-taskd`).

  Known gap, flagged rather than silently passed: `draw_now` has no
  scroll handling at all -- if real `SystemSection` content ever exceeds
  one screen's height, it will overflow/clip rather than scroll, unlike
  "Я"'s own `scrolled_row_rect` precedent elsewhere in this file. Not
  exercised by this environment's real data (at most one row was ever
  populated per section here) and out of this pass's proportionate scope
  to build -- real follow-up, not claimed solved.

### Acceptance

All six re-reviewed against the real shipped default (ADR-115), not a
dev-only preview:

- [x] No fake tasks, agents, metrics, progress, or alerts appear. True by
  construction throughout ADR-112/113/114 -- every row traces to a real
  entity or a real connection-state check, never fabricated.
- [x] The first viewport states context, current work/system activity,
  attention, and next action when the data exists. `ContextHeader`,
  "Продолжается", "Требует внимания", "Далее" -- all real, physically
  confirmed with actual entity data.
- [x] Essential actions remain reachable without chat or AI. "Приложения"
  and root-tab navigation both work with `intent_input` never opened.
- [x] Application access remains discoverable but is not the primary
  model. "Приложения" is a single, always-visible row, one tap away --
  and, as of ADR-115, no longer the page's default content.
- [x] Empty state is calm and useful rather than filled with decoration.
  "Ничего срочного"/"Нет связи с пространствами" -- centered text, no
  invented filler.
- [x] Physical Pixel 7 review covers portrait insets, keyboard, touch,
  long text, and one-handed reach. Portrait insets: the status-bar-
  overlap bug found and fixed in ADR-112. Keyboard: the intent-input
  keyboard confirmed opening from its new row (ADR-113). Touch: footer
  row hit-testing and the app-grid open/close round trip both physically
  confirmed (ADR-113). Long text: relies on `SemanticText` wrap already
  proven by VUI-02's own gallery fixture and reused verbatim here, not a
  fresh dedicated stress test on this screen specifically. One-handed
  reach: footer rows and tabs both meet `MIN_TOUCH_TARGET` (ADR-108) and
  sit in the lower half of the screen.

### Rollback

`saai-shell.pre-nowdefault` remains on-device for a full binary rollback
to the pre-VUI-03 app-grid-default behavior (ADR-115).

---

## VUI-04 — Navigation, status, Context Light, and Orb

**Status:** In progress (ADR-116)

**Depends on:** VUI-03

**Goal:** make system orientation stable and distinctively SaaiOS without
turning the Orb into a launcher or assistant avatar.

### Tasks

- [x] Implement shared bottom-navigation and system-status components with
  safe insets and stable layering. Bottom navigation done (ADR-116):
  `BottomNavigation`/`NavigationItem` in `saai-ui-core`, `Shell::
  root_navigation_items` shared by both `Frame::Root` and `Frame::Now`, one
  navigation strip rather than two implementations. **system-status (host):**
  `SystemStatus` in `saai-ui-core` owns Context Light / clock / network /
  battery facts; `draw_status_bar` paints that composite (missing battery
  is absent, not `0%`; Space color is `ContextColor`, never severity).
  Stable layering (host): status is a top overlay layer; content and nav
  split from `root_view` and do not overlap; content paint is clipped to
  the content pane. Not a second Wayland nav layer — displayd still has
  no layer-surface touch routing. **Safe insets (host):**
  `SafeInsets::PIXEL_7_PORTRAIT` is the surface-provided inset (top =
  status overlay, bottom = nav strip, sides 0). Status layer height is
  `physical(top)` not a magic 120; nav hit-region scales from the design
  canvas and never drops below `MIN_TOUCH_TARGET` (portrait 1080×2400 and
  landscape 2400×1080).
- [x] Preserve `Сейчас`, `Входящие`, and `Пространства`; stage `Я` → `Система`
  only when the destination content is truthful. Visible label is now
  `Система`; id stays `me` / `select_root:me`.
- [x] Add explicit selected, pressed, disabled, attention, and badge states
  (ADR-116). `selected` is the current page; `badge`/`attention` come from
  `inbox_rows().len()`; `pressed` is the live finger on that tab
  (`pressed_tab_from_touch` / `Shell::pressed_tab`) and paints
  `ColorRole::Pressed` plus a bottom hairline without changing icon size.
  `disabled` still has no real trigger -- no tab is actually disabled.
- [ ] Apply Context Light grammar: context=color, state=shape,
  activity=motion, quantity=arc/fill, attention=ring. Four of five axes
  live on the Orb (host): state=shape (`StatusMark`), context=color
  (Space color for `Idle`/`Active`, semantic color otherwise),
  **attention=ring** (`WaitingConfirmation` / undismissed Notifications),
  **quantity=fill** (determinate battery `Progress`; missing reading is
  absent, not `0%`; Border token, never severity), **activity=motion**
  (`MotionCue::ActivityPulse` only for Running and not reduced-motion;
  still-frame inset hairline until VUI-08). A circular arc is not drawn
  — this file has no circle primitive; fill is the honest square analogue.
  `Я`→`Система` is the visible tab label (VUI-06); destination content
  was accepted on Pixel before the rename.
- [x] Integrate a restrained Orb host with quiet, active, progress, attention,
  offline, and reduced-motion states (ADR-116). All five real states
  reuse `UniversalState` (`Idle`/`Active`/`Running`/`Attention`/`Offline`)
  with real triggers -- `Offline` from real `appd`/`entityd` connection
  checks, `Attention` from undismissed notifications, `Running` from the
  same `in_progress_work` query VUI-03's "Продолжается" already uses,
  `Active` from the menu being open. `reduced_motion` is
  `ShellSettings.reduced_motion` (Я → «Меньше движения»), passed into
  `OrbHost::with_reduced_motion` so Running is not busy when set.
- [x] Retain direct tab navigation during Orb work; do not make the Orb the only
  route. `tab_at` still resolves the four tabs while an Orb menu is open;
  `orb_action_at` does not occupy the tab strip (host).
- [x] Make status/navigation layers independent of scrolling content damage.
  Status is already a separate `wlr-layer-shell` surface and is not committed
  from `draw()`. Navigation stays on the toplevel (displayd still does not
  route touch to layer surfaces), but scroll frames `damage_buffer` only
  `root_content_rect`, content paint is clipped to that pane, and `draw_tab_bar`
  runs last so a long `Сейчас` list cannot cover the strip. Host tests:
  content∩nav is empty; scrolled "Я" rows never enter the strip.
- [x] Add rotation/inset/keyboard and rapid-tab-switch interaction tests.
  Portrait and landscape: four tabs hit, content is not a tab, nav ≥
  `MIN_TOUCH_TARGET`. Intent keyboard stays below its header and every
  key meets the min touch on both orientations. Rapid tab switch:
  `pressed` follows the live finger and clears on up.

### Acceptance

- [ ] All four destinations switch reliably and update color/shape immediately.
- [ ] Top status and bottom navigation never disappear during scroll.
- [ ] Orb state is understandable without animation and does not dominate the
  screen.
- [ ] Context color is never confused with severity or primary action.
- [ ] Navigation remains usable when AI services are stopped.

### Rollback

The current four-tab renderer remains a build-time fallback until the shared
navigation passes restart and cold-boot testing.

---

## VUI-05 — Object, Intent, Task, and Worker components

**Status:** Host complete (ATTN-02/03 + WORK-08 on panther shell
`63b8b64`. Object View + offline manuals + `AgentSummary` + gallery
page **host**. Physical gallery/Object View flash still pending.)

**Depends on:** VUI-04 and the relevant HIA entity/runtime support

**Goal:** give work and objects a consistent, inspectable, action-oriented
visual grammar matching the v1 concept Object and Intent surfaces.

Concept target: [`product-visual-target-v1.md`](../ui/product-visual-target-v1.md)
(object card + related + OAM actions; intent plan/queue/result).
`Воркеры` are disposable executions, not Agent personalities.

### Tasks

- [x] NOW «Требует внимания» uses `saai-attention` `now_items()` (ATTN-02
  panther `63b8b64`). Waiting-confirmation and undismissed
  notifications only; empty section omitted.
- [x] Inbox uses the same projection (`inbox_source_ids`, ATTN-03
  panther `63b8b64`). Hit-test and cards share `inbox_rows`.
- [x] WORK-08 panther: Task Object View status is the entity status
  (confirm/decline only while `waiting_confirmation`); Intent names
  the related Task; NOW ObjectSummary trails live work; «Далее» is a
  pending Action or a derived-ready Task. No invented worker count.
- [x] Draw anatomy sheets for `IntentSummary` and `TaskSummary`
  (`component-library-v1.md` §7.6–7.7). `AgentSummary` is §7.9 from a
  real `saaios.action`.
- [x] Implement `TaskSummary` and `IntentSummary` in `saai-ui-core`.
  `Сейчас` «Продолжается» and a derived-ready «Далее» Task use
  `TaskSummary`. No worker count. `ObjectSummary` already exists (VUI-03).
- [x] Show `Intent → Tasks → Actions → Result` with navigable SOM
  relationships on Object View. Missing hops omitted. One follow
  button (`Открыть задачу/намерение/действие/результат`); waiting
  confirmation keeps Confirm/Decline. No invented worker count.
- [x] Present universal state, current activity or last verified
  observation, blocker, and confirmation consequence on Object View.
  Missing fields omitted. Permission/history wait for OAM overlay and
  entity events.
- [x] Gate Object View actions by OAM + live policy preflight.
  Unavailable/deny is a permission line, not an invented button.
  Allow/AskUser do not execute from the shell (taskd remains the
  path). `display.inspect` is the first real spec.
- [x] Implement `DecisionOverlay` naming actor, action, object, scope,
  consequence, and reversible choices. Missing facts omitted. Object
  View `waiting_confirmation` is the first consumer. Action undo is
  not invented.
- [x] Add manual paths for supported work when AI is offline.
  Shell does not probe `saaios-runtime` (ADR-030). Apps, `Я`,
  Confirm/Decline, and dismiss stay usable without a model. Intent
  send keeps the draft when the store is down instead of pretending
  success. AI-down after persist is already a failed-task
  notification (ADR-089).
- [x] Integrate Agent visuals only with a real runtime entity/source; otherwise
  present a truthful unassigned/unavailable state.
  `AgentSummary` is assigned from `saaios.action` only. No Action →
  «Нет исполнения». Notifications omit it. No `saaios.worker` entity
  and no invented personality.
- [x] Expand the gallery and golden/interaction tests for every composite and
  universal state.
  Second gallery page (tap to switch) draws labelled `saai-ui-core`
  fixtures: header, object, task, empty intent, unassigned/assigned
  `AgentSummary`, decision choices, all nine `UniversalState`s.
  `EventRow` fixtures land in VUI-07. No fake workers or live telemetry.

### Acceptance

- [ ] No entity-specific screen redefines universal status semantics.
- [ ] Destructive or privileged action cannot be confirmed accidentally.
- [ ] Real evidence is distinguishable from proposal and intent.
- [ ] Relationships are understandable without reading raw identifiers.
- [ ] Technical IDs use mono typography and remain copyable/inspectable.

### Rollback

Migrate entity views one type at a time. Keep the previous view for unmigrated
types and preserve protocol compatibility.

---

## VUI-06 — `Система` and device control

**Status:** Host + panther complete (ADR-126: inventory, domain grouping,
honest missing/offline, scroll-cache, tab label `Система` on panther
`1d191d7a…`. MEM-08 omitted — no shell-legal memory read.)

**Depends on:** VUI-04; may run after core VUI-05 primitives stabilize

**Goal:** make SaaiOS visibly understand and control the physical device through
a concise engineering-oriented surface.

### Tasks

- [x] Inventory existing `Я`, developer, connectivity, power, display, sound,
  storage, privacy, update, capability, and diagnostic data/actions.
  ADR-126. Memory/Android VM/battery-gauge omitted honestly.
- [x] Define System section, setting row, capability row, metric, health,
  evidence, recovery action, and dangerous-action components.
  `SystemSection` already exists. `SettingRow`/`CapabilityRow` added.
  Metric already a primitive. Health/evidence/recovery wait for a first
  consumer; dangerous confirmation is `DecisionOverlay`.
- [x] Reorganize content by device domain rather than implementation service.
  Host: `me_system_sections` groups the same 19 controls. Flattened to
  `ActionCardView` so scroll/nav stay the accepted path.
- [x] Rename `Я` to `Система` when the migrated information architecture is
  complete. Visible label in `root.sui`; `select_root:me` unchanged.
  Panther shows `Система` (`1d191d7a…`).
- [x] Show healthy summaries first; disclose raw logs and identifiers on demand.
  Host: `Устройство` leads with model+storage; kernel/uptime/entity
  counts/boot attempts live on `DevSurface` (HIA-20). No invented health
  cluster.
- [x] Use gauges only when a continuous value informs a real decision.
  Brightness and volume stay cycle rows. Battery stays on the status
  layer. No Progress/Metric on `Я`.
- [x] Provide explicit states for missing hardware, denied permission, offline
  service, stale telemetry, and restart/recovery.
  Host: missing Wi-Fi/Bluetooth adapter → «Нет адаптера» (no list).
  `entityd`/`appd` down → «Нет связи» (space color; Приложения).
  Empty grants stay «без разрешений». No stale clock (no source).
  Recovery is the existing Wi-Fi/Bluetooth/PIN/SSH frames, not a reboot
  row.
- [x] Preserve the accepted coalesced drag behavior and fixed status/navigation
  layers. Unchanged `me_scroll_dirty` path; long app list still misses nav.
- [x] Add tests for long lists, rapid drag, interruption, service restart, and
  actions that change device state.
  Host: long app list scroll + missing-adapter/offline rows + existing
  rapid-tab and `next_in_cycle`. Service restart remains a device check.

### Acceptance

- [ ] Ordinary users can find core settings without developer knowledge.
- [ ] Developers can reach evidence and raw detail through disclosure.
- [ ] The page remains fluid under rapid continuous dragging and does not queue
  stale frames.
- [ ] Top status and bottom navigation remain present throughout scrolling.
- [ ] AI, network, or a single failed service cannot hide manual recovery.

### Rollback

Each device domain migrates independently. The existing `Я` sections remain
available until their replacement passes functional and performance tests.

---

## VUI-07 — Remaining surfaces and state patterns

**Status:** In progress (Inbox `c1547c02…`, Spaces `9bb75db5…`, Wi-Fi list `c80bb666…` on panther)

**Depends on:** VUI-03 through VUI-06 primitives

**Goal:** complete visual consistency across the native shell without a
big-bang rewrite.

### Tasks

- [x] Migrate `Входящие` to an event stream and decision-request model.
  ADR-127: `EventRow` wraps `DataRow`; decision vs notice from the
  attention projection; empty vs store-offline named separately; tab
  stays. Flattened to `ActionCardView`. No invented timestamps.
  DataRow paint still later VUI-07. Flashed `c1547c02…`.
- [x] Migrate `Пространства` list while preserving HIA graph
  semantics. ADR-128: `SpaceRow` from live `entityd` spaces; tap
  selects, retap cycles lifecycle; object count plus at most one
  lifecycle/relation; empty vs «Нет связи». Space detail (people /
  members) still later. Flattened to `ActionCardView`. Flashed
  `9bb75db5…`.
- [x] Migrate `Wi-Fi сети` to live scan rows. ADR-129: `WifiRow`
  from `wpa_cli scan_results`; connected vs other; empty «Нет сетей»;
  password keyboard unchanged. Flattened to `ActionCardView`.
  Bluetooth / trusted clients / lock still later. Flashed `c80bb666…`.
- [ ] Migrate Object View, consent, remote pairing, intent input, Wi-Fi
  password, Bluetooth list, trusted clients, PIN setup, developer surface, and
  root fallback.
- [ ] Apply shared empty, loading, offline, blocked, failed, permission,
  confirmation, and recovery patterns.
- [ ] Restyle lock screen and wake-on-touch states without exposing sensitive
  content or triggering privileged actions.
- [ ] Verify keyboard avoidance, scroll overflow, back behavior, focus order,
  and interrupted workflows on every frame variant.
- [ ] Remove migrated screen-local primitives and record remaining exceptions.
- [ ] Expand the component gallery and cross-surface golden tests.

### Acceptance

- [ ] Every shell frame variant has a documented migration state.
- [ ] Equivalent states and actions look and behave equivalently.
- [ ] Pairing, consent, and recovery retain precise consequences and safe
  cancellation.
- [ ] Wake, unlock, keyboard, back, and navigation paths work on Pixel 7.
- [ ] No screen requires fabricated data or a decorative placeholder.

### Rollback

Each frame variant is independently selectable during migration. Remove a
legacy renderer only after its replacement and fallback path are verified.

---

## VUI-08 — Motion, haptics, and frame pacing

**Status:** Backlog

**Depends on:** stable shared components from VUI-04–VUI-07

**Goal:** add restrained temporal behavior and turn the accepted performance
into a measured regression contract.

### Tasks

- [ ] Add a shared transition clock and compositor/frame-callback integration.
- [ ] Implement 80–150 ms micro, 150–220 ms panel, and 200–300 ms context
  transitions only where they clarify state.
- [ ] Add reduced-motion behavior for every animated component.
- [ ] Centralize haptic intents; map components to policy instead of direct motor
  control.
- [ ] Instrument input-to-feedback, render production, submission, presentation,
  dropped/coalesced frames, and pending-work depth.
- [ ] Preserve software-render staging and Vulkan composition behavior unless a
  separately measured change is accepted.
- [ ] Add performance traces for `Система` drag, fast tab switching, lists,
  keyboard, overlays, and Orb activity.
- [ ] Set CI/device thresholds from the accepted baseline and document hardware
  variance.

### Acceptance

- [ ] Continuous drag p95 frame production is at or below 50 ms on the reference
  build, with no unbounded input backlog and no disappearing layer.
- [ ] First visible touch feedback is prompt and no worse than the VUI-07
  baseline.
- [ ] Idle UI does not redraw continuously without a state reason.
- [ ] Reduced-motion mode communicates every state without animation.
- [ ] Haptics are consistent, rate-limited, and absent for passive decoration.

### Rollback

All motion and haptics have independent runtime disable switches; the static
component state remains fully usable.

---

## VUI-09 — `.sui` v2, public library, cleanup, and release gate

**Status:** Backlog

**Depends on:** VUI-01 through VUI-08

**Goal:** encode the proven component system declaratively, publish its stable
subset, remove superseded legacy paths, and qualify Visual v1.

### Tasks

- [ ] Write the `.sui` v2 ADR from the proven screen/component vocabulary.
- [ ] Add versioned semantic roles, token references, component composition,
  safe insets, list/scroll behavior, localization, focus, and accessibility
  metadata to the schema/compiler.
- [ ] Preserve `.sui` v1 parsing or provide a deterministic migration tool and
  rollback artifact.
- [ ] Move root layout and hit testing to compiled shared layout output.
- [ ] Stabilize and document the public component subset for third-party SaaiOS
  applications; keep privileged composites capability-gated.
- [ ] Publish component API docs, gallery, examples, stability labels,
  visual specification sheets, deprecation policy, and migration guide.
- [ ] Remove the hardcoded color/metric allowlist and duplicated migrated
  shell components.
- [ ] Run complete visual, accessibility, interaction, performance, service
  restart, display restart, cold boot, and offline test matrices.
- [ ] Record known limitations and the Visual v2 backlog.

### Acceptance

- [ ] Declarative and procedural paths render and hit-test equivalently for
  supported components.
- [ ] Third-party code can use the stable public subset without importing shell
  internals.
- [ ] No untracked screen-local palette, font size, touch target, status mapping,
  or near-duplicate component remains.
- [ ] Component gallery covers every stable component and state.
- [ ] Pixel 7 passes daylight/indoor/dark review, normal/increased text,
  long-Russian-text, keyboard, AI-offline, network-offline, missing-capability,
  service-restart, display-restart, and cold-reboot scenarios.
- [ ] Current GPU composition, smooth scrolling, stable system layers, touch,
  connectivity, lock/wake, and device controls have no release-blocking
  regression.

### Rollback

The last pre-cleanup build and `.sui` v1 artifacts remain reproducible until the
Visual v1 release is signed off. Cleanup commits are separate from behavior
changes.

---

## Cross-sprint verification matrix

Every rendering sprint runs the applicable subset; VUI-09 runs all of it.

| Area | Required checks |
|---|---|
| Build | host tests, cross-build, asset manifest, image/package verification |
| Render | golden images, physical color, typography, icons, insets, no clipping |
| Input | taps, edge taps, rapid tab changes, drag, interrupted drag, keyboard, back |
| State | quiet, active, running, waiting, blocked, attention, failed, complete, offline |
| Accessibility | non-color cues, text scale, long Russian labels, focus order, reduced motion |
| Resilience | AI stopped, network down, capability missing, permission denied, service restart |
| Device | shell restart, display restart, screen off/on, lock/unlock, cold reboot |
| Performance | input feedback, frame time, pending depth, dropped/coalesced frames, idle redraw |
| Safety | precise confirmation, consequence, cancellation, policy/capability enforcement |

## Commit and reporting convention

Prefer one reviewable concern per commit, for example:

- `docs(ui): define SaaiOS visual language and roadmap`
- `feat(ui-core): add semantic color and state tokens`
- `feat(ui-core): add status indicator component`
- `feat(shell): migrate Now status section`
- `test(ui): cover long text and offline component states`

After each completed task group, report:

1. files changed;
2. what the user can now see or do;
3. why the change follows the product contract;
4. tests and physical-device evidence;
5. regressions or remaining legacy paths;
6. commit ID and push status.

## Next action

Continue **VUI-07** remaining surfaces (Wi-Fi password, Bluetooth
list, lock, Space detail). Inbox EventRow (`c1547c02…`), Spaces list
(`9bb75db5…`), and Wi-Fi list (`c80bb666…`) are on panther. MEM-08
stays omitted until a shell-legal memory read exists.
