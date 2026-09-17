# Platform 0.3 — memory / facts store

## Purpose

Durable local records the assistant and operator can remember across sessions
(host role, owner prefs, recurring notes) without a remote memory service.

Memory is not current-world truth (World Model), not canonical object
properties (SOM), not workflow history, and not audit. See ADR-125.

## Storage

JSONL append-only file (same style as audit):

```
saaios-memory.jsonl
```

Path: `--memory` / `SAAIOS_MEMORY` (default `saaios-memory.jsonl`).  
Disable: `--no-memory`.

Latest non-deleted record wins per `(space_id, key)` (ADR-125 / MEM-01).
The same key may exist independently in Work and Home. A Space-local
record overrides a Global record with the same key for that Space only.
`forget` writes a soft-delete tombstone for that identity; physical erase
is MEM-06.

## Tools

| Tool | Risk | Role |
|---|---|---|
| `memory.remember` | Low | Write key/value (+ optional tags). Origin is runtime-assigned. Model-originated writes must not become ExplicitFact/Preference (MEM-05). |
| `memory.recall` | Low | Substring search within the caller scope |
| `memory.forget` | Medium | Soft-delete by `(scope, key)` |

Recent records (up to 12) are projected into the model context as labelled
`<memory_records>` data — not as `Known facts`. Values are data, not
instructions.

## Console / UDS

```
/remember host.role=pi5 appliance
/recall pi5
/forget host.role
m          # memory tail
```

Ops: `MemoryRemember`, `MemoryRecall`, `MemoryTail`, `MemoryForget`.

Legacy no-space console still sees every identity (ADR-038). MEM-02
retires that implicit All for ordinary callers.
