# ADR-003: CBOR control protocol v1

## Status

Accepted

## Context

Control messages need a compact binary format with typed structures, binary blobs, and explicit versioning. JSON is convenient for LLM/debug edges but weak as an internal IPC format.

## Decision

- Internal control plane uses **CBOR** (`ciborium`).
- Every envelope includes `v: 1`.
- Unknown major versions are rejected.
- JSON may be used only at external/debug boundaries (OpenAI HTTP API, human logs).

Envelope fields:

- `v`
- `msg_id`
- `correlation_id`
- `causation_id`
- `kind`
- `payload`

## Consequences

- Stable evolution path via version field.
- Less temptation to grow an ad-hoc JSON dialect inside the runtime.
