# SaaiOS Visual Language v1

Status: Approved target; implementation is tracked in
[`VISUAL-ROADMAP.md`](../sprints/VISUAL-ROADMAP.md).

This document defines how SaaiOS looks, communicates state, and behaves on a
handheld screen. It does not replace the
[Human Interface Architecture v2](human-interface-architecture-v2.md) (HIA).
HIA remains authoritative for the product model, objects, spaces, intents,
actions, Context Light, and Orb. This document is the visual and interaction
contract used to implement that model consistently.

The first-version **product destination** (Orb states, five surfaces,
object types, semantic color) is the accepted concept boards in
[`product-visual-target-v1.md`](../ui/product-visual-target-v1.md).
Those boards are the UI endpoint for v1. This file still governs how
that destination is implemented without fake activity, without a chat
home, and without replacing HIA.

## 1. Product goal

SaaiOS must feel like an AI-native operating environment that understands it is
running a physical device. It must not feel like Android with a different
launcher, a chatbot stretched to full screen, a desktop scaled down to a phone,
or a decorative science-fiction HUD.

The target character is:

- minimal, technical, and calm;
- informed by industrial computing, scientific instruments, and
  mission-critical interfaces;
- dense only where the information has real value;
- readable at a glance and usable without AI;
- distinctively SaaiOS rather than an imitation of Android, GNOME, KDE, XFCE,
  or a terminal.

Within one or two seconds the current surface should answer:

1. Where am I?
2. What am I doing?
3. What is the system doing?
4. What needs attention?
5. What can I do next?

Chat is one possible interaction, not the operating system's identity. Every
essential device operation needs a direct, predictable manual path. When AI is
offline or unavailable, the core OS remains understandable and usable.

## 2. Decision hierarchy

When source documents or implementations disagree, use this order:

1. safety, truthfulness, reversibility, and explicit user control;
2. HIA's entity and capability model;
3. clarity, accessibility, and direct manipulation on Pixel 7;
4. this visual language;
5. compatibility with the current implementation;
6. decorative consistency.

Never display invented agents, telemetry, progress, diagnostics, graphs, or
system activity to make a screen look richer. A truthful empty state is better
than fake activity.

## 3. Resolved design questions

### 3.1 Context color and status color

HIA assigns color to context in Context Light. The visual brief assigns color
to semantic state. Both rules are retained with strict scope:

- inside Context Light and the Orb nucleus, color identifies context;
- everywhere else, color communicates semantic state;
- status is never expressed by color alone: use a label, icon, shape, or motion
  cue as well;
- context color is not reused for ordinary buttons or severity.

### 3.2 Orb

The Orb is a small, precise system locus. It may summarize context, activity,
progress, or attention according to HIA. It is not a giant assistant avatar,
launcher button, glowing decoration, or substitute for navigation. Its quiet
state must be genuinely quiet.

On `Сейчас` the Orb may sit in the composer as the primary locus. On every
other surface it stays compact. Orb states (idle, listen, analyze, plan,
confirm, run, result) present workflow; they do not make Orb the Scheduler
or Policy. See [product-visual-target-v1.md](../ui/product-visual-target-v1.md).

### 3.3 Home and applications

`Сейчас` is a current-state surface, not an application grid. Installed
applications remain reachable through an explicit secondary `Приложения`
entry or sheet. The grid must never become the primary SaaiOS mental model.

### 3.4 Navigation

Navigation names stable OS domains, not implementation modules. During the
migration the existing four-tab model remains usable. The v1 destination
(concept boards) is:

- `Сейчас` — current context, work, attention, and next action;
- `Пространства` — user contexts and their objects;
- `Поиск` — find people, objects, intents (not a sixth OS);
- `Система` — the device, settings, capabilities, and diagnostics.

`Входящие` is not a primary tab in v1; decision/attention items live on
`Сейчас`. Object View and Intent View are reached from those domains, not
as extra tabs. The current `Я` tab is renamed only after its content has
been reorganized into the truthful `Система` model. Agent or application
tabs are not added without a real data source and a primary user need.

### 3.5 Transparency and blur

Visual v1 uses opaque or near-opaque surfaces. Blur and translucency are not
simulated. They may be introduced only after the compositor supports them
correctly, they preserve contrast, and measurements show no input or frame
pacing regression.

### 3.6 Display color

The values in this document are logical design tokens. Pixel 7 output must be
checked on the physical panel because scanout format, transfer behavior, and
camera photographs can shift perceived color. Calibration changes the display
pipeline or mapped output, not the semantic palette in screen code.

## 4. Visual principles

### 4.1 State first

