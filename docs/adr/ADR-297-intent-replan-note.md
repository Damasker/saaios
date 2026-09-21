# ADR-297: Intent names a bounded replan, not a second plan queue

## Статус

Принято, 2026-09-21. WORK-07 chrome. Not a Retry button. PIN stays null.
Do not flash shell or taskd.

## Нумерация

После ADR-296 следующий свободный номер — **297**. Не S33.

## Контекст

ADR-289 stores `replan_count` on the Intent after one Planner pass
for verification mismatch. The Intent screen showed plan → progress
without that fact, so a rewritten DAG looked like the original plan.
Retry stays Failed timeout chrome (ADR-296). Cap stays 1 in taskd.

## Decision

1. **Fact.** `replan_count >= 1` appends «Перепланировано» to the
   Intent related line (after «План: …» when a DAG exists).
2. **Missing is none.** `0` or absent is omitted. No invented queue.
3. **No tap.** Do not add Перепланировать. Do not flash.

## Consequences

- Object View / Intent progress stay the same Tasks.
- Rollback: drop `intent_replan_note`.

## Verification

Host: `cargo test -p saai-shell --offline names_a_replan`. No panther flash.
Leave Сейчас.
