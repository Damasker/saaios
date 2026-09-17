# SaaiOS Visual System — delivery roadmap

Status: **VUI-00 complete (planning); VUI-01 ready**

Target device: Pixel 7 (`panther`)

Product contract: [`visual-language-v1.md`](../architecture/visual-language-v1.md)

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
| VUI-01 | Semantic tokens and physically calibrated palette | **Ready** |
| VUI-02 | Typography, geometry, icons, and base component library | Backlog |
| VUI-03 | Reference `Сейчас` surface | Backlog |
| VUI-04 | Navigation, status surfaces, Context Light, and restrained Orb | Backlog |
| VUI-05 | Object, Intent, Task, and real Agent components | Backlog |
| VUI-06 | `Система` information architecture and settings components | Backlog |
| VUI-07 | Remaining system surfaces and state patterns | Backlog |
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

**Status:** Ready

**Depends on:** VUI-00

**Goal:** create one semantic source of truth for color and universal state
without changing the information architecture or layout.

### Tasks

- [ ] Write an ADR for theme ownership, compatibility, and the temporary
  hardcoded-value allowlist.
- [ ] Introduce typed color tokens from Visual Language v1 in `saai-ui-core`.
- [ ] Define typed universal state values: `IDLE`, `ACTIVE`, `RUNNING`,
  `WAITING`, `BLOCKED`, `ATTENTION`, `FAILED`, `COMPLETE`, and `OFFLINE`.
- [ ] Map universal states to color plus text/icon/shape cues.
- [ ] Keep Context Light/Orb context color separate from semantic status color.
- [ ] Replace the renderer's seven global constants with the theme interface.
- [ ] Move local shell RGB values behind theme or the documented context-color
  boundary.
- [ ] Add pressed, focused, disabled, and high-contrast derived tokens.
- [ ] Create a full-screen palette/state calibration fixture.
- [ ] Add deterministic token and state-mapping tests.
- [ ] Capture a reference render and physical Pixel 7 photographs under normal
  indoor light; record display-pipeline adjustments separately from tokens.
- [ ] Document migration and rollback.

### Acceptance

- [ ] Existing screen geometry and navigation behavior are unchanged.
- [ ] No migrated component creates ad-hoc RGB values.
- [ ] Every state remains distinguishable without color.
- [ ] Text and controls meet the agreed contrast checks on actual surfaces.
- [ ] Pixel 7 shows the intended neutral dark/cyan palette without channel
  swap, crushing, or unintended warm cast.
- [ ] Touch, current smooth `Система`/`Я` scrolling, status layer, and bottom
  navigation behave as before.
- [ ] Shell restart and cold reboot recover the themed UI.

### Rollback

One build switch restores the existing renderer palette. No entity, protocol,
storage, or authorization changes are included.

---

## VUI-02 — Typography, geometry, icons, and base components

**Status:** Backlog

**Depends on:** VUI-01

**Goal:** establish the reusable foundations and primitives needed to build a
real phone interface consistently.

### Tasks

- [ ] Add semantic typography roles independent of font filenames and raw
  point sizes.
- [ ] Compare the current Montserrat build with candidate UI faces on Pixel 7;
  keep Montserrat unless another face is demonstrably more readable.
- [ ] Select and license a static monospace face with Latin and Cyrillic
  coverage; verify `fontdue`, fallback, scaling, and boot-image packaging.
- [ ] Define spacing, radius, stroke, touch-target, safe-inset, and elevation
  tokens in logical units.
- [ ] Add one coherent line-icon source and a reproducible asset pipeline.
- [ ] Implement shared layout primitives: stack, row, inset, separator, scroll
  region, and focus/order metadata.
- [ ] Implement visual primitives: semantic text, icon, divider,
  `StatusIndicator`, progress, button, field, `DataRow`, metric, and disclosure.
- [ ] Make visual and hit-test bounds consume the same layout output.
- [ ] Define component accessibility names, roles, values, disabled states, and
  non-color cues.
- [ ] Build the first device component-gallery surface covering all primitive
  states, long Russian strings, and scaled text.
- [ ] Add golden render, layout, hit-test, press-state, and overflow tests.

### Acceptance

- [ ] Public primitives contain no shell-specific business logic.
- [ ] Primary touch targets are at least 48×48 logical units.
- [ ] Long labels wrap or reflow; they do not clip or overlap navigation.
- [ ] The sans and mono faces survive the actual Pixel boot-image asset path.
- [ ] Components look like one family at normal and increased text scale.
- [ ] Component gallery is runnable on device and clearly labels fixture data.
- [ ] No scrolling or frame-pacing regression against the VUI-01 baseline.

### Rollback

Primitives remain opt-in; existing screen drawing stays available until each
screen migration is accepted.

---

## VUI-03 — Reference `Сейчас` surface

**Status:** Backlog

**Depends on:** VUI-02

**Goal:** prove the visual language and component vocabulary on one useful,
truthful home surface before expanding the framework.

### Tasks

- [ ] Inventory the real sources for active Space, current Intent/Task, system
  activity, attention, and next action.
