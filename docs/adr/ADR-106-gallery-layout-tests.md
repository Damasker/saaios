# ADR-106: host-testable gallery row layout, closing VUI-02's task list

## Status

Accepted, 2026-09-17. Physically re-verified on Pixel 7.

## Context

VUI-02's last task-list item: "Add golden render, layout, hit-test,
press-state, and overflow tests." ADR-105's gallery had zero tests of its
own beyond the pre-existing `cargo test`/clippy gates, which is exactly why
its real bug (two lines of text overlapping inside a single row) needed a
physical device screenshot to find at all -- nothing in the test suite
could have caught it, because the row positions only existed as an inline
`cursor_y` variable mutated across the whole function body, never a value
anything could assert on, and the within-row line offsets were two
separate hand-picked constants (`+28`, `+30`) with nothing checking them
against the actual font metrics they needed to clear.

## Decision

Extracted `gallery_row_positions(height: u32) -> [u32; 11]` out of
`draw_gallery` -- pure function, no `Canvas`, no `Fonts`, no device --
returning the title's and each of the ten primitives' `cursor_y` in draw
order. `draw_gallery` now computes this once and indexes into it, instead of
threading a mutable `cursor_y` through ten sequential blocks; the tested
function is exactly what runs, not a parallel copy that could drift from it.

Four new host tests:

- `gallery_rows_never_overlap` -- every consecutive *row* gap must be at
  least one `Body` line height plus one `Caption` line height
  (`physical_line_height`, also extracted so it is nameable from a test).
  Checked directly, not assumed: temporarily restoring ADR-105's original
  hand-picked offsets (`+28`, `+30`) and rerunning this test showed it
  still passes -- it does **not** catch that specific bug. ADR-105's bug
  was two lines colliding *inside one row's own space* (the reason line
  under a `StatusIndicator`'s label, the secondary line under a
  `DataRow`'s primary), not one row spilling into the next; the row-gap
  budget was generous enough in both cases that the internal collision
  never crossed into a neighboring row. What this test does catch for real
  -- confirmed the same way -- is the title-to-first-row gap, which really
  was tighter than every other row's (`80` vs the `120` every other gap
  gets): reverting just that one number back to `80` reliably fails this
  test. Fixed by using the same formula for that gap instead of a separate
  hand-picked number.
- `gallery_rows_stay_within_the_screen` -- the last row must still be above
  `height`, catching a version of the same class of bug in the opposite
  direction (rows spilling off the bottom instead of into each other).
- `physical_line_height_matches_the_pixel_7_scale` -- a pinned golden value
  (72 for `Body`, 48 for `Caption`) making the actual numbers the other two
  tests depend on visible, not just implied.
- `gallery_divider_and_progress_draw_without_a_loaded_font` -- a real golden
  pixel test: `draw_gallery(canvas, width, height, None)` still paints the
  divider's `Border` color and the progress bar's `Accent`/`Grid` split at
  their exact row positions, confirming the "still shows something, not a
  blank screen" contract the two font-independent primitives now have
  (moved ahead of the `fonts` check in `draw_gallery` itself, a real
  behavior change from ADR-105, not just a test addition).

Writing this test caught one more real, if minor, inconsistency before it
shipped: the gap between the title and the first row was a hand-picked `80`,
smaller than the uniform minimum every other row gets. Fixed by using the
same `physical_line_height(Body) + physical_line_height(Caption)` formula
for that gap too, rather than special-casing the test to accept a looser
standard for just one row.

ADR-105's actual bug class -- a within-row second-line offset smaller than
the line above it needs -- is not caught by a dedicated regression test
here; as established above, no test in this pass catches it. It is instead
prevented structurally: both call sites (`StatusIndicator`'s reason line,
`DataRow`'s secondary line) now compute their offset by calling
`physical_line_height(TextRole::Body)` directly, the same function
`physical_line_height_matches_the_pixel_7_scale` pins. There is no longer a
separate hand-picked number either call site could drift from; the only way
to reintroduce this bug class is to deliberately stop calling the shared
helper, not to mistype a constant near it. This is treated as sufficient
here, not because it is untestable in principle, but because a test that
directly exercises it would need either a loaded font or another physical
screenshot -- the same constraint the next section explains.

## What "hit-test" and "press-state" tests mean here, honestly

The gallery is a passive display surface -- `paint_frame`'s early return for
`gallery_mode` never reaches the touch-handling code path at all, and none
of the ten primitives are wired to any action in this screen. There is
nothing to hit-test or press yet, so nothing was added under those names.
This is not a gap quietly left out: it is what "the gallery is a rendering
proof, not an interactive screen yet" honestly implies. A real hit-test/
press-state test needs an actual interactive consumer of these primitives
(a real `saai-shell` screen migrated to use them, which ADR-102's own "Not
in this ADR" already named as separate follow-up work), not something to
force into a display-only gallery.

## What "golden render" and "overflow" mean here, honestly

A true text-content golden render test (asserting on the rendered glyph
bitmap of a specific string) needs a loaded font, which host tests do not
have (no `/saaios/fonts/*.ttf` on a dev machine) -- the same constraint
every existing `render.rs` test already works around by calling with
`fonts: None` and asserting on non-text pixels only. This ADR's fourth test
does exactly that for the two primitives that can be meaningfully tested
this way; `SemanticText`'s actual line-wrapping/overflow behavior at real
text content and widths is not testable host-side without either an
embedded test font (a real infrastructure investment, not undertaken here)
or another physical device screenshot -- ADR-105's own screenshots are the
closest thing that exists today, not claimed as a repeatable automated test.

## Verification

- `cargo test -p saai-shell`: 91/91 (4 new).
- `cargo clippy -p saai-shell --all-targets -- -D warnings`: clean.
- Hot-swapped onto the live device once more (the refactor moved row
  positions by a few pixels via the title-gap fix), rollback still
  `saai-shell.pre-gallery` from ADR-105. Real device screenshot confirms no
  regression: all ten primitives still render correctly, with slightly more
  breathing room after the title than ADR-105's own fixed screenshot showed.
- Gallery marker removed and shell restarted back to normal mode afterward.

## Consequences

- VUI-02's task list is now fully closed (component inventory, typography,
  mono font, foundation tokens, icon pipeline, layout primitives, all ten
  visual primitives, visual/hit-test bounds unification, accessibility
  metadata, first device gallery, and this ADR's layout/golden tests).
  Everything shipped is Experimental, not Stable -- section 9's full
  promotion checklist (gallery covering every state, golden renders of real
  text, Pixel 7 review across scale/offline/blocked/failed, migration
  documentation) is still ahead for any of these ten primitives.
- `gallery_row_positions`/`physical_line_height` are now real, reusable,
  tested functions -- any future gallery row or primitive addition gets
  this regression coverage for free by construction, not by remembering to
  write a new test for it.

## Rollback

`saai-shell.pre-gallery` remains on-device. The row-position extraction is a
pure refactor (same positions except the title gap, which only grew); no
entity, protocol, or storage change.

## Links

- ADR-105 -- the gallery and the bug this ADR adds row-layout regression
  coverage for (and structurally, not test-coverage, prevention of).
- ADR-102, ADR-103, ADR-104 -- the ten component contracts.
- `docs/os/ui/component-library-v1.md` section 7 -- the full gallery/test
  matrix still ahead.
- `docs/os/sprints/VISUAL-ROADMAP.md` -- VUI-02, now fully checked off.
