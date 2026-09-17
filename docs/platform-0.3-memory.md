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
| `memory.recall` | Low | Substring search within caller scope. Missing space = Global only. |

`memory.remember` / `memory.forget` are **not** model tools (MEM-05).
Explicit writes go through the console/IRAB wire ops with `space_id` or
`global=true`.

Recent records (up to 12) are projected into the model context as labelled
`<memory_records>` data — not as `Known facts`. Values are data, not
instructions.

## Console / UDS

```
/remember --space work ui.detail=technical
/recall --space work pi5
/forget --global host.role
m          # global tail only
```

Default write is not silently Global. Console:

```
/remember --space work ui.detail=technical
/remember --global host.role=pi5
/recall --space work ui
/recall --all
/forget --space work ui.detail
```

`m` (memory tail) is Global only, not all Spaces.
