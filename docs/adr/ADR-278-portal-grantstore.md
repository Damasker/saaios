# ADR-278: Portal capabilities go through PolicyEngine; GrantStore stays

## Статус

Принято, 2026-09-21. AUTH-08 host. Not Visual v1 sign-off. PIN stays
null. Phone=no this slice (do not flash shell). ADR-023 native
Wayland clipboard is still a smithay gap.

## Нумерация

После ADR-277 следующий свободный номер — **278**. Не S33.

## Контекст

The S07 portal already gated `clipboard.read`/`write` against
`granted_capabilities` from `saai-appd`. AUTH-01…07 speak
AuthorityRequest. Two evaluators would drift. GrantStore is the
persistent app grant; PolicyEngine session grants must not replace it.

## Decision

1. **Adapter.** Portal builds `AuthorityRequest::app_capability`
   (Application + PeerCredentials + `AppCapabilityUse`).
   `PolicyEngine::decide_capability` Allow only when GrantStore names
   contain that capability.
2. **GrantStore stays** in `saai-appd`. The engine does not persist
   those names. An owner `grant_session` does not cover an app.
3. **Proof match.** Application cannot present LocalSystemSurface
   (AUTH-07). Unknown peer is still Deny before policy.
4. **Not ADR-023.** `wl_data_device_manager` is still ungated. This
   slice is the portal channel, already the only capability-checked
   clipboard.

## Consequences

- Portal and OAM share one engine vocabulary.
- AUTH-10 is the phone confirm/grant/reboot pass.
- Rollback: restore the `granted.iter().any` check in
  `portal_server`.

## Verification

Host: `cargo test -p saai-authority -p policy-engine -p saai-shell`
including `portal_grant_store_allows_clipboard_read`,
`owner_session_grant_does_not_cover_app_capability`,
`known_app_without_grant_is_denied`,
`authorized_clipboard_write_then_read_round_trips_real_text`.
No panther flash.