Show the current state and its consequence before metadata. Put the next safe
action close to the explanation. Developer detail is progressively disclosed.

### 4.2 Structure before containers

Use spacing, alignment, typography, fine dividers, and section labels before
adding a card. Cards are reserved for bounded objects, decisions, or actions
that benefit from a clear container. Do not wrap every row in a rounded box.

### 4.3 Strict but humane geometry

Prefer aligned edges, modest radii, thin borders, and a stable rhythm. Avoid
giant pills, oversized empty panels, ornamental brackets, dense grid overlays,
and sharp corners on primary touch controls.

### 4.4 Calm density

The screen may expose meaningful technical information without becoming a
dashboard. Empty space establishes grouping. Repeated chrome never competes
with the current task.

### 4.5 Terminal heritage in the right place

Monospace typography and terminal-like presentation belong to boot,
deployment, diagnostics, recovery, commands, identifiers, logs, and raw
telemetry. Normal navigation and prose remain a graphical phone interface.

### 4.6 Reversible evolution

The visual system is introduced component by component and screen by screen.
Business logic, protocols, and stored data do not change as an accidental side
effect of restyling. Every sprint retains a known-good rollback boundary.

## 5. Color system

### 5.1 Core tokens

| Token | Value | Purpose |
|---|---:|---|
| `color.bg.canvas` | `#071011` | Root background |
| `color.bg.surface` | `#0D181A` | Primary surface |
| `color.bg.elevated` | `#142326` | Raised or selected surface |
| `color.accent.primary` | `#63D4D6` | Active control and focus |
| `color.accent.highlight` | `#A1EEF0` | High-emphasis accent detail |
| `color.text.primary` | `#D7E2DF` | Primary text |
| `color.text.secondary` | `#829796` | Secondary text and inactive data |
| `color.state.success` | `#6FB79A` | Completed or healthy |
| `color.state.attention` | `#D4B658` | Waiting, blocked, or needs attention |
| `color.state.critical` | `#C7514B` | Failure, destructive risk, or denial |
| `color.border.default` | `#315054` | Functional border and divider |
| `color.grid.subtle` | `#20383A` | Sparse structural guide |

Typical screen area is approximately 80–90% dark backgrounds, 5–10% text and
structure, and 1–5% semantic accent. This is a composition guideline, not a
render-time quota.

All derived colors, disabled colors, pressed colors, and alpha variants belong
to the theme implementation. Screen renderers must not create local ad-hoc RGB
values.

### 5.2 Universal state vocabulary

| State | Primary cue | Meaning |
|---|---|---|
| `IDLE` | secondary grey + label | Available, not working |
| `ACTIVE` | cyan + active marker | Selected or presently engaged |
| `RUNNING` | cyan + restrained motion | Work is progressing |
| `WAITING` | secondary grey/amber + reason | Waiting for time or dependency |
| `BLOCKED` | amber + blocking reason | User or dependency action is required |
| `ATTENTION` | amber + attention marker | Review is needed, no failure yet |
| `FAILED` | critical red + failure label | Work did not complete |
| `COMPLETE` | green + completion marker | Verified completion |
| `OFFLINE` | secondary grey + offline label | Resource is unavailable by network |

`BLOCKED` and `ATTENTION` are not failures. Red is reserved for actual failure,
destructive risk, or denied access. Neutral important information uses primary
text, not an accent color.

## 6. Typography

Typography carries most of the hierarchy. Concrete font files are replaceable;
semantic roles are stable.

| Role | Typical logical size | Use |
|---|---:|---|
| `display` | 28–32 | Time, lock screen, rare hero state |
| `title` | 22–26 | Screen title |
| `section` | 16–18 | Section and object title |
| `body` | 14–16 | Primary readable content |
| `label` | 12–14 | Controls and metadata labels |
| `caption` | 11–12 | Secondary annotation |
| `mono.body` | 12–15 | Commands, addresses, identifiers, telemetry |

Use a legible sans-serif family for interface and prose. Use a licensed static
monospace family with complete Latin and Cyrillic coverage for technical data.
Montserrat remains the safe fallback until device comparison proves a better
choice. A font change is complete only when the asset license, boot image,
fallback behavior, Cyrillic shaping, font scaling, and physical rendering have
all been verified.

Do not use uppercase everywhere, excessively expanded tracking, fake terminal
letter spacing, or tiny captions as a substitute for hierarchy.

## 7. Geometry, spacing, and touch

- Layout uses logical units and safe insets; Pixel 7 framebuffer pixels are not
  treated as density-independent measurements.
- The base spacing rhythm is 4 logical units, with common steps 4, 8, 12, 16,
  24, and 32.
