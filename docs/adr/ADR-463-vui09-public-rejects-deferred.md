# ADR-463: VUI-09 — public subset rejects deferred names

## Статус

Принято, 2026-10-11. `compile_v2` already rejects `SpaceDetail` as
deferred (ADR-181). `MemoryReview`, `ChatThread`, and `Widget` were
on `sui_v2_deferred()` without a public-compile lock. Space detail
stays deferred. MEM-08 stays omitted. Public subset stays
Experimental. Leftover named chrome stays closed. Keyboard keys,
gallery page, and lock idle/wake stay formulas. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-462 следующий свободный номер — **463**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Known-limitations item 5 adds those four names only when a
shell-legal consumer exists. Inventing a public `SpaceDetail` or
Memory review chrome would skip that gate. `WidgetCard` stays
unknown, not deferred.

## Decision

1. **Host-lock** `compile_v2_public` rejects `SpaceDetail`,
   `MemoryReview`, `ChatThread`, and `Widget`. Stability stays
   Deferred.
2. **Public NOW / Spaces / object examples omit those names.**
3. **Not this slice.** Do not add them to the vocabulary. Do not
   promote Experimental → Stable. Do not check VUI-09 Acceptance.
   Do not convert ADR-232 leftovers. Do not start thin tuning.

## Consequences

- Deferred names fail the public gate. Rollback: drop the reject
  test. Still not Visual v1 sign-off.

## Verification

Host: public compile rejects the four deferred names. Panther: do
not flash; leave Сейчас.
