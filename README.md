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

### Model providers

```bash
# Mock (default demo)
just run-mock

# Local Ollama (OpenAI-compat on :11434)
export SAAIOS_LOCAL_MODEL=llama3.2
just run-local

# Auto: local → remote → mock
just run-auto

# Remote only
export SAAIOS_API_BASE=https://api.openai.com/v1
export SAAIOS_API_KEY=...
export SAAIOS_MODEL=gpt-4o-mini
just run
```

### Config file (optional)

```bash
cp saaios.example.toml saaios.toml
# edit provider / paths / telemetry
just run-config
# or: cargo run -p saaios-runtime -- --config saaios.toml
```

## Workspace

| Crate | Role |
|---|---|
| `protocol` | CBOR message envelope v1 |
| `audit-log` | Append-only audit trail |
| `tool-registry` | Typed tools |
| `policy-engine` | Allow / Deny / AskUser |
| `system-tools` | Linux + mock adapters |
| `model-provider` | Mock + local (Ollama) + remote + auto fallback |
| `memory-store` | Local JSONL facts + memory.* tools |
| `ai-runtime` | Request orchestration |
| `automation-engine` | Event-driven rules |
| `telemetry` | Periodic metrics → event bus |
| `config` | TOML settings + CLI/env merge |
| `console-tui` | First UI |
| `saaios-runtime` | Composition root / UDS server |

## Non-goals for 0.1

Own kernel, shared-memory transport, voice, GUI desktop, app store, personality memory.

Local inference (via Ollama), A/B `BOOT_OK` updates, and local memory facts landed in Platform 0.3.

## Docs

- [ADR-001: Platform is not the kernel](docs/adr/ADR-001-platform-not-kernel.md)
- [ADR-002: Hybrid IPC](docs/adr/ADR-002-hybrid-ipc.md)
- [ADR-003: CBOR control protocol](docs/adr/ADR-003-cbor-control-protocol.md)
- [Platform 0.1 status](docs/PLATFORM-0.1.md)
- [Platform 0.2 automation & budgets](docs/platform-0.2.md)
- [Platform 0.3 local models & A/B](docs/platform-0.3.md)
- [Platform 0.3 memory / facts](docs/platform-0.3-memory.md)
- [Platform 0.4 telemetry & status](docs/platform-0.4.md)
- [Platform 0.5 TOML config](docs/platform-0.5-config.md)
- [Platform 0.5 auto-diagnose](docs/platform-0.5-auto-diagnose.md)
- [Platform 0.5 richer tools](docs/platform-0.5-tools.md)
- [Platform 0.5 streaming / multi-turn](docs/platform-0.5-streaming.md)
- [Platform 0.5 appliance packaging](docs/platform-0.5-appliance.md)
- [Platform 0.6 Stage GUI](docs/platform-0.6-gui.md)
- [Raspberry Pi / Linux appliance](docs/appliance.md)

### Real Linux tools

```bash
just run-linux   # runtime with /proc adapters + mock model
just e2e-linux   # smoke test against live /proc
just audit-replay <correlation-id>
```
