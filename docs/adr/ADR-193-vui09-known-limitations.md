# ADR-193: VUI-09 — known limitations, not Visual v1 sign-off

## Статус

Принято, 2026-09-20. HEAD shell stays `e8865301…`. Known limitations
and the Visual v2 backlog are recorded in
`docs/os/ui/vui09-known-limitations.md`. The ledger stays in
progress. Lock cycle, display restart, and cold boot stay open.
`compile()` stays `sui 1`. No new daemon. Leave Сейчас.

## Нумерация

После ADR-192 следующий свободный номер — **193**. Не S33.

## Контекст

VUI-09 asked to run remaining live cells then close the ledger.
Lock, displayd kill, and cold boot are session-blocked: the
no-lock marker stays, displayd stays, `/run` is not dropped.
Pretending those cells are proven would fake Visual v1. Leaving
the gaps unnamed would hide why `compile_v2()` still cannot own
chrome.

## Decision

1. **Record limitations. Do not sign off.** Open cells stay open.
2. **Do not reflash, lock, reboot, or kill displayd.**
3. **Visual v2 starts at hit-test equivalence**, not at a new
   vocabulary pass.

## Consequences

- Visual v1 acceptance checkboxes stay unchecked. Rollback: delete
  the limitations page. Next: operator-approved lock/display/cold-boot,
  or Visual v2 layout emission. Not `compile_v2()` in `build.rs`.

## Verification

Host: compiler reads the limitations page. Panther: Сейчас on HEAD;
leave Сейчас.
