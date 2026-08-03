# SaaiOS

AI-native userspace platform for a personal intelligent terminal.

**Platform Track (this repo)** runs on Linux first.  
**Kernel Track** is a separate research effort and is intentionally not a dependency of this product.

## Platform 0.1 goal

User asks: *Why is the system slow?*

SaaiOS:

1. Calls `system.metrics` and `process.list`
2. Explains the likely cause
3. Proposes a safe action
4. Asks for confirmation before dangerous tools
5. Writes a full audit trail

## Quick start

```bash
just test
just e2e
just run-mock
```

In another terminal:

```bash
just run-tui
```

### Real remote model (optional)

```bash
export SAAIOS_API_BASE=https://api.openai.com/v1
export SAAIOS_API_KEY=...
export SAAIOS_MODEL=gpt-4o-mini
just run
```

## Workspace

| Crate | Role |
|---|---|
| `protocol` | CBOR message envelope v1 |
| `audit-log` | Append-only audit trail |
| `tool-registry` | Typed tools |
| `policy-engine` | Allow / Deny / AskUser |
| `system-tools` | Linux + mock adapters |
| `model-provider` | Mock + OpenAI-compatible |
| `ai-runtime` | Request orchestration |
| `console-tui` | First UI |
| `saaios-runtime` | Composition root / UDS server |

## Non-goals for 0.1

Own kernel, shared-memory transport, local inference, voice, GUI desktop, app store, personality memory.

## Docs

- [ADR-001: Platform is not the kernel](docs/adr/ADR-001-platform-not-kernel.md)
- [ADR-002: Hybrid IPC](docs/adr/ADR-002-hybrid-ipc.md)
- [ADR-003: CBOR control protocol](docs/adr/ADR-003-cbor-control-protocol.md)
- [Platform 0.1 status](docs/PLATFORM-0.1.md)
- [Raspberry Pi / Linux appliance](docs/appliance.md)

### Real Linux tools

```bash
just run-linux   # runtime with /proc adapters + mock model
just e2e-linux   # smoke test against live /proc
just audit-replay <correlation-id>
```
