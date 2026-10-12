# SaaiOS Attention & Proactive Context — delivery roadmap

Status: **ATTN-00/01/02/03/04 host complete. ATTN-02/03/04 shell прошит. ATTN-05 host ContextFrame relevance (ADR-506). ATTN-06 host (ADR-291 + shell wire ADR-293) + panther `4dc19018…` (ADR-308). ATTN-07 one OAM suggestion host (ADR-508).**
Phone: ATTN-02/03 ride VUI-05; ATTN-04 rides VUI-04. See [PIXEL-PATH.md](PIXEL-PATH.md).

Architecture: [ADR-123](../../adr/ADR-123-attention-projection.md)

Independent track. Not S33. Does not rewrite ADR-062 / 079 / 112 / 115 / 116.

## Outcome

One deterministic Attention Projection feeds NOW, Inbox and Orb.
No second notification subsystem. No attention database.

## Delivery

| ID | Result | State | Phone? |
|---|---|---|---|
| ATTN-00 | ADR-123 + this roadmap | **Done** | no |
| ATTN-01 | Pure crate: Task + Notification candidates; inbox parity tests | **Done** (host) | no |
| ATTN-02 | NOW «Требует внимания» uses projection | **Done** (host) | **yes (VUI-05)** |
| ATTN-03 | Inbox uses same projection | **Done** (host + panther) | **yes** |
| ATTN-04 | Orb Attention uses same projection (WaitingConfirmation lights Orb) | **Done** (host + live ContextFrame ADR-516) | **yes (VUI-04)** |
| ATTN-05 | Context relevance (no AI) | **Done** (host, ADR-506 + live `object_id` ADR-513 + ObjectRef ADR-514) | no |
| ATTN-06 | One World Model Health adapter | **Done** (host ADR-291 + shell wire ADR-293; panther `4dc19018…`) | **yes** |
| ATTN-07 | One OAM suggested action | **Host** (ADR-508) + live catalog **на panther** (ADR-511; `0fb762d1…`) | **yes** |

## ATTN-01

**Goal:** `attention_projection(entities)` has the same inbox sources as
`inbox_rows()` for current fixtures.

**Change:** `crates/saai-attention`. Shell still calls `inbox_rows`.

**Test:** host unit tests in §209 Task/Notification rows.

**Rollback:** drop the crate.

**Threat:** none — no IPC, no phone, no new entity type.

## ATTN-02

**Goal:** NOW «Требует внимания» is `now_items()` from the same projection
Orb already reads. Not a second `inbox_rows` filter.

**Change:** `now_attention_section` in `saai-shell`. Inbox follows in ATTN-03.

**Test:** waiting-confirmation + notification rows; running/dismissed
omitted; `surfaces.now = false` omitted.

**Rollback:** restore the `inbox_rows` loop in `now_sections`.

## ATTN-03

**Goal:** Inbox membership and order are `inbox_source_ids()` from the
same projection. Hit-test and cards cannot drift.

**Change:** `inbox_rows` is a lookup over the projection, not a second
Task/Notification filter.

**Test:** `inbox_rows_follow_the_attention_projection`.

**Rollback:** restore the local Task-then-Notification filters.

## ATTN-05

**Goal:** Attention knows the current ContextFrame. No AI rank.

**Change:** `AttentionContext` + `project_with_context`. Other-space
items are Background: Inbox keeps them, NOW/Orb omit them. Focused
object is CurrentObject. Health stays Global. Inbox order stays
ATTN-03.

**Test:** host `cargo test -p saai-attention` and the shell
`now_paint_omits_background_space_attention` /
`now_section_list_uses_the_same_context_as_paint` cases
(ADR-509). Live NOW object is `object_id` (ADR-513). CurrentObject
carries `ObjectRef` (ADR-514).

**Rollback:** drop context; restore Global-only `project_with_health`.

**Threat:** none — host projection, no phone binary.

## ATTN-06

**Goal:** one Health report can become Attention. Healthy is not news.

**Change:** `project_with_health`. Unknown/Healthy omitted. Degraded is
NOW only. Unhealthy lights Orb. Inbox stays Task/Notification.
Shell parses `status.health` and calls `project_with_health` (ADR-293).
Do not flash shell.

**Test:** host `cargo test -p saai-attention` and `cargo test -p saai-shell --offline live_health`.

**Rollback:** drop `AttentionSource::Health`.

**Threat:** none — host adapter, no phone binary.

## ATTN-07

**Goal:** at most one OAM verb on NOW. No invented labels.

**Change:** `one_suggested_action` + `project_with_suggestion`.
Exactly one permitted candidate. Inbox/Orb omit it. Tap opens the
object. No execution. ADR-511 catalogs live `system.identity` so
`display.inspect` can resolve. Panther `0fb762d1…` under dest-no-lock.
ADR-512: no swarm/`agentd` in this slice; that is deferral, not a
ban on Advisor or later distributed attention. ADR-513 fills live
`object_id` from the NOW ObjectSummary entity. ADR-515: NOW tap follows
that `ObjectRef` when present. ADR-522: unknown OAM reason is
«недоступно», not the English Policy string.

**Test:** host `cargo test -p saai-attention` and
`attn07_empty_tools_are_not_a_suggested_action` /
`attn07_live_identity_is_one_suggested_action` /
`now_attention_follow_prefers_current_object_ref`.

**Rollback:** drop the suggestion types.

**Threat:** none — host projection, no phone binary.
