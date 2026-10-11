# ADR-452: VUI-09 — drop leftover `present_lock_surface`

## Статус

Принято, 2026-10-11. Live lock already paints through
`present_lock_pin_entry` (ADR-134 / 149 / 154). The leftover
solid-color lock presenter had no live caller. Lock idle/wake stay
formulas (ADR-232). Status invents no action. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-451 следующий свободный номер — **452**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-024 / ADR-051 kept `present_lock_surface` as a solid-color lock
fill. Live lock then moved to `present_lock_pin_entry`, which
already paints idle, sleep, and PIN. The leftover fill path had
`#[allow(dead_code)]`.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover solid-color presenter was not on
that list. This slice does not convert those formulas.

## Decision

1. **Delete** `present_lock_surface`.
2. **Keep** `present_lock_pin_entry` / `draw_lock_idle` /
   `draw_lock_sleep` / `lock_dmabuf` / `lock_pool`.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas.

## Consequences

- Live chrome has no leftover solid-color lock fill. Rollback:
  restore `present_lock_surface`. Still not Visual v1 sign-off.

## Verification

Host: `present_lock_surface` is gone; lock pin-entry presenter
remains. Panther: do not flash; leave Сейчас.
