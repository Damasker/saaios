# ADR-199: VUI-09 — name NOW footer rows in the `.sui` v2 grammar

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. Nested `row <id>`
belongs on a `sui 2` screen. Ids are the live NOW footer destinations
(`apps`, `intent`). `layout_v2()` docks those rows above the tab
strip at `now_footer_action_rect` and emits `open_apps` /
`open_intent_input`. Empty screens invent no footer hits. Object
hits stay procedural. `compile()` stays v1. No new daemon. Leave
Сейчас.

## Нумерация

После ADR-198 следующий свободный номер — **199**. Не S33.

## Контекст

ADR-194/196 matched public NOW tab hits. Live Сейчас still has two
footer rows that `root.sui` does not name: Приложения and Новое
намерение. Those hits come from `now_footer_action_rect` in the
shell. `layout_v2()` treated every non-tab component as a
non-actionable Fill leaf, so 540,2080 missed. Borrowing v1
`content_actions` would revive leftover inspect cards (ADR-187).

## Decision

1. **Grammar.** A `sui 2` screen may list `row <id> { loc = … }`
   beside `component`. Closed ids: `apps` → `open_apps`, `intent` →
   `open_intent_input`. Unknown or duplicate ids fail.
2. **Layout.** Named rows dock at the bottom of the content column.
   Height scales 160 at 2400, the same formula as
   `now_footer_action_rect`. Horizontal inset stays `width/22`.
3. **Example.** `docs/os/ui/examples/now-public.sui` lists `row apps`
   then `row intent` so 540,1860 hits `open_apps` and 540,2080 hits
   `open_intent_input`.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   procedural footer + `layout_v1_root()`.

## Consequences

- v2 layout owns the public NOW footer destinations. ObjectSummary
  hits stay a later slice. Rollback: drop nested `row`. Next:
  leftover badge/kicker/swatch/app-tile paint, or Experimental→Stable
  after Visual v1 acceptance. Still not Visual v1 sign-off.

## Verification

Host: nested rows compile; `search` fails; duplicate `apps` fails;
public NOW 540,1860 / 540,2080 match live footer actions; a tab-only
v2 screen has no footer hits. Panther: Сейчас on HEAD; leave Сейчас.
