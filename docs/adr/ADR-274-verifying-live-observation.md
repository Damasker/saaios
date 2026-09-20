# ADR-274: Verifying reads Fresh Observation from runtime status

## Статус

Принято, 2026-09-21. Host-only. Not Visual v1 sign-off. Phone=no
(do not flash `saai-taskd`). No `saai-deviced`.

## Нумерация

После ADR-273 следующий свободный номер — **274**. Не S33.

## Контекст

WORK-03 left Tasks in `Verifying` until `decide_verification` saw
Fresh evidence. `advance_task_after_result` passed `None`, so nothing
ever settled. WORLD-05 is that feed. ObservationCache lives in
`saaios-runtime`. A second daemon is not justified: shell already
reads the same `{"op":"status"}` blob. ADR-030 keeps the bridge on
wire JSON.

## Decision

1. **Same status blob.** `runtime_bridge::status` reads Fresh
   `observations` from `{"op":"status"}` (ADR-246 already omits Stale).
2. **Listed key is Fresh evidence.** Missing key stays Verifying.
   Wrong value is Failed. Unreachable runtime is missing evidence,
   not Failed.
3. **Settle later.** Schedule tick and boot reconcile retry Verifying
   Tasks when a matching row appears. No contract still never Done.
4. **Host-only this slice.** Do not flash `saai-taskd` (diagnose
   without `verification_key` would stay Verifying on device).

## Consequences

- A Task with `verification_key` can become Done from live telemetry.
- WORLD-03 (`saai-deviced`) is still not required.
- Rollback: pass `None` into `status_after_verification` again.

## Verification

Host: `status_fresh_row_is_live_evidence`,
`status_omits_stale_so_missing_row_stays_verifying`,
`status_parses_fresh_observation_rows`. No panther flash. Leave Сейчас.
