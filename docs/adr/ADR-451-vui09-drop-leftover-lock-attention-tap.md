# ADR-451: VUI-09 — drop leftover `lock_attention_tap`

## Статус

Принято, 2026-10-11. Lock attention still *paints* through
`lock_attention_view` (ADR-148). The leftover unused tap-policy
oracle had no live caller. Existing tap-to-unlock stays. Lock
idle/wake stay formulas (ADR-232). Status invents no action. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-450 следующий свободный номер — **451**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-148 recorded `lock_attention_tap` as a re-check: locked lock
never reveals details. Live lock already consumes tap-to-unlock
in the down-handler. The leftover enum then had `#[allow(dead_code)]`
and only a host test.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover tap oracle was not on that list.
This slice does not convert those formulas.

## Decision

1. **Delete** `lock_attention_tap` and `LockAttentionTap`.
2. **Keep** `lock_attention_view` / `lock_wake_tap` / `lock_idle_view`
   / `lock_sleep_view`. Tap-to-unlock stays on the lock surface.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas.

## Consequences

- Live chrome has no leftover unused lock-tap oracle. Rollback:
  restore `lock_attention_tap`. Still not Visual v1 sign-off.

## Verification

Host: `lock_attention_tap` is gone; lock attention view remains.
Panther: do not flash; leave Сейчас.
