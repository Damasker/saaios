# ADR-113: relocate the application grid behind a "Приложения" entry

## Status

Accepted, 2026-09-17. Physically verified end-to-end on Pixel 7. Reachable
only when `now_composed` is also set (ADR-112's own dev marker) -- with it
off, `RootPage::Now` already always shows the grid, unaffected by this ADR.

## Context

VUI-03's fourth task: "Move the application grid behind an explicit
secondary `Приложения` entry or sheet without removing application access."
ADR-112 composed a real `Сейчас` but deliberately left the app grid as
the only thing `RootPage::Now` shows when its dev marker is off, exactly
to avoid combining a display change with an access-relocation change in
one commit. This ADR is that second, separate change.

## Decision

Two new fixed rows -- "Приложения" and "Новое намерение" -- sit just
above the tab bar on the composed screen, positioned by a new pure
function (`now_footer_action_rect`) rather than interleaved with
`SystemSection` content, so their position never depends on how much real
data is above them: the same "pure function shared by rendering and
hit-testing" pattern `now_grid_rect`/`stacked_row_rect` already use
elsewhere in this file. `Frame::Now` gained a `footer_actions: Vec<(Rect,
DataRow)>` field, computed once in `main.rs` and handed to `render::
draw_now` to just draw -- the same "position computed in `main.rs`,
drawn generically by `render.rs`" split `Frame::Root`'s own `content_cards`
already uses, not a new architecture.

A new `apps_open: bool` `Shell` field, same shape as `trusted_clients_open`/
`bluetooth_list_open`: "Приложения" sets it `true`; when set, `RootPage::
Now`'s frame construction falls through to the *existing*, completely
unmodified `Frame::Root`/`now_content_cards`/`now_grid_rect` app-grid path
-- the exact same rendering and touch handling `RootPage::Now` always used,
now reachable through a tap instead of being the page's only mode. Closed
by re-tapping the already-selected "Сейчас" tab (the same tap that was
already a no-op before this ADR, now doing something the first time it's
pressed while the grid is open). `apps_open` resets to `false` on every
root tab switch, so leaving and returning to `Сейчас` never re-opens the
grid unexpectedly.

"Новое намерение" reuses the existing `"open_intent_input"` action
verbatim -- the intent-creation entry point the old always-grid `Now` page
already exposed, now reachable directly from the composed screen instead
of only from inside the grid.

**Application access is not removed**: the two legacy `root.sui` cards
for `page=now` ("Объект пространства" / `inspect_selected_entity`,
"Новое намерение" / `open_intent_input`) are left inside the app grid
unchanged -- confirmed by opening the grid and seeing both still present
alongside the real installed apps. Trimming them from `root.sui` now that
the composed screen has its own `ObjectSummary` and its own "Новое
намерение" row would be reasonable follow-up, but is a `.sui`
build-pipeline change with its own review, not bundled into this ADR.

## Verification

- `cargo test -p saai-shell`: 96/96 (94 + 2 new -- `now_footer_action_
  rows_stack_above_the_tab_bar_in_order`, `now_footer_action_at_finds_
  each_row_and_misses_above_them`).
- `cargo clippy -p saai-shell --all-targets -- -D warnings`: clean.
- Hot-swapped onto the live device (backup `saai-shell.pre-appsfooter`,
  hash-verified). With `now_composed` and the device owner doing the real
  taps (no synthetic touch injection, matching this session's established
  preference):
  1. Composed `Сейчас` screenshot: "Приложения"/"Новое намерение" rows
     render correctly above the tab bar, alongside the real populated
     `SystemSection`/`ObjectSummary` content from ADR-112 -- no layout
     collision.
  2. Tapped "Приложения": app grid opened, showing the real installed
     apps plus the two legacy `root.sui` cards, confirming application
     access is intact.
  3. Tapped the already-selected "Сейчас" tab: grid closed, composed
     screen returned with its real content still correct.
  4. Tapped "Новое намерение": the existing intent-input keyboard opened
     correctly ("Наберите текст…" placeholder, full QWERTY layout,
     Отмена/123/Отправить row).

## Consequences

- `Сейчас` can now be the composed, data-truthful screen with the app
  grid one tap away, not the app grid itself -- once `now_composed`
  becomes the default (a decision this ADR does not make; see Rollback),
  this satisfies the task's "without removing application access" clause
  by construction, not by omission.
- `now_action_at`/`now_content_cards`/`now_grid_rect` are completely
  unmodified -- this ADR adds a second way to reach them, not a
  replacement.
- `root.sui`'s two `page=now` cards are now slightly redundant with the
  composed screen's own `ObjectSummary`/"Новое намерение" row while the
  grid is open -- left as-is deliberately, flagged as real follow-up
  rather than silently removed.

## Rollback

Both mechanisms stay behind `now_composed` (ADR-112's own marker) --
removing `/run/saaios/ui-now-composed` reverts `RootPage::Now` to its
pre-ADR-112 behavior entirely, `apps_open`/the footer rows included, with
no separate rollback needed for this ADR specifically. `saai-shell.pre-
appsfooter` remains on-device for a full binary rollback if ever needed.

## Links

- ADR-112 -- the composed `Сейчас` this ADR relocates the app grid out of.
- ADR-111 -- `DataRow`, reused unchanged for both footer rows.
