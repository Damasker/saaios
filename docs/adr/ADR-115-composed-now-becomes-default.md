# ADR-115: composed `Сейчас` becomes the real default, dev gate removed

## Status

Accepted, 2026-09-17. Physically verified on Pixel 7 -- this is now the
actual boot-time default, not a preview behind a marker.

## Context

ADR-112 deliberately shipped the composed `Сейчас` behind a dev-only
`SAAIOS_UI_NOW_COMPOSED`/`/run/saaios/ui-now-composed` marker, off by
default, specifically to avoid combining a display change with an
access-relocation change in one commit. ADR-113 then relocated the app
grid behind "Приложения" -- but that relocation only took effect while
the same marker was set, so in real production the app grid remained
`RootPage::Now`'s only, permanent content the whole time.

Reviewing VUI-03's own Acceptance checklist against this made the gap
concrete: criteria like "Application access remains discoverable but is
not the primary model" can only be honestly true of what actually ships
by default, not of a hidden preview. Raised this to the user rather than
silently checking those criteria off against dev-only behavior; the
decision was to flip the default now, closing the loop ADR-112 explicitly
left open.

## Decision

Removed `NOW_COMPOSED_MARKER`/`SAAIOS_UI_NOW_COMPOSED` and the
`now_composed: bool` `Shell` field entirely -- not just flipped a default
value, deleted the gate. `RootPage::Now`'s frame construction and touch
dispatch both simplify from `self.current_page == RootPage::Now &&
self.now_composed && !self.apps_open` to `self.current_page ==
RootPage::Now && !self.apps_open`: the composed screen is now
unconditional, with the app grid reachable only through `apps_open`
(ADR-113's "Приложения" row), never as `RootPage::Now`'s own default
content again.

Deliberately full removal rather than leaving the marker as permanent
dead-but-harmless scaffolding: unlike `UI_CALIBRATION_MARKER`/
`UI_GALLERY_MARKER`/`DEV_NO_LOCK_MARKER` (each a genuinely reusable
developer convenience with no planned expiry), `NOW_COMPOSED_MARKER` was
always scoped as a temporary staged-rollout gate for exactly this
transition -- keeping it around after the rollout it gated would be
dead code with no future purpose, not a feature.

## Verification

- `cargo test -p saai-shell`: 96/96, unchanged.
- `cargo clippy -p saai-shell --all-targets -- -D warnings`: clean.
- Hot-swapped onto the live device (backup `saai-shell.pre-nowdefault`,
  hash-verified). Restarted the shell with **no marker set at all** --
  confirmed via screenshot that the composed `Сейчас` (header, empty
  state, "Приложения"/"Новое намерение" footer rows) renders as the
  real, unconditional default for the first time.

## VUI-03 Acceptance, re-reviewed against the real default

- **No fake tasks, agents, metrics, progress, or alerts appear.** True by
  construction throughout ADR-112/113/114 -- every row traces to a real
  entity or a real connection-state check, never fabricated.
- **The first viewport states context, current work/system activity,
  attention, and next action when the data exists.** `ContextHeader`
  (context), "Продолжается" (current work/system activity), "Требует
  внимания" (attention), "Далее" (next action) -- all real, all
  physically confirmed with actual entity data (ADR-112).
- **Essential actions remain reachable without chat or AI.**
  "Приложения" and root-tab navigation both work with `intent_input`
  never opened.
- **Application access remains discoverable but is not the primary
  model.** Only now genuinely true: "Приложения" is a single, clearly
  labelled, always-visible row, one tap from any state of `Сейчас` --
  and, as of this ADR, no longer the page's default content.
- **Empty state is calm and useful rather than filled with decoration.**
  "Ничего срочного" (ADR-112), "Нет связи с пространствами" when honest
  about not knowing (ADR-114) -- centered text, no invented filler.
- **Physical Pixel 7 review covers portrait insets, keyboard, touch, long
  text, and one-handed reach.** Portrait insets: the status-bar-overlap
  bug found and fixed in ADR-112. Keyboard: the intent-input keyboard
  confirmed opening correctly from its new row (ADR-113). Touch: footer
  row hit-testing and the app-grid open/close round trip both physically
  confirmed (ADR-113). Long text: not freshly stress-tested on this
  specific screen with a dedicated long string -- relies on `SemanticText`
  wrap already being proven by VUI-02's own gallery fixture and reused
  verbatim here (`draw_semantic_text`), not a new claim. One-handed
  reach: footer rows and tabs both meet `MIN_TOUCH_TARGET` (verified
  structurally, ADR-108) and sit in the lower half of the screen.

All six items now checked off honestly, against real shipped behavior.

## Consequences

- `RootPage::Now` is now the truthful, composed `Сейчас` VUI-03 set out
  to build, not a hidden preview -- this is a real, user-visible product
  change on the development device.
- The app grid is fully preserved and fully reachable, just no longer the
  first thing shown -- confirmed by ADR-113's own round-trip test, now
  running under real default conditions instead of a marker-gated one.
- No further dev-only escape hatch exists for `Сейчас`'s composition --
  any future change to it is a change to the one real screen, not a
  toggleable variant.

## Rollback

`saai-shell.pre-nowdefault` remains on-device for a full binary rollback.
No settings or entity-store migration involved -- reverting the binary
alone restores the previous (app-grid-default) behavior completely.

## Links

- ADR-112/113/114 -- the composed screen, its app-grid relocation, and
  its honest-states work, all now running as real default behavior
  instead of behind a marker.
