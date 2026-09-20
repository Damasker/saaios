# ADR-276: Worker acts through a DelegationEnvelope

## Статус

Принято, 2026-09-21. AUTH-06 host. Not Visual v1 sign-off. PIN stays
null. Phone=no this slice. Do not flash `saai-taskd`.

## Нумерация

После ADR-275 следующий свободный номер — **276**. Не S33.

## Контекст

AUTH-05 lets OAM/IRAB name a Worker Principal. A worker still cannot
inherit the owner's session grant. High-risk tools AskUser forever
unless something bound is issued to that worker. A second GrantStore
or policyd would drift from ADR-124.

## Decision

1. **Envelope.** `DelegationEnvelope` is issuer + worker + execution
   id + semantic operation + target + canonical arguments + validity.
   Only LocalUser owner may issue. Worker cannot issue as owner.
2. **Cover.** `envelope_covers` requires DelegatedWorker proof with
   the same execution id, same operation/target/args. Key order does
   not break the bind. Persistent is not live.
3. **Engine.** `PolicyEngine::issue_delegation` is process-local like
   session grants. OneShot is consumed on Allow. Hard deny refuses
   issue. Reboot still clears the list.
4. **Not taskd this slice.** Workers in `saai-taskd` stay on the
   existing confirm path. Envelope is the type they will present later.

## Consequences

- Owner confirmation can mint a OneShot envelope instead of widening
  a session grant to every principal.
- AUTH-07 Automation Principal is still later. AUTH-08 GrantStore
  still later.
- Rollback: drop `delegations` and `issue_delegation`.

## Verification

Host: `cargo test -p saai-authority -p policy-engine -p saai-object-actions`
including `envelope_covers_the_bound_worker_request`,
`delegated_worker_allow_is_oneshot`, `delegated_worker_executes_once`.
No panther flash.
