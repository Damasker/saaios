# ADR-457: VUI-09 — public Search hits through `compile_v2_public`

## Статус

Принято, 2026-10-11. Inbox / Spaces / Me already have dedicated
public hit locks. Search had `search-public.sui` and the batch
compile, not a stacked-row lock. Button `DataRow` action stays the
interned `loc`. Status invents no action. Public subset stays
Experimental. Leftover named chrome stays closed (ADR-456).
Keyboard keys, gallery page, and lock idle/wake stay formulas.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-456 следующий свободный номер — **457**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-240 made Search a primary tab. `search-public.sui` names
`ContextHeader`, a Button `DataRow` at `search.item`, and nested
tabs. `layout_v2_public_inbox_event_row_matches_stacked_row` and
the Spaces/Me siblings lock those examples. Search only appeared
in `public_examples_layout_through_v2_without_shell`.

Inventing `open_object` on Search `DataRow` would disagree with
`v2_stacked_action` (interned `loc`). Converting Keyboard keys /
gallery page / lock idle/wake is not this gate.

## Decision

1. **Host-lock** `search-public.sui` through `compile_v2_public` +
   `layout_v2`. The Button `DataRow` matches `stacked_row_rect`
   index 0; action is `search.item`.
2. **Status invents no action.** A Status Search `DataRow` is not
   hittable. Nested `tab search` stays the primary-tab hit.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not start thin tuning.

## Consequences

- Public Search shares the same Experimental hit lock as Inbox /
  Spaces. Rollback: drop the Search stacked-row test. Still not
  Visual v1 sign-off.

## Verification

Host: public Search hits `search.item`; Status misses; tab
`search` stays. Panther: do not flash; leave Сейчас.
