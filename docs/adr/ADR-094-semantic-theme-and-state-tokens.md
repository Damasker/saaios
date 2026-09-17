# ADR-094: semantic theme and universal state tokens belong to `saai-ui-core`

## Status

Accepted, 2026-09-17.

## Context

VUI-01 is the first implementation sprint for
`docs/os/architecture/visual-language-v1.md`. The current shell renderer owns
seven panel-packed color constants while `main.rs` creates additional RGB
values for Space context and Orb attention. This makes a screen's meaning
depend on local drawing code and makes it possible to reuse the same color for
unrelated concepts.

The renderer's `Pixel = [u8; 4]` is not a product color model. It encodes the
physically verified Pixel 7 scanout packing `[X, R, G, B]`. Putting that packing
in the shared UI API would couple every component and future backend to one
panel quirk.

HIA also assigns color to context while Visual Language v1 assigns colors to
semantic state. Those uses must coexist without allowing a context color to be
mistaken for failure, attention, success, or an action.

## Decision

1. `saai-ui-core` owns backend-independent `Rgb`, `ColorRole`, `Theme`,
   `UniversalState`, `StateStyle`, `StatusMark`, `MotionCue`, and
   `ContextColor` types.
2. `Theme::SAAIOS_DARK` is the single Visual Language v1 palette. Components
   request a semantic role or universal state; they do not create RGB values.
3. The render backend converts logical `Rgb` to its native pixel packing. The
   current shell keeps `[X, R, G, B]` conversion in `render.rs`.
4. Context colors are addressed through `ContextColor` and
   `Theme::context_color()`. They are not `ColorRole` members and are never used
   for severity or ordinary controls.
5. Universal state maps to a semantic color role, translation key, non-color
   mark, and motion cue. UI text is localized outside `saai-ui-core`; the core
   exposes stable translation keys rather than Russian or English labels.
6. Derived interaction roles (pressed, focus, disabled, high-contrast text)
   are explicit theme members. Screen code does not calculate them.
7. Visual/high-contrast calibration changes the backend/display mapping or a
   complete theme variant, not arbitrary individual screen colors.

## Ownership boundaries

- `saai-ui-core`: semantics and backend-independent values.
- `saai-ui-compiler`/`.sui`: references semantic roles after the VUI reference
  screen proves the component vocabulary; VUI-01 does not change the grammar.
- `saai-shell`: chooses roles and supplies truthful state/context.
- `render.rs`: converts logical colors into panel pixels and draws them.
- display/compositor: scanout format and physical presentation, never product
  meaning.

## VUI-01 migration boundary

VUI-01 changes palette and state foundations only. Layout coordinates,
navigation, hit testing, data models, protocols, authorization, and storage do
not change.

Temporary allowlist:

- raw channel values inside `Theme::SAAIOS_DARK` and its context palette;
- the backend's panel-packing conversion;
- test fixture sentinel colors that are explicitly named as test-only.

No other shell production code may add `rgb(...)` or byte-array color literals.
The allowlist is removed when VUI-09 finishes library/platform migration.

## Verification

- exact token-value and role/state mapping unit tests in `saai-ui-core`;
- shell renderer tests proving the backend packing and semantic role mapping;
- no production shell RGB constructor calls outside the backend boundary;
- a deterministic full-screen calibration surface covering all core palette,
  context, interaction, and universal-state roles;
- host tests/clippy plus physical Pixel 7 review without geometry, touch,
  scrolling, status/navigation, restart, or cold-boot regression.

## Consequences

- A future renderer can consume the same product semantics without inheriting
  Pixel 7 memory layout.
- Theme changes become reviewable data changes instead of a search through
  screens.
- Universal states remain distinguishable without color and are ready for the
  VUI-02 component library.
- Existing screens change appearance during migration but not behavior or
  geometry.
- The first pass still has renderer-local calls that request semantic roles;
  `.sui` style declarations and public component APIs deliberately follow the
  reference-screen work instead of being guessed now.

## Rollback

Revert the VUI-01 commits. No persistent data or protocol migration is involved,
and the previous renderer constants remain available in the prior build.

## References

- `docs/os/architecture/visual-language-v1.md`
- `docs/os/sprints/VISUAL-ROADMAP.md`
- ADR-017 (`.sui` ownership and declarative UI direction)
- ADR-070 (font asset pipeline drift and device verification)
