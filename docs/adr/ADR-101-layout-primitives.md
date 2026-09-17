# ADR-101: inset, separator, and focus-order metadata in the shared layout tree

## Status

Accepted, 2026-09-17.

## Context

VUI-02's task list calls for shared layout primitives: stack, row, inset,
separator, scroll region, and focus/order metadata. `saai-ui-core::Node`/
`layout()`/`LayoutNode`/`hit_test()` (ADR-017) already existed as the
runtime-independent view tree `saai-shell` builds every screen from --
`Node::linear(id, Axis, children)` with `Axis::Horizontal`/`Vertical` already
is "row"/"stack" (a vertical linear container), so those two were not a real
gap. Padding, a divider convenience, and any focus-order concept did not
exist at all.

## Decision

Added to `crates/saai-ui-core/src/lib.rs`, with no change to any existing
call site's behavior (every new field defaults to a no-op value in
`Node::leaf`/`Node::linear`):

- `EdgeInsets { top, right, bottom, left }` (`u32`, physical pixels) and
  `Node::padding: EdgeInsets` (default `ZERO`), applied in `layout_node` to
  shrink the bounds children are laid out within. The node's own reported
  `rect` stays the outer, unpadded bounds -- a padded card is still one
  paintable/tappable rectangle; only its children move inward.
- `Node::separator(id, axis)`: a `StrokeToken::Hairline`-thick leaf sized to
  fill the cross axis of the container it sits inside. `axis` names that
  container's own axis (a divider between vertically stacked rows takes
  `Axis::Vertical`, matching the stack, not the line's own visual direction).
- `Node::focus_order: Option<u32>` (default `None`), propagated to
  `LayoutNode::focus_order` by `layout()`. Tab/switch-navigation order;
  `None` means "not a focus stop." Metadata only -- no navigation logic
  consumes it yet.

`EdgeInsets` is deliberately its own type, not `foundations::SafeInsets`:
this layout tree operates entirely in physical pixels already (`Rect`'s
fields, `Length::Px`, the existing `four_fill_tabs_cover_both_screen_edges`
test using literal `1080`/`2400`), and `layout()` takes no `SurfaceScale` to
convert a logical value with. Reconciling the two unit domains is real,
deferred work -- likely natural alongside VUI-09's "move root layout and hit
testing to compiled shared layout output" -- not something to paper over with
a same-named type that would silently need the wrong units at every call
site today.

## Not in this ADR

**Scroll region** is the one item from the task list deliberately not
attempted here. A real implementation needs `hit_test` to stop matching a
child once it has scrolled outside its viewport (today's rect-only
`contains` check does not know about an ancestor's visible bounds, so a
child shifted out of view by a scroll offset could still be hit if a touch
coordinate happens to land where that child's unclipped rect would be) --
a correctness-sensitive change that deserves its own focused design and
tests, not folding into a same-commit grab bag with three much simpler,
independently low-risk additions. `saai-shell`'s several existing ad-hoc
per-screen scroll implementations (the "Я" page, `Система`'s drag handling)
are exactly the duplicated logic a shared primitive should eventually
replace, but migrating them is separate, larger, and riskier work in its own
right (each already has its own physically-verified behavior this session
fixed real bugs in) -- not attempted or assumed here.

## Verification

- `cargo test -p saai-ui-core`: 17/17 (5 new: padding shrinks only children
  not the node's own rect, asymmetric per-edge padding, padding wider than
  bounds never underflows, separator sizes correctly on both axes, focus
  order defaults to `None` and survives `layout()`).
- `cargo clippy -p saai-ui-core --all-targets -- -D warnings`: clean.
- `cargo test -p saai-shell`: 87/87, unchanged -- confirms the new `Node`
  fields (defaulted in every existing constructor call) do not alter any
  currently-built screen's layout.
- No device deploy for this ADR: a pure library addition with no shell
  behavior change (no screen migrated to use padding/separator/focus-order
  yet), so there is nothing new to observe on a physical build.

## Consequences

- Any future component (VUI-02's own next task: text, icon, divider,
  `StatusIndicator`, progress, button, field, `DataRow`, metric, disclosure)
  can use real padding and a real divider primitive instead of each screen
  computing its own inset math by hand, which is what every current
  `saai-shell` screen still does.
- Focus order exists as data but nothing reads it yet -- a real
  keyboard/switch-navigation implementation is separate, future work.
- Scroll region remains open, explicitly, as the harder remaining piece of
  this same task-list line.

## Rollback

Purely additive fields with safe defaults; reverting this commit removes the
types and fields with no data, protocol, or behavior to migrate back.

## Links

- `docs/os/sprints/VISUAL-ROADMAP.md` -- VUI-02.
- ADR-017 -- the original `Node`/`layout()` view tree this extends.
