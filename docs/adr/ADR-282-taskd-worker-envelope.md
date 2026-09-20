# ADR-282: Task confirm presents a worker DelegationEnvelope

## Статус

Принято, 2026-09-21. AUTH-06 wire. Not Visual v1 sign-off. PIN stays
null. Do not flash `saaios-runtime` or `saai-taskd`.

## Нумерация

После ADR-281 следующий свободный номер — **282**. Не S33.

## Контекст

ADR-276 typed `DelegationEnvelope` but `saai-taskd` still asked
runtime to confirm as the owner (`grant_once` + `decide`). A worker
Principal never presented the envelope. ADR-030 keeps the bridge on
wire JSON only.

## Decision

1. **Optional `execution_id` on `{"op":"confirm"}`.** Absent keeps
   the owner grant path (shell / console). Present is the Task id.
2. **Runtime.** `confirm_in_session` with an id issues
   `issue_worker_delegation` and `decide_worker`. It does not
   `grant_once` for the owner. OneShot is consumed. Session confirm
   keeps a Session envelope.
3. **taskd.** `execute_runtime_action` sends `task.id` as
   `execution_id`. Decline / cancel omit it.
4. **No flash.** Old runtime ignores unknown JSON. Old taskd omits
   the field. Host tests cover the new path.

## Consequences

- Owner confirmation no longer widens a session grant when a Task
  is the worker.
- Envelope is process-local; reboot still clears it.
- Rollback: drop `execution_id` and restore owner `grant_once`.

## Verification

Host: `cargo test -p policy-engine -p saai-taskd --offline`
including `worker_delegation_allows_once_without_owner_grant` and
`confirm_as_worker_sends_execution_id_on_the_wire`. No panther
flash.
