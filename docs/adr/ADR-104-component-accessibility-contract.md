# ADR-104: accessibility names, roles, values, and disabled/busy state for every component contract

## Status

Accepted, 2026-09-17.

## Context

VUI-02's task list: "Define component accessibility names, roles, values,
disabled states, and non-color cues." ADR-102/ADR-103 shipped ten component
contracts (`SemanticText`, `Icon`, `Divider`, `StatusIndicator`, `Progress`,
`Button`, `Field`, `DataRow`, `Metric`, `Disclosure`) with none of them
exposing this. `component-library-v1.md` section 4's shared state contract
and each primitive's own section 6 notes (6.1's "preserves the full
untruncated string," 6.2's "decorative when paired with text; otherwise
requires a name," 6.7's password-masking rule, 6.10's "accessibility state
[updates] atomically with layout") already specify most of what was needed.

Non-color cues were already satisfied before this ADR: every primitive that
carries meaning does so through text or an enum, never color alone
(`StatusMark`, `MetricValue`, `Field::error` as a string, etc.) -- nothing
new was needed for that half of the task.

## Decision

Added `AccessibilityRole` (`Text`/`Image`/`Button`/`TextField`/`ListItem`/
`Disclosure`/`ProgressIndicator`/`Status`) and `AccessibilityInfo { role,
name, value, disabled, busy }` to `components.rs`. Not a complete platform
accessibility API -- SaaiOS has no screen reader integration yet -- but the
stable, backend-independent shape one would be built against, matching
section 8's "component contracts/state" ownership.

Every primitive gained an `accessibility()` method:

- **`SemanticText`**: `Text`, value = `accessible_value()` (already
  untruncated).
- **`Icon`**: gained a `name: Option<String>` field (didn't exist before --
  `Icon` had no way to be named at all). `None` (default) means decorative;
  `accessibility()` returns `None` for it, since section 6.2 says a
  decorative icon paired with text needs no independent exposure. This
  dropped `Icon`'s `Copy` derive (a `String`-bearing `Option` isn't `Copy`)
  -- confirmed harmless: nothing in `saai-shell` uses `Icon` yet.
- **`Divider`**: same pattern, gained `name: Option<String>`, `None` means
  "not exposed" per section 6.3 ("unless it represents a named boundary").
  Same `Copy` removal, same confirmed-harmless reasoning.
- **`StatusIndicator`**: `Status`, value = the state's own `label_key`
  (e.g. `"state.blocked"`), not display text -- this module's established
  translation-key convention, not a new one.
- **`Progress`**: `ProgressIndicator`, value = percent as text for
  `Determinate`, `busy: true` for `Indeterminate` (its "textual activity
  state," section 6.5).
- **`Button`**: `Button`, name = label, `disabled`/`busy` straight from the
  existing fields.
- **`Field`**: gained a `disabled: bool` field (didn't exist -- section 4's
  disabled state applies to every interactive component, and `Field` is
  one). `TextField`, name = the persistent label (never the placeholder --
  section 6.7: "placeholder is never the only accessible label"), value
  reuses `accessible_value()`, so a masked password stays masked here too.
- **`DataRow`**: role is `ListItem` for `Static`, `Button` for every other
  variant -- matching `is_actionable()`'s own distinction, so a row this
  crate already marks non-clickable can't also announce itself as a
  button. `disabled` is true only for a non-`Static` variant missing an
  action (a dead navigation row), never for `Static` itself (which isn't
  "disabled," it's simply not the kind of row that's ever interactive).
- **`Metric`**: value is the known text (plus unit) for `MetricValue::Known`,
  or the same `label_key()` string `MetricValue` already exposes for
  `Unknown`/`Unavailable` -- one field whose meaning follows the value
  type, documented explicitly since it's the one place this crate lets a
  single field mean either resolved text or a translation key depending on
  the case.
- **`Disclosure`**: value is a `"disclosure.collapsed"`/`"disclosure.
  expanded"` key. The "updates atomically with layout" requirement from
  section 6.10 is a call-site sequencing rule this pure-data type cannot
  itself guarantee -- documented as such rather than silently claimed.

## Verification

- `cargo test -p saai-ui-core`: 37/37 (7 new, one per primitive's
  documented accessibility rule).
- `cargo clippy -p saai-ui-core --all-targets -- -D warnings`: clean.
- `cargo test -p saai-shell`: 87/87, unchanged -- confirmed the `Icon`/
  `Divider` `Copy` removal has no effect, since neither type has a real
  call site in the shell yet.
- No device deploy: a pure library addition with no shell behavior change.

## Consequences

- VUI-02's accessibility task-list item is closed for all ten primitives.
  Non-color cues needed no new work, already satisfied by design.
- The remaining VUI-02 items -- device component gallery, golden/layout/
  hit-test/press-state/overflow tests, and the Montserrat-vs-candidates
  comparison -- are what's left before any of these ten can move past
  Experimental per section 9's promotion checklist.
- `Icon`/`Divider` losing `Copy` is a real, if currently inert, API change;
  worth remembering if either ever gains a real call site and some caller
  assumed cheap copying.

## Rollback

Purely additive/field-adding; no call sites exist yet, so reverting removes
only unused (from the shell's perspective) API surface.

## Links

- ADR-102, ADR-103 -- the ten component contracts this extends.
- `docs/os/ui/component-library-v1.md` sections 4, 6, 8, 9.
- `docs/os/sprints/VISUAL-ROADMAP.md` -- VUI-02.
