# ADR-107: real word-wrap and max_lines/ellipsis for SemanticText

## Status

Accepted, 2026-09-17. Physically verified on Pixel 7.

## Context

VUI-02's Acceptance checklist (`docs/os/sprints/VISUAL-ROADMAP.md`) includes:
"Long labels wrap or reflow; they do not clip or overlap navigation" --
directly restating `component-library-v1.md` section 6.1's contract for
`SemanticText`: "Overflow: wrap by default for prose; ellipsis only when a
full-value route exists."

ADR-105's gallery violated this from the start: `SemanticText.max_lines`
and `TextOverflow` existed as data fields since ADR-102, but no renderer
ever read them -- `draw_gallery_semantic_text` called `draw_text` with the
full string, unconditionally, on one line. The gallery's own demo string was
long enough to run straight off the right edge of the screen in every
screenshot from ADR-105 through ADR-106, visible but not previously called
out as its own defect.

## Decision

Added `wrap_text(font, text, size, max_width) -> Vec<String>` to
`render.rs`: greedy word-wrap using the already-existing `text_width`
measurement, one word at a time, never splitting a single word wider than
`max_width` on its own (the practical limit any non-hyphenating word-wrap
has).

`draw_gallery_semantic_text` now implements `SemanticText`'s full contract:
wraps via `wrap_text`, keeps at most `max_lines` lines when set, and appends
an ellipsis to the last visible line only when content was actually cut
*and* the caller asked for `TextOverflow::Ellipsis` -- never invented on a
line that already fit completely. The gallery's own demo `SemanticText` was
extended to a genuinely long string with `.with_max_lines(2)` and
`.with_overflow(TextOverflow::Ellipsis)`, so the gallery now demonstrates
the contract instead of only declaring it in unused fields.

`wrap_text` is not host-testable for its actual wrapping behavior --
like every text-measurement function in this file, it needs a loaded
`Font`, unavailable outside a device build (the same constraint ADR-106
already documented for golden text tests). No test was added for it;
verification is physical, below.

## Verification

- `cargo test -p saai-shell`: 91/91, unchanged (no new host-testable
  surface).
- `cargo clippy -p saai-shell --all-targets -- -D warnings`: clean.
- Hot-swapped onto the live device, rollback still `saai-shell.pre-gallery`.
  Real device screenshot confirms: the demo string now wraps across two
  lines at a word boundary, the second line ends with `…`, and neither line
  overlaps the `Wifi` icon row below it -- the row-spacing budget
  `gallery_row_positions` already provides (ADR-106) comfortably covers two
  `Body`-height lines for the real 2400px screen height.

## Not verified by this ADR

`gallery_row_positions`' generic minimum row gap (one `Body` line height
plus one `Caption` line height, from ADR-106) is smaller than what two full
`Body` lines actually need. For the real Pixel 7 height (2400px) the actual
row budget is comfortably larger than this generic minimum, so there is no
practical collision -- confirmed by the physical screenshot, not by a
tightened test. A shorter logical screen height, or a `SemanticText` row
whose content needs more than two wrapped lines, could still collide with
the row after it; this is a known, narrow gap in ADR-106's test coverage,
not fixed here.

## Consequences

- `wrap_text` is a real, reusable capability now -- any future `render.rs`
  screen needing word-wrap (a status reason, a long device name, a real
  `Сейчас` surface built from these primitives) can call it directly instead
  of re-deriving greedy wrapping.
- VUI-02's "long labels wrap or reflow" acceptance line is satisfied for the
  one primitive (`SemanticText`) that declares an overflow policy at all;
  the other primitives with label text (`Button`, `Field`, `DataRow`,
  `Metric`, `Disclosure`, `StatusIndicator`) do not wrap and were not
  changed here -- section 6.6-6.10's own anatomy notes (e.g. `Button`:
  "Long labels wrap to two lines and increase visual height") describe this
  as their own, separate, not-yet-implemented behavior.

## Rollback

`saai-shell.pre-gallery` remains on-device. Pure rendering addition; no
entity, protocol, or storage change.

## Links

- ADR-102 -- `SemanticText`'s `max_lines`/`TextOverflow` fields, unused
  until now.
- ADR-105, ADR-106 -- the gallery and its row-layout tests this builds on.
- `docs/os/ui/component-library-v1.md` section 6.1.
- `docs/os/sprints/VISUAL-ROADMAP.md` -- VUI-02 Acceptance.
