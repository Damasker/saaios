# Platform 0.3 — memory / facts store

## Purpose

Durable local facts the assistant and operator can remember across sessions
(host role, owner prefs, recurring culprits) without a remote memory service.

## Storage

JSONL append-only file (same style as audit):

```
saaios-memory.jsonl
```

Path: `--memory` / `SAAIOS_MEMORY` (default `saaios-memory.jsonl`).  
Disable: `--no-memory`.

Latest non-deleted fact wins per `key`. `forget` writes a soft-delete tombstone.

## Tools

| Tool | Risk | Role |
|---|---|---|
| `memory.remember` | Low | Write key/value (+ optional tags) |
| `memory.recall` | Low | Substring search |
| `memory.forget` | Medium | Soft-delete by key |

Recent facts (up to 12) are injected into the system prompt on every diagnose.

## Console / UDS

```
/remember host.role=pi5 appliance
/recall pi5
/forget host.role
m          # memory tail
```

Ops: `MemoryRemember`, `MemoryRecall`, `MemoryTail`, `MemoryForget`.
