# SaaiOS Unified Authority Model — delivery roadmap

Status: **AUTH-00/01 host complete.** Next valuable phone work is AUTH-04
(`decide_named` live grants) in the service queue, not AUTH-10 as a new
UI. Confirmation already exists (`TaskConfirm`). See [PIXEL-PATH.md](PIXEL-PATH.md).

Architecture: [ADR-124](../../adr/ADR-124-unified-authority-model.md)

Independent security track. Not S33. Does not rewrite ADR-020 / 074 / 084.

## Outcome

Every action answers: who, what, which object, which scope, how long, why.
No ambient authority. No new policyd.

## Delivery

| ID | Result | State | Phone? |
|---|---|---|---|
| AUTH-00 | ADR-124 + this roadmap | **Done** | no |
| AUTH-01 | Principal, AuthorityRequest, scope, reason codes | **Done** (host) | no |
| AUTH-02 | PolicyEngine adapter (same verdicts) | Backlog | no |
| AUTH-03 | Scoped session grants (not tool-name HashSet) | Backlog | no |
| AUTH-04 | Fix `decide_named` live grants; confirmation binding | Backlog | no |
| AUTH-05 | OAM/IRAB Principal on AuthorityRequest | Backlog | no |
| AUTH-06 | Worker DelegationEnvelope | Backlog | no |
| AUTH-07 | Automation Principal | Backlog | no |
| AUTH-08 | Portal adapter; GrantStore stays | Backlog | **yes** |
| AUTH-09 | Revocation review | Backlog | **yes** |
| AUTH-10 | Pixel 7: one-shot confirm, scoped grant, reboot, SSH regression | Backlog | **yes** |

## AUTH-01

**Goal:** typed Principal / request / scope / reason. No runtime change.

**Test:** host unit tests (identity kinds, exact-object scope, one-shot
binding differs when target/args change, hard-deny is not a grant).

**Rollback:** drop `crates/saai-authority`.

**Threat:** none — unused types until AUTH-02.
