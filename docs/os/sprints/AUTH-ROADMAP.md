# SaaiOS Unified Authority Model — delivery roadmap

Status: **AUTH-00/01 host complete. AUTH-02 adapter host (ADR-260). AUTH-03 scoped grants host (ADR-261). AUTH-04 live `decide_named` host. AUTH-05 OAM/IRAB Principal host (ADR-275). AUTH-06 DelegationEnvelope host (ADR-276).**
Next phone work is still AUTH-08/10 in the service queue, not AUTH-10 as a new
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
| AUTH-02 | PolicyEngine adapter (same verdicts) | **Done** (host, ADR-260) | no |
| AUTH-03 | Scoped session grants (not tool-name HashSet) | **Done** (host, ADR-261) | no |
| AUTH-04 | Fix `decide_named` live grants; confirmation binding | **Done** (host) | no |
| AUTH-05 | OAM/IRAB Principal on AuthorityRequest | **Done** (host, ADR-275) | no |
| AUTH-06 | Worker DelegationEnvelope | **Done** (host, ADR-276) | no |
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

## AUTH-02

**Goal:** AuthorityRequest hits the live PolicyEngine. Verdicts match
`decide_named` for the same spec and args.

**Change:** `PolicyEngine::decide_request`. Unverified is Deny. No new
daemon. ToolSpec risk stays trusted metadata.

**Test:** `adapter_matches_decide_named_verdicts`; unverified metrics
Deny; format still hard-denied.

**Rollback:** drop `decide_request`.

**Threat:** none — host adapter, no phone binary.

## AUTH-03

**Goal:** a session grant names who / what / which object / how long.

**Change:** `SessionGrant` + `grant_covers`. `grant_session(tool)` is
owner+Any+Session. Hard deny and Persistent are refused. OneShot is
consumed. Reboot still clears the process-local list.

**Test:** worker does not inherit owner; exact object does not leak;
expired Until asks; OneShot is single-use.

**Rollback:** restore `HashSet<String>`.

**Threat:** none — in-process only.

## AUTH-04

**Goal:** `decide_named` uses the live `PolicyEngine` (session grants
visible). One-shot confirm is bound to call/tool/canonical args.

**Change:** instance `decide_named`; `note_pending` / `take_bound_pending`;
runtime confirm consumes the binding. Scoped grants are AUTH-03
(`SessionGrant`), not a later HashSet patch.

**Test:** grant then `decide_named` Allows; a fresh engine still Asks;
mutated pid is rejected; key order does not break the bind.

**Rollback:** revert `policy-engine` + `ai-runtime` confirm gate.

**Threat:** forged confirm with different args no longer executes.

## AUTH-05

**Goal:** OAM/IRAB name Principal + semantic action + target on
`AuthorityRequest`. Policy uses bound ToolSpec risk even when the
action id is not the tool name.

**Change:** `decide_request` keeps scoped Principal when spec.name
differs. OAM `preflight` / `execute_if_allowed` call `decide_request`.
IRAB Direct builds the request and does not evaluate policy.

**Test:** inspect is `display.inspect` not `system.identity`;
unverified Deny; worker does not inherit owner grant.

**Rollback:** restore `decide_named` in OAM.

**Threat:** none — host adapter, no phone binary.

## AUTH-06

**Goal:** a worker executes a confirmed action only through a bound
`DelegationEnvelope`. Not a second GrantStore.

**Change:** `DelegationEnvelope` + `envelope_covers`.
`PolicyEngine::issue_delegation` is process-local. OneShot is consumed.
Hard deny and Persistent are refused. `saai-taskd` is not wired yet.

**Test:** matching worker Allow once; other worker AskUser; changed
args/target fail; owner cannot be covered as a worker.

**Rollback:** drop `delegations`.

**Threat:** none — host adapter, no phone binary.
