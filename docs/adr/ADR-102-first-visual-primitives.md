# ADR-102: SemanticText, Icon, Divider, StatusIndicator as Experimental component contracts

## Status

Accepted, 2026-09-17.

## Context

`docs/os/ui/component-library-v1.md` section 3's inventory groups the first
primitive layer as `SemanticText`, `Icon`, `Divider`, `StatusIndicator` --
"gallery and `Сейчас`" as their first real consumer, Experimental stability.
Sections 6.1-6.4 specify each one's anatomy and contract in detail. Before
this, `saai-ui-core` had the underlying tokens (`TextRole`, `IconGlyph`,
`ColorRole`, `StatusMark`, `UniversalState`) but no component type that
bundles them into the actual contract those sections describe (text with a
truncation policy that still exposes its full value to accessibility, an
icon with a default control size, a divider that names which semantic color
role it uses, a status indicator whose reason only shows in one variant).

## Decision

Added `crates/saai-ui-core/src/components.rs`:

- **`SemanticText`**: content, `TextRole`, `ColorRole`, optional `max_lines`,
  `TextOverflow` (`Wrap`/`Ellipsis`). `accessible_value()` always returns the
  untruncated content -- section 6.1's "Accessibility: preserves the full
  untruncated string" as an actual invariant, not just documentation.
- **`Icon`**: `IconGlyph`, `IconSize` (defaults to `Large`/24 units --
  section 6.2's "24 is the normal control size"), `ColorRole`.
- **`Divider`**: `ColorRole` (`new()` uses `Border`, `subtle()` uses `Grid`,
  matching section 6.3's `color.border.default`/`color.grid.subtle` naming)
  plus an optional leading inset so it can align with a neighboring row's
  text edge instead of running the full container width. Pairs with the
  already-existing `Node::separator` (ADR-101) for geometry -- `Node` has no
  paint concept, so geometry and color stay in separate types by the same
  ownership split section 8 already draws.
- **`StatusIndicator`**: `UniversalState`, label, optional reason,
  `StatusIndicatorVariant` (`Compact`/`Normal`). `mark()`/`color()` are
  derived from `state.style()`, never set independently, so a status
  indicator cannot invent a local meaning for a universal state. Section
  6.4's "Compact variant: mark plus label; normal variant may add reason" is
  encoded as `visible_reason()` -- the one place that decision is made, so a
  caller cannot leak a reason into a compact layout by forgetting to check
  the variant separately from whether one happens to be set.

All four are pure content/semantic descriptors -- none references `Rect` or
a physical pixel, matching section 8's split ("`saai-ui-core`: ... component
contracts/state ... render backend: font loading, glyph rasterization, icon
tessellation, pixels"). This sidesteps, for now, the unresolved tension
ADR-101 already flagged between this crate's existing physical-pixel layout
tree and the component spec's stated logical-unit model -- these types carry
no coordinates at all, so there is nothing to reconcile for this specific
increment.

## Stability

Experimental only, per the component library document's own section 2 and
9: no `saai-shell` render call site uses these types yet, no gallery, no
golden renders, no accessibility or Pixel 7 review. This ADR ships the
contract and its unit tests, not a promotion.

## Verification

- `cargo test -p saai-ui-core`: 22/22 (5 new, one per contract rule named
  above: accessible value ignores truncation, icon default size, divider
  token selection, status mark/color come from state style, compact hides a
  set reason).
- `cargo clippy -p saai-ui-core --all-targets -- -D warnings`: clean.
- `cargo test -p saai-shell`: 87/87, unchanged -- nothing in the shell
  references these types yet, so nothing could regress.
- No device deploy: a pure library addition with no shell behavior change.

## Consequences

- The remaining primitive row (`Progress`, `Button`, `Field`, `DataRow`,
  `Metric`, `Disclosure`) is the natural next increment, same pattern.
- Wiring any of these four into a real `saai-shell` screen (the "first real
  consumer" the inventory names) is separate follow-up work, not attempted
  here -- each existing screen currently draws its own text/icon/divider/
  status inline in `render.rs`; migrating one is a decision about which
  screen and how much regression risk to accept in one change, matching this
  project's own "screen migration... separable when practical" rule.

## Rollback

Purely additive; the module has no call sites yet, so reverting removes only
unused (from the shell's perspective) types.

## Links

- `docs/os/ui/component-library-v1.md` sections 3, 6.1-6.4, 8.
- ADR-101 -- the layout primitives (`Node::separator`, `EdgeInsets`) this
  pairs with.
- `docs/os/sprints/VISUAL-ROADMAP.md` -- VUI-02.
