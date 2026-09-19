# ADR-172: VUI-08 — FramePace commit log

## Статус

Принято, 2026-09-20. Each real main-surface commit records produce
time, input-to-commit, whether `wl_surface.frame` was requested,
pending-work depth, and dropped/coalesced counts. The last sample
is written to `/run/saaios/shell-frame.last`. Compositor timestamps
are not recorded (ADR-167). No new daemon. No CI threshold yet.
Do not tap Inbox rows. Leave Сейчас.

## Нумерация

После ADR-171 следующий свободный номер — **172**. Не S33.

## Контекст

S04 forbids idle `wl_surface.frame`. ADR-167–170 already request a
callback only while a clock needs a frame. VUI-08 still has no
measured sample of produce time or coalesced skips, so the 50 ms
drag p95 gate cannot be set honestly. Displayd presentation time
is untrusted.

## Decision

1. **`FramePace`** in `saai-ui-core` next to `MotionClock`. A 32-sample
   ring. `p95_produce_ms` sorts produce times. Host tests inject
   values; no wall clock.
2. **Shell records on commit**, not on the 16 ms dispatch. Scroll
   slot-busy is coalesced. SHM buffer-busy is dropped. `pending` is
   clocks plus a dirty scroll. Input-to-commit is taken from the
   latest `down()` and consumed on the first commit after it.
3. **Last line** at `/run/saaios/shell-frame.last`. Missing `/run`
   is a silent no-op (host tests). No CI fail this slice.

## Consequences

- A tab switch leaves a readable sample. Idle NOW does not grow a
  commit loop. Rollback: drop `FramePace` and the `/run` write.

## Verification

Host: p95 of a known set; coalesced/dropped counters; reason
input/motion/scroll. Panther: tap Входящие, `cat` the last line,
tap Сейчас. Do not open Inbox rows.
