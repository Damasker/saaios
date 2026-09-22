# SaaiOS Memory, Learning & Provenance — delivery roadmap

Status: **MEM-00/01/02/05 host complete.** Next phone step: flash
`saaios-runtime` (P0 in [PIXEL-PATH.md](PIXEL-PATH.md)) with scoped
identity, `None≠All`, and no model remember/forget. Review UI rides
VUI-06. Learning stays after typed record + erase.

Architecture: [ADR-125](../../adr/ADR-125-memory-learning-provenance-v1.md)

Independent memory track. Not S33. Extends ADR-038; does not migrate
Memory into `saai-entity-store`. Does not create `saai-memoryd`.

## Outcome

Typed, scoped, provenance-aware memory. Same key may exist in different
Spaces. Model inference cannot become ExplicitFact / ExplicitPreference.
User can inspect, correct, and erase without AI.

## Delivery

| ID | Result | State | Phone? |
|---|---|---|---|
| MEM-00 | ADR-125 + this roadmap | **Done** | no |
| MEM-01 | `(space_id, key)` compaction; labelled context; Work/Home same-key tests | **Done** (host) | flash runtime |
| MEM-02 | `None` is not All; explicit `MemoryAccessScope::All` | **Done** (host) | with P0 flash |
| MEM-03 | `MemoryRecord` v2 + legacy JSONL parser; provenance server-assigned | Backlog | no |
| MEM-04 | `MemoryContextProjection`; kind labels; sensitivity filter | Backlog | no |
| MEM-05 | Explicit remember/correct; model cannot write Explicit* | **Done** (host: model has no remember/forget tools) | with P0 flash |
| MEM-06 | Invalidate vs Erase; atomic JSONL rewrite | Backlog | no |
| MEM-07 | UAM on memory mutation (reuse ADR-124 Principal) | Backlog | no |
| MEM-08 | Manual review surface without AI | Backlog | **yes** (rides VUI-06; ADR-126 omitted the row — no shell-legal store read) |
| MEM-09 | LearnedHypothesis + one bounded pattern; no profiling | Backlog | no |
| MEM-10 | Pixel 7: scope, preference, correction, model isolation, erase, AI-off | Backlog | **yes** |

## MEM-01

**Goal:** stop cross-Space key collision. Stop calling memory "Known
facts" in the system prompt. Do not start Learning.

**Test:** host — Work `foo=A` and Home `foo=B` coexist either write
order; Global + Space same key: Space wins in that Space, Global remains
elsewhere; `format_context` has no `Known facts` heading; prompt-like
value stays data.

**Rollback:** revert `crates/memory-store` compaction and
`format_context`; ADR-038 `visible_to(None)=all` unchanged.

**Threat:** console `space_id=None` still sees every identity (MEM-02).
`memory.remember` tool still exists for the model (MEM-05).
