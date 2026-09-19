# ADR-149: VUI-07 -- remove `Frame::Root`, the pre-migration dead fallback

## Status

Accepted, 2026-09-19. Host-verified only (full workspace test +
clippy). Not yet physically re-confirmed on Pixel 7 -- see Verification.

## Context

VUI-07's own task list: "Remove migrated screen-local primitives and
record remaining exceptions." ADR-127 through ADR-148 each migrated one
real screen off `Frame::Root`/`draw_root` (the S04-era diagnostic-scaffold
renderer) onto its own dedicated `Frame` variant and a real
`ContextHeader`. Each of those ADRs' own doc comments say "X is no
longer `Frame::Root`" -- but nothing ever went back to check whether
`Frame::Root` itself still had a reason to exist once every real page
had left it.

It didn't. `RootPage` has exactly four variants (`Now`, `Inbox`,
`Spaces`, `Me`). Both the frame-construction `if`/`else if` chain
(`Self::frame`) and the touch-dispatch `if`/`else if` chain
(`TouchHandler::up`) already matched all four exhaustively -- `Now`
twice, once for each `apps_open` state -- before ever reaching their own
final `else` branch. That branch, and everything it alone called, was
unreachable:

- `Frame::Root` (the enum variant) and `draw_root` (180 lines --
  Surface-bar header, placeholder cards, letter-square grid -- the
  exact styling every ADR in this wave replaced).
- `content_action_at`/`content_action_rect`, the touch-side lookup
  against root.sui's static content-action table.
- `content_card`, which built a `Frame::Root` card from a
  `ContentActionDefinition` (its `select_space:` branch was already
  stale -- Spaces has painted from live `entityd` data since ADR-128).
- `invoke_content_action`, `content_action_at`'s only caller.
- `context_label`, `Frame::Root`'s only status-bar-label source.
- `RootPage::index`/`RootPage::id`, once their own last caller
  (`current_page_index` and the `page.id()` table lookup above) went.
- `intent_compose_status`, once `content_card`'s `open_intent_input`
  branch went -- the live "Новое намерение" footer row
  (`now_footer_action_views`) is a static Navigation row with no status
  subtitle and never called this.

Not found by a speculative audit -- found while reading `Frame::Root`'s
own construction site to plan the (separately declined, see the
"Space detail" discussion this session) Space-detail work, and
recognizing the exhaustive-if-chain-with-a-dead-tail shape.

## Decision

Removed, not `#[allow(dead_code)]`'d: unlike ADR-148's four
"host-tested-but-unwired" functions (real logic with no call site,
genuinely ambiguous whether staged or abandoned), everything here had
zero callers **and** zero tests exercising it as a live path -- its
only tests (`now_page_static_actions_come_from_sui_markup`,
`space_actions_come_from_live_spaces_not_sui_markup`,
`selected_indicator_moves_between_edge_tabs`,
`scroll_frame_does_not_repaint_navigation` in `render.rs`) existed
purely to reach the dead code, not to guard a real path. This is not
staged work; it is the tail end of a migration that already finished
elsewhere.

**Recorded exception, not removed**: root.sui's own `content` block
still names two static cards (`selected-entity`/`inspect_selected_entity`,
`new-intent`/`open_intent_input`), and `build.rs` still generates
`ROOT_CONTENT_ACTIONS`/`ContentActionDefinition` from them. Deleting
these would mean editing `root.sui`'s grammar and the shared
`saai-ui-compiler` crate's `content_actions` codegen path -- generic
build-time infrastructure, not screen-local rendering code, and a much
larger, separately-scoped change than this cleanup. Left declared,
`#[allow(dead_code)]`'d at both the struct (`main.rs`) and the
generated const (`build.rs`'s own template, so the attribute travels
with the codegen), and reduced to one shape test
(`root_content_actions_is_now_inert_declared_data`) instead of the two
dispatch tests above.

**Test fallout**: `selected_indicator_moves_between_edge_tabs`
(render.rs) tested `draw_tab_bar`'s own selected-accent-color behavior
through `draw_root` as an incidental vehicle -- retargeted to call
`draw_tab_bar` directly, same assertions, no coverage lost.
`scroll_frame_does_not_repaint_navigation` tested the same
`paint_navigation: false` contract `draw_root` also happened to
implement -- not retargeted, since `me_scroll_does_not_repaint_navigation`
already covers this exact behavior against the real, live
`draw_context_row_list` path. Net: 193 -> 191 tests (194 - 2 content-
action-at tests - 1 draw_root scroll test + 1 new shape test), all
still real assertions, none deleted to silence a lint.

**Orb comment fix, found while editing the adjacent `matches!` guard**:
the guard already correctly listed all six live frames including
`Frame::Root`; only its own doc comment still said "`Frame::Root` (the
only frame the Orb ever draws on)", stale since `Frame::Now`/
`AppsGrid`/`Inbox`/`Spaces`/`Me` joined that list across this sprint.
Corrected the comment to match the guard it sits above, and dropped the
now-nonexistent `Frame::Root` arm from the list itself.

## Verification

- `cargo test -p saai-shell`: 191/191 (193 before this change; see
  Decision for the exact accounting).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
  Iterative -- the first pass surfaced `ContentActionDefinition`/
  `ROOT_CONTENT_ACTIONS` (no non-test reader) and `RootPage::index`/
  `id` (no reader anywhere) as further dead code once `Frame::Root`
  itself was gone; `RootPage::index`/`id` were deleted outright
  (checked: `page_from_id`, the reverse direction `tab_at` actually
  uses, is an independent hand-written match, not built on `id()`, so
  nothing else needed them), `ContentActionDefinition`/
  `ROOT_CONTENT_ACTIONS` were `#[allow(dead_code)]`'d per the recorded-
  exception decision above.
- `cargo test --workspace`: unchanged pass count everywhere else (77
  test-result blocks, all `ok`).
- Hot-swapped onto the live device (backup `saai-shell.pre-frameroot`,
  hash-verified). **Not yet physically re-confirmed**: this change
  removes only unreachable code paths -- every real screen already
  routed through its own dedicated `Frame` before this ADR, so it
  should be visually and behaviorally a no-op on-device. A same-day
  spot check (Сейчас/Входящие/Пространства/Я/lock idle) is the
  proportionate verification for a pure dead-code removal, not a full
  re-walk of every VUI-07 screen this sprint already flashed
  individually.

## Consequences

- `draw_root`, `content_action_at`/`content_action_rect`,
  `content_card`, `invoke_content_action`, `context_label`,
  `RootPage::index`/`id` are gone. Nothing on any real screen changes
  behavior -- these were unreachable before this ADR too.
- root.sui's two static content-action cards remain declared but are
  now honestly documented as inert, verified by a shape test rather
  than a dispatch test that only ever exercised dead code.
- VUI-07's "Remove migrated screen-local primitives and record
  remaining exceptions" task is now substantively done for the
  `Frame::Root` era; the recorded exception (root.sui's content-action
  table) is the one deliberately left for whoever next touches the
  `saai-ui-compiler` codegen path generically.

## Rollback

`saai-shell.pre-frameroot` remains on-device. No settings/entity
changes involved -- purely dead-code removal plus two lint-satisfaction
`#[allow(dead_code)]` attributes.

## Links

- ADR-127 through ADR-148 -- the migration wave that made `Frame::Root`
  unreachable, one real screen at a time, without ever itself declaring
  the fallback dead.
- ADR-128 -- Space detail (people/members) stays deferred; this ADR
  does not touch that decision, it only removes what was already dead
  regardless of it.
