# ADR-111: `ContextHeader`, `SystemSection`, `ObjectSummary` composite contracts

## Status

Accepted, 2026-09-17. Host-verified (`cargo test -p saai-ui-core`,
`cargo clippy`). Not yet wired into a real screen -- that is VUI-03's next
Task List item ("Compose `Сейчас`..."), tracked separately.

## Context

VUI-03's second Task List item asked to design `ContextHeader`,
`SystemSection`, "object/work summary", "event row", and a "next-action"
component from actual data. Before writing any of them, found a real
scope conflict between two planning documents: `VISUAL-ROADMAP.md`'s own
task list wording versus `component-library-v1.md` section 3 (the document
explicitly titled "the reviewed design boundary"), whose inventory table
scopes VUI-03 to exactly three composites -- `ContextHeader`,
`SystemSection`, `ObjectSummary` -- and explicitly defers `EventRow` to
VUI-04/05 alongside `IntentSummary`/`TaskSummary`/`AgentSummary`, deferred
to VUI-05. No "next-action" composite is named anywhere.

Raised this to the user rather than picking a side silently; the decision
was to follow `component-library-v1.md` as authoritative. `VISUAL-ROADMAP.md`
was corrected to match (separate commit) rather than the other way around.

## Decision

Added section 7 ("Composite specification sheets") to
`component-library-v1.md`, defining `ContextHeader`/`SystemSection`/
`ObjectSummary`'s anatomy, layout, first real consumer, and accessibility
contract (renumbering the document's old sections 7-9 to 8-10). Implemented
all three in a new `crates/saai-ui-core/src/composites.rs` module, wired
into `lib.rs` alongside the existing `components`/`foundations` modules --
a separate module rather than appending to `components.rs`, matching
section 3's own Primitive/Composite layer distinction.

Each composite is built entirely from section 6 primitives it already had
access to (`SemanticText`, `Divider`, `DataRow`, `StatusIndicator`,
`Metric`) -- none of the three draws its own text or owns a rendering path
a primitive does not already provide, matching the module's own top
comment and section 8's API ownership split.

- **`ContextHeader`**: `context_name`, optional `section_title`, optional
  `lifecycle: StatusIndicator`. `heading_text()` produces
  `"{context_name} · {section_title}"`, deliberately matching
  `services/saai-shell/src/render.rs`'s current
  `format!("{context_label} · {title}")` inside `draw_root` byte-for-byte --
  this type is meant as a drop-in replacement for that ad hoc string, not
  a new information design.
- **`SystemSection`**: `title` plus `Vec<SystemSectionRow>`, where
  `SystemSectionRow` is a thin enum over `DataRow`/`StatusIndicator`/
  `Metric` -- the composition boundary section 7.2 calls for ("any
  `DataRow`/`StatusIndicator`/`Metric` a caller composes into it"), not a
  fourth primitive. `is_empty()` exists so a caller can decide whether an
  empty section renders title-only or is omitted -- `SystemSection` itself
  takes no position on which is correct, matching
  `human-interface-architecture-v2.md` section 13's "Пустой интерфейс
  считается нормальным."
- **`ObjectSummary`**: `title`, `meta`, optional
  `trailing: ObjectSummaryTrailing` (`Value(String)` or
  `Status(StatusIndicator)` -- never a bare color). Distinct from `DataRow`
  by fixed semantic meaning ("this is the object I am currently working
  with," section 14's OBJECT) rather than `DataRow`'s caller-assembled
  meaning.

Added `AccessibilityRole::Heading`, used by both `ContextHeader` and
`SystemSection`'s title -- distinct from the existing `Text` role so a
future accessibility tree can group a section's children under it. Made
`AccessibilityInfo::new` `pub(crate)` (was private to `components.rs`) so
the sibling `composites.rs` module can build values the same way every
primitive does, instead of duplicating field-by-field construction.

Every composite's own `accessibility()` follows the same never-flatten
rule established for primitives: `SystemSection` exposes only its title as
a heading, never flattening its children's own accessibility into one
string; `ObjectSummary` exposes name+meta, with a trailing `StatusIndicator`
reachable separately via `trailing_accessibility()`; `ContextHeader`
exposes its heading text, with a lifecycle `StatusIndicator` reachable via
`lifecycle_accessibility()`.

## Verification

- `cargo test -p saai-ui-core`: 42/42 (37 previously existing + 5 new),
  covering `ContextHeader::heading_text`'s exact match to `draw_root`'s
  current string format, lifecycle/trailing accessibility staying separate
  from each composite's own name/value, and `SystemSection::is_empty`.
- `cargo clippy -p saai-ui-core --all-targets -- -D warnings`: clean.
- `cargo test -p saai-shell`: 91/91, unchanged -- confirms the new
  `pub(crate)` visibility change and the new re-exports do not affect any
  existing consumer.
- Not yet screenshot-verified on device: none of the three composites are
  wired into any real screen yet. `draw_root` still builds its own ad hoc
  string and hardcoded rectangles today; replacing it is VUI-03's next
  Task List item, not this one.

## Consequences

- `component-library-v1.md` section 3's Composite row for VUI-03 is now
  backed by a real, tested contract, not just a table entry.
- `VISUAL-ROADMAP.md`'s VUI-03 task list wording was corrected to match
  this document instead of the reverse, so the two documents no longer
  disagree about EventRow/next-action's timing.
- Deliberately unfinished: no renderer draws any of these three yet,
  and no gallery fixture demonstrates them (`draw_gallery`, VUI-02, has
  no composite section -- adding one was judged out of scope for a
  design-only task; the real proof will be composing `Сейчас` itself).
- `SystemSectionRow`'s three-variant enum is deliberately not extensible
  by a caller (no `Custom(Box<dyn ...>)` arm) -- adding a fourth primitive
  to it later is a real, visible change to this file, not a silent
  capability a caller could smuggle in.

## Rollback

Both new files (`composites.rs`, the section 7 addition to
`component-library-v1.md`) are additive; nothing existing changed shape
except `AccessibilityInfo::new`'s visibility and `AccessibilityRole`
gaining one variant, both backward compatible. Reverting this commit
removes the composites and the visibility change together with no data or
running-system impact, since nothing consumes them yet.

## Links

- ADR-102/103/104 -- the ten primitives this ADR builds directly on top of.
- `docs/os/ui/component-library-v1.md` section 7 -- the specification this
  ADR implements.
- `docs/os/architecture/human-interface-architecture-v2.md` sections 13
  (`Сейчас` mockup) and 14 (five-thing mental model) -- the product intent
  these three composites exist to serve.