- [ ] Design `ContextHeader`, `SystemSection`, object/work summary, event row,
  and next-action components from actual data.
- [ ] Compose `Сейчас` so its first viewport answers the five product questions.
- [ ] Move the application grid behind an explicit secondary `Приложения`
  entry or sheet without removing application access.
- [ ] Keep prompt/chat input secondary to direct system actions.
- [ ] Implement honest quiet, empty, stale, offline, blocked, and failed states.
- [ ] Validate progressive disclosure for developer and diagnostic details.
- [ ] Extract only components with a confirmed second use; document candidates
  that intentionally remain reference-screen private.
- [ ] Add reference renders and end-to-end touch/navigation tests.

### Acceptance

- [ ] No fake tasks, agents, metrics, progress, or alerts appear.
- [ ] The first viewport states context, current work/system activity,
  attention, and next action when the data exists.
- [ ] Essential actions remain reachable without chat or AI.
- [ ] Application access remains discoverable but is not the primary model.
- [ ] Empty state is calm and useful rather than filled with decoration.
- [ ] Physical Pixel 7 review covers portrait insets, keyboard, touch, long text,
  and one-handed reach.

### Rollback

Keep the previous root composition selectable until the reference surface passes
the complete device checklist.

---

## VUI-04 — Navigation, status, Context Light, and Orb

**Status:** Backlog

**Depends on:** VUI-03

**Goal:** make system orientation stable and distinctively SaaiOS without
turning the Orb into a launcher or assistant avatar.

### Tasks

- [ ] Implement shared bottom-navigation and system-status components with safe
  insets and stable layering.
- [ ] Preserve `Сейчас`, `Входящие`, and `Пространства`; stage `Я` → `Система`
  only when the destination content is truthful.
- [ ] Add explicit selected, pressed, disabled, attention, and badge states.
- [ ] Apply Context Light grammar: context=color, state=shape,
  activity=motion, quantity=arc/fill, attention=ring.
- [ ] Integrate a restrained Orb host with quiet, active, progress, attention,
  offline, and reduced-motion states.
- [ ] Retain direct tab navigation during Orb work; do not make the Orb the only
  route.
- [ ] Make status/navigation layers independent of scrolling content damage.
- [ ] Add rotation/inset/keyboard and rapid-tab-switch interaction tests.

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

## VUI-05 — Object, Intent, Task, and Agent components

**Status:** Backlog

**Depends on:** VUI-04 and the relevant HIA entity/runtime support

**Goal:** give work and objects a consistent, inspectable, action-oriented
visual grammar.

### Tasks

- [ ] Implement `ObjectSummary`, `IntentSummary`, `TaskSummary`, `AgentSummary`,
  action row, observation/evidence row, and relationship path components.
- [ ] Show `Intent → Tasks → Agents → Actions` with navigable relationships.
- [ ] Present universal state, current activity, last verified observation,
  blocker, permissions, consequences, and history consistently.
- [ ] Gate actions by capability and policy; explain unavailable actions.
- [ ] Implement decision and confirmation overlays that name actor, action,
  object, scope, consequence, and reversibility.
- [ ] Add manual paths for supported work when AI is offline.
- [ ] Integrate Agent visuals only with a real runtime entity/source; otherwise
  present a truthful unassigned/unavailable state.
- [ ] Expand the gallery and golden/interaction tests for every composite and
  universal state.

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

**Status:** Backlog

**Depends on:** VUI-04; may run after core VUI-05 primitives stabilize

**Goal:** make SaaiOS visibly understand and control the physical device through
a concise engineering-oriented surface.

### Tasks

- [ ] Inventory existing `Я`, developer, connectivity, power, display, sound,
  storage, privacy, update, capability, and diagnostic data/actions.
- [ ] Define System section, setting row, capability row, metric, health,
  evidence, recovery action, and dangerous-action components.
- [ ] Reorganize content by device domain rather than implementation service.
- [ ] Rename `Я` to `Система` when the migrated information architecture is
  complete.
- [ ] Show healthy summaries first; disclose raw logs and identifiers on demand.
- [ ] Use gauges only when a continuous value informs a real decision.
- [ ] Provide explicit states for missing hardware, denied permission, offline
  service, stale telemetry, and restart/recovery.
- [ ] Preserve the accepted coalesced drag behavior and fixed status/navigation
  layers.
- [ ] Add tests for long lists, rapid drag, interruption, service restart, and
  actions that change device state.

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

**Status:** Backlog

**Depends on:** VUI-03 through VUI-06 primitives

**Goal:** complete visual consistency across the native shell without a
big-bang rewrite.

### Tasks

- [ ] Migrate `Входящие` to an event stream and decision-request model.
- [ ] Migrate `Пространства` and Space detail while preserving HIA graph
  semantics.
- [ ] Migrate Object View, consent, remote pairing, intent input, Wi-Fi list and
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
  deprecation policy, and migration guide.
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

Start **VUI-01**. It changes only theme/state foundations and a calibration
fixture; it must not redesign navigation or content yet. That produces a small,
measurable first implementation step and the base required by the full SaaiOS
graphical component library.
