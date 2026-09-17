# ADR-103: Progress, Button, Field, DataRow, Metric, Disclosure component contracts

## Status

Accepted, 2026-09-17.

## Context

ADR-102 shipped the first primitive row (`SemanticText`, `Icon`, `Divider`,
`StatusIndicator`) from `component-library-v1.md` section 6. This closes the
second and last row named in VUI-02's task list: `Progress`, `Button`,
`Field`, `DataRow`, `Metric`, `Disclosure` (sections 6.5-6.10).

## Decision

Added to `crates/saai-ui-core/src/components.rs`, same Experimental
stability level and pure-data/no-`Rect` posture as ADR-102:

- **`Progress`**: `Determinate(u8)` (constructed only through
  `determinate()`, which clamps to 0-100, so an out-of-range value cannot
  exist) or `Indeterminate`. Deliberately has no method deriving
  `UniversalState::Complete` from reaching 100 -- section 6.5: "Completion
  changes to `COMPLETE` only after the owning operation verifies it," not
  from the progress value alone.
- **`Button`**: label, action, `ButtonVariant` (`Primary`/`Secondary`/
  `Quiet`/`Destructive`), `enabled`, `busy`, optional icon.
  `can_activate()` is the single source of truth a renderer's touch handler
  checks before honoring a release -- section 6.6: "Emits exactly one
  action after a valid press/release sequence."
- **`Field`**: label, value, placeholder, help, error, leading/trailing
  icon, `FieldKind` (`Text`/`Password`), `revealed`. `accessible_value()`
  masks a `Password` field's value unless explicitly revealed -- section
  6.7: "Password/PIN variants never expose their value through logs or
  accessibility unless the user explicitly reveals it," mirroring
  `SemanticText::accessible_value`'s role from ADR-102. `is_empty()` keeps
  "empty value is distinct from placeholder" checkable without comparing
  strings.
- **`DataRow`**: optional icon, primary/secondary text, value,
  `DataRowVariant` (`Static`/`Navigation`/`Toggle`/`Status`), optional
  action. `is_actionable()` encodes "static data is not styled as
  clickable" as one boolean instead of a per-call-site check.
  `min_hit_height()` returns the already-existing `MIN_TOUCH_TARGET`/
  `TWO_LINE_ROW_HEIGHT` constants depending on whether a second line is
  present, per section 6.8's stated minimums.
- **`Metric`**: label, `MetricValue` (`Known(String)`/`Unknown`/
  `Unavailable`), optional unit, opaque already-formatted verified-at text.
  A caller cannot substitute a numeric zero for missing data -- it must
  choose one of the three variants -- encoding section 6.9's "Unknown and
  unavailable are text states, not zero" as a type-level fact, not a
  convention to remember. `MetricValue::label_key()` resolves the non-
  `Known` variants through a translation key (`"metric.unknown"`,
  `"metric.unavailable"`), matching `UniversalState::style()`'s own
  `label_key` convention -- documented in `components.rs`'s module comment
  as this crate's general rule: it never embeds display-language strings
  directly.
- **`Disclosure`**: label, named target, `DisclosureState`
  (`Collapsed`/`Expanded`). `chevron()` maps state to
  `IconGlyph::ChevronRight`/`ChevronDown` (both already existed from
  ADR-100) in the one place that mapping is decided. `toggled()` returns
  the flipped state without mutating in place, consistent with every other
  builder in this module.

This closes VUI-02's "visual primitives" task-list line in full (both rows).
Every type stays free of `Rect`/pixels, for the same reason ADR-102 gave:
sidesteps the physical/logical unit tension ADR-101 flagged, since nothing
here carries a coordinate to reconcile.

## Verification

- `cargo test -p saai-ui-core`: 30/30 (8 new, one per contract rule named
  above).
- Caught and fixed during this pass: the new types compiled with 8
  "never used" warnings because `lib.rs`'s `pub use components::{...}`
  re-export list wasn't updated for them -- an unreachable-from-outside
  `pub` item inside a private module is genuinely dead code, not a false
  positive. Fixed by adding all eleven new names to the re-export list;
  `cargo clippy -p saai-ui-core --all-targets -- -D warnings` is clean
  after the fix.
- `cargo test -p saai-shell`: 87/87, unchanged -- nothing in the shell
  references these types yet.
- No device deploy: a pure library addition with no shell behavior change.

## Consequences

- VUI-02's full ten-primitive inventory (both rows of section 3's table)
  now exists as tested Experimental contracts. The next task-list items --
  "make visual and hit-test bounds consume the same layout output,"
  accessibility metadata, the device component gallery, golden tests -- are
  what section 9's promotion checklist actually requires before any of
  these ten can become Stable, and are the natural next work.
- None of these ten primitives has a real `saai-shell` render call site yet.
  Wiring one in is a deliberate, separate decision per screen, not a
  side effect of adding the contract.

## Rollback

Purely additive; no call sites exist yet, so reverting removes only unused
(from the shell's perspective) types.

## Links

- ADR-102 -- the first primitive row and the pattern this follows.
- `docs/os/ui/component-library-v1.md` sections 6.5-6.10, 9.
- `docs/os/sprints/VISUAL-ROADMAP.md` -- VUI-02.