- Primary touch targets are at least 48 by 48 logical units; compact controls
  may render smaller but retain the full hit region.
- Common corner radii are small or moderate. Fully rounded pills are reserved
  for compact status and segmented controls where the shape carries meaning.
- Borders and dividers are normally one logical unit and remain subordinate to
  text.
- Long Russian labels, system font scaling, the on-screen keyboard, scroll
  overflow, and the bottom navigation inset are first-class layout cases.
- Visual bounds and hit-test bounds must derive from the same layout result.

## 8. Motion and haptics

Motion explains change; it does not decorate idle screens.

- micro feedback: 80–150 ms;
- panel or selection transition: 150–220 ms;
- context transition: 200–300 ms;
- running state: quiet, low-frequency, and cancellable;
- reduced-motion mode: replaces nonessential transitions with immediate state
  changes.

Avoid bounce, elastic overshoot, continuous glow, animated backgrounds, noisy
particle effects, and motion that implies nonexistent work. Touch feedback must
begin promptly and use existing haptic policy rather than each component
driving the motor independently.

## 9. Information architecture

### 9.1 Сейчас

The reference home surface contains only real information, in this order:

1. current context or Space;
2. current intent or task, if one exists;
3. what the system is doing now;
4. items requiring attention;
5. the next direct action;
6. a restrained system summary and secondary `Приложения` entry.

It supports honest quiet, empty, offline, blocked, and failure states. A prompt
field may be available but must not dominate the page or hide manual actions.

### 9.2 Входящие / внимание

Not a primary tab in v1. Decision requests and event history appear on
`Сейчас` (and on the Object they concern). Each decision states the actor,
intended action, affected object, scope, consequence, and reversible choices.

### 9.3 Пространства

Shows user contexts and their meaningful objects. Context Light may express the
active Space. Navigation follows the Space graph described by HIA and exposes a
clear return path. The Space surface lists focus work, people, and related
objects (SOM members), not an Android-style app drawer.

### 9.4 Object, Intent, Task, and Worker

These surfaces share one visual grammar:

- identity and universal state;
- relationship path: `Intent → Tasks → Actions → Result` (SOM);
- current activity or last verified observation;
- blockers, permissions, and consequences;
- history and technical details on demand;
- direct actions permitted by OAM and policy.

`Очередь` / `Воркеры` on the Intent board are derived ready work and
disposable executions (ADR-121), not persistent Agent entities. Do not
invent workers or agents to fill the layout.

### 9.5 Система

`Система` is the engineering view of the physical device for ordinary users and
developers at appropriate disclosure levels. It groups connectivity, power,
display, sound, storage, privacy, updates, capabilities, and diagnostics.
Healthy state is concise. Problems show cause, effect, evidence, and a direct
recovery action. Do not add gauges unless their continuously changing value is
useful for a decision.

### 9.6 Lock screen

The lock screen prioritizes time, device state, essential attention, and a
clear unlock affordance. Wake-on-touch must not perform a destructive or
privileged action. Sensitive content respects lock-state policy.

## 10. SaaiOS component library

The component library is a product deliverable, not a collection of drawing
helpers. Its implementation lives with the Saai UI stack and remains usable by
the native shell and future third-party surfaces without copying shell code.

### 10.1 Layers

1. **Foundations** — color, type, spacing, radius, stroke, elevation, motion,
   safe-inset, and interaction tokens.
2. **Layout primitives** — stack, row, inset, separator, scroll region, and
   focus/order primitives sharing layout and hit testing.
3. **Visual primitives** — text, icon, status mark, progress, button, field,
   data row, metric, and disclosure control.
4. **System composites** — context header, system section, object summary,
   intent summary, task summary, agent summary, event row, decision overlay,
   bottom navigation, and Orb host.
5. **Patterns** — empty, loading, offline, blocked, failed, confirmation,
   permission, recovery, and progressive-disclosure patterns.

### 10.2 Required component contract

Every public component defines:

- a reviewed visual specification showing anatomy, measurements, variants,
  states, and responsive behavior before its API is declared stable;
- semantic purpose and when not to use it;
- inputs, emitted actions, and state model;
- layout, touch, focus, and scroll behavior;
- all supported universal states;
- compact, normal, long-text, scaled-text, disabled, offline, and error cases;
- accessibility name, value, role, and non-color cue;
- motion and haptic behavior;
- golden/reference render and interaction tests;
- whether it is stable, experimental, or shell-private.

Specifications may begin as wireframes, but the approved version must use the
real palette, type roles, spacing, iconography, Russian labels, and Pixel 7
viewport. They are stored with the library documentation and updated in the
same commit whenever a public component's appearance or behavior changes.

