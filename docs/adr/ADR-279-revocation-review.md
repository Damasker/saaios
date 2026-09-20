# ADR-279: Revocation is an API, not a Me chrome

## Статус

Принято, 2026-09-21. AUTH-09 host. Not Visual v1 sign-off. PIN stays
null. Phone=no this slice. Do not flash shell. Do not tap Отозвать.

## Нумерация

После ADR-278 следующий свободный номер — **279**. Не S33.

## Контекст

AUTH-03 session grants and AUTH-06 envelopes die on reboot but had
no live revoke. GrantStore could decline (`granted = []` with the
same `requested_hash`) which still `covers()` — the next launch
does not re-ask. Me `CapabilityRow` is Static (ADR-208): there is
still no revoke hit on Система. SSH `authorized_keys` stays itself.

## Decision

1. **GrantStore.revoke** unlinks the app's JSON and fsyncs the
   directory. Next launch re-asks. Decline is not revoke.
2. **PolicyEngine.revoke_session** drops matching live grants.
   **revoke_delegations** drops that worker's envelopes.
3. **No chrome.** `CapabilityRow` stays Static. Trusted-client
   `revoke_trusted_client` is already privileged and is not this
   slice. `authorized_keys` is not GrantStore.
4. **Reboot still clears** process-local grants and envelopes.
   Durable app grants need an explicit revoke.

## Consequences

- Portal `decide_capability` sees empty GrantStore names after
  appd reloads the list. AUTH-10 still owns the phone confirm
  pass. Rollback: drop `revoke` / `revoke_session` /
  `revoke_delegations`.

## Verification

Host: `cargo test -p saai-appd -p policy-engine` including
`revoke_drops_coverage_so_the_next_launch_reasks`,
`decline_is_not_revoke`, `revoke_session_drops_the_owner_grant`,
`revoke_delegations_drops_the_worker_envelope`. No panther flash.
No tap on Отозвать / CapabilityRow.
