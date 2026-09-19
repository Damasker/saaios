# ADR-176: VUI-08 — first visible touch is the down commit

## Статус

Принято, 2026-09-20. `Pressed` paints on the same main-surface commit
as `down()`. Motion tokens hold after release; they do not delay the
first frame. `FramePace` remembers the last non-scroll
`input_to_commit_ms` and compares it to 50 ms. No new daemon. Do not
tap Inbox rows. Leave Сейчас.

## Нумерация

После ADR-175 следующий свободный номер — **176**. Не S33.

## Контекст

VUI-07 already painted `ColorRole::Pressed` from `down()`. VUI-08
clocks (120 / 180 / 240) hold that state after a short tap so it
stays visible. Acceptance still needs a measured first-visible
commit, not a panel slide. Scroll coalescing is a different number
(ADR-173). Presentation timestamps stay omitted.

## Decision

1. **First visible is the `down()` commit.** Finger-down `Pressed` is
   not a clock. Micro / Selection / Context start on that same tap
   only to keep the state after `up()`. No panel translation.
2. **`FIRST_FEEDBACK_LIMIT_MS` is 50.** `record` keeps
   `last_feedback_ms` from a non-scroll sample that carries
   `input_to_commit_ms`. Later clock frames with `input_ms=-` do not
   clear it. `first_feedback_within_limit` is `None` until that
   sample exists.
3. **Last line** appends `input_ok` (`1` / `0` / `-`). The live shell
   does not abort.

## Consequences

- A tab or key tap leaves a readable `input_ok` after Selection /
  MicroFeedback expire. A Система drag does not overwrite it.
  Rollback: drop `last_feedback_ms` and the field.

## Verification

Host: 20 ms motion input passes; 80 ms fails; scroll 100 ms is ignored.
Panther: tap Входящие, `cat /run/saaios/shell-frame.last` has
`input_ok=1`, tap Сейчас.