### 10.3 Ownership and boundaries

- `saai-ui-core` owns semantic tokens, layout primitives, component contracts,
  and shared hit-test behavior.
- `saai-ui-compiler` and `.sui` gain declarative component/style support only
  after the reference screen proves the vocabulary.
- `saai-shell` composes components and supplies real system state; it must not
  become the permanent owner of reusable visuals.
- render backends translate tokens and primitives to pixels; they do not decide
  product semantics.
- third-party applications use the public stable subset. Privileged system
  composites remain capability-gated.

The library includes a device-runnable component gallery that displays every
state with real or explicitly labelled fixture data. The gallery is a review
and regression tool, not a user-facing application.

### 10.4 Evolution rule

A reusable component is extracted after it works in one reference surface and
has a confirmed second use. Foundations and obvious primitives may be created
earlier. Do not build abstraction for its own sake, and do not allow screens to
fork near-identical private components to avoid the library contract.

## 11. Accessibility and resilience

- Text and status contrast is checked against its actual surface.
- Color never carries the only meaning.
- Focus order, screen-reader semantics, and hardware-button alternatives are
  defined even before a screen reader is fully integrated.
- Font scaling and long localization strings may grow rows and sections rather
  than clip text.
- AI unavailable, network offline, capability missing, permission denied,
  service restart, and stale data each have distinct truthful states.
- Essential system actions remain available through deterministic manual
  navigation.
- Confirmations name the action and consequence; destructive actions are never
  disguised as generic `OK`.

## 12. Performance contract

Visual work must preserve the accepted GPU/compositor path and the now-fluid
`Система`/former `Я` page scrolling behavior.

- Input state is coalesced; dragging must not build an unbounded render queue.
- Status and navigation layers remain visible during scroll and redraw.
- Idle surfaces do not animate or redraw continuously without a real state
  reason.
- Component abstraction must not add per-frame allocation or duplicated full
  frame work without measurement.
- VUI-08 establishes repeatable instrumentation; until then, every migration
  must be no worse than the accepted device baseline.

## 13. Current implementation audit (2026-09-17)

| Area | Current state | Required direction |
|---|---|---|
| Root declaration | `services/saai-shell/ui/root.sui` describes root content/actions/tabs only | Add semantics incrementally after the component vocabulary is proven |
| UI core | `saai-ui-core` provides basic nodes, layout, rectangles, and hit testing | Add tokens, roles, insets, scrolling, and stable components |
| Compiler | `.sui` parser targets the root schema | Evolve to a versioned declarative component grammar; retain fallback |
| Screens | 11 shell frame variants are largely procedural | Migrate one surface at a time through shared components |
| Color | Seven renderer constants plus local RGB values | One semantic theme with an explicit temporary allowlist |
| Typography | Montserrat Regular/Semibold | Semantic roles, verified fallback, and a licensed Cyrillic mono face |
| Icons | No unified icon asset pipeline | One line-icon family and build synchronization checks |
| Scaling | 1080×2400 reference coordinates and literal sizes | Logical metrics, safe insets, and shared layout/hit-test output |
| Accessibility | Global text scaling and contrast postprocess exist | Per-component semantics, reflow, focus, and non-color status cues |
| Rendering | CPU raster staging into DMA-BUF, GPU composition | Preserve working path; optimize only with measurements |

## 14. Explicit non-goals

Visual v1 does not:

- clone Android, iOS, KDE, GNOME, XFCE, or an existing design system;
- import another toolkit's widgets as SaaiOS identity;
- replace the object/capability/policy architecture with applications;
- turn every screen into chat;
- use neon cyberpunk, Matrix motifs, fake diagnostics, ornamental graphs,
  permanent grid backgrounds, or excessive glow;
- require compositor blur or transparency;
- rewrite every screen in one change;
- change protocols, authorization, storage, or hardware behavior merely to fit
  a mock-up.

External systems are references for solved interaction problems. SaaiOS owns
its model, component contracts, code, and visual identity.

## 15. Definition of Visual Done

A surface is visually complete only when:

- it uses semantic tokens and library components or documents a temporary
  exception;
- it exposes only real state and provides a manual path for essential actions;
- normal, empty, loading, offline, blocked, failed, and scaled-text cases are
  reviewed as applicable;
- touch targets, hit testing, scroll bounds, insets, focus order, and long
  Russian text are verified;
- it has reference renders and interaction tests;
- it is tested on Pixel 7 for perceived color, legibility, touch, frame pacing,
  status/navigation stability, shell restart, and cold reboot;
- no accepted device behavior or GPU/compositor regression is introduced;
- documentation, migration notes, rollback path, commit, and remote push are
  complete.
