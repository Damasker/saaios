# ADR-002: Hybrid async IPC

## Status

Accepted

## Context

Synchronous rendezvous IPC (L4-style) is a poor fit for LLM token streaming and unpredictable tool I/O. Blocking AI Runtime on every transfer risks thread-pool exhaustion.

## Decision

Use a hybrid model:

1. **Control plane:** asynchronous message channels for RPC, events, and capability/handle transfer.
2. **Data plane (later):** shared-memory ring buffers for token streams, logs, and large payloads.
3. **Event bus:** userspace daemon with classed queues (critical / lossy / session).

### Platform 0.1 mapping (Linux)

| Mechanism | Implementation |
|---|---|
| Message channels | Unix Domain Sockets |
| Handles | deferred (single-process runtime composition is OK in 0.1) |
| Shared memory | not in 0.1 |
| Event notifications | framed CBOR messages over UDS |

## Consequences

- AI Runtime stays non-blocking for control messages.
- The same control/data split can later map onto a native kernel.
- 0.1 prioritizes correct tool/policy/audit flow over zero-copy performance.
