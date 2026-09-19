# ADR-173: VUI-08 — 50 ms scroll p95

## Статус

Принято, 2026-09-20. Scroll-only `p95_produce_ms` is compared to
`FRAME_PACE_P95_LIMIT_MS` (50). Host tests inject values; the shell
does not abort on device. The last line adds `p95_scroll` / `p95_ok`.
No new daemon. Do not tap Система rows. Leave Сейчас.

## Нумерация

После ADR-172 следующий свободный номер — **173**. Не S33.

## Контекст

ADR-172 records every main-surface commit. Tab produce times on
panther were 6–7 ms, which would hide a slow drag if p95 mixed
reasons. VUI-08 acceptance is continuous **drag** p95 ≤ 50 ms.

## Decision

1. **`p95_produce_ms_for(Scroll)`** on the same 32-sample ring.
   `scroll_p95_within_limit` is `None` with no scroll samples, `Some(true)`
   at or under 50 ms, `Some(false)` above. CI asserts both injected
   sides. The live shell only writes the line.
2. **Last line** keeps ADR-172 fields and appends `p95_scroll` and
   `p95_ok` (`-` when no scroll sample). Presentation time stays omitted.
3. **Panther** opens Система, drags the list, `cat`s the line, returns
   to Сейчас. No settings row. No 7-tap.

## Consequences

- A tab switch without a drag leaves `p95_scroll=-`. A Система drag
  leaves a number that can be checked against 50. Rollback: drop the
  two fields and the limit constant.

## Verification

Host: 20 scroll samples at 10/40 ms pass; 10/80 ms fail the helper.
Panther: tap Система, swipe, `cat /run/saaios/shell-frame.last`, tap
Сейчас.
