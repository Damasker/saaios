# ADR-189: VUI-09 — verification ledger, not Visual v1 sign-off

## Статус

Принято, 2026-09-20. The cross-sprint matrix now has a ledger
(`docs/os/ui/vui09-verification.md`) that cites proven ADRs and names
the cells this slice does not re-run. Chrome unchanged. `compile()`
stays `sui 1`. No new daemon. Do not 7-tap. Do not lock. Leave Сейчас.

## Нумерация

После ADR-188 следующий свободный номер — **189**. Не S33.

## Контекст

VUI-09 asked for the full visual/a11y/interaction/perf/restart/
cold-boot/offline matrix before known limitations. Treating every
cell as unproven would ignore VUI-07/08 panther evidence. Pretending
the matrix is closed would hide that `compile_v2()` still has no
rectangles, cold boot was not run on HEAD, and nothing is Stable.

## Decision

1. **Ledger, not sign-off.** Each matrix area is `proven` / `host` /
   `open`. Open cells stay open: v2 hit-test equivalence, daylight
   booth, increased text on panther this session, lock/unlock cycle,
   display restart, cold boot, 7-tap gallery.
2. **Do not reflash.** HEAD chrome is ADR-188 (`e8865301…`). This
   slice only records and re-shots Сейчас.
3. **Do not kill displayd or reboot.** Marker `dev-no-lock` stays.
   PIN stays null.

## Consequences

- Visual v1 acceptance checkboxes stay unchecked. Rollback: delete
  the ledger. Next: remaining live cells, then known limitations.

## Verification

Host: compiler test reads the ledger. Panther: Сейчас + Inbox tab,
leave Сейчас, no reflash.
