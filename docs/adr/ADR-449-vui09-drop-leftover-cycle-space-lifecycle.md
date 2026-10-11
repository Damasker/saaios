# ADR-449: VUI-09 — drop leftover `cycle_space_lifecycle`

## Статус

Принято, 2026-10-11. Space cards still *read* lifecycle (ADR-086
label / archived). The leftover tap-to-cycle write path had no
caller after ADR-448. That writer is gone. Live color cycle stays
`cycle_space_color`. Status invents no action. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-448 следующий свободный номер — **449**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

HIA-01 stored `saaios.space-lifecycle` and cycled it by tapping an
already-selected space card. That gesture lived on leftover
`content_card` / `invoke_content_action`. ADR-448 removed that
dispatch. `cycle_space_lifecycle` then had no live caller.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover lifecycle writer was not on that
list.

## Decision

1. **Delete** `cycle_space_lifecycle`, `SpaceLifecycle::as_str`,
   `SpaceLifecycle::next`, and `space_lifecycle_entity`.
2. **Keep** `space_lifecycle` / `SpaceLifecycle::label` /
   `SpaceLifecycle::parse` for live card status and archived
   checks. Keep `cycle_space_color`.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas.

## Consequences

- Live chrome never writes leftover lifecycle cycles. Rollback:
  restore the writer. Still not Visual v1 sign-off.

## Verification

Host: `cycle_space_lifecycle` is gone; `cycle_space_color` remains.
Panther: do not flash; leave Сейчас.
