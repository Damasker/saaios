# SaaiOS Attention & Proactive Context — delivery roadmap

Status: **ATTN-00/01 host complete.** Phone: ATTN-02 rides VUI-04/05,
not a separate weekend. See [PIXEL-PATH.md](PIXEL-PATH.md).

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
| ATTN-02 | NOW «Требует внимания» uses projection | Backlog | **yes (VUI-05)** |
| ATTN-03 | Inbox uses same projection | Backlog | **yes** |
| ATTN-04 | Orb Attention uses same projection (WaitingConfirmation lights Orb) | **In progress** (host) | **yes (VUI-04)** |
| ATTN-05 | Context relevance (no AI) | Backlog | no |
| ATTN-06 | One World Model Health adapter | Backlog | after WORLD Health |
| ATTN-07 | One OAM suggested action | Backlog | **yes** |

## ATTN-01

**Goal:** `attention_projection(entities)` has the same inbox sources as
`inbox_rows()` for current fixtures.

**Change:** `crates/saai-attention`. Shell still calls `inbox_rows`.

**Test:** host unit tests in §209 Task/Notification rows.

**Rollback:** drop the crate.

**Threat:** none — no IPC, no phone, no new entity type.
