# ADR-213: VUI-09 — park thin-tuning work last

## Статус

Принято, 2026-09-20. Operator: Visual queue continues. Physical
lighting, leftover visual nits, gallery-fixture completeness, and
other panel-tuning work are not next. They belong to a last
**thin-tuning** sprint. `compile()` stays `sui 1`. No new daemon.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-212 следующий свободный номер — **213**. Не S33.

## Контекст

Runnable VUI-09 device cells are proven (ADR-209–212). The remaining
Visual work is hit-test equivalence: live Me flatten/scroll and list
trailing controls still procedural. Panel lighting, color balance,
leftover paint nits, and gallery-state completeness do not unblock
`compile_v2()` and do not make the phone more of a phone.

## Decision

1. **Thin-tuning is last.** Do not schedule or propose it as next
   work while remaining VUI-09 layout/hit-test work exists.
2. **Same bucket:** physical lighting/color/contrast review, leftover
   named-size nits, gallery fixture completeness, golden-panel polish.
3. **Not in that bucket:** Me flatten/scroll, trailing list hits,
   a later deliberate `build.rs` switch, Experimental→Stable after
   Visual v1, SpaceDetail/MEM-08.
4. **Do not reflash.** Chrome stays ADR-198.

## Consequences

- Next Visual slice is declarative ≡ procedural. Thin-tuning stays
  parked. Rollback: restore it as an open next cell.

## Verification

Host: ledger and limitations name `thin tuning` and ADR-213. Panther:
chrome unchanged; leave Сейчас.
